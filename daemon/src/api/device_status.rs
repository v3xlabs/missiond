use poem_openapi::Object;
use serde::Serialize;

use super::{auth::Access, SidebarState};

#[derive(Debug, Clone, Serialize, Object)]
pub struct DeviceStatus {
    pub device_id: String,
    pub device_name: String,
    pub current_playlist_id: Option<String>,
    pub current_tab_id: Option<String>,
    pub auto_rotate: bool,
    pub screen_on: bool,
    pub brightness: u32,
    pub uptime_seconds: u64,
    pub sidebar: SidebarState,
    pub current_tab_opened_at: Option<u64>,
    /// True when the config directory is managed elsewhere, so a change made here applies now
    /// but does not survive a restart.
    pub config_read_only: bool,
    /// Whether an admin key is configured at all. False means every mutation is open.
    pub requires_auth: bool,
    /// What the key this request carried allows. Lets the web UI show that a mutation will be
    /// refused before the reader clicks it.
    pub access: Access,
}
