# Phase 26: Axum Server & CLI Command - Research

**Researched:** 2026-03-22
**Domain:** Rust web server (axum), cargo feature flags, embedded SPA serving, browser auto-open
**Confidence:** HIGH

## Summary

Phase 26 integrates the spike-validated axum + rust-embed + React Flow stack into the production binary behind a cargo feature flag. The spike (`spike/src/main.rs`) already proves the core pattern works; production work is primarily about feature-gating dependencies, adding the CLI subcommand with port selection logic, auto-opening the browser, and setting up Tailwind CSS v4 for the SPA.

All recommended crates are already validated in the spike with compatible versions. The main technical risks are: (1) getting the feature flag conditional compilation right across Cargo.toml, build.rs, cli.rs, and main.rs, and (2) making build.rs conditionally run npm build only when the `browser` feature is active.

**Primary recommendation:** Follow the spike's proven dependency versions exactly. Use `#[cfg(feature = "browser")]` guards on module imports and match arms. Use the `open` crate (v5) for browser launching. Use `@tailwindcss/vite` plugin (no PostCSS needed) for Tailwind v4.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- Default port: try 3000, auto-fallback to port 0 if taken. Explicit `--port N` errors on conflict (no fallback)
- Bind to `127.0.0.1` only (localhost)
- Include `--no-open` flag for headless/SSH environments
- `browser` feature flag gates the command; without it, subcommand prints "not enabled" message and exits
- CI/CD always builds with `--features browser`; end-users never see the flag
- Reuse spike's React Flow demo as initial SPA content
- SPA connects to `/ws` and shows connection status
- Include `/api/status` REST endpoint returning project info from DB/config
- Set up CSS/design system foundation for Phase 28

### Claude's Discretion
- Exact cargo feature flag name and conditional compilation approach
- build.rs implementation for conditional frontend build
- Browser auto-open implementation (crate choice)
- REST API endpoint design and response shape
- Design system choice (Tailwind, CSS modules, etc.)
- SPA project structure and component organization
- Test strategy for the new command and server
- Migration of spike code to production modules

### Deferred Ideas (OUT OF SCOPE)
None captured.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SRV-01 | `squad-station browser` starts embedded axum HTTP server serving React SPA from rust-embed | Spike validates full pipeline; feature-gated deps in Cargo.toml; `ServeEmbed` pattern proven |
| SRV-02 | Auto-opens default system browser to server URL after startup | `open` crate v5.3.3 — `open::that(url)` is the standard approach |
| SRV-03 | Graceful shutdown on Ctrl+C/SIGTERM with no orphaned processes | `shutdown_signal()` pattern from spike; `axum::serve().with_graceful_shutdown()` |
| SRV-04 | `--port` flag allows custom port (default: auto-select available) | Port 3000 try-then-fallback pattern using `TcpListener::bind` + `AddrInUse` detection |
| UI-01 | SPA assets bundled via rust-embed, served from binary | `#[derive(Embed)] #[folder = "web/dist/"]` + `ServeEmbed` — proven in spike |

</phase_requirements>

## Standard Stack

### Core (Production Dependencies — all optional, behind `browser` feature)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.7 (features: ws) | HTTP server + WebSocket | Already in spike; dominant Rust async web framework |
| rust-embed | 8.11 (features: axum-ex, debug-embed) | Embed SPA dist/ into binary | Proven in spike; `debug-embed` serves from filesystem in dev |
| axum-embed | 0.1 | Serve rust-embed assets as axum service | Thin bridge between rust-embed and axum; used in spike |
| tower-http | 0.5 (features: trace, timeout) | HTTP middleware | Standard companion to axum |
| open | 5 | Cross-platform browser launch | 5.3.3 is current; `open::that(url)` is the API; used by cargo itself |

### Frontend

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|-------------|
| tailwindcss | 4.x | Utility-first CSS | v4 uses Vite plugin — no PostCSS/config file needed |
| @tailwindcss/vite | 4.x | Vite integration for Tailwind | Official plugin, replaces old PostCSS approach |

Already in `web/package.json` (from spike):
- React 19, React DOM 19, @xyflow/react 12, Vite 8, TypeScript 5.9

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `open` crate | `webbrowser` crate | `webbrowser` guarantees browser (not just default app) but `open` is simpler, used by cargo itself, sufficient for URLs |
| `open` crate | `std::process::Command("open"/"xdg-open")` | Manual platform detection; `open` crate handles macOS/Linux/Windows/WSL |
| Tailwind v4 | CSS modules | Tailwind provides design system foundation Phase 28 needs; CSS modules lack utility classes |

**Installation (frontend additions):**
```bash
cd web && npm install tailwindcss @tailwindcss/vite
```

**Cargo.toml additions (all optional):**
```toml
[features]
browser = ["dep:axum", "dep:rust-embed", "dep:axum-embed", "dep:tower-http", "dep:open"]

[dependencies]
axum = { version = "0.7", features = ["ws"], optional = true }
rust-embed = { version = "8", features = ["axum-ex", "debug-embed"], optional = true }
axum-embed = { version = "0.1", optional = true }
tower-http = { version = "0.5", features = ["trace", "timeout"], optional = true }
open = { version = "5", optional = true }
```

## Architecture Patterns

### Recommended Project Structure
```
src/
  commands/
    browser.rs       # Server startup, port selection, browser open, shutdown
  cli.rs             # Browser variant added to Commands enum
  main.rs            # Browser dispatch arm (cfg-gated)
  db/
    mod.rs           # Add connect_readonly() (from spike)
web/
  src/               # React SPA source
  dist/              # Build output (gitignored, embedded by rust-embed)
  vite.config.ts     # Add @tailwindcss/vite plugin
build.rs             # Conditional npm build (only when browser feature active)
```

### Pattern 1: Feature-Gated Subcommand

**What:** The `Browser` variant exists in `Commands` enum always (for help text), but the implementation module and heavy match arm are `#[cfg(feature = "browser")]` gated.

**Example:**
```rust
// cli.rs — Always present (so --help shows it)
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing variants ...

    /// Launch browser visualization (requires --features browser)
    Browser {
        /// Port to bind to (default: 3000, fallback to random if taken)
        #[arg(long)]
        port: Option<u16>,
        /// Skip auto-opening browser
        #[arg(long)]
        no_open: bool,
    },
}
```

```rust
// main.rs — Conditional dispatch
#[cfg(feature = "browser")]
Browser { port, no_open } => commands::browser::run(port, no_open).await,

#[cfg(not(feature = "browser"))]
Browser { .. } => {
    eprintln!("Browser feature not enabled. Rebuild with: cargo build --features browser");
    std::process::exit(1);
}
```

```rust
// At top of main.rs or lib.rs — conditional module import
#[cfg(feature = "browser")]
pub mod browser;
```

### Pattern 2: Port Selection with Fallback

**What:** Try preferred port, catch `AddrInUse`, fallback to OS-assigned port.

**Example:**
```rust
use std::io::ErrorKind;
use tokio::net::TcpListener;

async fn bind_listener(explicit_port: Option<u16>) -> anyhow::Result<TcpListener> {
    let preferred = explicit_port.unwrap_or(3000);
    let addr = format!("127.0.0.1:{}", preferred);

    match TcpListener::bind(&addr).await {
        Ok(listener) => Ok(listener),
        Err(e) if e.kind() == ErrorKind::AddrInUse && explicit_port.is_none() => {
            // Auto-fallback: bind port 0 for OS-assigned port
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let actual_port = listener.local_addr()?.port();
            eprintln!("Port {} in use, using port {} instead", preferred, actual_port);
            Ok(listener)
        }
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            anyhow::bail!("Port {} already in use", preferred);
        }
        Err(e) => Err(e.into()),
    }
}
```

### Pattern 3: Conditional build.rs

**What:** build.rs detects the `browser` feature via environment variable and only runs npm when active.

**Example:**
```rust
// build.rs
fn main() {
    // Only build frontend when browser feature is enabled
    if std::env::var("CARGO_FEATURE_BROWSER").is_err() {
        return;
    }

    println!("cargo::rerun-if-changed=web/src");
    println!("cargo::rerun-if-changed=web/package.json");

    // npm availability check + install + build (from spike build.rs)
    // Paths are relative to Cargo.toml (repo root), NOT spike/
}
```

**Key difference from spike:** Spike build.rs uses `../web` because it lives in `spike/`. Production build.rs lives at repo root, so paths are just `web/`.

### Pattern 4: Tailwind v4 with Vite

**What:** Tailwind v4 uses a Vite plugin instead of PostCSS. No `tailwind.config.js` needed.

**Setup:**
```typescript
// web/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
  ],
})
```

```css
/* web/src/index.css */
@import "tailwindcss";
```

**Key point:** No `postcss.config.js`, no `tailwind.config.js`, no `autoprefixer`. The `@tailwindcss/vite` plugin handles everything.

### Pattern 5: REST Endpoint with axum::Json

**What:** Simple JSON endpoint alongside SPA and WebSocket routes.

**Example:**
```rust
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Serialize)]
struct StatusResponse {
    project: String,
    agents: usize,
    uptime_secs: u64,
    version: String,
}

async fn api_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let agent_count = match &state.db {
        Some(pool) => db::agents::list_agents(pool).await.map(|a| a.len()).unwrap_or(0),
        None => 0,
    };
    Json(StatusResponse {
        project: state.project_name.clone(),
        agents: agent_count,
        uptime_secs: state.started_at.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

// Router setup:
let app = Router::new()
    .route("/api/status", get(api_status))
    .route("/ws", get(ws_handler))
    .nest_service("/", ServeEmbed::<FrontendAssets>::new())
    .with_state(state);
```

**Important:** Routes must be ordered: explicit routes first, then the catch-all SPA service last via `nest_service("/", ...)`.

### Anti-Patterns to Avoid
- **Sharing the single-writer pool with the server:** Server MUST use `connect_readonly()` with its own pool. The existing `db::connect()` is single-writer for CLI commands.
- **Running npm build without the feature flag:** build.rs must check `CARGO_FEATURE_BROWSER` and exit early if absent. Otherwise core CLI builds break without node installed.
- **Hardcoding port in printed URL:** Always get actual port from `listener.local_addr()` after binding, even for the default port case.
- **Blocking browser open:** Use `open::that()` which is non-blocking. Do NOT wait for it to complete.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cross-platform browser open | Platform detection + `Command::new("open"/"xdg-open")` | `open` crate v5 | Handles macOS, Linux (xdg-open), Windows, WSL edge cases |
| SPA asset embedding | Custom file-reading + MIME detection | `rust-embed` + `axum-embed` | Handles MIME types, compression, dev-mode file serving, SPA fallback |
| CSS utility framework | Custom CSS classes | Tailwind v4 | Phase 28 needs a design system; Tailwind provides one out of the box |
| Graceful shutdown | Manual signal handling | `tokio::signal` + `axum::serve().with_graceful_shutdown()` | Handles Ctrl+C + SIGTERM, drains connections properly |

## Common Pitfalls

### Pitfall 1: build.rs Path Confusion
**What goes wrong:** Spike build.rs uses `../web` because spike/ is a subdirectory. Production build.rs at repo root needs `web/` (no `../`).
**How to avoid:** Production build.rs uses `Path::new("web")` not `Path::new("../web")`.

### Pitfall 2: Feature Flag Not Gating sqlx Migrations
**What goes wrong:** `rust-embed`'s `#[derive(Embed)]` with `#[folder = "web/dist/"]` fails if `web/dist/` doesn't exist. This breaks `cargo build` without the feature even if the module is cfg-gated.
**How to avoid:** The `#[derive(Embed)]` struct must live inside a `#[cfg(feature = "browser")]` module so it's only compiled when the feature is active.

### Pitfall 3: Route Ordering with nest_service
**What goes wrong:** If `nest_service("/", ...)` is placed before explicit routes like `/api/status` or `/ws`, it catches all requests and the API/WS routes never fire.
**How to avoid:** Always define explicit routes first, then `nest_service("/", spa)` last.

### Pitfall 4: npm install Running on Every Build
**What goes wrong:** build.rs runs `npm install` + `npm run build` on every `cargo build`, adding 5-10 seconds even when nothing changed.
**How to avoid:** Use `cargo::rerun-if-changed=web/src` and `cargo::rerun-if-changed=web/package.json` directives. Cargo only reruns build.rs when these change. The spike already does this correctly.

### Pitfall 5: Missing web/dist/ on First Clone
**What goes wrong:** A contributor clones the repo, runs `cargo build --features browser`, and build.rs needs npm/node which may not be installed.
**How to avoid:** build.rs checks npm availability first and prints a clear error message with installation URL. The spike's build.rs already implements this pattern.

## Code Examples

### Browser Command Entry Point
```rust
// src/commands/browser.rs
pub async fn run(port: Option<u16>, no_open: bool) -> anyhow::Result<()> {
    let config = crate::config::load_config(
        std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE)
    )?;
    let db_path = crate::config::resolve_db_path(&config)?;

    // Connect read-only pool (from spike pattern)
    let db = match crate::db::connect_readonly(&db_path).await {
        Ok(pool) => Some(pool),
        Err(e) => {
            eprintln!("Warning: Could not connect to DB: {e} (continuing without DB)");
            None
        }
    };

    let listener = bind_listener(port).await?;
    let actual_port = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{}", actual_port);

    println!("Squad Station browser at {}", url);

    if !no_open {
        if let Err(e) = open::that(&url) {
            eprintln!("Could not open browser: {e}");
        }
    }

    let state = AppState { db, /* ... */ };
    let app = build_router(state);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");
    Ok(())
}
```

### Browser Open with `open` Crate
```rust
// open::that() returns Result<()>
// Non-blocking on GUI browsers (Firefox, Chrome, etc.)
if let Err(e) = open::that(&url) {
    eprintln!("Could not open browser: {e}");
    // Not fatal — user can navigate manually
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Tailwind v3 (PostCSS + config file) | Tailwind v4 (@tailwindcss/vite plugin) | Jan 2025 | No postcss.config.js or tailwind.config.js needed |
| `@tailwind base/components/utilities` directives | `@import "tailwindcss"` | Tailwind v4 | Single import replaces three directives |
| `open` crate v3-4 | `open` crate v5 | 2024 | API stable; `that()`, `with()`, `that_detached()` |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (tokio async) |
| Config file | Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test --features browser browser` |
| Full suite command | `cargo test --features browser` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SRV-01 | Server starts and serves SPA | integration | `cargo test --features browser test_browser_serves_spa -x` | Wave 0 |
| SRV-02 | Browser auto-open called | unit | `cargo test --features browser test_browser_open -x` | Wave 0 |
| SRV-03 | Graceful shutdown on signal | integration | `cargo test --features browser test_graceful_shutdown -x` | Wave 0 |
| SRV-04 | Port selection and fallback | unit | `cargo test --features browser test_port_selection -x` | Wave 0 |
| UI-01 | Assets embedded in binary | unit | `cargo test --features browser test_assets_embedded -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --features browser`
- **Per wave merge:** `cargo test` (full suite including non-browser tests)
- **Phase gate:** Full suite green before verify

### Wave 0 Gaps
- [ ] `tests/test_browser.rs` -- integration tests for browser command server startup and port selection
- [ ] Unit tests in `src/commands/browser.rs` for `bind_listener` logic
- [ ] Feature-gated test compilation: tests need `#[cfg(feature = "browser")]`

## Open Questions

1. **axum-embed 0.1 long-term maintenance**
   - What we know: It works (validated in spike), version 0.1.0 only release
   - What's unclear: Whether it will keep up with axum updates past 0.7
   - Recommendation: Use it for now; if abandoned, replacing it with a manual `ServeDir`-style handler is straightforward

2. **debug-embed behavior in production build.rs flow**
   - What we know: `debug-embed` feature serves files from disk at runtime (great for dev). In release builds, files are embedded.
   - What's unclear: Whether to include `debug-embed` in the production feature set
   - Recommendation: Include it — it only activates in debug builds, harmless in release

## Sources

### Primary (HIGH confidence)
- Spike code (`spike/src/main.rs`, `spike/build.rs`, `spike/Cargo.toml`) -- validated working implementation
- [Tailwind CSS official docs](https://tailwindcss.com/docs/installation/using-vite) -- v4 Vite installation
- [open crate docs.rs](https://docs.rs/open) -- v5.3.3 API surface
- [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html) -- optional deps + cfg

### Secondary (MEDIUM confidence)
- cargo search results for crate versions (rust-embed 8.11.0, axum-embed 0.1.0)
- [Tailwind v4 blog post](https://tailwindcss.com/blog/tailwindcss-v4) -- v4 changes overview

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all crates validated in spike with exact versions
- Architecture: HIGH -- patterns proven in spike, just need feature-gating
- Pitfalls: HIGH -- build.rs path issues and route ordering are well-documented patterns

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable crates, 30-day window)
