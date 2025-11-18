//! Database operations for persistent storage
//!
//! This module handles:
//! - SQLite database connections and schema management
//! - Data persistence for probes, sessions, and readings
//! - Query operations

mod sqlite;

pub use sqlite::{create_pool, init_schema};
pub use sqlx::sqlite::SqlitePool;
