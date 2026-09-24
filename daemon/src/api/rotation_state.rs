use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Whether rotation ended up running, so a caller that toggled it does not have to ask again.
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct RotationState {
    pub auto_rotate: bool,
}
