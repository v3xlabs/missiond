use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use chromiumoxide::{
    cdp::browser_protocol::{
        browser::{Bounds, GetWindowForTargetParams, SetWindowBoundsParams, WindowState},
        emulation::SetDeviceMetricsOverrideParams,
        page::{NavigateParams, StopScreencastParams},
    },
    Browser, BrowserConfig, Page,
};
use futures::StreamExt;
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{error, info, warn};

use crate::{
    config::{ChromiumConfig, Source, Tab},
    niri, player,
    state::AppState,
};

use super::{capture, ChromeMessage, ChromeResponse, ChromeState, Preview, Request};

const PROFILE_DIRECTORY: &str = "chromium-profile";

/// The tab the daemon opens for itself. It is not in `tabs.toml` and never appears in a playlist.
const ALERT_TAB: &str = "missiond:alert";

/// The full-screen agenda, which a button somewhere else puts up and takes away again.
pub const CALENDAR_TAB: &str = "missiond:calendar";

/// The page a built-in tab shows, or nothing if the id names a tab from `tabs.toml`.
///
/// One page serves all of these. Which presentation it draws is the query it is opened with, so
/// adding one is a name here rather than a second bundle for the browser to load.
fn own_page(tab_id: &str) -> Option<&'static str> {
    match tab_id {
        ALERT_TAB => Some("notify.html"),
        CALENDAR_TAB => Some("notify.html?agenda=1"),
        _ => None,
    }
}

/// Chromium records the owning instance as a `<hostname>-<pid>` symlink at `SingletonLock`.
fn lock_owner(profile: &std::path::Path) -> Option<u32> {
    let target = std::fs::read_link(profile.join("SingletonLock")).ok()?;

    target.to_str()?.rsplit('-').next()?.parse().ok()
}

fn is_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Chromium treats these as evidence that another instance owns the profile: it hands the launch
/// off to that instance, prints no DevTools URL, and exits.
///
/// Removing them unconditionally is worse than leaving them. A lock whose owner is still running
/// is doing its job, and clearing it lets a second Chromium open the same profile, which is what
/// produces "something went wrong during profile initialization" and a window per attempt.
fn clear_stale_locks(profile: &std::path::Path) {
    if let Some(pid) = lock_owner(profile) {
        if is_alive(pid) {
            warn!(
                "chromium {pid} still holds {}, leaving its lock alone",
                profile.display()
            );

            return;
        }

        info!("clearing the profile lock left by chromium {pid}");
    }

    for name in ["SingletonLock", "SingletonSocket", "SingletonCookie"] {
        let _ = std::fs::remove_file(profile.join(name));
    }
}

/// A browser holding our profile that we cannot talk to is worse than no browser: every launch
/// after it either fails or opens a second instance on the same profile. The daemon owns this
/// profile exclusively, so whatever holds it is ours to stop.
fn terminate_profile_holder(profile: &std::path::Path) {
    let Some(pid) = lock_owner(profile).filter(|pid| is_alive(*pid)) else {
        return;
    };

    warn!("terminating orphaned chromium {pid} holding the profile");

    // SIGTERM lets it write the profile out and remove its own lock.
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();

    for _ in 0..20 {
        if !is_alive(pid) {
            return;
        }

        std::thread::sleep(Duration::from_millis(250));
    }

    warn!("chromium {pid} did not exit after SIGTERM");
}

pub struct ChromeController {
    pub state: Arc<Mutex<ChromeState>>,
    browser: Arc<Mutex<Option<Browser>>>,
    pages: Arc<Mutex<HashMap<String, Page>>>,
    previews: Arc<Mutex<HashMap<String, Preview>>>,
    viewport: Arc<Mutex<HashMap<String, (i32, i32)>>>,
    auto_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    sender: mpsc::Sender<Request>,
    receiver: Mutex<Option<mpsc::Receiver<Request>>>,
    running: AtomicBool,
    fullscreen: AtomicBool,
}

impl Default for ChromeController {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromeController {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);

        Self {
            state: Arc::new(Mutex::new(ChromeState::default())),
            browser: Arc::new(Mutex::new(None)),
            pages: Arc::new(Mutex::new(HashMap::new())),
            previews: Arc::new(Mutex::new(HashMap::new())),
            viewport: Arc::new(Mutex::new(HashMap::new())),
            auto_task: Arc::new(Mutex::new(None)),
            sender,
            receiver: Mutex::new(Some(receiver)),
            running: AtomicBool::new(false),
            fullscreen: AtomicBool::new(false),
        }
    }

    pub fn sender(&self) -> mpsc::Sender<Request> {
        self.sender.clone()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// The process the display browser runs as.
    ///
    /// A surface that has to sit beside the display, or over it, needs the compositor's id for
    /// that window. Chromium's app id is not something the daemon chooses, so the process is the
    /// only handle on it that the daemon can be sure of.
    pub async fn browser_pid(&self) -> Option<u32> {
        self.browser
            .lock()
            .await
            .as_mut()?
            .get_mut_child()?
            .as_mut_inner()
            .id()
    }

    pub async fn start(self: &Arc<Self>, app_state: &Arc<AppState>) -> Result<()> {
        let config = app_state.config.read().await;

        self.fullscreen
            .store(config.device.chromium.fullscreen, Ordering::Relaxed);

        if self.browser.lock().await.is_none() {
            let cache = &app_state.config.dirs.cache;

            // systemd kills the browser with the rest of the cgroup, but a crash or a manual
            // kill of the daemon alone leaves it running and holding the profile.
            terminate_profile_holder(&cache.join(PROFILE_DIRECTORY));
            self.launch_with_retry(&config.device.chromium, cache).await;
        }

        let receiver = self
            .receiver
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow!("chrome controller already started"))?;

        let controller = Arc::clone(self);
        let state = app_state.clone();

        self.running.store(true, Ordering::Relaxed);
        tokio::spawn(async move { controller.run_message_loop(receiver, state).await });

        self.activate_default_playlist(app_state).await
    }

    fn build_browser_config(
        config: &ChromiumConfig,
        cache: &std::path::Path,
    ) -> Result<BrowserConfig> {
        let mut builder = BrowserConfig::builder()
            .chrome_executable(
                config
                    .binary_path
                    .clone()
                    .or_else(|| std::env::var("CHROMIUM_BINARY").ok())
                    .unwrap_or_else(|| "chromium".to_string()),
            )
            .user_data_dir(cache.join(PROFILE_DIRECTORY))
            .with_head()
            .disable_default_args()
            .arg("--disable-background-networking")
            .arg("--enable-features=NetworkService,NetworkServiceInProcess")
            .arg("--disable-background-timer-throttling")
            .arg("--disable-backgrounding-occluded-windows")
            .arg("--disable-breakpad")
            .arg("--disable-client-side-phishing-detection")
            .arg("--disable-component-extensions-with-background-pages")
            .arg("--disable-default-apps")
            .arg("--disable-extensions")
            .arg("--disable-features=TranslateUI")
            .arg("--disable-hang-monitor")
            .arg("--disable-ipc-flooding-protection")
            .arg("--disable-popup-blocking")
            .arg("--disable-prompt-on-repost")
            .arg("--disable-renderer-backgrounding")
            .arg("--disable-sync")
            .arg("--force-color-profile=srgb")
            .arg("--metrics-recording-only")
            .arg("--no-first-run")
            .arg("--password-store=basic")
            .arg("--use-mock-keychain")
            .arg("--lang=en_US")
            .arg("--ozone-platform=wayland")
            .arg("--disable-infobars")
            .arg("--disable-session-crashed-bubble")
            .arg("--hide-crash-restore-bubble")
            .viewport(None);

        // `--kiosk` alone is not enough, because a CDP-created tab drags the window back to its
        // decorated form. `go_fullscreen` finishes the job once a page exists.
        if config.fullscreen {
            builder = builder.arg("--kiosk");
        } else {
            builder = builder.arg("--start-maximized");
        }

        for arg in &config.extra_args {
            builder = builder.arg(arg.as_str());
        }

        builder
            .build()
            .map_err(|error| anyhow!("failed to build browser config: {error}"))
    }

    /// A display that loses its browser has to get it back on its own. Giving up after one
    /// attempt leaves the daemon serving an API with nothing on the screen until a person
    /// notices.
    async fn launch_with_retry(&self, config: &ChromiumConfig, cache: &std::path::Path) {
        let mut delay = Duration::from_secs(1);

        loop {
            match self.launch_browser(config, cache).await {
                Ok(()) => return,
                Err(error) => {
                    error!("chromium did not start: {error}. retrying in {delay:?}");

                    // A failed launch can still leave a live Chromium behind: the timeout is on
                    // reading the DevTools URL, not on the process. Retrying past it would put a
                    // second instance on the same profile.
                    terminate_profile_holder(&cache.join(PROFILE_DIRECTORY));

                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(60));
                }
            }
        }
    }

    async fn launch_browser(&self, config: &ChromiumConfig, cache: &std::path::Path) -> Result<()> {
        let profile = cache.join(PROFILE_DIRECTORY);

        clear_stale_locks(&profile);
        super::mark_clean_exit(&profile);

        let (browser, mut handler) =
            Browser::launch(Self::build_browser_config(config, cache)?).await?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(error) = event {
                    error!("chromium handler error: {error:?}");
                }
            }
            warn!("chromium handler loop ended");
        });

        *self.browser.lock().await = Some(browser);
        info!("chromium launched");

        Ok(())
    }

    async fn run_message_loop(
        self: Arc<Self>,
        mut receiver: mpsc::Receiver<Request>,
        app_state: Arc<AppState>,
    ) {
        while let Some(Request { message, reply }) = receiver.recv().await {
            info!("chrome message: {message:?}");

            let response = match self.handle_message(message, &app_state).await {
                Ok(response) => response,
                Err(error) => ChromeResponse::Error {
                    message: error.to_string(),
                },
            };

            // A message can change the rotation or a hold without changing the tab, and the
            // stream carries both.
            app_state.events.publish(&app_state).await;

            let _ = reply.send(response);
        }

        error!("chrome message loop exited");
    }

    async fn handle_message(
        &self,
        message: ChromeMessage,
        app_state: &Arc<AppState>,
    ) -> Result<ChromeResponse> {
        match message {
            ChromeMessage::ActivatePlaylist { playlist_id } => {
                self.activate_playlist(&playlist_id, app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::ActivateTab {
                tab_id,
                playlist_id,
            } => {
                self.activate_tab(&tab_id, &playlist_id, app_state).await?;
                self.hold(&playlist_id, app_state).await;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::NextTab => {
                self.step(1, app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::PreviousTab => {
                self.step(-1, app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::Pause => {
                self.stop_auto_rotation().await;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::Resume => {
                self.resume(app_state).await;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::ToggleRotation => {
                if self.state.lock().await.auto_rotate() {
                    self.stop_auto_rotation().await;
                } else {
                    self.resume(app_state).await;
                }

                Ok(ChromeResponse::Success)
            }
            ChromeMessage::RefreshTab { tab_id } => {
                self.refresh_tab(&tab_id).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::RecreateTab { tab_id } => {
                self.recreate_tab(&tab_id, app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::CloseTab { tab_id } => {
                self.close_tab(&tab_id).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::GetStatus => {
                let state = self.state.lock().await;

                Ok(ChromeResponse::Status {
                    current_playlist_id: state.current_playlist_id.clone(),
                    current_tab_id: state.current_tab_id.clone(),
                    is_running: state.is_running,
                    auto_rotate: state.auto_rotate(),
                })
            }
            ChromeMessage::Takeover {
                tab_id,
                stinger,
                seconds,
            } => {
                self.takeover(tab_id, stinger, seconds, app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::EndTakeover => {
                self.end_takeover(app_state).await?;
                Ok(ChromeResponse::Success)
            }
            ChromeMessage::Shutdown => {
                self.shutdown().await?;
                Ok(ChromeResponse::Success)
            }
        }
    }

    async fn activate_default_playlist(&self, app_state: &Arc<AppState>) -> Result<()> {
        let playlists = app_state.config.playlists().await;

        let Some(playlist) = playlists
            .iter()
            .find(|playlist| playlist.is_default)
            .or_else(|| playlists.first())
        else {
            warn!("no playlists configured");
            return Ok(());
        };

        self.activate_playlist(&playlist.playlist_id.clone(), app_state)
            .await
    }

    async fn resume(&self, app_state: &Arc<AppState>) {
        let playlist_id = self.state.lock().await.current_playlist_id.clone();

        let Some(playlist_id) = playlist_id else {
            return;
        };

        if let Some(playlist) = app_state.config.playlist(&playlist_id).await {
            self.state.lock().await.hold_until = None;
            self.start_auto_rotation(playlist.interval.into()).await;
        }
    }

    async fn activate_playlist(&self, playlist_id: &str, app_state: &Arc<AppState>) -> Result<()> {
        let playlist = app_state
            .config
            .playlist(playlist_id)
            .await
            .ok_or_else(|| anyhow!("playlist {playlist_id} not found"))?;

        let tabs = app_state.config.playlist_tabs(playlist_id).await;

        if tabs.is_empty() {
            return Err(anyhow!("playlist {playlist_id} has no enabled tabs"));
        }

        {
            let mut state = self.state.lock().await;

            state.current_playlist_id = Some(playlist_id.to_string());
            state.is_running = true;
            state.hold_until = None;
        }

        app_state
            .hass
            .publish_playlist_options(
                app_state
                    .config
                    .playlists()
                    .await
                    .iter()
                    .map(|playlist| playlist.playlist_id.clone())
                    .collect(),
                Some(playlist_id),
            )
            .await;

        // A camera has nothing to preload. It is connected when it comes on screen, because a
        // stream nobody is watching is bandwidth off the camera for no reason.
        let pages = tabs
            .iter()
            .filter(|tab| tab.persist && matches!(tab.source, Source::Url(_)));

        for tab in pages {
            if !self.pages.lock().await.contains_key(&tab.tab_id) {
                if let Err(error) = self.create_tab_page(tab).await {
                    warn!("failed to preload tab {}: {error}", tab.tab_id);
                }
            }
        }

        self.activate_tab(&tabs[0].tab_id.clone(), playlist_id, app_state)
            .await?;
        self.start_auto_rotation(playlist.interval.into()).await;

        Ok(())
    }

    async fn activate_tab(
        &self,
        tab_id: &str,
        playlist_id: &str,
        app_state: &Arc<AppState>,
    ) -> Result<()> {
        let tab = app_state.config.tab(tab_id).await;

        match tab.as_ref().map(|tab| &tab.source) {
            Some(Source::Rtsp(stream)) => {
                let mpv = app_state.config.read().await.device.mpv;

                app_state.player.show(tab_id, stream, &mpv).await?;

                self.focus_camera(tab_id, app_state).await;
            }
            _ => {
                // An open page is enough to bring forward. The alert and transition pages are
                // opened by the daemon rather than declared in `tabs.toml`, so requiring a config
                // entry here would make them impossible to show.
                if !self.pages.lock().await.contains_key(tab_id) {
                    let tab = tab
                        .as_ref()
                        .ok_or_else(|| anyhow!("tab {tab_id} not found"))?;

                    self.create_tab_page(tab).await?;
                }

                let page = self
                    .pages
                    .lock()
                    .await
                    .get(tab_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("no page for tab {tab_id}"))?;

                // The camera window sits over the browser, so a page can only come forward once
                // that window is gone.
                app_state.player.hide().await;

                page.bring_to_front().await?;
            }
        }

        let previous = {
            let mut state = self.state.lock().await;
            let previous = state.current_tab_id.replace(tab_id.to_string());

            state.current_playlist_id = Some(playlist_id.to_string());
            state.current_tab_opened_at = Some(std::time::SystemTime::now());

            previous
        };

        if let Some(previous) = previous.filter(|previous| previous != tab_id) {
            self.set_capture_rate(&previous, None).await;
        }

        self.set_capture_rate(tab_id, Some(capture::FOREGROUND_FPS))
            .await;

        let tabs = app_state.config.playlist_tabs(playlist_id).await;

        app_state
            .hass
            .publish_tab_options(&tabs, Some(tab_id))
            .await;

        if let Some(tab) = tab {
            app_state.hass.publish_url(tab.source.describe()).await;
        }

        app_state.events.publish(app_state).await;

        Ok(())
    }

    /// Puts an alert on screen. `tab_id` shows an existing tab, such as a camera feed; without
    /// one the alert page shows the message itself.
    ///
    /// The stinger is what makes a slow tab usable. The target starts loading first and the clip
    /// covers the wait, so the viewer sees a deliberate transition rather than a blank page.
    async fn takeover(
        &self,
        tab_id: Option<String>,
        stinger: Option<String>,
        seconds: Option<u64>,
        app_state: &Arc<AppState>,
    ) -> Result<()> {
        let playlist_id = self.state.lock().await.current_playlist_id.clone();

        {
            let mut state = self.state.lock().await;

            // Only the first alert records where to go back to. A second one arriving during a
            // takeover must not make the alert page itself the thing we return to.
            if state.interrupted_tab_id.is_none() {
                state.interrupted_tab_id = state.current_tab_id.clone();
            }
        }

        self.stop_auto_rotation().await;

        let target = match tab_id {
            Some(tab_id) => tab_id,
            None => ALERT_TAB.to_string(),
        };

        // Creating the page starts the navigation and returns; the load continues underneath.
        // Doing it before the clip plays is the whole point of playing one.
        self.ensure_page(&target, app_state).await;

        let clip = match stinger {
            Some(name) => self.start_stinger(&name, app_state).await,
            None => None,
        };

        let playlist_id = playlist_id.unwrap_or_default();

        // The target comes up behind the clip rather than after it. A camera needs seconds to
        // connect, and those are the seconds the clip is there to cover.
        self.activate_tab(&target, &playlist_id, app_state).await?;

        if let Some(hold) = clip {
            // Activating the target took the focus, and the compositor shows the focused window.
            // The clip goes back on top of what is now underneath it.
            app_state.overlay.raise().await;

            tokio::time::sleep(hold).await;
            app_state.overlay.stop().await;
            self.focus_camera(&target, app_state).await;
        }

        {
            let mut state = self.state.lock().await;

            // A takeover with no end holds by leaving auto rotation stopped. Setting a deadline
            // here instead would let the next rotation tick step off the page a person asked for.
            state.hold_until = seconds.map(|seconds| Instant::now() + Duration::from_secs(seconds));
        }

        Ok(())
    }

    /// Returns to whatever the playlist was showing, and lets rotation continue.
    async fn end_takeover(&self, app_state: &Arc<AppState>) -> Result<()> {
        let (playlist_id, interrupted) = {
            let mut state = self.state.lock().await;

            state.hold_until = None;

            (
                state.current_playlist_id.clone(),
                state.interrupted_tab_id.take(),
            )
        };

        let Some(playlist_id) = playlist_id else {
            return Ok(());
        };

        if let Some(tab_id) = interrupted {
            self.activate_tab(&tab_id, &playlist_id, app_state).await?;
        }

        if let Some(playlist) = app_state.config.playlist(&playlist_id).await {
            self.start_auto_rotation(playlist.interval.into()).await;
        }

        Ok(())
    }

    /// The daemon's own pages are not in `tabs.toml`; it serves them and opens them on demand.
    async fn ensure_page(&self, tab_id: &str, app_state: &Arc<AppState>) {
        if let Some(page) = own_page(tab_id) {
            let own = Tab {
                tab_id: tab_id.to_string(),
                name: None,
                persist: false,
                scale: None,
                stinger: None,
                source: Source::Url(self.own_url(app_state, page).await),
            };

            if let Err(error) = self.recreate_from(&own).await {
                warn!("failed to open {tab_id}: {error}");
            }

            return;
        }

        if self.pages.lock().await.contains_key(tab_id) {
            return;
        }

        let Some(tab) = app_state.config.tab(tab_id).await else {
            warn!("takeover asked for {tab_id}, which is not a configured tab");

            return;
        };

        // A camera has no page to open ahead of the clip. It connects when it is activated, which
        // for a camera is fast enough that the clip has nothing to cover.
        if matches!(tab.source, Source::Rtsp(_)) {
            return;
        }

        if let Err(error) = self.create_tab_page(&tab).await {
            warn!("failed to open {tab_id} for a takeover: {error}");
        }
    }

    /// The pages the daemon serves to itself. Loopback, because the browser is on this machine
    /// whatever address the API is bound to.
    async fn own_url(&self, app_state: &Arc<AppState>, path: &str) -> String {
        let port = app_state.config.read().await.device.http.port;

        format!("http://127.0.0.1:{port}/{path}")
    }

    /// Puts the clip on screen and reports how long it may hold it.
    async fn start_stinger(&self, name: &str, app_state: &Arc<AppState>) -> Option<Duration> {
        let config = app_state.config.read().await;

        let Some(stinger) = config.notifications.stinger(name) else {
            warn!("no stinger named {name}");

            return None;
        };

        let file = app_state
            .config
            .dirs
            .config
            .join("media")
            .join(&stinger.file);

        match app_state.overlay.start(&file, &config.device.mpv).await {
            Ok(()) => Some(stinger.max_duration.into()),
            Err(error) => {
                warn!("could not play the stinger {name}: {error}");

                None
            }
        }
    }

    /// A camera window opens without the focus, and the panel shows the focused window, so a
    /// camera is on the wall only while it holds the focus. Anything else on screen is a page,
    /// which the browser window already has.
    async fn focus_camera(&self, tab_id: &str, app_state: &Arc<AppState>) {
        let is_camera = app_state
            .config
            .tab(tab_id)
            .await
            .is_some_and(|tab| matches!(tab.source, Source::Rtsp(_)));

        if !is_camera {
            return;
        }

        if let Err(error) = niri::focus(player::APP_ID).await {
            warn!("could not bring the camera window forward: {error}");
        }
    }

    async fn recreate_from(&self, tab: &Tab) -> Result<()> {
        if self.pages.lock().await.contains_key(&tab.tab_id) {
            self.close_tab(&tab.tab_id).await?;
        }

        self.create_tab_page(tab).await
    }

    async fn hold(&self, playlist_id: &str, app_state: &Arc<AppState>) {
        let Some(hold) = app_state
            .config
            .playlist(playlist_id)
            .await
            .and_then(|playlist| playlist.hold)
        else {
            return;
        };

        self.state.lock().await.hold_until = Some(Instant::now() + hold.into());
        info!("holding {playlist_id} for {hold}");
    }

    async fn create_tab_page(&self, tab: &Tab) -> Result<()> {
        let Source::Url(url) = &tab.source else {
            return Err(anyhow!("tab {} is a camera and has no page", tab.tab_id));
        };

        let page = {
            let browser = self.browser.lock().await;
            let browser = browser
                .as_ref()
                .ok_or_else(|| anyhow!("browser not ready"))?;

            browser.new_page(url.as_str()).await?
        };

        if self.fullscreen.load(Ordering::Relaxed) {
            if let Err(error) = go_fullscreen(&page).await {
                warn!("could not put the browser window into fullscreen: {error}");
            }
        }

        if let Some(scale) = tab.scale {
            let (width, height) = self.output_size(&page).await.unwrap_or((1920, 1080));

            let _ = page
                .execute(
                    SetDeviceMetricsOverrideParams::builder()
                        .width(width as i64)
                        .height(height as i64)
                        .device_scale_factor(scale)
                        .mobile(false)
                        .build()
                        .map_err(|error| anyhow!("{error}"))?,
                )
                .await;
        }

        if let Some((width, height)) = self.output_size(&page).await {
            self.viewport
                .lock()
                .await
                .insert(tab.tab_id.clone(), (width, height));
        }

        self.pages
            .lock()
            .await
            .insert(tab.tab_id.clone(), page.clone());
        self.previews
            .lock()
            .await
            .insert(tab.tab_id.clone(), Preview::default());

        info!("opened page for tab {}", tab.tab_id);

        Ok(())
    }

    async fn output_size(&self, page: &Page) -> Option<(i32, i32)> {
        let width = page.evaluate("window.innerWidth").await.ok()?;
        let height = page.evaluate("window.innerHeight").await.ok()?;

        Some((
            width.value()?.as_u64()? as i32,
            height.value()?.as_u64()? as i32,
        ))
    }

    pub async fn watch_preview(&self, tab_id: &str) -> Option<watch::Receiver<Option<Vec<u8>>>> {
        let receiver = self
            .previews
            .lock()
            .await
            .get(tab_id)
            .map(|preview| preview.frames.subscribe())?;

        self.set_capture_rate(tab_id, Some(capture::MAX_FPS)).await;

        Some(receiver)
    }

    async fn set_capture_rate(&self, tab_id: &str, requested: Option<u64>) {
        let is_foreground = self.state.lock().await.current_tab_id.as_deref() == Some(tab_id);
        let mut previews = self.previews.lock().await;

        let Some(preview) = previews.get_mut(tab_id) else {
            return;
        };

        let fps = match (requested, is_foreground) {
            (Some(fps), _) => Some(fps),
            (None, true) => Some(capture::FOREGROUND_FPS),
            (None, false) if preview.has_viewers() => Some(capture::MAX_FPS),
            (None, false) => None,
        };

        let Some(fps) = fps else {
            self.stop_capture(preview, tab_id);

            return;
        };

        if preview
            .task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
        {
            return;
        }

        let Some(page) = self.pages.lock().await.get(tab_id).cloned() else {
            return;
        };

        preview.task = Some(tokio::spawn(capture::run(
            page,
            preview.frames.clone(),
            fps,
            tab_id.to_string(),
            self.state.clone(),
        )));
    }

    fn stop_capture(&self, preview: &mut Preview, tab_id: &str) {
        if let Some(task) = preview.task.take() {
            task.abort();
        }

        let pages = self.pages.clone();
        let tab_id = tab_id.to_string();

        tokio::spawn(async move {
            if let Some(page) = pages.lock().await.get(&tab_id) {
                let _ = page.execute(StopScreencastParams::default()).await;
            }
        });
    }

    async fn step(&self, offset: i64, app_state: &Arc<AppState>) -> Result<()> {
        let (playlist_id, current_tab_id) = {
            let state = self.state.lock().await;

            (
                state.current_playlist_id.clone(),
                state.current_tab_id.clone(),
            )
        };

        let Some(playlist_id) = playlist_id else {
            return Ok(());
        };

        let tabs = app_state.config.playlist_tabs(&playlist_id).await;

        if tabs.is_empty() {
            return Ok(());
        }

        let current = current_tab_id
            .and_then(|tab_id| tabs.iter().position(|tab| tab.tab_id == tab_id))
            .unwrap_or(0) as i64;

        let count = tabs.len() as i64;
        let next = ((current + offset) % count + count) % count;

        self.activate_tab(&tabs[next as usize].tab_id.clone(), &playlist_id, app_state)
            .await
    }

    async fn start_auto_rotation(&self, interval: Duration) {
        if interval.is_zero() {
            return;
        }

        self.stop_auto_rotation().await;
        self.state.lock().await.rotation = Some(interval);

        let sender = self.sender.clone();
        let state = self.state.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(align_to_wall_clock(interval)).await;

                {
                    let mut state = state.lock().await;

                    if state.rotation.is_none() {
                        break;
                    }

                    match state.hold_until {
                        Some(until) if until > Instant::now() => continue,
                        Some(_) => state.hold_until = None,
                        None => {}
                    }
                }

                let (reply, _answer) = tokio::sync::oneshot::channel();

                if sender
                    .send(Request {
                        message: ChromeMessage::NextTab,
                        reply,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        *self.auto_task.lock().await = Some(handle);
    }

    async fn stop_auto_rotation(&self) {
        self.state.lock().await.rotation = None;

        if let Some(handle) = self.auto_task.lock().await.take() {
            handle.abort();
        }
    }

    async fn refresh_tab(&self, tab_id: &str) -> Result<()> {
        if let Some(page) = self.pages.lock().await.get(tab_id) {
            page.reload().await?;
        }

        Ok(())
    }

    async fn recreate_tab(&self, tab_id: &str, app_state: &Arc<AppState>) -> Result<()> {
        let tab = app_state
            .config
            .tab(tab_id)
            .await
            .ok_or_else(|| anyhow!("tab {tab_id} not found"))?;

        self.close_tab(tab_id).await?;
        self.create_tab_page(&tab).await
    }

    async fn close_tab(&self, tab_id: &str) -> Result<()> {
        if let Some(preview) = self.previews.lock().await.remove(tab_id) {
            if let Some(task) = preview.task {
                task.abort();
            }
        }

        // Dropping the handle leaves the Chromium target open. Closing it is what frees the
        // renderer process.
        if let Some(page) = self.pages.lock().await.remove(tab_id) {
            page.close().await?;
        }

        self.viewport.lock().await.remove(tab_id);

        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.stop_auto_rotation().await;

        let tab_ids: Vec<String> = self.pages.lock().await.keys().cloned().collect();

        for tab_id in tab_ids {
            if let Err(error) = self.close_tab(&tab_id).await {
                warn!("failed to close tab {tab_id}: {error}");
            }
        }

        if let Some(mut browser) = self.browser.lock().await.take() {
            browser.close().await?;
            let _ = browser.wait().await;
        }

        Ok(())
    }

    pub async fn update_url(&self, tab_id: &str, url: &str) -> Result<()> {
        if let Some(page) = self.pages.lock().await.get(tab_id) {
            page.execute(
                NavigateParams::builder()
                    .url(url)
                    .build()
                    .map_err(|error| anyhow!("{error}"))?,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn viewport(&self, tab_id: &str) -> Option<(i32, i32)> {
        self.viewport.lock().await.get(tab_id).copied()
    }
}

fn align_to_wall_clock(interval: Duration) -> Duration {
    let seconds = interval.as_secs();

    if seconds == 0 {
        return interval;
    }

    let now = chrono::Local::now().timestamp() as u64;
    let remainder = now % seconds;

    if remainder == 0 {
        interval
    } else {
        Duration::from_secs(seconds - remainder)
    }
}

/// `--kiosk` covers the output but leaves the tab strip and omnibox drawn, because a target
/// created through CDP is a tab and a tab needs somewhere to live. Putting the window itself into
/// fullscreen is the presentation change `F11` makes, and that does hide them.
async fn go_fullscreen(page: &Page) -> Result<()> {
    let window = page
        .execute(
            GetWindowForTargetParams::builder()
                .target_id(page.target_id().clone())
                .build(),
        )
        .await?;

    page.execute(SetWindowBoundsParams::new(
        window.window_id,
        Bounds::builder()
            .window_state(WindowState::Fullscreen)
            .build(),
    ))
    .await?;

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_alignment_never_exceeds_the_interval() {
        for seconds in [1_u64, 30, 60, 300, 3600] {
            let interval = Duration::from_secs(seconds);
            let aligned = align_to_wall_clock(interval);

            assert!(aligned <= interval);
            assert!(!aligned.is_zero());
        }
    }

    #[test]
    fn a_zero_interval_is_left_alone() {
        assert!(align_to_wall_clock(Duration::ZERO).is_zero());
    }
}
