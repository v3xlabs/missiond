use std::sync::Arc;

use anyhow::Result;

use crate::{
    chrome::ChromeController,
    config::{ConfigStore, Dirs, SecretRef},
    db::Runtime,
    display::{capture::OutputCapture, Display},
    events::Events,
    hass::HassManager,
    notifications::{Notifications, Surfaces},
    player::{overlay::Overlay, Player},
};

pub type State = Arc<AppState>;

pub struct AppState {
    pub chrome: Arc<ChromeController>,
    pub config: Arc<ConfigStore>,
    pub display: Arc<Display>,
    pub capture: Arc<OutputCapture>,
    pub events: Arc<Events>,
    pub notifications: Arc<Notifications>,
    pub surfaces: Arc<Surfaces>,
    pub player: Arc<Player>,
    pub overlay: Arc<Overlay>,
    pub hass: Arc<HassManager>,
    pub runtime: Runtime,
    pub admin_key: Option<String>,
    pub control_key: Option<String>,
    pub started_at: std::time::Instant,
}

impl AppState {
    pub async fn new(dirs: Dirs) -> Result<Arc<Self>> {
        dirs.create_state_and_cache()?;

        let runtime = Runtime::open(&dirs.state).await?;
        let config = Arc::new(ConfigStore::load(dirs)?);
        let device = config.read().await.device;

        let admin_key = device
            .admin_key
            .as_ref()
            .map(SecretRef::resolve)
            .transpose()?;
        let control_key = device
            .control_key
            .as_ref()
            .map(SecretRef::resolve)
            .transpose()?;

        let hass = HassManager::new(&device).await?;
        let dirs_state = config.dirs.state.clone();
        let player = Arc::new(Player::new(&config.dirs.state));

        let state = Arc::new(Self {
            chrome: Arc::new(ChromeController::new()),
            config,
            display: Arc::new(Display::new()),
            capture: Arc::new(OutputCapture::new()),
            events: Arc::new(Events::new()),
            notifications: Arc::new(Notifications::new()),
            surfaces: Arc::new(Surfaces::new()),
            player,
            overlay: Arc::new(Overlay::new(&dirs_state)),
            hass: Arc::new(hass),
            runtime,
            admin_key,
            control_key,
            started_at: std::time::Instant::now(),
        });

        state.events.publish(&state).await;

        Ok(state)
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}
