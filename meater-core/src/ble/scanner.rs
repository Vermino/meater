use anyhow::anyhow;
use btleplug::api::{Central, Manager, Peripheral, ScanFilter};
use btleplug::platform;
use std::time::Duration;

use crate::data::SERVICE_UUID;

/// BLE scanner for discovering MEATER probes.
pub struct Scanner {
    timeout: Duration,
}

impl Scanner {
    /// Create a new scanner with the given timeout.
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Create a new scanner with default 30 second timeout.
    pub fn with_default_timeout() -> Self {
        Self::new(Duration::from_secs(30))
    }

    /// Scan for MEATER devices and return their MAC addresses.
    pub async fn scan(&self) -> anyhow::Result<Vec<String>> {
        tracing::info!("starting BLE scan for MEATER devices");

        let manager = platform::Manager::new().await?;
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("no bluetooth adapter found"))?;

        central
            .start_scan(ScanFilter {
                services: vec![SERVICE_UUID],
            })
            .await?;

        tracing::debug!("scan started, waiting for timeout");
        tokio::time::sleep(self.timeout).await;

        central.stop_scan().await?;

        let mut mac_addresses = Vec::new();

        for peripheral in central.peripherals().await? {
            if let Some(properties) = peripheral.properties().await? {
                if let Some(name) = properties.local_name {
                    if name == "MEATER" {
                        let addr = peripheral.address().to_string();
                        tracing::info!(address = %addr, "found MEATER device");
                        mac_addresses.push(addr);
                    }
                }
            }
        }

        Ok(mac_addresses)
    }
}
