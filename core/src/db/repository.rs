//! Repository layer for data persistence operations
//!
//! This module provides methods to save and retrieve probe data, sessions, and readings.

use anyhow::{Context, Result};
use sqlx::{Row, SqlitePool};
use tracing::{debug, info};

use crate::data::{Probe, Reading, ReadingsQuery, Session};

/// Save or update a probe in the database
///
/// If the probe has an ID, it updates the existing record.
/// If the device_id already exists, it updates that record.
/// Otherwise, it inserts a new probe.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `probe` - Probe to save
///
/// # Returns
/// * `Result<i64>` - The probe ID
pub async fn save_probe(pool: &SqlitePool, probe: &Probe) -> Result<i64> {
    debug!("Saving probe: {:?}", probe);

    // Update last_seen timestamp
    let result = sqlx::query(
        r#"
        INSERT INTO probes (device_id, name, last_seen)
        VALUES (?, ?, CURRENT_TIMESTAMP)
        ON CONFLICT(device_id) DO UPDATE SET
            name = COALESCE(excluded.name, name),
            last_seen = CURRENT_TIMESTAMP
        RETURNING id
        "#,
    )
    .bind(&probe.device_id)
    .bind(&probe.name)
    .fetch_one(pool)
    .await
    .context("Failed to save probe")?;

    let id: i64 = result.get("id");
    info!("Probe saved with ID: {}", id);
    Ok(id)
}

/// Get a probe by its database ID
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Probe ID
///
/// # Returns
/// * `Result<Option<Probe>>` - The probe if found
pub async fn get_probe_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Probe>> {
    debug!("Fetching probe by ID: {}", id);

    let probe = sqlx::query_as::<_, Probe>("SELECT * FROM probes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch probe by ID")?;

    Ok(probe)
}

/// Get a probe by its device ID
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `device_id` - BLE device ID
///
/// # Returns
/// * `Result<Option<Probe>>` - The probe if found
pub async fn get_probe_by_device_id(pool: &SqlitePool, device_id: &str) -> Result<Option<Probe>> {
    debug!("Fetching probe by device_id: {}", device_id);

    let probe = sqlx::query_as::<_, Probe>("SELECT * FROM probes WHERE device_id = ?")
        .bind(device_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch probe by device_id")?;

    Ok(probe)
}

/// Get all probes from the database
///
/// # Arguments
/// * `pool` - Database connection pool
///
/// # Returns
/// * `Result<Vec<Probe>>` - List of all probes
pub async fn get_all_probes(pool: &SqlitePool) -> Result<Vec<Probe>> {
    debug!("Fetching all probes");

    let probes = sqlx::query_as::<_, Probe>("SELECT * FROM probes ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
        .context("Failed to fetch all probes")?;

    Ok(probes)
}

/// Create a new cooking session
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `session` - Session to create
///
/// # Returns
/// * `Result<i64>` - The session ID
pub async fn create_session(pool: &SqlitePool, session: &Session) -> Result<i64> {
    info!("Creating session for probe_id: {}", session.probe_id);

    let result = sqlx::query(
        r#"
        INSERT INTO sessions (probe_id, target_temperature, notes)
        VALUES (?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(session.probe_id)
    .bind(session.target_temperature)
    .bind(&session.notes)
    .fetch_one(pool)
    .await
    .context("Failed to create session")?;

    let id: i64 = result.get("id");
    info!("Session created with ID: {}", id);
    Ok(id)
}

/// End a cooking session by setting the ended_at timestamp
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `session_id` - Session to end
///
/// # Returns
/// * `Result<()>` - Success or error
pub async fn end_session(pool: &SqlitePool, session_id: i64) -> Result<()> {
    info!("Ending session: {}", session_id);

    sqlx::query("UPDATE sessions SET ended_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await
        .context("Failed to end session")?;

    info!("Session {} ended successfully", session_id);
    Ok(())
}

/// Get a session by ID
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `session_id` - Session ID
///
/// # Returns
/// * `Result<Option<Session>>` - The session if found
pub async fn get_session(pool: &SqlitePool, session_id: i64) -> Result<Option<Session>> {
    debug!("Fetching session: {}", session_id);

    let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch session")?;

    Ok(session)
}

/// Get the active session for a probe (where ended_at is NULL)
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `probe_id` - Probe ID
///
/// # Returns
/// * `Result<Option<Session>>` - The active session if found
pub async fn get_active_session(pool: &SqlitePool, probe_id: i64) -> Result<Option<Session>> {
    debug!("Fetching active session for probe: {}", probe_id);

    let session = sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE probe_id = ? AND ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
    )
    .bind(probe_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch active session")?;

    Ok(session)
}

/// Get all sessions for a probe
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `probe_id` - Probe ID
///
/// # Returns
/// * `Result<Vec<Session>>` - List of sessions
pub async fn get_probe_sessions(pool: &SqlitePool, probe_id: i64) -> Result<Vec<Session>> {
    debug!("Fetching sessions for probe: {}", probe_id);

    let sessions = sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE probe_id = ? ORDER BY started_at DESC",
    )
    .bind(probe_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch probe sessions")?;

    Ok(sessions)
}

/// Save a temperature reading to the database
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `reading` - Reading to save
///
/// # Returns
/// * `Result<i64>` - The reading ID
pub async fn save_reading(pool: &SqlitePool, reading: &Reading) -> Result<i64> {
    debug!("Saving reading: {:?}", reading);

    let result = sqlx::query(
        r#"
        INSERT INTO readings (session_id, probe_id, tip_temperature, ambient_temperature, battery_percent)
        VALUES (?, ?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(reading.session_id)
    .bind(reading.probe_id)
    .bind(reading.tip_temperature)
    .bind(reading.ambient_temperature)
    .bind(reading.battery_percent)
    .fetch_one(pool)
    .await
    .context("Failed to save reading")?;

    let id: i64 = result.get("id");
    Ok(id)
}

/// Get readings based on query parameters
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `query` - Query parameters
///
/// # Returns
/// * `Result<Vec<Reading>>` - List of readings matching the query
pub async fn get_readings(pool: &SqlitePool, query: &ReadingsQuery) -> Result<Vec<Reading>> {
    debug!("Fetching readings with query: {:?}", query);

    // Build dynamic query based on filters
    let mut sql = String::from("SELECT * FROM readings WHERE 1=1");

    if query.session_id.is_some() {
        sql.push_str(" AND session_id = ?");
    }

    if query.probe_id.is_some() {
        sql.push_str(" AND probe_id = ?");
    }

    if query.from_timestamp.is_some() {
        sql.push_str(" AND timestamp >= ?");
    }

    if query.to_timestamp.is_some() {
        sql.push_str(" AND timestamp <= ?");
    }

    sql.push_str(" ORDER BY timestamp DESC");

    if query.limit.is_some() {
        sql.push_str(" LIMIT ?");
    }

    // Bind parameters in order
    let mut db_query = sqlx::query_as::<_, Reading>(&sql);

    if let Some(session_id) = query.session_id {
        db_query = db_query.bind(session_id);
    }

    if let Some(probe_id) = query.probe_id {
        db_query = db_query.bind(probe_id);
    }

    if let Some(ref from) = query.from_timestamp {
        db_query = db_query.bind(from);
    }

    if let Some(ref to) = query.to_timestamp {
        db_query = db_query.bind(to);
    }

    if let Some(limit) = query.limit {
        db_query = db_query.bind(limit);
    }

    let readings = db_query
        .fetch_all(pool)
        .await
        .context("Failed to fetch readings")?;

    debug!("Found {} readings", readings.len());
    Ok(readings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_schema;

    async fn setup_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        init_schema(&pool).await?;
        Ok(pool)
    }

    #[tokio::test]
    async fn test_save_and_get_probe() -> Result<()> {
        let pool = setup_test_db().await?;

        let probe = Probe::new("test-device-001".to_string()).with_name("Test Probe".to_string());

        let probe_id = save_probe(&pool, &probe).await?;
        assert!(probe_id > 0);

        let fetched = get_probe_by_id(&pool, probe_id).await?;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().device_id, "test-device-001");

        Ok(())
    }

    #[tokio::test]
    async fn test_save_probe_upsert() -> Result<()> {
        let pool = setup_test_db().await?;

        // Insert probe
        let probe1 = Probe::new("test-device-002".to_string());
        let id1 = save_probe(&pool, &probe1).await?;

        // Update same probe (by device_id)
        let probe2 = Probe::new("test-device-002".to_string()).with_name("Updated Name".to_string());
        let id2 = save_probe(&pool, &probe2).await?;

        // Should be the same ID (upsert)
        assert_eq!(id1, id2);

        let fetched = get_probe_by_device_id(&pool, "test-device-002").await?;
        assert_eq!(fetched.unwrap().name, Some("Updated Name".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_end_session() -> Result<()> {
        let pool = setup_test_db().await?;

        // Create a probe first
        let probe = Probe::new("test-device-003".to_string());
        let probe_id = save_probe(&pool, &probe).await?;

        // Create a session
        let session = Session::new(probe_id)
            .with_target_temperature(65.0)
            .with_notes("Medium rare steak".to_string());

        let session_id = create_session(&pool, &session).await?;
        assert!(session_id > 0);

        // Verify it's active
        let active = get_active_session(&pool, probe_id).await?;
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, Some(session_id));

        // End the session
        end_session(&pool, session_id).await?;

        // Verify it's no longer active
        let active_after = get_active_session(&pool, probe_id).await?;
        assert!(active_after.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_save_and_get_readings() -> Result<()> {
        let pool = setup_test_db().await?;

        // Setup: create probe and session
        let probe = Probe::new("test-device-004".to_string());
        let probe_id = save_probe(&pool, &probe).await?;

        let session = Session::new(probe_id);
        let session_id = create_session(&pool, &session).await?;

        // Save multiple readings
        for i in 0..5 {
            let reading = Reading::new(session_id, probe_id, 20.0 + i as f64, 18.0 + i as f64)
                .with_battery(90 - i * 2);
            save_reading(&pool, &reading).await?;
        }

        // Query all readings for session
        let query = ReadingsQuery::for_session(session_id);
        let readings = get_readings(&pool, &query).await?;
        assert_eq!(readings.len(), 5);

        // Query with limit
        let query_limited = ReadingsQuery::for_session(session_id).with_limit(3);
        let limited_readings = get_readings(&pool, &query_limited).await?;
        assert_eq!(limited_readings.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_probe_sessions() -> Result<()> {
        let pool = setup_test_db().await?;

        let probe = Probe::new("test-device-005".to_string());
        let probe_id = save_probe(&pool, &probe).await?;

        // Create multiple sessions
        for i in 0..3 {
            let session = Session::new(probe_id)
                .with_target_temperature(60.0 + i as f64);
            create_session(&pool, &session).await?;
        }

        let sessions = get_probe_sessions(&pool, probe_id).await?;
        assert_eq!(sessions.len(), 3);

        Ok(())
    }
}
