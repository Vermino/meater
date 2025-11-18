//! Continuous temperature logger for Meater probes
//!
//! This module provides automatic temperature logging to the database,
//! capturing readings every second for each connected probe.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::data::{Probe, Reading};
use crate::db::{create_session, end_session, save_probe, save_reading, SqlitePool};

use super::manager::ProbeEvent;

/// Tracks logging state for a single probe
struct ProbeLoggingState {
    probe_id: i64,
    session_id: i64,
    last_reading: Option<TemperatureReading>,
    cancel_token: CancellationToken,
    task_handle: JoinHandle<()>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TemperatureReading {
    tip: f32,
    ambient: f32,
    battery: Option<u16>,
}

impl ProbeLoggingState {
    /// Start logging for a probe
    fn new(
        probe_id: i64,
        session_id: i64,
        pool: SqlitePool,
        device_id: String,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let task_cancel = cancel_token.clone();

        // Spawn background logging task
        let task_handle = tokio::spawn(async move {
            log_probe_temperatures(probe_id, session_id, pool, device_id, task_cancel).await;
        });

        Self {
            probe_id,
            session_id,
            last_reading: None,
            cancel_token,
            task_handle,
        }
    }

    /// Update the latest reading
    fn update_reading(&mut self, tip: f32, ambient: f32) {
        self.last_reading = Some(TemperatureReading {
            tip,
            ambient,
            battery: self.last_reading.as_ref().and_then(|r| r.battery),
        });
    }

    /// Update battery level
    fn update_battery(&mut self, percent: u16) {
        if let Some(ref mut reading) = self.last_reading {
            reading.battery = Some(percent);
        }
    }

    /// Stop logging for this probe
    async fn stop(self, pool: &SqlitePool) {
        info!("Stopping logging for probe {} session {}", self.probe_id, self.session_id);

        // Cancel the logging task
        self.cancel_token.cancel();

        // End the session
        if let Err(e) = end_session(pool, self.session_id).await {
            error!("Failed to end session {}: {}", self.session_id, e);
        }

        // Wait for task to complete
        if let Err(e) = self.task_handle.await {
            warn!("Error waiting for logging task: {}", e);
        }
    }
}

/// Continuous temperature logger that integrates with ProbeManager
pub struct TemperatureLogger {
    pool: SqlitePool,
    probe_states: HashMap<String, ProbeLoggingState>,
    event_receiver: Option<mpsc::Receiver<ProbeEvent>>,
}

impl TemperatureLogger {
    /// Create a new temperature logger
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `event_receiver` - Receiver for probe events from ProbeManager
    pub fn new(pool: SqlitePool, event_receiver: mpsc::Receiver<ProbeEvent>) -> Self {
        Self {
            pool,
            probe_states: HashMap::new(),
            event_receiver: Some(event_receiver),
        }
    }

    /// Start the logging service
    ///
    /// This will process probe events and automatically manage logging sessions.
    /// Runs until the event receiver is closed or cancellation is requested.
    ///
    /// # Arguments
    /// * `cancel` - Cancellation token for graceful shutdown
    pub async fn run(mut self, cancel: CancellationToken) -> Result<()> {
        info!("Starting temperature logger service");

        let mut receiver = self.event_receiver.take()
            .ok_or_else(|| anyhow::anyhow!("Event receiver already consumed"))?;

        loop {
            tokio::select! {
                Some(event) = receiver.recv() => {
                    if let Err(e) = self.handle_event(event).await {
                        error!("Error handling event: {}", e);
                    }
                }
                _ = cancel.cancelled() => {
                    info!("Temperature logger shutdown requested");
                    break;
                }
            }
        }

        // Cleanup: stop all active logging sessions
        self.shutdown().await?;

        info!("Temperature logger service stopped");
        Ok(())
    }

    /// Handle a probe event
    async fn handle_event(&mut self, event: ProbeEvent) -> Result<()> {
        match event {
            ProbeEvent::ProbeConnected { device_id } => {
                self.start_logging(&device_id).await?;
            }
            ProbeEvent::ProbeDisconnected { device_id } => {
                self.stop_logging(&device_id).await?;
            }
            ProbeEvent::Temperature { device_id, tip, ambient } => {
                self.update_temperature(&device_id, tip, ambient).await?;
            }
            ProbeEvent::Battery { device_id, percent } => {
                self.update_battery(&device_id, percent).await?;
            }
            _ => {
                // Ignore other events
            }
        }
        Ok(())
    }

    /// Start logging for a newly connected probe
    async fn start_logging(&mut self, device_id: &str) -> Result<()> {
        info!("Starting logging for probe: {}", device_id);

        // Save or update probe in database
        let probe = Probe::new(device_id.to_string());
        let probe_id = save_probe(&self.pool, &probe)
            .await
            .context("Failed to save probe")?;

        // Create a new session
        let session = crate::data::Session::new(probe_id);
        let session_id = create_session(&self.pool, &session)
            .await
            .context("Failed to create session")?;

        info!("Created session {} for probe {}", session_id, device_id);

        // Start logging task
        let state = ProbeLoggingState::new(
            probe_id,
            session_id,
            self.pool.clone(),
            device_id.to_string(),
        );

        self.probe_states.insert(device_id.to_string(), state);

        Ok(())
    }

    /// Stop logging for a disconnected probe
    async fn stop_logging(&mut self, device_id: &str) -> Result<()> {
        if let Some(state) = self.probe_states.remove(device_id) {
            info!("Stopping logging for probe: {}", device_id);
            state.stop(&self.pool).await;
        }
        Ok(())
    }

    /// Update temperature reading for a probe and save to database
    async fn update_temperature(&mut self, device_id: &str, tip: f32, ambient: f32) -> Result<()> {
        if let Some(state) = self.probe_states.get_mut(device_id) {
            state.update_reading(tip, ambient);
            debug!("Updated temperature for {}: tip={}, ambient={}", device_id, tip, ambient);

            // Save reading to database with retry logic
            let reading = Reading::new(
                state.session_id,
                state.probe_id,
                tip as f64,
                ambient as f64,
            );

            if let Some(ref battery) = state.last_reading.as_ref().and_then(|r| r.battery) {
                save_reading_with_retry(&self.pool, &reading.with_battery(*battery as i64), 3).await?;
            } else {
                save_reading_with_retry(&self.pool, &reading, 3).await?;
            }

            debug!("Saved reading for probe {} session {}", state.probe_id, state.session_id);
        }
        Ok(())
    }

    /// Update battery level for a probe
    async fn update_battery(&mut self, device_id: &str, percent: u16) -> Result<()> {
        if let Some(state) = self.probe_states.get_mut(device_id) {
            state.update_battery(percent);
            debug!("Updated battery for {}: {}%", device_id, percent);
        }
        Ok(())
    }

    /// Gracefully shutdown all logging sessions
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down temperature logger, stopping {} sessions", self.probe_states.len());

        let device_ids: Vec<String> = self.probe_states.keys().cloned().collect();

        for device_id in device_ids {
            if let Err(e) = self.stop_logging(&device_id).await {
                error!("Error stopping logging for {}: {}", device_id, e);
            }
        }

        Ok(())
    }
}

/// Background task placeholder (readings are logged immediately on events)
async fn log_probe_temperatures(
    probe_id: i64,
    session_id: i64,
    _pool: SqlitePool,
    _device_id: String,
    cancel: CancellationToken,
) {
    info!("Temperature logging task active for probe {} session {}", probe_id, session_id);

    // This task keeps the session active until cancelled
    // Actual logging happens immediately when temperature events are received
    // in the main event loop for better responsiveness
    cancel.cancelled().await;

    info!("Temperature logging task ended for probe {} session {}", probe_id, session_id);
}

/// Save a reading to the database with retry logic
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `reading` - Reading to save
/// * `max_retries` - Maximum number of retry attempts
async fn save_reading_with_retry(
    pool: &SqlitePool,
    reading: &Reading,
    max_retries: u32,
) -> Result<()> {
    let mut attempts = 0;

    loop {
        match save_reading(pool, reading).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e).context(format!(
                        "Failed to save reading after {} attempts",
                        max_retries
                    ));
                }

                warn!(
                    "Failed to save reading (attempt {}/{}): {}. Retrying...",
                    attempts, max_retries, e
                );

                // Exponential backoff: 100ms, 200ms, 400ms, etc.
                let delay = Duration::from_millis(100 * 2u64.pow(attempts - 1));
                tokio::time::sleep(delay).await;
            }
        }
    }
}
