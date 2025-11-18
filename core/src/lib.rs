//! Meater Core Library
//!
//! This library provides the core functionality for interacting with Meater BLE thermometer probes.
//! It includes modules for BLE communication, data structures, and database operations.

/// BLE module for discovering and communicating with Meater probes
pub mod ble;

/// Data structures and models
pub mod data;

/// Database operations for persistent storage
pub mod db;
