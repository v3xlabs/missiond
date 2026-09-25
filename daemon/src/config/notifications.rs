use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{version, HumanDuration, NotificationMode, Stinger, Webhook};

/// `notifications.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationsDocument {
    #[serde(default = "version::current")]
    pub version: u32,
    #[serde(default)]
    pub mode: NotificationMode,
    #[serde(default = "default_duration")]
    pub default_duration: HumanDuration,
    /// How wide the rail is, in logical pixels. The daemon narrows the display by this much and
    /// gives the column the remainder, so the number is the one that reaches the screen.
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default = "default_toast_width")]
    pub toast_width: u32,
    #[serde(default = "default_toast_height")]
    pub toast_height: u32,
    /// Named clips a tab or a notification can ask for.
    #[serde(default)]
    pub stingers: HashMap<String, Stinger>,
    /// Alerts a caller raises by name, keyed by the name in `/api/webhooks/:name`.
    #[serde(default)]
    pub webhooks: HashMap<String, Webhook>,
}

fn default_duration() -> HumanDuration {
    HumanDuration(std::time::Duration::from_secs(20))
}

const fn default_sidebar_width() -> u32 {
    480
}

const fn default_toast_width() -> u32 {
    420
}

const fn default_toast_height() -> u32 {
    180
}

impl Default for NotificationsDocument {
    fn default() -> Self {
        Self {
            version: version::CURRENT,
            mode: NotificationMode::default(),
            default_duration: default_duration(),
            sidebar_width: default_sidebar_width(),
            toast_width: default_toast_width(),
            toast_height: default_toast_height(),
            stingers: HashMap::new(),
            webhooks: HashMap::new(),
        }
    }
}

impl NotificationsDocument {
    pub fn stinger(&self, name: &str) -> Option<&Stinger> {
        self.stingers.get(name)
    }
}
