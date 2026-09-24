use std::sync::Arc;

use poem_openapi::{
    payload::{Json, PlainText},
    OpenApi,
};

use crate::state::AppState;

use super::{
    auth::{Access, Authorization},
    ApiError, ApiResult, DeviceStatus,
};

pub struct StatusApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl StatusApi {
    /// What is on screen, and how the daemon is configured to persist changes.
    #[oai(path = "/status", method = "get")]
    async fn status(&self, authorization: Authorization) -> ApiResult<Json<DeviceStatus>> {
        let device = self.state.config.read().await.device;
        let chrome = self.state.chrome.state.lock().await;

        Ok(Json(DeviceStatus {
            device_id: device.device_id,
            device_name: device.name,
            current_playlist_id: chrome.current_playlist_id.clone(),
            current_tab_id: chrome.current_tab_id.clone(),
            auto_rotate: chrome.auto_rotate(),
            screen_on: self.state.display.is_on(),
            brightness: self.state.display.brightness(),
            uptime_seconds: self.state.uptime_seconds(),
            current_tab_opened_at: chrome
                .current_tab_opened_at
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|since| since.as_secs()),
            config_read_only: self.state.config.is_read_only(),
            requires_auth: self.state.admin_key.is_some(),
            access: Access::of(&self.state, authorization.0.as_deref()),
        }))
    }

    /// The whole configuration as TOML, with any inline secret replaced by a reference.
    #[oai(path = "/config/export", method = "get")]
    async fn export(&self) -> ApiResult<PlainText<String>> {
        self.state
            .config
            .export()
            .await
            .map(PlainText)
            .map_err(ApiError::internal)
    }
}
