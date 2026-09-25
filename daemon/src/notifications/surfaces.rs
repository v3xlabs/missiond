use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use crate::{api::SidebarState, config::NotificationMode, state::AppState};

use super::{Sidebar, SidebarMode, Toast};

pub struct Surfaces {
    pub sidebar: Sidebar,
    pub toast: Toast,
    sidebar_mode: Mutex<SidebarMode>,
}

impl Default for Surfaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Surfaces {
    pub fn new() -> Self {
        Self {
            sidebar: Sidebar::new(),
            toast: Toast::new(),
            sidebar_mode: Mutex::new(SidebarMode::Auto),
        }
    }

    /// Pins the rail to the opposite of what is on screen now.
    pub async fn toggle_sidebar(&self, app_state: &Arc<AppState>) -> SidebarState {
        let mode = if self.sidebar.is_open().await {
            SidebarMode::Closed
        } else {
            SidebarMode::Open
        };

        self.set_sidebar_mode(mode, app_state).await
    }

    pub async fn set_sidebar_mode(
        &self,
        mode: SidebarMode,
        app_state: &Arc<AppState>,
    ) -> SidebarState {
        *self.sidebar_mode.lock().await = mode;

        info!(?mode, "sidebar mode set");
        self.reconcile(app_state).await;

        self.sidebar_state().await
    }

    pub async fn sidebar_state(&self) -> SidebarState {
        SidebarState {
            open: self.sidebar.is_open().await,
            mode: *self.sidebar_mode.lock().await,
        }
    }

    pub async fn reconcile(&self, app_state: &Arc<AppState>) {
        let active = app_state.notifications.active().await;

        let wanted = match *self.sidebar_mode.lock().await {
            SidebarMode::Open => true,
            SidebarMode::Closed => false,
            SidebarMode::Auto => active
                .iter()
                .any(|notification| notification.mode == NotificationMode::Sidebar),
        };

        match (wanted, self.sidebar.is_open().await) {
            (true, false) => self.sidebar.open(app_state).await,
            (false, true) => self.sidebar.close().await,
            _ => {}
        }

        let toast_wanted = active
            .iter()
            .any(|notification| notification.mode == NotificationMode::Toast);

        match (toast_wanted, self.toast.is_open().await) {
            (true, false) => self.toast.open(app_state).await,
            (false, true) => self.toast.close(app_state).await,
            _ => {}
        }

        app_state.events.publish(app_state).await;
    }

    pub async fn shutdown(&self, app_state: &Arc<AppState>) {
        self.sidebar.close().await;
        self.toast.close(app_state).await;
    }
}

/// Expiry reaches this loop only because the notification stream calls `Notifications::active`
/// on a timer. Nothing here notices a meeting ending on its own.
pub async fn run(app_state: Arc<AppState>) {
    let mut changed = app_state.notifications.subscribe();

    loop {
        app_state.surfaces.reconcile(&app_state).await;

        if changed.changed().await.is_err() {
            break;
        }
    }
}
