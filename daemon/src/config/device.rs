use serde::{Deserialize, Serialize};

use super::{version, ChromiumConfig, HomeAssistantConfig, HttpConfig, MpvConfig, SecretRef};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceDocument {
    #[serde(default = "version::current")]
    pub version: u32,
    pub name: String,
    pub device_id: String,
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_key: Option<SecretRef>,
    /// Changes what is on screen and nothing else, for a button panel that must not be able to
    /// edit configuration. Only checked when `admin_key` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_key: Option<SecretRef>,
    /// Advertise the API as `_missiond._tcp` so a controller on the network finds it unasked.
    #[serde(default = "advertise_by_default")]
    pub mdns: bool,
    #[serde(default)]
    pub chromium: ChromiumConfig,
    #[serde(default)]
    pub mpv: MpvConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homeassistant: Option<HomeAssistantConfig>,
}

impl Default for DeviceDocument {
    fn default() -> Self {
        Self {
            version: version::CURRENT,
            name: "Mission Control".to_string(),
            device_id: "missiond".to_string(),
            http: HttpConfig::default(),
            admin_key: None,
            control_key: None,
            mdns: true,
            chromium: ChromiumConfig::default(),
            mpv: MpvConfig::default(),
            homeassistant: None,
        }
    }
}

const fn advertise_by_default() -> bool {
    true
}
