use poem_openapi::Object;
use serde::{Deserialize, Serialize};

use crate::notifications::SidebarMode;

/// Where the rail ended up, so a caller that changed it does not have to ask again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct SidebarState {
    pub open: bool,
    pub mode: SidebarMode,
}
