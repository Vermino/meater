//! Data structures and models for Meater probes
//!
//! This module contains data structures for:
//! - Probe information
//! - Temperature readings
//! - Cooking sessions

use serde::{Deserialize, Serialize};

/// Represents a Meater probe device
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Probe {
    pub id: Option<i64>,
    pub device_id: String,
    pub name: Option<String>,
    pub created_at: Option<String>,
    pub last_seen: Option<String>,
}

impl Probe {
    /// Create a new probe with a device ID
    pub fn new(device_id: String) -> Self {
        Self {
            id: None,
            device_id,
            name: None,
            created_at: None,
            last_seen: None,
        }
    }

    /// Set the probe's friendly name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
}

/// Represents a cooking session
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Option<i64>,
    pub probe_id: i64,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub target_temperature: Option<f64>,
    pub notes: Option<String>,
}

impl Session {
    /// Create a new session for a probe
    pub fn new(probe_id: i64) -> Self {
        Self {
            id: None,
            probe_id,
            started_at: None,
            ended_at: None,
            target_temperature: None,
            notes: None,
        }
    }

    /// Set target temperature for the session
    pub fn with_target_temperature(mut self, temp: f64) -> Self {
        self.target_temperature = Some(temp);
        self
    }

    /// Add notes to the session
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
}

/// Represents a temperature reading from a probe
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reading {
    pub id: Option<i64>,
    pub session_id: i64,
    pub probe_id: i64,
    pub timestamp: Option<String>,
    pub tip_temperature: f64,
    pub ambient_temperature: f64,
    pub battery_percent: Option<i64>,
}

impl Reading {
    /// Create a new temperature reading
    pub fn new(
        session_id: i64,
        probe_id: i64,
        tip_temperature: f64,
        ambient_temperature: f64,
    ) -> Self {
        Self {
            id: None,
            session_id,
            probe_id,
            timestamp: None,
            tip_temperature,
            ambient_temperature,
            battery_percent: None,
        }
    }

    /// Set battery percentage
    pub fn with_battery(mut self, percent: i64) -> Self {
        self.battery_percent = Some(percent);
        self
    }
}

/// Query parameters for retrieving readings
#[derive(Debug, Clone, Default)]
pub struct ReadingsQuery {
    pub session_id: Option<i64>,
    pub probe_id: Option<i64>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
    pub limit: Option<i64>,
}

impl ReadingsQuery {
    /// Create a new query for a specific session
    pub fn for_session(session_id: i64) -> Self {
        Self {
            session_id: Some(session_id),
            ..Default::default()
        }
    }

    /// Create a new query for a specific probe
    pub fn for_probe(probe_id: i64) -> Self {
        Self {
            probe_id: Some(probe_id),
            ..Default::default()
        }
    }

    /// Set time range filter
    pub fn with_time_range(mut self, from: String, to: String) -> Self {
        self.from_timestamp = Some(from);
        self.to_timestamp = Some(to);
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }
}
