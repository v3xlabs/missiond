use poem_openapi::Enum;
use serde::{Deserialize, Serialize};

/// Who decides whether the rail is up. `Open` and `Closed` hold until the mode goes back to
/// `Auto`, whatever arrives in the meantime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, Enum)]
#[serde(rename_all = "lowercase")]
#[oai(rename_all = "lowercase")]
pub enum SidebarMode {
    /// Up while a sidebar notification is active, down otherwise.
    #[default]
    Auto,
    Open,
    Closed,
}
