//! SQLite database implementation for Meater probe data persistence
//!
//! This module provides database schema and operations for storing:
//! - Probe information
//! - Cooking sessions
//! - Temperature readings

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::info;

/// Initialize the database schema
///
/// Creates all required tables (probes, sessions, readings) with appropriate
/// foreign key constraints and indexes.
///
/// # Arguments
/// * `pool` - SQLite connection pool
///
/// # Returns
/// * `Result<()>` - Success or error during schema initialization
pub async fn init_schema(pool: &SqlitePool) -> Result<()> {
    info!("Initializing database schema");

    // Enable foreign key support (not enabled by default in SQLite)
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(pool)
        .await
        .context("Failed to enable foreign keys")?;

    // Create probes table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS probes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT UNIQUE NOT NULL,
            name TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen DATETIME,
            CONSTRAINT unique_device_id UNIQUE (device_id)
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create probes table")?;

    // Create sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            probe_id INTEGER NOT NULL,
            started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            ended_at DATETIME,
            target_temperature REAL,
            notes TEXT,
            FOREIGN KEY (probe_id) REFERENCES probes(id) ON DELETE CASCADE,
            CONSTRAINT valid_session_times CHECK (ended_at IS NULL OR ended_at >= started_at)
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create sessions table")?;

    // Create readings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS readings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            probe_id INTEGER NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            tip_temperature REAL NOT NULL,
            ambient_temperature REAL NOT NULL,
            battery_percent INTEGER,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (probe_id) REFERENCES probes(id) ON DELETE CASCADE,
            CONSTRAINT valid_battery CHECK (battery_percent IS NULL OR (battery_percent >= 0 AND battery_percent <= 100)),
            CONSTRAINT valid_temperatures CHECK (tip_temperature > -273.15 AND ambient_temperature > -273.15)
        );
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create readings table")?;

    // Create indexes for efficient queries
    create_indexes(pool).await?;

    info!("Database schema initialized successfully");
    Ok(())
}

/// Create database indexes for optimized queries
async fn create_indexes(pool: &SqlitePool) -> Result<()> {
    info!("Creating database indexes");

    // Index on readings for session-based queries (most common query pattern)
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_readings_session_timestamp
        ON readings(session_id, timestamp DESC);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create readings session index")?;

    // Index on readings for probe-based queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_readings_probe
        ON readings(probe_id, timestamp DESC);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create readings probe index")?;

    // Index on sessions for probe-based queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_probe_started
        ON sessions(probe_id, started_at DESC);
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create sessions probe index")?;

    // Index on sessions for active session queries
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_active
        ON sessions(probe_id, ended_at)
        WHERE ended_at IS NULL;
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create active sessions index")?;

    info!("Database indexes created successfully");
    Ok(())
}

/// Create a new SQLite connection pool
///
/// # Arguments
/// * `database_path` - Path to the SQLite database file
///
/// # Returns
/// * `Result<SqlitePool>` - Connection pool or error
pub async fn create_pool<P: AsRef<Path>>(database_path: P) -> Result<SqlitePool> {
    let path = database_path.as_ref();
    info!("Creating database connection pool for: {}", path.display());

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // Enable foreign keys for this connection
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .context("Failed to create database connection pool")?;

    info!("Database connection pool created successfully");
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_schema() -> Result<()> {
        // Create an in-memory database for testing
        let pool = SqlitePool::connect("sqlite::memory:").await?;

        // Initialize the schema
        init_schema(&pool).await?;

        // Verify tables were created
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;",
        )
        .fetch_all(&pool)
        .await?;

        let table_names: Vec<String> = tables.into_iter().map(|(name,)| name).collect();

        assert!(table_names.contains(&"probes".to_string()));
        assert!(table_names.contains(&"sessions".to_string()));
        assert!(table_names.contains(&"readings".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_foreign_key_constraints() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        init_schema(&pool).await?;

        // Insert a probe
        sqlx::query("INSERT INTO probes (device_id) VALUES (?)")
            .bind("test-device-001")
            .execute(&pool)
            .await?;

        // Try to insert a session with non-existent probe_id (should fail)
        let result = sqlx::query("INSERT INTO sessions (probe_id) VALUES (?)")
            .bind(999) // Non-existent probe_id
            .execute(&pool)
            .await;

        assert!(result.is_err(), "Foreign key constraint should prevent invalid probe_id");

        Ok(())
    }

    #[tokio::test]
    async fn test_indexes_created() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        init_schema(&pool).await?;

        // Verify indexes were created
        let indexes: Vec<(String,)> = sqlx::query_as(
            "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name;",
        )
        .fetch_all(&pool)
        .await?;

        let index_names: Vec<String> = indexes.into_iter().map(|(name,)| name).collect();

        assert!(index_names.contains(&"idx_readings_session_timestamp".to_string()));
        assert!(index_names.contains(&"idx_readings_probe".to_string()));
        assert!(index_names.contains(&"idx_sessions_probe_started".to_string()));
        assert!(index_names.contains(&"idx_sessions_active".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_pool() -> Result<()> {
        // Create a temporary database file
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_meater.db");

        // Clean up if exists
        let _ = std::fs::remove_file(&db_path);

        // Create pool
        let pool = create_pool(&db_path).await?;

        // Verify we can execute a query
        sqlx::query("SELECT 1").execute(&pool).await?;

        // Clean up
        drop(pool);
        let _ = std::fs::remove_file(&db_path);

        Ok(())
    }
}
