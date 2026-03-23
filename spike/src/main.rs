use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use axum_embed::ServeEmbed;
use rust_embed::Embed;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Embed, Clone)]
#[folder = "../web/dist/"]
struct FrontendAssets;

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    db: Option<sqlx::SqlitePool>,
}

async fn connect_readonly(db_path: &std::path::Path) -> anyhow::Result<sqlx::SqlitePool> {
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .busy_timeout(std::time::Duration::from_secs(5));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;
    Ok(pool)
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = match connect_readonly(std::path::Path::new(".squad/station.db")).await {
        Ok(pool) => {
            println!("Connected to .squad/station.db (read-only)");
            Some(pool)
        }
        Err(e) => {
            eprintln!("Warning: Could not connect to DB: {e} (continuing without DB)");
            None
        }
    };

    let _state = AppState { db };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeEmbed::<FrontendAssets>::new());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("Spike server running at http://127.0.0.1:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
