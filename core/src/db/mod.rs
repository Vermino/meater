//! Database operations for persistent storage
//!
//! This module handles:
//! - SQLite database connections and schema management
//! - Data persistence for probes, sessions, and readings
//! - Query operations

mod repository;
mod sqlite;

// Re-export repository functions
pub use repository::{
    create_session, end_session, get_active_session, get_all_probes, get_probe_by_device_id,
    get_probe_by_id, get_probe_sessions, get_readings, get_session, save_probe, save_reading,
};

// Re-export schema functions
pub use sqlite::{create_pool, init_schema};

// Re-export SQLite pool type
pub use sqlx::sqlite::SqlitePool;
