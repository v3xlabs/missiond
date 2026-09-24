use std::sync::Arc;

use poem_openapi::{param::Path, payload::Json, OpenApi};

use crate::state::AppState;

use super::{
    auth::{authorize_control, Authorization},
    ApiError, ApiResult, MutationResult, PowerState, SetBrightnessRequest,
};

pub struct DisplayApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl DisplayApi {
    /// Turn the screen on or off.
    #[oai(path = "/display/power/:on", method = "post")]
    async fn set_power(
        &self,
        on: Path<bool>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;
        self.power(on.0).await?;

        Ok(Json(MutationResult::applied()))
    }

    /// Turn the screen off if it is on, on if it is off.
    #[oai(path = "/display/power/toggle", method = "post")]
    async fn toggle_power(&self, authorization: Authorization) -> ApiResult<Json<PowerState>> {
        authorize_control(&self.state, &authorization)?;

        let on = !self.state.display.is_on();

        self.power(on).await?;

        Ok(Json(PowerState { screen_on: on }))
    }

    /// Set panel brightness over DDC.
    #[oai(path = "/display/brightness", method = "put")]
    async fn set_brightness(
        &self,
        request: Json<SetBrightnessRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        let display = self.state.config.read().await.display;

        self.state
            .display
            .set_brightness(&display, request.0.percent)
            .await
            .map_err(ApiError::internal)?;
        self.state.events.publish(&self.state).await;

        Ok(Json(MutationResult::applied()))
    }
}

impl DisplayApi {
    async fn power(&self, on: bool) -> ApiResult<()> {
        let display = self.state.config.read().await.display;

        self.state
            .display
            .set_power(&display, on)
            .await
            .map_err(ApiError::internal)?;
        self.state.hass.publish_backlight(on).await;
        self.state.events.publish(&self.state).await;

        Ok(())
    }
}
