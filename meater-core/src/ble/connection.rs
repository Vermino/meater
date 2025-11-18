use anyhow::anyhow;
use btleplug::api::{Central, Manager, Peripheral};
use btleplug::platform;
use std::time::Duration;
use uuid::Uuid;

use crate::data::{ProbeReading, BATTERY_UUID, TEMPERATURE_UUID};

const FIRMWARE_VERSION_UUID: Uuid =
    uuid::uuid!("00002a26-0000-1000-8000-00805f9b34fb"); // Standard Bluetooth firmware version characteristic

/// Connection to a single MEATER probe.
pub struct ProbeConnection {
    peripheral: platform::Peripheral,
    max_retries: u32,
    timeout: Duration,
}

impl ProbeConnection {
    /// Create a new probe connection with default settings (3 retries, 10 second timeout).
    pub async fn new(mac_address: &str) -> anyhow::Result<Self> {
        Self::with_settings(mac_address, 3, Duration::from_secs(10)).await
    }

    /// Create a new probe connection with custom settings.
    pub async fn with_settings(
        mac_address: &str,
        max_retries: u32,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let manager = platform::Manager::new().await?;
        let central = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("no bluetooth adapter found"))?;

        let peripherals = central.peripherals().await?;
        let peripheral = peripherals
            .into_iter()
            .find(|p| p.address().to_string() == mac_address)
            .ok_or(anyhow!("device with MAC address {} not found", mac_address))?;

        Ok(Self {
            peripheral,
            max_retries,
            timeout,
        })
    }

    /// Connect to the probe with retry logic.
    pub async fn connect(&self) -> anyhow::Result<()> {
        let mut attempts = 0;

        loop {
            attempts += 1;
            tracing::info!(attempt = attempts, "attempting to connect to MEATER");

            match tokio::time::timeout(self.timeout, self.peripheral.connect()).await {
                Ok(Ok(_)) => {
                    tracing::info!("successfully connected to MEATER");
                    break;
                }
                Ok(Err(err)) => {
                    tracing::error!("connection failed: {err}");
                    if attempts >= self.max_retries {
                        return Err(anyhow!("failed to connect after {} attempts", attempts));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(_) => {
                    tracing::error!("connection timeout");
                    if attempts >= self.max_retries {
                        return Err(anyhow!(
                            "connection timeout after {} attempts",
                            attempts
                        ));
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        tracing::debug!("discovering services");
        self.peripheral.discover_services().await?;

        Ok(())
    }

    /// Read the firmware version from the probe.
    pub async fn read_firmware_version(&self) -> anyhow::Result<String> {
        for characteristic in self.peripheral.characteristics() {
            if characteristic.uuid == FIRMWARE_VERSION_UUID {
                let data = self.peripheral.read(&characteristic).await?;
                return String::from_utf8(data)
                    .map_err(|e| anyhow!("invalid firmware version string: {}", e));
            }
        }
        Err(anyhow!("firmware version characteristic not found"))
    }

    /// Disconnect from the probe gracefully.
    pub async fn disconnect(&self) -> anyhow::Result<()> {
        tracing::info!("disconnecting from MEATER");
        self.peripheral
            .disconnect()
            .await
            .map_err(|e| anyhow!("failed to disconnect: {}", e))
    }

    /// Check if the probe is connected.
    pub async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }

    /// Read temperature data from the probe.
    pub async fn read_temperature(&self) -> anyhow::Result<ProbeReading> {
        let mut tip_temperature = None;
        let mut ambient_temperature = None;
        let mut battery_percent = None;

        for characteristic in self.peripheral.characteristics() {
            match characteristic.uuid {
                TEMPERATURE_UUID => {
                    let data = self.peripheral.read(&characteristic).await?;
                    if data.len() != 8 {
                        return Err(anyhow!(
                            "temperature does not contain correct number of bytes"
                        ));
                    }

                    let tip = to_u16(data[1], data[0]);
                    let ra = to_u16(data[3], data[2]);
                    let oa = to_u16(data[5], data[4]);
                    let ambient = tip + ((ra - 48.min(oa)) * 16 * 589) / 1487;

                    tip_temperature = Some(to_degree_celsius(tip));
                    ambient_temperature = Some(to_degree_celsius(ambient));
                }
                BATTERY_UUID => {
                    let data = self.peripheral.read(&characteristic).await?;
                    if data.len() < 2 {
                        return Err(anyhow!(
                            "battery does not contain correct number of bytes"
                        ));
                    }
                    battery_percent = Some(to_u16(data[1], data[0]) * 10);
                }
                _ => {}
            }
        }

        Ok(ProbeReading {
            tip_temperature: tip_temperature.ok_or(anyhow!("temperature not found"))?,
            ambient_temperature: ambient_temperature.ok_or(anyhow!("ambient temperature not found"))?,
            battery_percent: battery_percent.unwrap_or(0),
            timestamp: std::time::SystemTime::now(),
        })
    }
}

fn to_u16(msb: u8, lsb: u8) -> u16 {
    u16::from(msb) * 256 + u16::from(lsb)
}

fn to_degree_celsius(value: u16) -> f32 {
    (f32::from(value) + 8.0) / 16.0
}
