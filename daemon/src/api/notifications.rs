use std::{sync::Arc, time::Duration};

use poem_openapi::{param::Path, payload::Json, OpenApi};
use tracing::warn;

use crate::{
    calendar,
    chrome::{tell, ChromeMessage, CALENDAR_TAB},
    config::NotificationMode,
    notifications::Notification,
    state::AppState,
};

use super::{
    auth::{authorize_control, Authorization},
    ApiError, ApiResult, CalendarState, MutationResult, NotifyRequest, SidebarState, StingerInfo,
};

pub struct NotificationApi {
    pub state: Arc<AppState>,
}

#[OpenApi]
impl NotificationApi {
    /// Raise an alert.
    #[oai(path = "/notify", method = "post")]
    async fn notify(
        &self,
        request: Json<NotifyRequest>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        let defaults = self.state.config.read().await.notifications;
        let (notification, duration) = request.0.into_notification(&defaults)?;
        let mode = notification.mode;
        let tab_id = notification.tab_id.clone();
        let stinger = notification.stinger.clone();

        self.state
            .notifications
            .push(notification, duration.into())
            .await;

        if mode == NotificationMode::Takeover {
            let state = self.state.clone();

            tokio::spawn(async move {
                if let Err(error) = tell(
                    &state.chrome,
                    ChromeMessage::Takeover {
                        tab_id,
                        stinger,
                        seconds: Some(duration.seconds()),
                    },
                )
                .await
                {
                    warn!("takeover failed: {error}");

                    return;
                }

                tokio::time::sleep(Duration::from(duration)).await;

                if state
                    .notifications
                    .current_in(NotificationMode::Takeover)
                    .await
                    .is_none()
                {
                    let _ = tell(&state.chrome, ChromeMessage::EndTakeover).await;
                }
            });
        }

        Ok(Json(MutationResult::applied()))
    }

    /// The configured clips, so the transition page can resolve a name to a file.
    #[oai(path = "/stingers", method = "get")]
    async fn stingers(&self) -> ApiResult<Json<Vec<StingerInfo>>> {
        let stingers = self.state.config.read().await.notifications.stingers;

        Ok(Json(
            stingers
                .into_iter()
                .map(|(name, stinger)| StingerInfo {
                    name,
                    file: stinger.file,
                })
                .collect(),
        ))
    }

    /// Everything currently showing.
    #[oai(path = "/notifications", method = "get")]
    async fn list(&self) -> ApiResult<Json<Vec<Notification>>> {
        Ok(Json(self.state.notifications.active().await))
    }

    /// Clear one early.
    #[oai(path = "/notifications/:notification_id", method = "delete")]
    async fn dismiss(
        &self,
        notification_id: Path<u64>,
        authorization: Authorization,
    ) -> ApiResult<Json<MutationResult>> {
        authorize_control(&self.state, &authorization)?;

        if !self.state.notifications.dismiss(notification_id.0).await {
            return Err(ApiError::not_found(notification_id.0));
        }

        if self
            .state
            .notifications
            .current_in(NotificationMode::Takeover)
            .await
            .is_none()
        {
            let _ = tell(&self.state.chrome, ChromeMessage::EndTakeover).await;
        }

        Ok(Json(MutationResult::applied()))
    }

    /// Open the rail if it is closed, close it if it is open. A rail closed this way stays closed
    /// until something new arrives for it.
    #[oai(path = "/sidebar/toggle", method = "post")]
    async fn toggle_sidebar(&self, authorization: Authorization) -> ApiResult<Json<SidebarState>> {
        authorize_control(&self.state, &authorization)?;

        let open = self.state.surfaces.toggle_sidebar(&self.state).await;

        Ok(Json(SidebarState { open }))
    }

    /// Whether the rail is up.
    #[oai(path = "/sidebar", method = "get")]
    async fn sidebar(&self) -> ApiResult<Json<SidebarState>> {
        Ok(Json(SidebarState {
            open: self.state.surfaces.sidebar.is_open().await,
        }))
    }

    /// Put the full-screen agenda on the display, or take it away and resume the playlist. The
    /// hold has no end: the agenda stays until the second call.
    #[oai(path = "/calendar/toggle", method = "post")]
    async fn toggle_calendar(
        &self,
        authorization: Authorization,
    ) -> ApiResult<Json<CalendarState>> {
        authorize_control(&self.state, &authorization)?;

        let showing = self
            .state
            .chrome
            .state
            .lock()
            .await
            .current_tab_id
            .as_deref()
            == Some(CALENDAR_TAB);

        let message = if showing {
            ChromeMessage::EndTakeover
        } else {
            ChromeMessage::Takeover {
                tab_id: Some(CALENDAR_TAB.to_string()),
                stinger: None,
                seconds: None,
            }
        };

        tell(&self.state.chrome, message)
            .await
            .map_err(ApiError::internal)?;

        Ok(Json(CalendarState { showing: !showing }))
    }

    /// The entries the configured feeds put in their window, as the agenda page reads them.
    #[oai(path = "/calendar/agenda", method = "get")]
    async fn agenda(&self) -> ApiResult<Json<Vec<Notification>>> {
        Ok(Json(calendar::agenda(&self.state).await))
    }
}
