pub mod device;
pub mod entity;

pub use device::HassDevice;
pub use entity::HassEntity;

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use reqwest::Url;
use rumqttc::{AsyncClient, Event, EventLoop, LastWill, MqttOptions, Packet, QoS};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::{
    chrome::{tell, ChromeMessage},
    config::{DeviceDocument, Tab},
    state::AppState,
};

pub struct HassManager {
    client: Option<AsyncClient>,
    event_loop: Mutex<Option<EventLoop>>,
    availability_topic: String,

    brightness_entity: HassEntity,
    backlight_entity: HassEntity,
    playlist_entity: HassEntity,
    tab_entity: HassEntity,
    url_entity: HassEntity,
}

impl HassManager {
    pub async fn new(device: &DeviceDocument) -> Result<Self> {
        let Some(config) = device.homeassistant.as_ref() else {
            return Ok(Self::disabled());
        };

        let url = config
            .mqtt_url
            .parse::<Url>()
            .with_context(|| format!("cannot parse mqtt_url {}", config.mqtt_url))?;
        let host = url.host_str().context("mqtt_url has no host")?.to_string();
        let port = url.port().unwrap_or(1883);

        let availability_topic = format!("homeassistant/device/{}/availability", device.device_id);

        let mut options = MqttOptions::new(format!("missiond-{}", device.device_id), host, port);

        options.set_keep_alive(Duration::from_secs(15));
        options.set_last_will(LastWill::new(
            &availability_topic,
            "offline",
            QoS::AtMostOnce,
            true,
        ));

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            options.set_credentials(username, &password.resolve()?);
            info!(username, "mqtt credentials set");
        }

        let (client, event_loop) = AsyncClient::new(options, 10);

        Ok(Self {
            client: Some(client),
            event_loop: Mutex::new(Some(event_loop)),
            availability_topic: availability_topic.clone(),
            brightness_entity: HassEntity::new_brightness(device, &availability_topic),
            backlight_entity: HassEntity::new_backlight(device, &availability_topic),
            playlist_entity: HassEntity::new_playlist(device, &availability_topic),
            tab_entity: HassEntity::new_tab(device, &availability_topic),
            url_entity: HassEntity::new_url(device, &availability_topic),
        })
    }

    fn disabled() -> Self {
        let device = DeviceDocument::default();
        let availability_topic = String::new();

        Self {
            client: None,
            event_loop: Mutex::new(None),
            availability_topic: availability_topic.clone(),
            brightness_entity: HassEntity::new_brightness(&device, &availability_topic),
            backlight_entity: HassEntity::new_backlight(&device, &availability_topic),
            playlist_entity: HassEntity::new_playlist(&device, &availability_topic),
            tab_entity: HassEntity::new_tab(&device, &availability_topic),
            url_entity: HassEntity::new_url(&device, &availability_topic),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    pub async fn publish_playlist_options(&self, playlists: Vec<String>, active: Option<&str>) {
        let Some(client) = &self.client else { return };
        let mut entity = self.playlist_entity.clone();

        entity.options = Some(playlists);
        entity.publish_config(client).await;

        if let Some(active) = active {
            entity.update_state(client, active).await;
        }
    }

    pub async fn publish_tab_options(&self, tabs: &[Tab], active: Option<&str>) {
        let Some(client) = &self.client else { return };
        let mut entity = self.tab_entity.clone();

        entity.options = Some(tabs.iter().map(|tab| tab.tab_id.clone()).collect());
        entity.publish_config(client).await;

        if let Some(active) = active {
            entity.update_state(client, active).await;
        }
    }

    pub async fn publish_url(&self, url: &str) {
        if let Some(client) = &self.client {
            self.url_entity.update_state(client, url).await;
        }
    }

    pub async fn publish_backlight(&self, on: bool) {
        if let Some(client) = &self.client {
            self.backlight_entity
                .update_state(client, if on { "ON" } else { "OFF" })
                .await;
        }
    }

    async fn announce(&self) {
        let Some(client) = &self.client else { return };

        let _ = client
            .publish(&self.availability_topic, QoS::AtLeastOnce, true, "online")
            .await;

        for entity in [
            &self.brightness_entity,
            &self.backlight_entity,
            &self.playlist_entity,
            &self.tab_entity,
            &self.url_entity,
        ] {
            entity.publish_config(client).await;
        }

        for entity in [
            &self.brightness_entity,
            &self.backlight_entity,
            &self.playlist_entity,
            &self.tab_entity,
        ] {
            entity.subscribe(client).await;
        }
    }

    pub async fn run(&self, app_state: Arc<AppState>) {
        let Some(mut event_loop) = self.event_loop.lock().await.take() else {
            return;
        };

        self.announce().await;

        loop {
            match event_loop.poll().await {
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    let payload = String::from_utf8_lossy(&publish.payload).to_string();

                    self.handle_command(&publish.topic, &payload, &app_state)
                        .await;
                }
                Ok(_) => {}
                Err(error) => {
                    warn!("mqtt error: {error}");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn handle_command(&self, topic: &str, payload: &str, app_state: &Arc<AppState>) {
        if topic == self.backlight_entity.command_topic {
            let on = payload.eq_ignore_ascii_case("ON");
            let display = app_state.config.read().await.display;

            if let Err(error) = app_state.display.set_power(&display, on).await {
                warn!("failed to set display power: {error}");
                return;
            }

            self.publish_backlight(on).await;
            app_state.events.publish(app_state).await;
        } else if topic == self.brightness_entity.command_topic {
            let Ok(fraction) = payload.parse::<f32>() else {
                warn!("brightness payload is not a number: {payload}");
                return;
            };

            let display = app_state.config.read().await.display;
            let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as u32;

            if let Err(error) = app_state.display.set_brightness(&display, percent).await {
                warn!("failed to set brightness: {error}");
                return;
            }

            if let Some(client) = &self.client {
                self.brightness_entity.update_state(client, payload).await;
            }

            app_state.events.publish(app_state).await;
        } else if topic == self.playlist_entity.command_topic {
            if let Err(error) = tell(
                &app_state.chrome,
                ChromeMessage::ActivatePlaylist {
                    playlist_id: payload.to_string(),
                },
            )
            .await
            {
                warn!("failed to activate playlist {payload}: {error}");
            }
        } else if topic == self.tab_entity.command_topic {
            let playlist_id = app_state
                .chrome
                .state
                .lock()
                .await
                .current_playlist_id
                .clone();

            if let Some(playlist_id) = playlist_id {
                if let Err(error) = tell(
                    &app_state.chrome,
                    ChromeMessage::ActivateTab {
                        tab_id: payload.to_string(),
                        playlist_id,
                    },
                )
                .await
                {
                    warn!("failed to activate tab {payload}: {error}");
                }
            }
        }
    }
}
