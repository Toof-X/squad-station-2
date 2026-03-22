use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Json, Router,
};
use axum_embed::ServeEmbed;
use rust_embed::Embed;
use serde::Serialize;
use std::io::ErrorKind;
use std::time::Instant;
use tokio::net::TcpListener;

#[derive(Embed, Clone)]
#[folder = "web/dist/"]
struct FrontendAssets;

#[derive(Clone)]
struct AppState {
    db: Option<sqlx::SqlitePool>,
    project_name: String,
    started_at: Instant,
}

/// Bind a TCP listener with port fallback logic:
/// - No explicit port: try 3000, fall back to random port if taken
/// - Explicit port: use exactly that port; error if taken (no fallback)
async fn bind_listener(explicit_port: Option<u16>) -> anyhow::Result<TcpListener> {
    match explicit_port {
        None => {
            // Try port 3000 first
            match TcpListener::bind("127.0.0.1:3000").await {
                Ok(listener) => Ok(listener),
                Err(e) if e.kind() == ErrorKind::AddrInUse => {
                    eprintln!("Port 3000 is in use, falling back to random port...");
                    Ok(TcpListener::bind("127.0.0.1:0").await?)
                }
                Err(e) => Err(e.into()),
            }
        }
        Some(port) => {
            match TcpListener::bind(format!("127.0.0.1:{port}")).await {
                Ok(listener) => Ok(listener),
                Err(e) if e.kind() == ErrorKind::AddrInUse => {
                    anyhow::bail!("Port {port} is already in use. Choose a different port or omit --port to use automatic fallback.");
                }
                Err(e) => Err(e.into()),
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Ctrl+C handler failed");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler failed")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Close(_) => break,
            _ => {
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[derive(Serialize)]
struct StatusResponse {
    project: String,
    agents: usize,
    uptime_secs: u64,
    version: String,
}

async fn api_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let agent_count = match &state.db {
        Some(pool) => crate::db::agents::list_agents(pool)
            .await
            .map(|a| a.len())
            .unwrap_or(0),
        None => 0,
    };
    Json(StatusResponse {
        project: state.project_name.clone(),
        agents: agent_count,
        uptime_secs: state.started_at.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn run(port: Option<u16>, no_open: bool) -> anyhow::Result<()> {
    use std::path::Path;

    // Load config — gracefully degrade if not in a squad project
    let (project_name, db) = match crate::config::load_config(Path::new(
        crate::config::DEFAULT_CONFIG_FILE,
    )) {
        Ok(config) => {
            let project = config.project.clone();
            let db = match crate::config::resolve_db_path(&config) {
                Ok(db_path) => match crate::db::connect_readonly(&db_path).await {
                    Ok(pool) => {
                        Some(pool)
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not connect to DB: {e} (continuing without DB)");
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Could not resolve DB path: {e} (continuing without DB)");
                    None
                }
            };
            (project, db)
        }
        Err(e) => {
            eprintln!("Warning: Could not load squad config: {e} (continuing without DB)");
            ("unknown".to_string(), None)
        }
    };

    let listener = bind_listener(port).await?;
    let actual_port = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{actual_port}");

    println!("Squad Station browser at {url}");

    if !no_open {
        if let Err(e) = open::that(&url) {
            eprintln!("Warning: Could not open browser: {e}");
        }
    }

    let state = AppState {
        db,
        project_name,
        started_at: Instant::now(),
    };

    // IMPORTANT: explicit routes (/api/status, /ws) MUST come before nest_service("/")
    // to ensure they take priority over the SPA fallback handler
    let app = Router::new()
        .route("/api/status", get(api_status))
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeEmbed::<FrontendAssets>::new())
        .with_state(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");
    Ok(())
}
