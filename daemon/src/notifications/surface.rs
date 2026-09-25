use std::sync::Arc;

use anyhow::{Context as _, Result};
use tokio::{process::Child, sync::Mutex};
use tracing::{info, warn};

use crate::{chrome::mark_clean_exit, niri, state::AppState};

pub struct Surface {
    profile: &'static str,
    page: &'static str,
    window: Mutex<Option<Child>>,
}

impl Surface {
    pub fn new(profile: &'static str, page: &'static str) -> Self {
        Self {
            profile,
            page,
            window: Mutex::new(None),
        }
    }

    /// A `Child` that has exited is still `Some`, so the handle alone cannot answer this.
    pub async fn is_running(&self) -> bool {
        let mut window = self.window.lock().await;

        let Some(child) = window.as_mut() else {
            return false;
        };

        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                warn!(self.page, %status, "surface window exited");
                *window = None;

                false
            }
            Err(error) => {
                warn!(
                    self.page,
                    "cannot tell whether the surface window is alive: {error}"
                );

                false
            }
        }
    }

    /// Returns the compositor's id for the window, so the caller can place it.
    pub async fn open(&self, app_state: &Arc<AppState>) -> Result<u64> {
        let config = app_state.config.read().await;
        let port = config.device.http.port;
        let binary = config
            .device
            .chromium
            .binary_path
            .clone()
            .or_else(|| std::env::var("CHROMIUM_BINARY").ok())
            .unwrap_or_else(|| "chromium".to_string());

        let profile = app_state.config.dirs.cache.join(self.profile);
        let page = self.page;

        mark_clean_exit(&profile);

        let child = tokio::process::Command::new(binary)
            .arg(format!("--app=http://127.0.0.1:{port}/{page}"))
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("--ozone-platform=wayland")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--password-store=basic")
            .arg("--use-mock-keychain")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            // A surface shows one local page, but Chromium still registers for push notifications
            // on every open and fills the log with the failures.
            .arg("--disable-background-networking")
            .arg("--disable-session-crashed-bubble")
            .arg("--hide-crash-restore-bubble")
            .spawn()
            .with_context(|| format!("cannot open the window for {page}"))?;

        let pid = child
            .id()
            .context("the surface window reported no process id")?;

        *self.window.lock().await = Some(child);

        // A process with no window would count as open, and nothing would try to open it again.
        let window_id = match niri::wait_for_pid(pid).await {
            Ok(window) => window.id,
            Err(error) => {
                self.close().await;

                return Err(error);
            }
        };

        info!(page, window_id, "surface window opened");

        Ok(window_id)
    }

    pub async fn close(&self) {
        if let Some(mut child) = self.window.lock().await.take() {
            let _ = child.kill().await;
            info!(self.page, "surface window closed");
        }
    }
}
