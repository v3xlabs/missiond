use serde::Serialize;

use crate::api::auth::Access;

use super::DisplayEvent;

/// The `state` frame of `/api/events`. The display is shared by every subscriber, the access
/// is what the key this subscriber connected with allows.
#[derive(Debug, Serialize)]
pub struct StateFrame<'a> {
    #[serde(flatten)]
    pub display: &'a DisplayEvent,
    /// Whether an admin key is configured at all. False means every mutation is open.
    pub requires_auth: bool,
    pub access: Access,
}
