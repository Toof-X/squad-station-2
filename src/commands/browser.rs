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
use futures::{SinkExt, StreamExt};
use rust_embed::Embed;
use serde::Serialize;
use std::io::ErrorKind;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::{db, tmux};

#[derive(Embed, Clone)]
#[folder = "web/dist/"]
struct FrontendAssets;

#[derive(Clone)]
struct AppState {
    db: Option<sqlx::SqlitePool>,
    db_write: Option<sqlx::SqlitePool>,
    project_name: String,
    started_at: Instant,
    tx: broadcast::Sender<String>,
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

/// Build a full JSON snapshot of agents + messages for the initial WS frame on connect.
async fn build_snapshot(state: &AppState) -> Option<String> {
    let pool = state.db.as_ref()?;
    let agents = db::agents::list_agents(pool).await.unwrap_or_default();
    let messages = db::messages::list_messages(pool, None, None, 100)
        .await
        .unwrap_or_default();
    let json = serde_json::json!({
        "type": "snapshot",
        "agents": &agents,
        "messages": &messages,
    });
    serde_json::to_string(&json).ok()
}

/// Returns true if the agent list has changed in any observable field.
fn agents_changed(
    prev: &[db::agents::Agent],
    curr: &[db::agents::Agent],
) -> bool {
    if prev.len() != curr.len() {
        return true;
    }
    for (p, c) in prev.iter().zip(curr.iter()) {
        if p.name != c.name
            || p.status != c.status
            || p.status_updated_at != c.status_updated_at
            || p.current_task != c.current_task
        {
            return true;
        }
    }
    false
}

/// Returns true if the message list has changed in any observable field.
fn messages_changed(
    prev: &[db::messages::Message],
    curr: &[db::messages::Message],
) -> bool {
    if prev.len() != curr.len() {
        return true;
    }
    for (p, c) in prev.iter().zip(curr.iter()) {
        if p.id != c.id || p.status != c.status || p.updated_at != c.updated_at {
            return true;
        }
    }
    false
}

/// Reconcile agent statuses against live tmux sessions.
/// Mirrors helpers::reconcile_agent_statuses but uses the server's own writable pool
/// to avoid coupling server code to CLI helpers.
/// Check if a tmux pane last line indicates the agent is at an idle prompt.
/// Returns true for known prompt patterns across supported providers.
fn is_idle_prompt(last_line: &str) -> bool {
    let trimmed = last_line.trim();
    if trimmed.is_empty() {
        return true; // Empty pane = idle (no output)
    }
    // Claude Code: prompt "❯", status bar lines at bottom
    trimmed.ends_with('❯')
        || trimmed.ends_with("bypass permissions on")
        || trimmed.ends_with("(shift+tab to cycle)")
        || trimmed.ends_with("to cycle)")
        // Gemini CLI: shell prompts
        || trimmed.ends_with("$ ")
        || trimmed == "$"
        || trimmed == ">"
}

/// Capture the last non-empty line from a tmux pane using async Command.
/// Uses tokio::process::Command to avoid spawn_blocking overhead.
async fn capture_pane_last_line(session_name: &str) -> Option<String> {
    let output = tokio::process::Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p", "-l", "5"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
}

async fn reconcile_for_server(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let agents = db::agents::list_agents(pool).await?;

    // Collect agents that need tmux session checks (skip frozen and db-only)
    let checkable: Vec<&db::agents::Agent> = agents
        .iter()
        .filter(|a| a.status != "frozen" && a.tool != "antigravity")
        .collect();

    // Check all tmux sessions and capture last lines in parallel (all async, no spawn_blocking)
    let futures: Vec<_> = checkable
        .iter()
        .map(|a| {
            let name = a.name.clone();
            async move {
                let alive = tmux::session_exists(&name).await;
                let last_line = if alive {
                    capture_pane_last_line(&name).await
                } else {
                    None
                };
                (alive, last_line)
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    // Apply status updates sequentially (single-writer DB)
    for (agent, (session_alive, last_line)) in checkable.iter().zip(results) {
        if !session_alive && agent.status != "dead" {
            db::agents::update_agent_status(pool, &agent.name, "dead").await?;
        } else if session_alive && agent.status == "dead" {
            db::agents::update_agent_status(pool, &agent.name, "idle").await?;
        } else if session_alive {
            // Detect busy/idle from pane content
            let pane_idle = last_line
                .as_deref()
                .map(is_idle_prompt)
                .unwrap_or(true);

            if !pane_idle && agent.status != "busy" {
                db::agents::update_agent_status(pool, &agent.name, "busy").await?;
            } else if pane_idle && agent.status == "busy" {
                db::agents::update_agent_status(pool, &agent.name, "idle").await?;
            }
        }
    }
    Ok(())
}

/// Background task: poll agent DB state at 1s intervals.
/// Runs tmux reconciliation each tick (if writable pool available).
/// Broadcasts an agent_update event only when agent state has changed.
async fn poll_agents(
    db_read: sqlx::SqlitePool,
    db_write: Option<sqlx::SqlitePool>,
    tx: broadcast::Sender<String>,
) {
    let mut cached: Vec<db::agents::Agent> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        // Run reconciliation if writable pool available
        if let Some(ref pool) = db_write {
            if let Err(e) = reconcile_for_server(pool).await {
                eprintln!("browser: reconcile error: {e}");
            }
        }
        // Read current agents
        let current = match db::agents::list_agents(&db_read).await {
            Ok(a) => a,
            Err(e) => {
                eprintln!("browser: poll_agents error: {e}");
                continue;
            }
        };
        if agents_changed(&cached, &current) {
            let json = serde_json::json!({
                "type": "agent_update",
                "agents": &current,
            });
            if let Ok(serialized) = serde_json::to_string(&json) {
                let _ = tx.send(serialized);
            }
            cached = current;
        }
    }
}

/// Background task: poll message DB state at 500ms intervals.
/// Broadcasts a message_update event only when message state has changed.
async fn poll_messages(db_read: sqlx::SqlitePool, tx: broadcast::Sender<String>) {
    let mut cached: Vec<db::messages::Message> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    loop {
        interval.tick().await;
        let current = match db::messages::list_messages(&db_read, None, None, 100).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("browser: poll_messages error: {e}");
                continue;
            }
        };
        if messages_changed(&cached, &current) {
            let json = serde_json::json!({
                "type": "message_update",
                "messages": &current,
            });
            if let Ok(serialized) = serde_json::to_string(&json) {
                let _ = tx.send(serialized);
            }
            cached = current;
        }
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // CRITICAL: Subscribe to broadcast BEFORE building snapshot
    // to avoid missing events during snapshot build
    let mut rx = state.tx.subscribe();

    // Send initial full snapshot
    if let Some(snapshot_json) = build_snapshot(&state).await {
        if ws_sender
            .send(Message::Text(snapshot_json.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Forward broadcast events to this WS client
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if ws_sender
                        .send(Message::Text(msg.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Skip missed messages -- next update has current state
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Listen for client disconnect
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    // Wait for either task to finish, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
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
    let (project_name, db, db_write) = match crate::config::load_config(Path::new(
        crate::config::DEFAULT_CONFIG_FILE,
    )) {
        Ok(config) => {
            let project = config.project.clone();
            let db_path_result = crate::config::resolve_db_path(&config);
            match db_path_result {
                Ok(db_path) => {
                    let read_pool = match crate::db::connect_readonly(&db_path).await {
                        Ok(pool) => Some(pool),
                        Err(e) => {
                            eprintln!("Warning: Could not connect to DB (read): {e} (continuing without DB)");
                            None
                        }
                    };
                    let write_pool = match crate::db::connect(&db_path).await {
                        Ok(pool) => Some(pool),
                        Err(e) => {
                            eprintln!("Warning: Could not connect to DB (write): {e} (reconciliation disabled)");
                            None
                        }
                    };
                    (project, read_pool, write_pool)
                }
                Err(e) => {
                    eprintln!("Warning: Could not resolve DB path: {e} (continuing without DB)");
                    (project, None, None)
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not load squad config: {e} (continuing without DB)");
            ("unknown".to_string(), None, None)
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

    let (tx, _rx) = broadcast::channel::<String>(128);

    let state = AppState {
        db,
        db_write,
        project_name,
        started_at: Instant::now(),
        tx,
    };

    // Spawn polling tasks before starting the axum server
    if let Some(ref read_pool) = state.db {
        let read_pool_clone = read_pool.clone();
        let write_pool_clone = state.db_write.clone();
        let tx_clone = state.tx.clone();
        tokio::spawn(poll_agents(read_pool_clone, write_pool_clone, tx_clone));

        let read_pool_clone2 = read_pool.clone();
        let tx_clone2 = state.tx.clone();
        tokio::spawn(poll_messages(read_pool_clone2, tx_clone2));
    }

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

/// Spawn the browser server as a detached background process and return immediately.
/// The child process runs `squad-station browser --no-open [--port N]` with stdout/stderr
/// redirected to `.squad/log/browser.log`.
pub async fn run_detached(port: Option<u16>) -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let log_dir = std::path::Path::new(".squad/log");
    std::fs::create_dir_all(log_dir)?;
    let log_file = std::fs::File::create(log_dir.join("browser.log"))?;
    let log_err = log_file.try_clone()?;

    let mut cmd = std::process::Command::new(exe);
    cmd.arg("browser").arg("--no-open");
    if let Some(p) = port {
        cmd.arg("--port").arg(p.to_string());
    }
    cmd.stdout(log_file).stderr(log_err);

    // Detach: don't wait for child, don't inherit stdin
    cmd.stdin(std::process::Stdio::null());
    let child = cmd.spawn()?;

    // Write PID for later stop
    let pid_file = log_dir.join("browser.pid");
    std::fs::write(&pid_file, child.id().to_string())?;

    // Wait briefly for server to bind, then read log for URL
    tokio::time::sleep(Duration::from_secs(1)).await;
    let log_content = std::fs::read_to_string(log_dir.join("browser.log")).unwrap_or_default();
    let url = log_content
        .lines()
        .find_map(|l| {
            l.find("http://").map(|i| &l[i..])
        })
        .unwrap_or("http://127.0.0.1:3000");

    if let Err(e) = open::that(url) {
        eprintln!("Warning: Could not open browser: {e}");
    }

    println!("Server running in background (pid={})", child.id());
    println!("{url}");
    println!("Stop: kill $(cat .squad/log/browser.pid)");
    Ok(())
}
