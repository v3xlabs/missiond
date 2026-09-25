use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{watch, RwLock};
use tracing::{info, warn};

use super::{DeviceDocument, Dirs, Document, Documents, Persisted, Playlist, Tab};

/// The config directory is the source of truth. When Nix generates it, the directory is a store
/// path and every write fails, so the store reports itself as read-only and the API applies
/// changes to memory alone.
pub struct ConfigStore {
    pub dirs: Dirs,
    read_only: bool,
    inner: RwLock<Documents>,
    /// Bumped by every mutation, so a reader can wait for configuration to change.
    revision: watch::Sender<u64>,
}

impl ConfigStore {
    pub fn load(dirs: Dirs) -> Result<Self> {
        let read_only = read_only_requested() || !is_writable(&dirs.config);

        Self::with_mode(dirs, read_only)
    }

    pub fn with_mode(dirs: Dirs, read_only: bool) -> Result<Self> {
        let config = Documents {
            device: read_document(&dirs.config, Document::Device)?,
            display: read_document(&dirs.config, Document::Display)?,
            tabs: read_document(&dirs.config, Document::Tabs)?,
            playlists: read_document(&dirs.config, Document::Playlists)?,
            notifications: read_document(&dirs.config, Document::Notifications)?,
            calendars: read_document(&dirs.config, Document::Calendars)?,
        };

        info!(
            path = %dirs.config.display(),
            read_only,
            tabs = config.tabs.tabs.len(),
            playlists = config.playlists.playlists.len(),
            "loaded configuration"
        );

        Ok(Self {
            dirs,
            read_only,
            inner: RwLock::new(config),
            revision: watch::channel(0).0,
        })
    }

    pub const fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub async fn read(&self) -> Documents {
        self.inner.read().await.clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }

    pub async fn device(&self) -> DeviceDocument {
        self.inner.read().await.device.clone()
    }

    pub async fn tabs(&self) -> Vec<Tab> {
        self.inner.read().await.tabs.tabs.clone()
    }

    pub async fn tab(&self, tab_id: &str) -> Option<Tab> {
        self.inner
            .read()
            .await
            .tabs
            .tabs
            .iter()
            .find(|tab| tab.tab_id == tab_id)
            .cloned()
    }

    pub async fn playlists(&self) -> Vec<Playlist> {
        self.inner.read().await.playlists.playlists.clone()
    }

    pub async fn playlist(&self, playlist_id: &str) -> Option<Playlist> {
        self.inner
            .read()
            .await
            .playlists
            .playlists
            .iter()
            .find(|playlist| playlist.playlist_id == playlist_id)
            .cloned()
    }

    pub async fn playlist_tabs(&self, playlist_id: &str) -> Vec<Tab> {
        let config = self.inner.read().await;
        let Some(playlist) = config
            .playlists
            .playlists
            .iter()
            .find(|playlist| playlist.playlist_id == playlist_id)
        else {
            return Vec::new();
        };

        playlist
            .enabled_tabs()
            .filter_map(|tab_id| {
                config
                    .tabs
                    .tabs
                    .iter()
                    .find(|tab| &tab.tab_id == tab_id)
                    .cloned()
            })
            .collect()
    }

    pub async fn mutate<F>(&self, document: Document, change: F) -> Result<Persisted>
    where
        F: FnOnce(&mut Documents),
    {
        let mut config = self.inner.write().await;

        change(&mut config);
        self.revision.send_modify(|revision| *revision += 1);

        if self.read_only {
            return Ok(Persisted::MemoryOnly);
        }

        match document {
            Document::Device => write_document(&self.dirs.config, document, &config.device),
            Document::Display => write_document(&self.dirs.config, document, &config.display),
            Document::Tabs => write_document(&self.dirs.config, document, &config.tabs),
            Document::Playlists => write_document(&self.dirs.config, document, &config.playlists),
            Document::Notifications => {
                write_document(&self.dirs.config, document, &config.notifications)
            }
            Document::Calendars => write_document(&self.dirs.config, document, &config.calendars),
        }?;

        Ok(Persisted::ToDisk)
    }

    pub async fn export(&self) -> Result<String> {
        let config = self.inner.read().await;
        let mut device = config.device.clone();

        device.admin_key = device
            .admin_key
            .as_ref()
            .map(|key| key.export("MISSIOND_ADMIN_KEY"));
        device.control_key = device
            .control_key
            .as_ref()
            .map(|key| key.export("MISSIOND_CONTROL_KEY"));

        if let Some(hass) = device.homeassistant.as_mut() {
            hass.password = hass
                .password
                .as_ref()
                .map(|password| password.export("MISSIOND_MQTT_PASSWORD"));
        }

        // A calendar's address is the credential for it, so an export carries the reference
        // rather than the link, the same way the admin key does.
        let mut calendars = config.calendars.clone();

        for calendar in &mut calendars.calendars {
            let placeholder = format!(
                "MISSIOND_CALENDAR_{}",
                calendar.calendar_id.to_uppercase().replace('-', "_")
            );

            calendar.url = calendar.url.export(&placeholder);
        }

        let mut notifications = config.notifications.clone();

        for (name, webhook) in &mut notifications.webhooks {
            let placeholder = format!("MISSIOND_WEBHOOK_{}", name.to_uppercase().replace('-', "_"));

            webhook.token = webhook
                .token
                .as_ref()
                .map(|token| token.export(&placeholder));
        }

        Ok([
            format!("# device.toml\n{}", toml::to_string_pretty(&device)?),
            format!(
                "# display.toml\n{}",
                toml::to_string_pretty(&config.display)?
            ),
            format!("# tabs.toml\n{}", toml::to_string_pretty(&config.tabs)?),
            format!(
                "# playlists.toml\n{}",
                toml::to_string_pretty(&config.playlists)?
            ),
            format!(
                "# notifications.toml\n{}",
                toml::to_string_pretty(&notifications)?
            ),
            format!("# calendars.toml\n{}", toml::to_string_pretty(&calendars)?),
        ]
        .join("\n"))
    }
}

fn read_only_requested() -> bool {
    std::env::var("MISSIOND_CONFIG_READ_ONLY")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn is_writable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => !metadata.permissions().readonly(),
        // A directory that does not exist yet is one the daemon will create and own.
        Err(_) => true,
    }
}

fn read_document<T: DeserializeOwned + Default>(base: &Path, document: Document) -> Result<T> {
    let path = base.join(document.file_name());

    match std::fs::read_to_string(&path) {
        Ok(body) => {
            toml::from_str(&body).with_context(|| format!("cannot parse {}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            warn!(path = %path.display(), "no such document, using defaults");
            Ok(T::default())
        }
        Err(error) => Err(error).with_context(|| format!("cannot read {}", path.display())),
    }
}

fn write_document<T: Serialize>(base: &Path, document: Document, value: &T) -> Result<()> {
    std::fs::create_dir_all(base)?;

    let path = base.join(document.file_name());
    let temporary: PathBuf = path.with_extension("toml.tmp");

    std::fs::write(&temporary, toml::to_string_pretty(value)?)
        .with_context(|| format!("cannot write {}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .with_context(|| format!("cannot replace {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{super::Source, *};

    fn store_over(dir: &Path) -> ConfigStore {
        store_in_mode(dir, false)
    }

    /// The mode is passed rather than read from the environment, so tests running in parallel
    /// cannot see each other's read-only setting.
    fn store_in_mode(dir: &Path, read_only: bool) -> ConfigStore {
        ConfigStore::with_mode(
            Dirs {
                config: dir.to_path_buf(),
                state: dir.join("state"),
                cache: dir.join("cache"),
            },
            read_only,
        )
        .unwrap()
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("missiond-test-{name}"));

        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        dir
    }

    #[tokio::test]
    async fn a_missing_config_directory_loads_defaults() {
        let store = store_over(&temp_dir("missing").join("nested"));

        assert!(store.tabs().await.is_empty());
        assert!(store.playlists().await.is_empty());
    }

    #[tokio::test]
    async fn reordering_a_playlist_survives_a_reload() {
        let dir = temp_dir("reorder");

        std::fs::write(
            dir.join("playlists.toml"),
            r#"
version = 1

[[playlists]]
playlist_id = "wall"
interval = "1m"
tabs = ["one", "two", "three"]
"#,
        )
        .unwrap();

        let store = store_over(&dir);
        let persisted = store
            .mutate(Document::Playlists, |config| {
                config.playlists.playlists[0].tabs =
                    ["three", "one", "two"].map(String::from).to_vec();
            })
            .await
            .unwrap();

        assert_eq!(persisted, Persisted::ToDisk);

        let reloaded = store_over(&dir);

        assert_eq!(
            reloaded.playlists().await[0].tabs,
            ["three", "one", "two"].map(String::from).to_vec()
        );
    }

    #[tokio::test]
    async fn read_only_applies_the_change_without_writing_it() {
        let dir = temp_dir("read-only");

        std::fs::write(
            dir.join("tabs.toml"),
            r#"
version = 1

[[tabs]]
tab_id = "one"
url = "https://example.com/one"
"#,
        )
        .unwrap();

        let store = store_in_mode(&dir, true);
        let persisted = store
            .mutate(Document::Tabs, |config| {
                config.tabs.tabs[0].source = Source::Url("https://example.com/two".to_string());
            })
            .await
            .unwrap();

        assert_eq!(persisted, Persisted::MemoryOnly);
        assert_eq!(
            store.tabs().await[0].source.describe(),
            "https://example.com/two"
        );
        assert!(std::fs::read_to_string(dir.join("tabs.toml"))
            .unwrap()
            .contains("example.com/one"));
    }

    #[tokio::test]
    async fn playlist_tabs_are_ordered_and_skip_disabled_ones() {
        let dir = temp_dir("ordering");

        std::fs::write(
            dir.join("tabs.toml"),
            r#"
version = 1
[[tabs]]
tab_id = "one"
url = "https://example.com/one"
[[tabs]]
tab_id = "two"
url = "https://example.com/two"
[[tabs]]
tab_id = "three"
url = "https://example.com/three"
"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("playlists.toml"),
            r#"
version = 1
[[playlists]]
playlist_id = "wall"
interval = "1m"
tabs = ["three", "one", "two"]
disabled_tabs = ["one"]
"#,
        )
        .unwrap();

        let store = store_over(&dir);
        let tabs = store.playlist_tabs("wall").await;

        assert_eq!(
            tabs.iter()
                .map(|tab| tab.tab_id.as_str())
                .collect::<Vec<_>>(),
            ["three", "two"]
        );
    }

    #[tokio::test]
    async fn an_export_never_carries_an_inline_secret() {
        let dir = temp_dir("export");

        std::fs::write(
            dir.join("device.toml"),
            r#"
version = 1
name = "Wall"
device_id = "wall"
admin_key = "hunter2"
"#,
        )
        .unwrap();

        let exported = store_over(&dir).export().await.unwrap();

        assert!(!exported.contains("hunter2"));
        assert!(exported.contains("MISSIOND_ADMIN_KEY"));
    }
}

#[cfg(test)]
mod writability {
    use super::is_writable;

    #[test]
    fn a_directory_with_no_write_bits_is_read_only() {
        let base = std::env::temp_dir().join("missiond-readonly-probe");
        let writable = base.join("writable");
        let locked = base.join("locked");

        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&writable).unwrap();
        std::fs::create_dir_all(&locked).unwrap();

        assert!(is_writable(&writable));

        let mut permissions = std::fs::metadata(&locked).unwrap().permissions();

        permissions.set_readonly(true);
        std::fs::set_permissions(&locked, permissions).unwrap();

        assert!(!is_writable(&locked));
    }
}
