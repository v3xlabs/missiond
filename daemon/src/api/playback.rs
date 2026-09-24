use std::sync::Arc;

use poem_openapi::{payload::Json, OpenApi};

use crate::{
    chrome::{tell, ChromeMessage},
    state::AppState,
};

use super::{
    auth::{authorize_control, Authorization},
    ApiError, ApiResult, MutationResult, RotationState,
};

pub struct PlaybackApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl PlaybackApi {
    /// Advance to the next tab in the current playlist.
    #[oai(path = "/playback/next", method = "post")]
    async fn next(&self, authorization: Authorization) -> ApiResult<Json<MutationResult>> {
        self.send(ChromeMessage::NextTab, &authorization).await
    }

    /// Step back to the previous tab.
    #[oai(path = "/playback/previous", method = "post")]
    async fn previous(&self, authorization: Authorization) -> ApiResult<Json<MutationResult>> {
        self.send(ChromeMessage::PreviousTab, &authorization).await
    }

    /// Stop rotating. The tab on screen stays there.
    #[oai(path = "/playback/pause", method = "post")]
    async fn pause(&self, authorization: Authorization) -> ApiResult<Json<MutationResult>> {
        self.send(ChromeMessage::Pause, &authorization).await
    }

    /// Resume rotating, clearing any hold left by a tab chosen by hand.
    #[oai(path = "/playback/resume", method = "post")]
    async fn resume(&self, authorization: Authorization) -> ApiResult<Json<MutationResult>> {
        self.send(ChromeMessage::Resume, &authorization).await
    }

    /// Pause if rotating, resume if paused. The answer says which way it went.
    #[oai(path = "/playback/toggle", method = "post")]
    async fn toggle(&self, authorization: Authorization) -> ApiResult<Json<RotationState>> {
        self.send(ChromeMessage::ToggleRotation, &authorization)
            .await?;

        Ok(Json(RotationState {
            auto_rotate: self.state.chrome.state.lock().await.auto_rotate(),
        }))
    }
}

impl PlaybackApi {
    async fn send(
        &self,
        message: ChromeMessage,
        authorization: &Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, authorization)?;
        tell(&self.state.chrome, message)
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(MutationResult::applied()))
    }
}
