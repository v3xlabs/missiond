pub mod auth;
pub mod calendar_state;
pub mod create_playlist_request;
pub mod device_status;
pub mod display;
pub mod error;
pub mod error_body;
pub mod mutation_result;
pub mod notifications;
pub mod notify_request;
pub mod playback;
pub mod playlist_info;
pub mod playlists;
pub mod power_state;
pub mod reorder_request;
pub mod rotation_state;
pub mod set_brightness_request;
pub mod set_enabled_request;
pub mod sidebar_state;
pub mod status;
pub mod stinger_info;
pub mod system;
pub mod tab_info;
pub mod tabs;
pub mod upsert_tab_request;

pub use calendar_state::CalendarState;
pub use create_playlist_request::CreatePlaylistRequest;
pub use device_status::DeviceStatus;
pub use error::{ApiError, ApiResult};
pub use error_body::ErrorBody;
pub use mutation_result::MutationResult;
pub use notify_request::NotifyRequest;
pub use playlist_info::PlaylistInfo;
pub use power_state::PowerState;
pub use reorder_request::ReorderRequest;
pub use rotation_state::RotationState;
pub use set_brightness_request::SetBrightnessRequest;
pub use set_enabled_request::SetEnabledRequest;
pub use sidebar_state::SidebarState;
pub use stinger_info::StingerInfo;
pub use tab_info::TabInfo;
pub use upsert_tab_request::UpsertTabRequest;

use std::sync::Arc;

use poem_openapi::OpenApiService;

use crate::state::AppState;

use self::{
    display::DisplayApi, notifications::NotificationApi, playback::PlaybackApi,
    playlists::PlaylistApi, status::StatusApi, system::SystemApi, tabs::TabApi,
};

type Apis = (
    StatusApi,
    PlaylistApi,
    TabApi,
    PlaybackApi,
    DisplayApi,
    SystemApi,
    NotificationApi,
);

pub fn create_api_service(state: Arc<AppState>) -> OpenApiService<Apis, ()> {
    let apis = (
        StatusApi {
            state: state.clone(),
        },
        PlaylistApi {
            state: state.clone(),
        },
        TabApi {
            state: state.clone(),
        },
        PlaybackApi {
            state: state.clone(),
        },
        DisplayApi {
            state: state.clone(),
        },
        SystemApi {
            state: state.clone(),
        },
        NotificationApi { state },
    );

    OpenApiService::new(apis, "Mission Control API", "0.1.0").server("/")
}
