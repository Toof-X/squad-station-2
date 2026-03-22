# Phase 25: Architecture Research - Research

**Researched:** 2026-03-22
**Domain:** Rust axum HTTP/WebSocket server + rust-embed static assets + React/Vite/React Flow frontend + build pipeline integration
**Confidence:** HIGH (all four integration points verified against official docs and crates.io)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Spike format & output**
- Produce runnable proof-of-concept code, committed to repo
- Spike code lives in a separate `spike/` directory as a Cargo workspace member — completely isolated from main crate, honoring the additive-only constraint
- Architecture decisions go directly into PROJECT.md Key Decisions table — no separate research document
- Pass/fail bar is "it compiles and works" — no formal test-first criteria, keep the spike fast and MVP-focused

**Build pipeline integration**
- `build.rs` script in the Rust cargo project auto-runs `npm run build` for the frontend SPA whenever `cargo build` is executed — seamless single-command build
- If `dist/` doesn't exist (first clone, no node/npm installed), `cargo build` hard-fails with a clear error message telling the user what's missing
- CI/CD pipeline updates deferred to Phase 26 — no CI changes for the research spike
- React app has its own independent `web/package.json` inside its directory, separate from the existing `npm-package/` distribution wrapper

**Frontend project location & structure**
- React app lives at `web/` at the repo root (`web/package.json`, `web/src/`, `web/dist/`)
- Use Vite's React-TS template with TypeScript to catch errors early
- Install and validate React Flow in the spike — prove the full chain (Vite -> React Flow -> dist -> rust-embed -> axum serve)
- Build artifacts not committed: `web/dist/` and `web/node_modules/` added to `.gitignore`

**Tokio runtime coexistence**
- `browser` command uses the existing `#[tokio::main]` runtime — no separate dedicated runtime
- Axum server opens a separate read-only connection pool for DB access — prevents locking issues with the existing single-writer CLI commands
- `squad-station browser` blocks the terminal until Ctrl+C — matches `ui` command behavior, easier to manage
- Concurrent DB access (server reading while CLI writes) assumed safe under WAL mode — no spike validation needed, SQLite WAL is designed for this

### Claude's Discretion
- Exact spike crate structure and Cargo.toml dependencies
- `build.rs` implementation details (error messages, npm detection)
- rust-embed configuration and axum route setup
- WebSocket echo handler implementation
- Vite config and React Flow minimal component
- Read-only pool configuration (connection count, options)
- Order of spike validation (which integration point first)

### Deferred Ideas (OUT OF SCOPE)

None captured during discussion.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

Phase 25 has no direct v1.9 requirements (it is a pre-implementation research spike). All v1.9 requirements (SRV-01 through UI-03) are assigned to Phases 26-28. Phase 25's success criteria come from the roadmap:

| ID | Description | Research Support |
|----|-------------|-----------------|
| SPIKE-1 | rust-embed integration pattern validated: test binary embeds static asset, serves via axum | Architecture Patterns section: rust-embed + axum-embed pattern |
| SPIKE-2 | axum WebSocket upgrade path proven: minimal WS handler echoes message | Code Examples section: WS echo handler |
| SPIKE-3 | Event-detection strategy decided: tmux pane polling interval, DB change-detection mechanism, debounce approach documented | Architecture Patterns section: event detection pattern |
| SPIKE-4 | React + React Flow build pipeline proven: Vite build produces dist/ for rust-embed | Standard Stack section: Vite + @xyflow/react + build.rs |
| SPIKE-5 | Architecture decisions recorded in PROJECT.md Key Decisions table | Research output consumed by planner |
</phase_requirements>

---

## Summary

Phase 25 is a validation spike with four distinct integration points, each of which requires research on a specific library or pattern. All four points have been verified against official documentation and crates.io.

**Integration point 1 (rust-embed + axum):** The `axum-embed` crate (v0.1.0) wraps `rust-embed` (v8.11.0) into a `ServeEmbed<T>` tower service that plugs directly into an axum Router via `nest_service`. In debug builds, rust-embed reads from the filesystem (folder path relative to binary working directory); in release builds the files are compiled into the binary. The spike should use the `debug-embed` feature to force embedding during development so the spike accurately tests the release-mode behavior without requiring a release build every time.

**Integration point 2 (axum WebSocket):** axum 0.7 ships a built-in `axum::extract::ws` module. No `tokio-tungstenite` dependency is required. The upgrade pattern uses `WebSocketUpgrade` as an extractor and `on_upgrade(handler_fn)` to complete the upgrade. For multi-client push (needed in Phase 27), the canonical pattern is a `tokio::sync::broadcast` channel stored in `Arc<AppState>` with one `tokio::spawn`'d receive task per connected client.

**Integration point 3 (event detection):** No external crate is needed. The pattern is: spawn a background `tokio::task` that ticks a `tokio::time::interval` (recommended 500 ms for agent status, 200 ms for message state — matching production patterns observed in comparable tmux-polling dashboards), snapshots DB state each tick, diffs against previous snapshot, and sends changed events through a `broadcast::Sender`. This is simpler and more reliable for SQLite than SQLite update hooks because WAL mode does not deliver update hooks across connections.

**Integration point 4 (Vite + React Flow + build.rs):** The official `vite-react-flow-template` from xyflow uses `@xyflow/react ^12.5.1` with Vite 5, TypeScript 5, and `tsc && vite build` as the build command. The build produces `dist/` containing `index.html`, `assets/index-[hash].js`, and `assets/index-[hash].css`. `build.rs` uses `std::process::Command::new("npm")` to invoke `npm install` and `npm run build` in the `web/` directory, emits `cargo::rerun-if-changed=web/src` to avoid redundant rebuilds, and fails the build with a clear error if `npm` is not on PATH.

**Primary recommendation:** Use `axum-embed` (not raw `rust-embed` + manual MIME routing) for serving the SPA — it handles ETag caching, compression, directory redirects, and fallback behavior out of the box. The spike should validate all four integration points as a single cohesive mini-app, not as isolated pieces.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.7 | HTTP server + WebSocket upgrade | Officially maintained by tokio-rs; integrates natively with tokio 1.x; already implied by project's tokio dependency |
| rust-embed | 8.11.0 | Compile-time file embedding | De facto standard for embedding static assets in Rust binaries; maintained, widely used |
| axum-embed | 0.1.0 | Tower service wrapping rust-embed for axum | Handles ETag, compression, directory redirects automatically; avoids hand-rolling MIME + 404 logic |
| tower-http | 0.5 | Middleware (TraceLayer, TimeoutLayer) | Official tower ecosystem; needed for graceful shutdown timeout layer |
| tokio | 1.37 (existing) | Async runtime | Already in project; axum runs on tokio |
| @xyflow/react | 12.5.1 | React Flow node-graph component | Official v12 package (renamed from `reactflow`); SSR-capable; TypeScript-first |
| vite | 5.x | Frontend build tool | Industry standard as of 2025; `tsc && vite build` produces optimized `dist/` |
| react | 18.x | UI framework | Required by @xyflow/react |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @vitejs/plugin-react | 4.x | Vite plugin for React JSX transform | Required in vite.config.ts for React projects |
| typescript | 5.x | Type checking | Required by `tsc &&` build step |
| tokio::sync::broadcast | (tokio stdlib) | Fan-out events to all WS clients | When Phase 27 adds multi-client push |
| sqlx | 0.8 (existing) | Read-only DB pool for server | Already in project; use `read_only(true)` option |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| axum-embed | raw rust-embed + manual IntoResponse | axum-embed saves ~40 lines of MIME/ETag/404 boilerplate; use raw only if fine-grained control is needed |
| axum built-in WS | tokio-tungstenite | axum's ws module is sufficient for the spike; tokio-tungstenite adds complexity without benefit here |
| Vite | webpack / parcel | Vite is the 2025 standard; simpler config, faster builds |
| @xyflow/react | vis-network, d3 | React Flow is the standard for node-graph React UIs; d3 requires significantly more custom code |

**Installation (spike crate):**
```bash
# Cargo dependencies (spike/Cargo.toml)
axum = { version = "0.7", features = ["ws"] }
rust-embed = { version = "8", features = ["axum-ex", "debug-embed"] }
axum-embed = "0.1"
tower-http = { version = "0.5", features = ["trace", "timeout"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite"] }
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

```bash
# Frontend (web/ directory)
npm create vite@latest web -- --template react-ts
cd web && npm install @xyflow/react
```

---

## Architecture Patterns

### Recommended Spike Project Structure

```
spike/
├── Cargo.toml              # Workspace member, standalone dependencies
├── build.rs                # Runs npm install + npm run build in web/
└── src/
    └── main.rs             # axum server: rust-embed SPA + WS echo + DB read

web/                        # React app (repo root, not inside spike/)
├── package.json
├── vite.config.ts
├── tsconfig.json
├── src/
│   ├── main.tsx
│   ├── App.tsx             # ReactFlow component with WS connection
│   └── index.css
└── dist/                   # Built by build.rs (gitignored)
    ├── index.html
    └── assets/
        ├── index-[hash].js
        └── index-[hash].css
```

**Key constraint:** `web/` lives at the repo root, not inside `spike/`. The `build.rs` in `spike/` must use a relative path `../web` to reach the frontend source. rust-embed's `#[folder]` attribute in `spike/src/main.rs` should point to `../web/dist/` (resolved relative to the spike crate's `Cargo.toml`).

### Pattern 1: Workspace Member Setup

**What:** Add `spike/` as a member of the existing Cargo workspace.
**When to use:** The existing `Cargo.toml` at repo root is a regular (non-virtual) workspace. Add `[workspace]` with `members` if not already present.

```toml
# Root Cargo.toml — add workspace section
[workspace]
members = ["spike"]
resolver = "2"

# spike/Cargo.toml
[package]
name = "spike"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[dependencies]
# ... (see installation section above)
```

Note: The existing root `Cargo.toml` does not currently have a `[workspace]` section. Adding one makes `spike/` a member that shares `target/` and `Cargo.lock` with the main crate.

### Pattern 2: rust-embed + axum-embed SPA Serving

**What:** Embed the `web/dist/` directory at compile time and serve it via axum's `nest_service`.
**When to use:** All static asset serving in this project.

```rust
// Source: https://docs.rs/axum-embed/latest/axum_embed/
// Source: https://docs.rs/crate/rust-embed/latest

use rust_embed::Embed;
use axum_embed::ServeEmbed;

#[derive(Embed, Clone)]
#[folder = "../web/dist/"]   // relative to spike/Cargo.toml
struct FrontendAssets;

// In router setup:
let serve_spa = ServeEmbed::<FrontendAssets>::new();
let app = Router::new()
    .route("/ws", get(ws_handler))
    .nest_service("/", serve_spa);
```

**Critical:** The `debug-embed` feature must be enabled in `spike/Cargo.toml` so that assets are embedded even in `cargo build` (debug mode), not just `cargo build --release`. Without it, debug builds read from disk — which only works if the binary is run from the correct working directory.

```toml
rust-embed = { version = "8", features = ["axum-ex", "debug-embed"] }
```

### Pattern 3: axum WebSocket Echo Handler

**What:** Minimal WS handler that upgrades the HTTP connection and echoes messages.
**When to use:** Spike validation of SPIKE-2.

```rust
// Source: https://docs.rs/axum/latest/axum/extract/ws/index.html
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if socket.send(msg).await.is_err() {
            break;
        }
    }
}
```

### Pattern 4: Multi-Client Broadcast (needed in Phase 27, validate structure in spike)

**What:** `tokio::sync::broadcast` channel in shared `AppState` — each WS client subscribes on connect.
**When to use:** Anytime the server must push events to all connected browsers simultaneously.

```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    event_tx: broadcast::Sender<String>,  // JSON event strings
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.event_tx.subscribe();  // each client gets own receiver
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
    });

    // recv_task handles client -> server messages (ignore for now)
    tokio::select! {
        _ = &mut send_task => {}
    }
}
```

### Pattern 5: Event Detection via Polling + Broadcast

**What:** Background tokio task polls DB at fixed interval, diffs against last snapshot, broadcasts JSON events.
**When to use:** Whenever DB state changes need to be pushed to browser clients. Preferred over SQLite update hooks because WAL mode does not reliably deliver update hooks across connection boundaries.

```rust
// Recommended polling interval: 500ms for agent status, 200ms for messages
fn spawn_event_loop(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        let mut last_snapshot: Option<AppSnapshot> = None;

        loop {
            interval.tick().await;

            let current = fetch_snapshot(&state.db).await;
            if let Some(ref prev) = last_snapshot {
                if let Some(events) = diff_snapshots(prev, &current) {
                    for event in events {
                        let _ = state.event_tx.send(serde_json::to_string(&event).unwrap());
                    }
                }
            }
            last_snapshot = Some(current);
        }
    });
}
```

**Polling interval rationale:** 500ms matches the TmuxCC project (configurable, default 500ms) and AgentDock (200ms for raw pane streaming). For agent status transitions (which are coarse), 500ms is sufficient and avoids excessive DB reads. For the spike, 500ms is fine; Phase 27 can tune based on measurement.

**tmux pane polling note:** The spike should validate that `std::process::Command::new("tmux").args(["capture-pane", "-p", "-t", session])` works from within a tokio-spawned blocking task (`tokio::task::spawn_blocking`). Tmux `capture-pane` is a subprocess call — do NOT call it from async context directly.

### Pattern 6: build.rs Frontend Build Integration

**What:** `spike/build.rs` runs `npm install` + `npm run build` before the Rust crate compiles, ensuring `web/dist/` exists for rust-embed.
**When to use:** Every `cargo build` in the spike crate.

```rust
// spike/build.rs
use std::process::Command;
use std::path::Path;

fn main() {
    // Tell Cargo to re-run this script when frontend source changes
    println!("cargo::rerun-if-changed=../web/src");
    println!("cargo::rerun-if-changed=../web/package.json");
    println!("cargo::rerun-if-changed=build.rs");

    let web_dir = Path::new("../web");

    // Check npm is available
    let npm_check = Command::new("npm").arg("--version").output();
    if npm_check.is_err() || !npm_check.unwrap().status.success() {
        eprintln!("cargo::error=npm not found. Install Node.js from https://nodejs.org to build the browser UI.");
        std::process::exit(1);
    }

    // npm install
    let install = Command::new("npm")
        .arg("install")
        .current_dir(web_dir)
        .status()
        .expect("Failed to run npm install");
    if !install.success() {
        eprintln!("cargo::error=npm install failed in web/");
        std::process::exit(1);
    }

    // npm run build
    let build = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(web_dir)
        .status()
        .expect("Failed to run npm run build");
    if !build.success() {
        eprintln!("cargo::error=npm run build failed in web/. Check web/src for TypeScript errors.");
        std::process::exit(1);
    }
}
```

### Pattern 7: Read-Only SQLite Pool for Server

**What:** Second connection pool with `read_only(true)` and higher `max_connections` for the HTTP server — completely separate from the existing single-writer pool.
**When to use:** Any new server/daemon that reads from the same SQLite DB the CLI writes to.

```rust
// Source: https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub async fn connect_readonly(db_path: &std::path::Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        // Do NOT set journal_mode — WAL is already set on the DB file;
        // setting journal_mode on a read-only connection would fail anyway
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)   // reads don't contend; higher limit is fine
        .connect_with(opts)
        .await?;

    Ok(pool)
}
```

**Critical:** Do NOT run `sqlx::migrate!()` on the read-only pool. Migrations require write access and should only run via the existing `db::connect()` (single-writer pool). The server's read-only pool is for queries only.

### Pattern 8: Graceful Shutdown via tokio::signal

**What:** `axum::serve(...).with_graceful_shutdown(shutdown_signal())` waits for Ctrl+C / SIGTERM before stopping.
**When to use:** The `browser` command must block until Ctrl+C, matching `ui` command behavior.

```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
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
```

### Pattern 9: Minimal React Flow Component (spike frontend)

**What:** Simplest valid React Flow rendering with static nodes/edges + WebSocket connection.
**When to use:** Spike validation of SPIKE-4.

```typescript
// Source: https://reactflow.dev/learn
// web/src/App.tsx
import { ReactFlow, Node, Edge } from '@xyflow/react';
import '@xyflow/react/dist/style.css';

const initialNodes: Node[] = [
  { id: '1', position: { x: 0, y: 0 }, data: { label: 'Orchestrator' } },
  { id: '2', position: { x: 0, y: 100 }, data: { label: 'Worker 1' } },
];
const initialEdges: Edge[] = [
  { id: 'e1-2', source: '1', target: '2' },
];

// CRITICAL: Parent must have explicit width + height
export default function App() {
  return (
    <div style={{ width: '100vw', height: '100vh' }}>
      <ReactFlow nodes={initialNodes} edges={initialEdges} fitView />
    </div>
  );
}
```

```typescript
// web/src/main.tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
```

### Anti-Patterns to Avoid

- **Calling tmux capture-pane from async context directly:** Use `tokio::task::spawn_blocking` for all subprocess calls. `std::process::Command` blocks the thread.
- **Sharing the existing single-writer pool with the server:** The server must have its own read-only pool. Sharing the writer pool will cause WAL deadlocks.
- **Running migrations on the read-only pool:** Will fail with a write permission error.
- **Setting journal_mode on the read-only pool:** The WAL journal mode is a DB-level persistent setting in SQLite; setting it on a read-only connection errors out.
- **Missing `debug-embed` feature flag:** Without it, debug builds of the spike read files from disk relative to CWD — tests pass locally but fail when run from a different working directory.
- **Placing `web/` inside `spike/`:** The `web/` directory must be at the repo root (per locked decision). The `build.rs` uses `../web` relative path.
- **Forgetting `@xyflow/react/dist/style.css` import:** React Flow renders a blank white box without this import.
- **Forgetting parent dimensions on `<ReactFlow>`:** The component requires an explicit-sized parent `div`.
- **Using `reactflow` (old package name):** v12 was renamed to `@xyflow/react`. The old package is deprecated.
- **Running `npm run build` without `npm install` first:** On fresh clone, `node_modules/` doesn't exist. The build.rs must run install before build.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MIME type detection + ETag + 304 handling + gzip for static files | Custom `IntoResponse` impl reading rust-embed assets | `axum-embed` crate | Handles compression negotiation, ETags, directory redirects — ~200 lines of correct code vs 5 lines with the crate |
| WebSocket client registry + fan-out messaging | `Arc<RwLock<Vec<Sender>>>` manual registry | `tokio::sync::broadcast::channel` | Broadcast handles lagged-receiver errors, message dropping policies, and Clone semantics correctly |
| SPA index.html fallback (HTML5 history routing) | Manual 404 -> index.html redirect logic | `axum_embed::FallbackBehavior::Ok` | Built into axum-embed; one method call |
| Frontend build detection in build.rs | Custom npm path detection logic | `Command::new("npm").arg("--version")` + clear error message | Simplest reliable check; do NOT use `which` crate for this |
| Change detection between DB snapshots | SQLite UPDATE triggers + notification channels | Timestamp-based snapshot diff in polling loop | SQLite update hooks don't cross connection boundaries in WAL mode; polling is simpler and sufficient |

**Key insight:** The value of this spike is proving the *seams* between components work. Do not spend time building production-quality versions of any component — the pass bar is "it compiles and works."

---

## Common Pitfalls

### Pitfall 1: rust-embed Debug vs Release Path Resolution
**What goes wrong:** In debug builds (without `debug-embed` feature), `#[folder = "../web/dist/"]` resolves relative to where the binary is *run from*, not where `Cargo.toml` is. Running the spike from a subdirectory gives "Asset not found" errors.
**Why it happens:** rust-embed 8.x resolves the folder path at *runtime* in debug mode using the working directory, not the source tree location.
**How to avoid:** Add `features = ["debug-embed"]` to the rust-embed dependency in `spike/Cargo.toml`. This forces compile-time embedding even in debug builds.
**Warning signs:** "404 Not Modified" for all assets in debug build; works fine in release build.

### Pitfall 2: Workspace Cargo.toml Missing `[workspace]` Section
**What goes wrong:** `cargo build` in `spike/` works, but running `cargo build` at the repo root fails with "package `spike` not found" or includes the spike's heavy dependencies in the main crate build.
**Why it happens:** The current root `Cargo.toml` does not have a `[workspace]` section. Without it, adding `spike/` as a member requires explicitly declaring the workspace.
**How to avoid:** Add `[workspace]` to the root `Cargo.toml` with `members = ["spike"]`. The spike shares `target/` and `Cargo.lock` with the main crate, which is the correct behavior.
**Warning signs:** `error: could not find `Cargo.toml` in any parent directory` when running cargo from spike/.

### Pitfall 3: axum Feature Flag for WebSocket
**What goes wrong:** `use axum::extract::ws::WebSocketUpgrade` fails to compile with "module `ws` not found".
**Why it happens:** axum gates the WebSocket module behind a feature flag.
**How to avoid:** In `spike/Cargo.toml`, use `axum = { version = "0.7", features = ["ws"] }`.
**Warning signs:** Compile error mentioning `ws` module not found.

### Pitfall 4: build.rs Relative Path from Workspace Root
**What goes wrong:** `Command::new("npm").current_dir("../web")` fails when `cargo build` is run from the workspace root (not from `spike/`).
**Why it happens:** `build.rs` working directory is always the directory containing the crate's `Cargo.toml`, not the workspace root.
**How to avoid:** Use `Path::new("../web")` — this is relative to `spike/Cargo.toml`, which is always `spike/`'s parent `web/`. Alternatively, use the `CARGO_MANIFEST_DIR` environment variable: `let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web");`
**Warning signs:** "No such file or directory: ../web" in build script output.

### Pitfall 5: broadcast::channel Lagged Error
**What goes wrong:** Slow WebSocket clients receive `RecvError::Lagged(n)` and miss events silently.
**Why it happens:** `broadcast::channel` has a fixed capacity; slow receivers fall behind and messages are dropped.
**How to avoid:** For the spike, use a large capacity (e.g., `broadcast::channel(100)`) and ignore `Lagged` errors (break the loop, close the WS connection). Phase 27 can implement reconnect-with-full-snapshot to recover.
**Warning signs:** WebSocket client reconnects frequently; events missing from browser.

### Pitfall 6: npm Windows Path on macOS
**What goes wrong:** `Command::new("npm")` works on macOS/Linux but not in CI or on some systems where npm is installed via NVM without shell initialization.
**Why it happens:** NVM-installed npm may not be on PATH in non-interactive shells (which is what cargo build.rs runs in).
**How to avoid:** Check if `npm` resolves; if not, also try `npx npm`. Provide a clear error message: "npm not found on PATH. If using NVM, ensure your shell profile is configured. Install Node.js from https://nodejs.org".
**Warning signs:** `Err(Os { code: 2, kind: NotFound })` in build.rs when npm is installed via NVM.

### Pitfall 7: React Flow Blank Render
**What goes wrong:** `<ReactFlow>` component renders but shows nothing — no nodes visible.
**Why it happens:** Either (a) the CSS import is missing, or (b) the parent element has no explicit dimensions.
**How to avoid:** Always `import '@xyflow/react/dist/style.css'` in `main.tsx` or `App.tsx`. Wrap `<ReactFlow>` in a `<div style={{ width: '100vw', height: '100vh' }}>`.
**Warning signs:** React Flow renders, no error in console, but canvas is empty or nodes invisible.

---

## Code Examples

### Complete Spike Server Structure

```rust
// spike/src/main.rs — cohesive mini-app validating all 4 integration points
use anyhow::Result;
use axum::{
    Router,
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::Response,
    routing::get,
};
use axum_embed::ServeEmbed;
use rust_embed::Embed;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Embed, Clone)]
#[folder = "../web/dist/"]
struct FrontendAssets;

#[derive(Clone)]
struct AppState {
    // Read-only DB pool (to validate pattern — query agents/messages)
    db: sqlx::SqlitePool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = connect_readonly(std::path::Path::new(".squad/station.db")).await?;
    let state = Arc::new(AppState { db });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeEmbed::<FrontendAssets>::new())
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("Spike server running at http://127.0.0.1:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, _state: Arc<AppState>) {
    // Echo: proves WS upgrade works end-to-end
    while let Some(Ok(msg)) = socket.recv().await {
        if socket.send(msg).await.is_err() {
            break;
        }
    }
}
```

### Read-Only Pool Connection

```rust
// Source: https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::time::Duration;

async fn connect_readonly(db_path: &std::path::Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    // DO NOT run migrations on read-only pool
    Ok(pool)
}
```

### Shutdown Signal

```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("Ctrl+C handler failed");
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
```

### Vite Config for SPA (web/vite.config.ts)

```typescript
// Source: https://vitejs.dev/guide/
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',      // default; rust-embed points to this
    emptyOutDir: true,   // clean dist/ before each build
  },
})
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `reactflow` npm package | `@xyflow/react` (v12) | 2024-07 (React Flow 12 release) | Old package deprecated; new one has SSR support, renamed import |
| `tower::ServiceExt` for static file serving | `axum_embed::ServeEmbed<T>` | 2023-2024 | Avoids manual MIME+ETag implementation |
| `tokio-tungstenite` for WS in axum apps | `axum::extract::ws` (built-in) | axum 0.6+ | No extra dependency needed for server-side WS |
| CRA (Create React App) | Vite | 2023 (CRA deprecated) | Vite is now the standard; faster, simpler config |

**Deprecated/outdated:**
- `reactflow` (old npm package): replaced by `@xyflow/react` in v12; do not use the old package name
- `axum::Server::bind` (axum 0.6 API): replaced by `TcpListener::bind` + `axum::serve()` in axum 0.7
- CRA (`create-react-app`): deprecated 2023; use `npm create vite@latest -- --template react-ts`

---

## Open Questions

1. **Cargo.toml workspace section: will adding `[workspace]` to the root Cargo.toml break anything?**
   - What we know: The root `Cargo.toml` currently has no `[workspace]` section. Adding it is safe if `resolver = "2"` is set (matches the existing `edition = "2021"`).
   - What's unclear: Whether any existing CI or build scripts assume the root is not a workspace root.
   - Recommendation: Add `[workspace]` to root Cargo.toml with `members = ["spike"]` as the first task. Run `cargo test` after to verify nothing breaks.

2. **rust-embed `#[folder]` path when workspace root != spike/ crate root**
   - What we know: rust-embed resolves `#[folder]` relative to `Cargo.toml` location in release/debug-embed mode.
   - What's unclear: Exact behavior when `spike/Cargo.toml` is a workspace member and folder path uses `../web/dist/`.
   - Recommendation: Verify empirically in first spike task by running `cargo build -p spike` and checking embed works. If path resolution fails, use `CARGO_MANIFEST_DIR` env var in a build.rs that copies dist/ to a local path before embedding.

3. **tmux availability in the spike runtime environment**
   - What we know: `capture-pane` requires a running tmux session. In testing, there may not be a tmux session to poll.
   - What's unclear: Whether the spike needs to validate tmux polling directly, or just prove the tokio interval + broadcast pattern.
   - Recommendation: For SPIKE-3, the event-detection spike only needs to prove the interval + broadcast channel pattern works. Tmux polling can be stubbed in the spike (return hardcoded agent data instead of real capture-pane output) — the interval/diff/broadcast pattern is what needs validation, not the tmux subprocess itself.

---

## Validation Architecture

`nyquist_validation` is enabled. However, Phase 25 is a research spike with the explicit constraint: "Pass/fail bar is 'it compiles and works' — no formal test-first criteria, keep the spike fast and MVP-focused."

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (built-in) |
| Config file | none (spike is standalone) |
| Quick run command | `cargo build -p spike` |
| Full suite command | `cargo test -p spike` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPIKE-1 | spike binary compiles with embedded assets | build | `cargo build -p spike` | ❌ Wave 0 |
| SPIKE-2 | WS echo handler upgrades connection | smoke | manual: `wscat -c ws://localhost:3000/ws` | ❌ Wave 0 |
| SPIKE-3 | Event detection strategy documented in PROJECT.md | manual | n/a | ❌ Wave 0 |
| SPIKE-4 | `npm run build` produces dist/ consumed by rust-embed | build | `cargo build -p spike` (includes build.rs) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo build -p spike` (verifies it compiles)
- **Per wave merge:** `cargo build -p spike` + manual browser smoke test
- **Phase gate:** All 4 spike validations pass before moving to Phase 26

### Wave 0 Gaps
- [ ] `spike/` directory — create Cargo workspace member
- [ ] `spike/build.rs` — frontend build integration
- [ ] `spike/src/main.rs` — cohesive mini-app
- [ ] `web/` — Vite React-TS project with @xyflow/react
- [ ] Root `Cargo.toml` — add `[workspace]` with `members = ["spike"]`
- [ ] `.gitignore` — add `web/dist/` and `web/node_modules/` entries

---

## Sources

### Primary (HIGH confidence)
- `https://docs.rs/axum/latest/axum/extract/ws/index.html` — WebSocket upgrade pattern, echo handler, key types
- `https://docs.rs/crate/rust-embed/latest` — Version 8.11.0, feature flags, debug vs release path resolution
- `https://docs.rs/axum-embed/latest/axum_embed/` — ServeEmbed usage, compression, ETag, fallback
- `https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html` — `read_only(true)`, `busy_timeout`, WAL notes
- `https://doc.rust-lang.org/cargo/reference/workspaces.html` — Workspace member setup, virtual vs regular workspace
- `https://doc.rust-lang.org/cargo/reference/build-scripts.html` — `cargo::rerun-if-changed`, external command execution, failure handling
- `https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs` — `with_graceful_shutdown`, `shutdown_signal()` pattern
- `https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs` — `broadcast::channel` + multi-client WebSocket pattern
- `https://reactflow.dev/learn` — React Flow minimal setup, required CSS import, node/edge data shapes

### Secondary (MEDIUM confidence)
- `https://github.com/xyflow/vite-react-flow-template` — Official vite-react-flow-template package.json; `@xyflow/react ^12.5.1`, `vite ^5.0.12`
- `https://crates.io/crates/rust-embed` — Version 8.11.0 confirmed current (released 2026-01-14)
- `https://crates.io/crates/axum-embed` — Version 0.1.0 confirmed
- `https://github.com/nyanko3141592/tmuxcc` — Polling interval precedent: 500ms default for tmux pane state detection
- `https://github.com/vishalnarkhede/agentdock` — 200ms polling interval for tmux capture-pane over WebSocket

### Tertiary (LOW confidence)
- WebSearch results for build.rs npm integration patterns — no single canonical reference; pattern synthesized from Cargo docs + general Rust build script practice

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates verified against crates.io (current versions) and official docs
- Architecture: HIGH — all patterns sourced from official axum examples and sqlx docs
- Pitfalls: MEDIUM-HIGH — most verified against official docs; rust-embed debug path behavior verified against docs.rs; some pitfalls (NVM PATH) based on known ecosystem behavior
- Build pipeline: HIGH — Cargo build scripts are stable, well-documented
- Frontend stack: HIGH — official React Flow template and Vite docs used

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable ecosystem; Vite, axum, rust-embed are mature crates with low churn)
