use poem_openapi::{param::Header, Enum};
use serde::Serialize;
use tracing::warn;

use crate::{config::SecretRef, state::AppState};

use super::{ApiError, ApiResult};

pub type Authorization = Header<Option<String>>;

/// What the key a request carried lets it do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Enum)]
#[serde(rename_all = "lowercase")]
#[oai(rename_all = "lowercase")]
pub enum Access {
    /// Everything, including configuration. Also what every request gets when no admin key is
    /// configured.
    Admin,
    /// What is on screen, and nothing that edits configuration.
    Control,
    None,
}

impl Access {
    pub fn of(state: &AppState, presented: Option<&str>) -> Self {
        let Some(admin) = state.admin_key.as_deref() else {
            return Self::Admin;
        };

        let presented = presented
            .and_then(|value| value.strip_prefix("Bearer "))
            .unwrap_or_default()
            .as_bytes();

        if constant_time_eq(presented, admin.as_bytes()) {
            Self::Admin
        } else if state
            .control_key
            .as_deref()
            .is_some_and(|control| constant_time_eq(presented, control.as_bytes()))
        {
            Self::Control
        } else {
            Self::None
        }
    }
}

/// Only the admin key passes. Everything that edits configuration asks for this.
pub fn authorize(state: &AppState, presented: &Authorization) -> ApiResult<()> {
    match Access::of(state, presented.0.as_deref()) {
        Access::Admin => Ok(()),
        Access::Control | Access::None => Err(ApiError::unauthorized("admin key")),
    }
}

/// The admin key or the control key passes. Changes what is on screen, never configuration.
pub fn authorize_control(state: &AppState, presented: &Authorization) -> ApiResult<()> {
    match Access::of(state, presented.0.as_deref()) {
        Access::Admin | Access::Control => Ok(()),
        Access::None => Err(ApiError::unauthorized("admin or control key")),
    }
}

/// A webhook with a token accepts that token as its bearer and nothing else, even when no admin
/// key is configured: the token is the one credential its caller was given. A webhook without one
/// is a control route like any other.
pub fn authorize_webhook(
    state: &AppState,
    token: Option<&SecretRef>,
    presented: &Authorization,
) -> ApiResult<()> {
    let Some(token) = token else {
        return authorize_control(state, presented);
    };

    let expected = token.resolve().map_err(|error| {
        warn!("cannot read a webhook token: {error:#}");

        ApiError::internal("the webhook token cannot be read")
    })?;

    let presented = presented
        .0
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();

    if constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(ApiError::unauthorized("webhook token"))
    }
}

/// Comparing byte by byte with an early return leaks the length of the matching prefix through
/// timing.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn keys_match_only_when_identical() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secrez"));
    }

    #[test]
    fn a_length_difference_never_matches() {
        assert!(!constant_time_eq(b"secret", b"secretary"));
        assert!(!constant_time_eq(b"", b"secret"));
    }
}
