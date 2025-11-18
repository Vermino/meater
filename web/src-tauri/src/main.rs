// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use meater_core::{
    ble::{ProbeEvent, ProbeManager, TemperatureLogger},
    db::{create_pool, init_schema},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ProbeStatus {
    id: String,
    name: String,
    tip_temp: f32,
    ambient_temp: f32,
    target_temp: Option<f32>,
    battery: u16,
    active: bool,
    color: String,
    cook_time: u64,
    status: String,
}

struct AppState {
    probe_statuses: Arc<RwLock<Vec<ProbeStatus>>>,
    db_pool: meater_core::db::SqlitePool,
    event_tx: broadcast::Sender<ProbeEvent>,
}

#[tauri::command]
async fn get_probes(state: State<'_, Arc<AppState>>) -> Result<Vec<ProbeStatus>, String> {
    let probes = state.probe_statuses.read().await;
    Ok(probes.clone())
}

#[tauri::command]
async fn start_monitoring(_state: State<'_, Arc<AppState>>) -> Result<(), String> {
    // Probes are auto-discovered and monitored
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize database
    let pool = create_pool("meater.db").await?;
    init_schema(&pool).await?;

    // Create broadcast channel
    let (event_tx, _event_rx) = broadcast::channel::<ProbeEvent>(100);

    // Initialize app state
    let app_state = Arc::new(AppState {
        probe_statuses: Arc::new(RwLock::new(vec![
            ProbeStatus {
                id: "probe-1".to_string(),
                name: "Probe 1".to_string(),
                tip_temp: 0.0,
                ambient_temp: 0.0,
                target_temp: None,
                battery: 0,
                active: false,
                color: "#ef4444".to_string(),
                cook_time: 0,
                status: "idle".to_string(),
            },
            ProbeStatus {
                id: "probe-2".to_string(),
                name: "Probe 2".to_string(),
                tip_temp: 0.0,
                ambient_temp: 0.0,
                target_temp: None,
                battery: 0,
                active: false,
                color: "#f59e0b".to_string(),
                cook_time: 0,
                status: "idle".to_string(),
            },
            ProbeStatus {
                id: "probe-3".to_string(),
                name: "Probe 3".to_string(),
                tip_temp: 0.0,
                ambient_temp: 0.0,
                target_temp: None,
                battery: 0,
                active: false,
                color: "#10b981".to_string(),
                cook_time: 0,
                status: "idle".to_string(),
            },
            ProbeStatus {
                id: "probe-4".to_string(),
                name: "Probe 4".to_string(),
                tip_temp: 0.0,
                ambient_temp: 0.0,
                target_temp: None,
                battery: 0,
                active: false,
                color: "#6b7280".to_string(),
                cook_time: 0,
                status: "idle".to_string(),
            },
        ])),
        db_pool: pool.clone(),
        event_tx: event_tx.clone(),
    });

    // Spawn probe manager
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        run_probe_manager(state_clone).await;
    });

    // Build Tauri app
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_probes, start_monitoring])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

async fn run_probe_manager(state: Arc<AppState>) {
    tracing::info!("Starting probe manager");

    // Initialize ProbeManager
    let (mut manager, event_rx) = match ProbeManager::new().await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create ProbeManager: {}", e);
            return;
        }
    };

    // Start discovery
    if let Err(e) = manager.start_discovery().await {
        tracing::error!("Failed to start discovery: {}", e);
        return;
    }

    // Initialize temperature logger
    let logger = TemperatureLogger::new(state.db_pool.clone(), event_rx);
    let cancel = CancellationToken::new();

    // Run logger
    tokio::spawn(async move {
        if let Err(e) = logger.run(cancel).await {
            tracing::error!("Logger error: {}", e);
        }
    });

    tracing::info!("Probe manager running");
}
