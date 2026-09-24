use std::{net::IpAddr, time::Duration};

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tracing::{info, warn};

use crate::config::DeviceDocument;

const SERVICE_TYPE: &str = "_missiond._tcp.local.";

/// The API announced on the local network, so a controller can list displays without being told
/// their addresses.
pub struct Advertisement {
    daemon: ServiceDaemon,
}

impl Advertisement {
    /// Nothing is announced when `mdns` is off, or when the API listens on loopback and nobody
    /// who could hear the announcement could reach it.
    pub fn start(device: &DeviceDocument) -> Result<Option<Self>> {
        if !device.mdns {
            return Ok(None);
        }

        let bound = device.http.host.parse::<IpAddr>().ok();

        if bound.is_some_and(|address| address.is_loopback()) {
            info!("api is bound to loopback: not advertising over mdns");
            return Ok(None);
        }

        let host_name = format!("{}.local.", device.device_id);
        let properties = [
            ("device_id", device.device_id.as_str()),
            ("name", device.name.as_str()),
            ("version", env!("CARGO_PKG_VERSION")),
        ];

        let service = match bound.filter(|address| !address.is_unspecified()) {
            Some(address) => ServiceInfo::new(
                SERVICE_TYPE,
                &device.name,
                &host_name,
                address,
                device.http.port,
                &properties[..],
            )?,
            None => ServiceInfo::new(
                SERVICE_TYPE,
                &device.name,
                &host_name,
                "",
                device.http.port,
                &properties[..],
            )?
            .enable_addr_auto(),
        };

        let daemon = ServiceDaemon::new()?;

        daemon.register(service)?;
        info!(name = %device.name, port = device.http.port, "advertising over mdns");

        Ok(Some(Self { daemon }))
    }

    /// Sends the goodbye, so a listener drops the display now rather than when the record
    /// expires. The daemon thread sends it, so exiting without waiting can lose it.
    pub async fn stop(self) {
        match self.daemon.shutdown() {
            Ok(status) => {
                let _ = tokio::time::timeout(Duration::from_secs(1), status.recv_async()).await;
            }
            Err(error) => warn!("failed to stop the mdns advertisement: {error}"),
        }
    }
}
