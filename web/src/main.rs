//! Meater Web Server
//!
//! Web-based dashboard for monitoring Meater probe temperatures in real-time.

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, Response},
    routing::{get, get_service},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use meater_core::{
    ble::{ProbeEvent, ProbeManager, TemperatureLogger},
    db::{create_pool, get_all_probes, get_probe_sessions, get_readings, init_schema},
    data::ReadingsQuery,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing::{error, info};

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

#[derive(Clone, Serialize, Deserialize)]
struct DashboardState {
    probes: Vec<ProbeStatus>,
    connection_mode: String,
    timestamp: u64,
}

struct AppState {
    probe_statuses: Arc<RwLock<Vec<ProbeStatus>>>,
    db_pool: meater_core::db::SqlitePool,
    event_tx: broadcast::Sender<ProbeEvent>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Meater Web Server");

    // Initialize database
    let pool = create_pool("meater.db").await?;
    init_schema(&pool).await?;

    // Create broadcast channel for real-time updates
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

    // Spawn probe manager and logger
    tokio::spawn(run_probe_manager(
        app_state.clone(),
        event_tx.clone(),
    ));

    // Build the router
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/probes", get(get_probes))
        .route("/api/history/:probe_id", get(get_history))
        .route("/ws", get(websocket_handler))
        .nest_service(
            "/static",
            get_service(ServeDir::new("web/frontend/dist")).handle_error(|error| async move {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled error: {}", error),
                )
            }),
        )
        .nest_service(
            "/libs",
            get_service(ServeDir::new("web/frontend/libs")).handle_error(|error| async move {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled error: {}", error),
                )
            }),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("🌐 Server running at http://localhost:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../frontend/index.html"))
}

async fn get_probes(State(state): State<Arc<AppState>>) -> Json<Vec<ProbeStatus>> {
    let probes = state.probe_statuses.read().await;
    Json(probes.clone())
}

async fn get_history(
    State(state): State<Arc<AppState>>,
    Path(probe_id): Path<String>,
) -> Json<Vec<serde_json::Value>> {
    // Get probe from database
    let probes = get_all_probes(&state.db_pool).await.unwrap_or_default();

    let probe = probes.iter().find(|p| p.device_id == probe_id);

    if let Some(probe) = probe {
        if let Some(pid) = probe.id {
            // Get sessions
            let sessions = get_probe_sessions(&state.db_pool, pid)
                .await
                .unwrap_or_default();

            if let Some(session) = sessions.first() {
                if let Some(sid) = session.id {
                    // Get readings
                    let query = ReadingsQuery::for_session(sid).with_limit(100);
                    let readings = get_readings(&state.db_pool, &query)
                        .await
                        .unwrap_or_default();

                    let data: Vec<serde_json::Value> = readings
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "time": r.timestamp,
                                "tip": r.tip_temperature,
                                "ambient": r.ambient_temperature,
                            })
                        })
                        .collect();

                    return Json(data);
                }
            }
        }
    }

    Json(vec![])
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

async fn websocket_connection(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();
    let mut event_rx = state.event_tx.subscribe();

    // Send initial state
    let probes = state.probe_statuses.read().await;
    let initial_state = DashboardState {
        probes: probes.clone(),
        connection_mode: "ble".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    drop(probes);

    if let Ok(json) = serde_json::to_string(&initial_state) {
        let _ = sender.send(Message::Text(json)).await;
    }

    // Spawn task to receive client messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_msg)) = receiver.next().await {
            // Handle client messages if needed
        }
    });

    // Send updates to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            // Update probe statuses based on events
            let message = serde_json::json!({
                "type": "probe_event",
                "event": format!("{:?}", event),
            });

            if sender
                .send(Message::Text(message.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

async fn run_probe_manager(
    state: Arc<AppState>,
    _event_tx: broadcast::Sender<ProbeEvent>,
) {
    info!("Starting probe manager");

    // Initialize ProbeManager
    let (mut manager, event_rx) = match ProbeManager::new().await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to create ProbeManager: {}", e);
            return;
        }
    };

    // Start discovery
    if let Err(e) = manager.start_discovery().await {
        error!("Failed to start discovery: {}", e);
        return;
    }

    // Initialize temperature logger
    let logger = TemperatureLogger::new(state.db_pool.clone(), event_rx);
    let cancel = CancellationToken::new();

    // Run logger
    tokio::spawn(async move {
        if let Err(e) = logger.run(cancel).await {
            error!("Logger error: {}", e);
        }
    });

    info!("Probe manager running");
}
