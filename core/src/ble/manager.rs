//! Multi-probe manager for handling multiple simultaneous Meater probe connections
//!
//! This module provides the ProbeManager which can manage up to 4 concurrent
//! probe connections, handling discovery, connection, and disconnection events.

use anyhow::{anyhow, Context, Result};
use btleplug::api::{
    Central, CentralEvent, CharPropFlags, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use super::client::{Event, State, SERVICE_UUID};

/// Maximum number of simultaneous probe connections
const MAX_PROBES: usize = 4;

/// Represents a single probe connection with its monitoring task
struct ProbeConnection {
    /// The BLE peripheral device
    peripheral: Peripheral,
    /// Cancellation token to stop the monitoring task
    cancel_token: CancellationToken,
    /// Handle to the monitoring task
    task_handle: JoinHandle<()>,
    /// Device ID (address)
    device_id: String,
}

impl ProbeConnection {
    /// Create a new probe connection
    fn new(
        peripheral: Peripheral,
        device_id: String,
        sender: mpsc::Sender<ProbeEvent>,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let task_cancel = cancel_token.clone();
        let task_peripheral = peripheral.clone();
        let task_device_id = device_id.clone();

        // Spawn monitoring task for this probe
        let task_handle = tokio::spawn(async move {
            let id = task_device_id.clone();
            tokio::select! {
                _ = monitor_probe(task_peripheral, task_device_id, sender) => {
                    info!("Probe {} monitoring ended", id);
                }
                _ = task_cancel.cancelled() => {
                    debug!("Probe {} monitoring cancelled", id);
                }
            }
        });

        Self {
            peripheral,
            cancel_token,
            task_handle,
            device_id,
        }
    }

    /// Disconnect and cleanup this probe connection
    async fn disconnect(self) {
        info!("Disconnecting probe: {}", self.device_id);

        // Cancel the monitoring task
        self.cancel_token.cancel();

        // Disconnect the peripheral
        if let Err(e) = self.peripheral.disconnect().await {
            warn!("Error disconnecting probe {}: {}", self.device_id, e);
        }

        // Wait for task to complete
        if let Err(e) = self.task_handle.await {
            warn!("Error waiting for probe {} task: {}", self.device_id, e);
        }
    }
}

/// Events emitted by the ProbeManager
#[derive(Debug, Clone)]
pub enum ProbeEvent {
    /// A probe was discovered
    ProbeDiscovered { device_id: String },
    /// A probe connected successfully
    ProbeConnected { device_id: String },
    /// A probe disconnected
    ProbeDisconnected { device_id: String },
    /// Probe state changed
    ProbeStateChange { device_id: String, state: State },
    /// Temperature reading from a probe
    Temperature {
        device_id: String,
        tip: f32,
        ambient: f32,
    },
    /// Battery level from a probe
    Battery {
        device_id: String,
        percent: u16,
    },
    /// Error occurred with a probe
    ProbeError {
        device_id: String,
        error: String,
    },
}

/// Manages multiple Meater probe connections
pub struct ProbeManager {
    /// BLE adapter
    adapter: Adapter,
    /// Active probe connections
    connections: HashMap<String, ProbeConnection>,
    /// Event sender for all probe events
    event_sender: mpsc::Sender<ProbeEvent>,
    /// Cancellation token for the discovery task
    discovery_cancel: Option<CancellationToken>,
}

impl ProbeManager {
    /// Create a new ProbeManager
    ///
    /// # Returns
    /// * `Result<(Self, mpsc::Receiver<ProbeEvent>)>` - Manager and event receiver
    pub async fn new() -> Result<(Self, mpsc::Receiver<ProbeEvent>)> {
        info!("Initializing ProbeManager");

        let manager = Manager::new().await?;
        let adapter = manager
            .adapters()
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No Bluetooth adapter found"))?;

        let (event_sender, event_receiver) = mpsc::channel(100);

        Ok((
            Self {
                adapter,
                connections: HashMap::new(),
                event_sender,
                discovery_cancel: None,
            },
            event_receiver,
        ))
    }

    /// Start scanning for Meater probes
    ///
    /// This will automatically discover and connect to probes up to MAX_PROBES.
    pub async fn start_discovery(&mut self) -> Result<()> {
        if self.discovery_cancel.is_some() {
            warn!("Discovery already running");
            return Ok(());
        }

        info!("Starting probe discovery");

        let cancel_token = CancellationToken::new();
        self.discovery_cancel = Some(cancel_token.clone());

        let adapter = self.adapter.clone();
        let sender = self.event_sender.clone();

        // Start scanning for Meater probes
        adapter
            .start_scan(ScanFilter {
                services: vec![SERVICE_UUID],
            })
            .await
            .context("Failed to start BLE scan")?;

        // Spawn discovery task
        tokio::spawn(async move {
            if let Err(e) = run_discovery(adapter, sender, cancel_token).await {
                error!("Discovery task error: {}", e);
            }
        });

        Ok(())
    }

    /// Stop scanning for probes
    pub async fn stop_discovery(&mut self) -> Result<()> {
        info!("Stopping probe discovery");

        if let Some(cancel) = self.discovery_cancel.take() {
            cancel.cancel();
        }

        self.adapter.stop_scan().await?;
        Ok(())
    }

    /// Connect to a specific probe by device ID
    ///
    /// # Arguments
    /// * `device_id` - The BLE device ID/address
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub async fn connect_probe(&mut self, device_id: String) -> Result<()> {
        // Check if already connected
        if self.connections.contains_key(&device_id) {
            warn!("Probe {} already connected", device_id);
            return Ok(());
        }

        // Check connection limit
        if self.connections.len() >= MAX_PROBES {
            return Err(anyhow!(
                "Maximum number of probes ({}) already connected",
                MAX_PROBES
            ));
        }

        info!("Connecting to probe: {}", device_id);

        // Find the peripheral
        let peripherals = self.adapter.peripherals().await?;
        let peripheral = peripherals
            .iter()
            .find(|p| {
                p.address()
                    .to_string()
                    .to_lowercase()
                    == device_id.to_lowercase()
            })
            .ok_or_else(|| anyhow!("Probe {} not found", device_id))?
            .clone();

        // Verify it's a Meater device
        if !is_meater_device(&peripheral).await? {
            return Err(anyhow!("Device {} is not a Meater probe", device_id));
        }

        // Connect to the probe
        connect_to_probe(&peripheral).await?;

        // Create connection and start monitoring
        let connection = ProbeConnection::new(peripheral, device_id.clone(), self.event_sender.clone());

        self.connections.insert(device_id.clone(), connection);

        // Notify connection
        self.event_sender
            .send(ProbeEvent::ProbeConnected {
                device_id: device_id.clone(),
            })
            .await?;

        info!("Probe {} connected successfully", device_id);
        Ok(())
    }

    /// Disconnect a specific probe
    ///
    /// # Arguments
    /// * `device_id` - The BLE device ID/address
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub async fn disconnect_probe(&mut self, device_id: &str) -> Result<()> {
        info!("Disconnecting probe: {}", device_id);

        if let Some(connection) = self.connections.remove(device_id) {
            connection.disconnect().await;

            self.event_sender
                .send(ProbeEvent::ProbeDisconnected {
                    device_id: device_id.to_string(),
                })
                .await?;

            info!("Probe {} disconnected successfully", device_id);
            Ok(())
        } else {
            Err(anyhow!("Probe {} not connected", device_id))
        }
    }

    /// Disconnect all probes
    pub async fn disconnect_all(&mut self) -> Result<()> {
        info!("Disconnecting all probes");

        let device_ids: Vec<String> = self.connections.keys().cloned().collect();

        for device_id in device_ids {
            if let Err(e) = self.disconnect_probe(&device_id).await {
                error!("Error disconnecting probe {}: {}", device_id, e);
            }
        }

        Ok(())
    }

    /// Get the number of connected probes
    pub fn connected_count(&self) -> usize {
        self.connections.len()
    }

    /// Check if a probe is connected
    pub fn is_connected(&self, device_id: &str) -> bool {
        self.connections.contains_key(device_id)
    }

    /// Get list of connected probe device IDs
    pub fn connected_devices(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }
}

/// Run the discovery task
async fn run_discovery(
    adapter: Adapter,
    sender: mpsc::Sender<ProbeEvent>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut events = adapter.events().await?;

    loop {
        tokio::select! {
            Some(event) = events.next() => {
                match event {
                    CentralEvent::DeviceDiscovered(id) => {
                        let peripheral = adapter.peripheral(&id).await?;
                        if is_meater_device(&peripheral).await.unwrap_or(false) {
                            let device_id = peripheral.address().to_string();
                            info!("Discovered Meater probe: {}", device_id);

                            let _ = sender.send(ProbeEvent::ProbeDiscovered {
                                device_id,
                            }).await;
                        }
                    }
                    CentralEvent::DeviceDisconnected(id) => {
                        let device_id = id.to_string();
                        debug!("Device disconnected: {}", device_id);

                        let _ = sender.send(ProbeEvent::ProbeDisconnected {
                            device_id,
                        }).await;
                    }
                    _ => {}
                }
            }
            _ = cancel.cancelled() => {
                info!("Discovery task cancelled");
                break;
            }
        }
    }

    Ok(())
}

/// Monitor a single probe for notifications
async fn monitor_probe(peripheral: Peripheral, device_id: String, sender: mpsc::Sender<ProbeEvent>) {
    match peripheral.notifications().await {
        Ok(mut notifications) => {
            while let Some(notification) = notifications.next().await {
                match Event::try_from(notification) {
                    Ok(Event::Temperature { tip, ambient }) => {
                        let _ = sender
                            .send(ProbeEvent::Temperature {
                                device_id: device_id.clone(),
                                tip,
                                ambient,
                            })
                            .await;
                    }
                    Ok(Event::Battery { percent }) => {
                        let _ = sender
                            .send(ProbeEvent::Battery {
                                device_id: device_id.clone(),
                                percent,
                            })
                            .await;
                    }
                    Ok(Event::State(state)) => {
                        let _ = sender
                            .send(ProbeEvent::ProbeStateChange {
                                device_id: device_id.clone(),
                                state,
                            })
                            .await;
                    }
                    Err(e) => {
                        warn!("Error parsing notification for {}: {}", device_id, e);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to get notifications for {}: {}", device_id, e);
        }
    }
}

/// Check if a peripheral is a Meater device
async fn is_meater_device(peripheral: &Peripheral) -> Result<bool> {
    Ok(peripheral
        .properties()
        .await?
        .and_then(|props| props.local_name)
        .map(|name| name == "MEATER")
        .unwrap_or(false))
}

/// Connect to a probe and subscribe to notifications
async fn connect_to_probe(peripheral: &Peripheral) -> Result<()> {
    info!("Connecting to Meater probe");

    // Connect
    peripheral.connect().await.context("Failed to connect")?;

    // Discover services
    peripheral
        .discover_services()
        .await
        .context("Failed to discover services")?;

    // Subscribe to all notify characteristics
    for characteristic in peripheral.characteristics() {
        if characteristic.properties.contains(CharPropFlags::NOTIFY) {
            debug!("Subscribing to characteristic: {:?}", characteristic.uuid);
            peripheral
                .subscribe(&characteristic)
                .await
                .context("Failed to subscribe to characteristic")?;
        }
    }

    info!("Probe connected and subscribed to notifications");
    Ok(())
}
