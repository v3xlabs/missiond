pub mod catalogue;
pub mod catalogue_playlist;
pub mod catalogue_tab;
pub mod display_event;
pub mod state_frame;

pub use catalogue::Catalogue;
pub use catalogue_playlist::CataloguePlaylist;
pub use catalogue_tab::CatalogueTab;
pub use display_event::DisplayEvent;
pub use state_frame::StateFrame;

use std::sync::Arc;

use tokio::sync::watch;

use crate::state::AppState;

pub struct Events {
    sender: watch::Sender<DisplayEvent>,
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Events {
    pub fn new() -> Self {
        Self {
            sender: watch::channel(DisplayEvent::default()).0,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<DisplayEvent> {
        self.sender.subscribe()
    }

    pub async fn publish(&self, app_state: &Arc<AppState>) {
        let device = app_state.config.device().await;
        let chrome = app_state.chrome.state.lock().await;

        let event = DisplayEvent {
            device_id: device.device_id,
            device_name: device.name,
            current_playlist_id: chrome.current_playlist_id.clone(),
            current_tab_id: chrome.current_tab_id.clone(),
            auto_rotate: chrome.auto_rotate(),
            next_rotation_at: chrome.next_rotation_at(),
            screen_on: app_state.display.is_on(),
            brightness: app_state.display.brightness(),
        };

        self.sender.send_if_modified(|current| {
            if *current == event {
                false
            } else {
                *current = event;
                true
            }
        });
    }
}
