use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::notifications::SidebarMode;

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct SetSidebarModeRequest {
    pub mode: SidebarMode,
}
