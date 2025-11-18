//! BLE (Bluetooth Low Energy) module for Meater probe communication
//!
//! This module handles all BLE-related operations including:
//! - Discovering Meater probes
//! - Connecting to probes
//! - Reading temperature and battery data
//! - Managing connection state

mod client;

pub use client::{Client, Event, State};
