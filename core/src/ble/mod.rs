//! BLE (Bluetooth Low Energy) module for Meater probe communication
//!
//! This module handles all BLE-related operations including:
//! - Discovering Meater probes
//! - Connecting to probes (single or multiple)
//! - Reading temperature and battery data
//! - Managing connection state

mod client;
mod manager;

// Single-probe client (legacy)
pub use client::{Client, Event, State};

// Multi-probe manager
pub use manager::{ProbeEvent, ProbeManager};
