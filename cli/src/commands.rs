//! CLI commands for session management
//!
//! This module provides command-line tools for managing cooking sessions,
//! viewing history, and exporting data.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use meater_core::data::{Probe, ReadingsQuery, Session};
use meater_core::db::{
    create_pool, create_session, end_session, get_active_session, get_all_probes,
    get_probe_by_device_id, get_probe_sessions, get_readings, get_session, init_schema,
};
use std::path::PathBuf;

/// Meater CLI for session management
#[derive(Parser)]
#[command(name = "meater")]
#[command(about = "Meater probe session management CLI", long_about = None)]
pub struct Cli {
    /// Path to the SQLite database file
    #[arg(short, long, default_value = "meater.db")]
    pub database: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a new cooking session for a probe
    Start {
        /// Device ID of the probe (BLE address)
        device_id: String,

        /// Target temperature (optional)
        #[arg(short, long)]
        target: Option<f64>,

        /// Notes for this session
        #[arg(short, long)]
        notes: Option<String>,
    },

    /// Stop an active cooking session
    Stop {
        /// Device ID of the probe
        device_id: String,
    },

    /// List active sessions
    List {
        /// Show all sessions (not just active)
        #[arg(short, long)]
        all: bool,
    },

    /// View session history
    History {
        /// Device ID to filter by (optional)
        #[arg(short, long)]
        device: Option<String>,

        /// Limit number of sessions shown
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Export session data to CSV
    Export {
        /// Session ID to export
        session_id: i64,

        /// Output file path
        #[arg(short, long, default_value = "export.csv")]
        output: PathBuf,
    },

    /// List all registered probes
    Probes,
}

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Initialize database
    let pool = create_pool(&cli.database)
        .await
        .context("Failed to create database connection")?;

    init_schema(&pool)
        .await
        .context("Failed to initialize database schema")?;

    // Execute command
    match cli.command {
        Commands::Start {
            device_id,
            target,
            notes,
        } => cmd_start(&pool, device_id, target, notes).await?,

        Commands::Stop { device_id } => cmd_stop(&pool, device_id).await?,

        Commands::List { all } => cmd_list(&pool, all).await?,

        Commands::History { device, limit } => cmd_history(&pool, device, limit).await?,

        Commands::Export {
            session_id,
            output,
        } => cmd_export(&pool, session_id, output).await?,

        Commands::Probes => cmd_probes(&pool).await?,
    }

    Ok(())
}

/// Start a new session
async fn cmd_start(
    pool: &meater_core::db::SqlitePool,
    device_id: String,
    target: Option<f64>,
    notes: Option<String>,
) -> Result<()> {
    // Get or create probe
    let probe = match get_probe_by_device_id(pool, &device_id).await? {
        Some(p) => p,
        None => {
            println!("Creating new probe: {}", device_id);
            let probe = Probe::new(device_id.clone());
            let probe_id = meater_core::db::save_probe(pool, &probe).await?;
            Probe {
                id: Some(probe_id),
                device_id,
                ..probe
            }
        }
    };

    let probe_id = probe.id.unwrap();

    // Check for active session
    if let Some(active) = get_active_session(pool, probe_id).await? {
        anyhow::bail!(
            "Probe {} already has an active session (ID: {})",
            probe.device_id,
            active.id.unwrap()
        );
    }

    // Create session
    let mut session = Session::new(probe_id);
    if let Some(t) = target {
        session = session.with_target_temperature(t);
    }
    if let Some(n) = notes {
        session = session.with_notes(n);
    }

    let session_id = create_session(pool, &session).await?;

    println!("✓ Started session {} for probe {}", session_id, probe.device_id);
    if let Some(t) = target {
        println!("  Target temperature: {}°C", t);
    }

    Ok(())
}

/// Stop an active session
async fn cmd_stop(pool: &meater_core::db::SqlitePool, device_id: String) -> Result<()> {
    // Find probe
    let probe = get_probe_by_device_id(pool, &device_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Probe {} not found", device_id))?;

    let probe_id = probe.id.unwrap();

    // Find active session
    let session = get_active_session(pool, probe_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No active session for probe {}", device_id))?;

    let session_id = session.id.unwrap();

    // End session
    end_session(pool, session_id).await?;

    println!("✓ Stopped session {} for probe {}", session_id, device_id);

    Ok(())
}

/// List sessions
async fn cmd_list(pool: &meater_core::db::SqlitePool, all: bool) -> Result<()> {
    let probes = get_all_probes(pool).await?;

    if probes.is_empty() {
        println!("No probes registered");
        return Ok(());
    }

    println!("\n{:<15} {:<10} {:<12} {:<20} {:<20}", "Device ID", "Session", "Status", "Started", "Target");
    println!("{}", "-".repeat(80));

    for probe in probes {
        let probe_id = probe.id.unwrap();
        let sessions = get_probe_sessions(pool, probe_id).await?;

        for session in sessions {
            let session_id = session.id.unwrap();
            let status = if session.ended_at.is_none() {
                "Active"
            } else if all {
                "Ended"
            } else {
                continue; // Skip ended sessions unless --all
            };

            let started = session.started_at.as_deref().unwrap_or("N/A");
            let target = session
                .target_temperature
                .map(|t| format!("{}°C", t))
                .unwrap_or_else(|| "N/A".to_string());

            println!(
                "{:<15} {:<10} {:<12} {:<20} {:<20}",
                probe.device_id, session_id, status, started, target
            );
        }
    }

    println!();
    Ok(())
}

/// Show session history
async fn cmd_history(
    pool: &meater_core::db::SqlitePool,
    device: Option<String>,
    limit: usize,
) -> Result<()> {
    let probes = if let Some(dev_id) = device {
        vec![get_probe_by_device_id(pool, &dev_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Probe {} not found", dev_id))?]
    } else {
        get_all_probes(pool).await?
    };

    println!("\n{:<10} {:<15} {:<20} {:<20} {:<12} {:<30}", "Session", "Device", "Started", "Ended", "Duration", "Notes");
    println!("{}", "-".repeat(110));

    let mut count = 0;
    for probe in probes {
        let probe_id = probe.id.unwrap();
        let sessions = get_probe_sessions(pool, probe_id).await?;

        for session in sessions {
            if count >= limit {
                break;
            }

            let started = session.started_at.as_deref().unwrap_or("N/A");
            let ended = session.ended_at.as_deref().unwrap_or("Active");

            // Calculate duration if ended
            let duration = if session.ended_at.is_some() {
                "Calculated" // Would need proper datetime parsing
            } else {
                "Ongoing"
            };

            let notes = session
                .notes
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(30)
                .collect::<String>();

            println!(
                "{:<10} {:<15} {:<20} {:<20} {:<12} {:<30}",
                session.id.unwrap(),
                probe.device_id,
                started,
                ended,
                duration,
                notes
            );

            count += 1;
        }

        if count >= limit {
            break;
        }
    }

    println!("\nShowing {} of available sessions\n", count.min(limit));
    Ok(())
}

/// Export session data to CSV
async fn cmd_export(
    pool: &meater_core::db::SqlitePool,
    session_id: i64,
    output: PathBuf,
) -> Result<()> {
    // Verify session exists
    let _session = get_session(pool, session_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;

    // Get readings
    let query = ReadingsQuery::for_session(session_id);
    let readings = get_readings(pool, &query).await?;

    if readings.is_empty() {
        println!("No readings found for session {}", session_id);
        return Ok(());
    }

    // Write CSV
    let mut wtr = csv::Writer::from_path(&output)
        .context("Failed to create CSV file")?;

    // Write header
    wtr.write_record(&[
        "timestamp",
        "tip_temperature",
        "ambient_temperature",
        "battery_percent",
    ])?;

    // Write readings
    for reading in &readings {
        wtr.write_record(&[
            reading.timestamp.as_deref().unwrap_or("N/A"),
            &reading.tip_temperature.to_string(),
            &reading.ambient_temperature.to_string(),
            &reading
                .battery_percent
                .map(|b| b.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
        ])?;
    }

    wtr.flush()?;

    println!(
        "✓ Exported {} readings from session {} to {}",
        readings.len(),
        session_id,
        output.display()
    );

    Ok(())
}

/// List all probes
async fn cmd_probes(pool: &meater_core::db::SqlitePool) -> Result<()> {
    let probes = get_all_probes(pool).await?;

    if probes.is_empty() {
        println!("No probes registered");
        return Ok(());
    }

    println!("\n{:<15} {:<20} {:<20} {:<20}", "Device ID", "Name", "Created", "Last Seen");
    println!("{}", "-".repeat(80));

    for probe in probes {
        let name = probe.name.as_deref().unwrap_or("N/A");
        let created = probe.created_at.as_deref().unwrap_or("N/A");
        let last_seen = probe.last_seen.as_deref().unwrap_or("Never");

        println!(
            "{:<15} {:<20} {:<20} {:<20}",
            probe.device_id, name, created, last_seen
        );
    }

    println!();
    Ok(())
}
