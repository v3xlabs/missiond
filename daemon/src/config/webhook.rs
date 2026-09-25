use serde::{Deserialize, Serialize};

use crate::notifications::{Level, Notification};

use super::{HumanDuration, NotificationMode, NotificationsDocument, SecretRef};

/// An alert raised by `POST /api/webhooks/:name`. The request body is never read, so any service
/// that can call a URL can raise one, whatever payload it sends.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Webhook {
    /// The bearer this webhook alone accepts, so its caller holds no key to anything else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<SecretRef>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<NotificationMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<HumanDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stinger: Option<String>,
}

impl Webhook {
    /// Keyed by the webhook's name, so a doorbell pressed twice replaces its alert rather than
    /// stacking a second one beside it.
    pub fn notification(
        &self,
        name: &str,
        defaults: &NotificationsDocument,
    ) -> (Notification, HumanDuration) {
        let duration = self.duration.unwrap_or(defaults.default_duration);

        (
            Notification {
                notification_id: 0,
                key: Some(format!("webhook:{name}")),
                title: self.title.clone(),
                body: self.body.clone(),
                level: self.level.unwrap_or_default(),
                mode: self.mode.unwrap_or(defaults.mode),
                expires_in_seconds: duration.seconds(),
                starts_at: None,
                ends_at: None,
                location: None,
                meeting: None,
                tab_id: self.tab_id.clone(),
                stinger: self.stinger.clone(),
            },
            duration,
        )
    }
}
