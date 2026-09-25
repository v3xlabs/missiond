use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::api::SidebarState;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct DisplayEvent {
    pub device_id: String,
    pub device_name: String,
    pub current_playlist_id: Option<String>,
    pub current_tab_id: Option<String>,
    pub auto_rotate: bool,
    /// When rotation next steps, as Unix seconds. Nothing while rotation is stopped.
    pub next_rotation_at: Option<u64>,
    pub screen_on: bool,
    pub brightness: u32,
    pub sidebar: SidebarState,
}
