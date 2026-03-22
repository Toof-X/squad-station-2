# Phase 26: Axum Server & CLI Command - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Add the `squad-station browser` command that starts an embedded axum web server serving a React + React Flow SPA from rust-embed bundled assets. The command auto-opens the default browser, supports `--port` and `--no-open` flags, and shuts down gracefully on Ctrl+C. The browser feature is cargo feature-gated so the core CLI stays node-free. Phase 26 delivers the server + CLI + working SPA shell; real-time WS streaming (Phase 27) and full node graph (Phase 28) come later.

</domain>

<decisions>
## Implementation Decisions

### Port selection & conflict behavior
- Default (no `--port`): try port 3000 first, auto-fallback to a random available port (bind port 0) if 3000 is taken
- Explicit `--port N`: bind to that port or error with a clear message ("port N already in use") — no auto-fallback when user specifies a port
- Bind to `127.0.0.1` only (localhost) — no network exposure
- Include `--no-open` flag to skip auto-opening the browser (for headless/SSH environments)

### Feature gating
- `browser` command is behind a cargo feature flag (`--features browser`) — core CLI stays zero-dependency, no npm/node required
- When built WITHOUT the feature: `squad-station browser` subcommand still appears in `--help` but prints "browser feature not enabled, rebuild with `--features browser`" and exits
- CI/CD always builds with `--features browser` — published binaries (npm + curl installer) always include the SPA
- End-users never see or think about the feature flag — it's purely a dev/contributor concern

### SPA content for Phase 26
- Reuse the spike's React Flow demo as the initial SPA — shows nodes/edges, proves the full embed pipeline works end-to-end
- SPA connects to `/ws` WebSocket endpoint and shows connection status — proves the full round-trip (even though real WS streaming comes in Phase 27)
- Set up a CSS/design system foundation now (e.g., Tailwind or equivalent) so Phase 28 can focus on components, not styling infrastructure
- Include a REST endpoint (e.g., `/api/status`) that returns basic project info from DB/config — proves the data flow pipeline works before Phase 27/28 builds on it

### Claude's Discretion
- Exact cargo feature flag name and conditional compilation approach (`#[cfg(feature = "...")]`)
- build.rs implementation for conditional frontend build (only when feature enabled)
- Browser auto-open implementation (`open` crate vs raw `std::process::Command`)
- REST API endpoint design and response shape
- Design system choice (Tailwind, CSS modules, etc.)
- SPA project structure and component organization
- Test strategy for the new command and server
- Migration of spike code to production modules

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — Phase 26 requirements: SRV-01, SRV-02, SRV-03, SRV-04, UI-01

### Roadmap
- `.planning/ROADMAP.md` — Phase 26 success criteria (5 items) define the acceptance bar

### Prior phase context
- `.planning/milestones/v1.9-phases/25-architecture-research/25-CONTEXT.md` — All architecture decisions from spike phase; patterns validated there are canonical

### Spike code (reference implementation)
- `spike/src/main.rs` — Validated axum + rust-embed + WS echo + read-only DB pool pattern
- `spike/build.rs` — Validated npm install + build pipeline in build.rs
- `spike/Cargo.toml` — Dependency versions that compile and work together

### Existing patterns to follow
- `src/cli.rs` — `Commands` enum; add `Browser` variant here
- `src/main.rs` — Command dispatch; add `Browser` arm following existing pattern
- `src/commands/ui.rs` — Long-running terminal-blocking command; `browser` follows same pattern
- `src/commands/watch.rs` — Graceful shutdown reference
- `src/db/mod.rs` — `db::connect()` with single-writer pool; server needs separate `connect_readonly()`
- `src/config.rs` — `load_config()` for squad.yml; server needs this for `/api/status`

### Build & distribution
- `Cargo.toml` — Add axum, rust-embed, axum-embed, tower-http as optional deps behind feature flag
- `.github/workflows/` — CI matrix needs `--features browser` added to build command
- `web/` — Frontend source (from spike); `web/dist/` and `web/node_modules/` already in `.gitignore`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable from spike
- `connect_readonly()` in `spike/src/main.rs:22-32` — Read-only pool factory; move to `src/db/mod.rs` as `pub fn connect_readonly()`
- `shutdown_signal()` in `spike/src/main.rs:51-68` — Ctrl+C + SIGTERM handler; move to `src/commands/browser.rs`
- `ws_handler()` + `handle_socket()` — WebSocket echo; move to browser module, will be replaced in Phase 27
- `ServeEmbed::<FrontendAssets>::new()` — SPA serving pattern; use as-is
- `build.rs` — npm build pipeline; adapt for conditional execution behind feature flag

### Patterns to maintain
- Single-writer pool (`max_connections(1)`) in `src/db/mod.rs` — NEVER share with server; server uses read-only pool
- Connect-per-refresh in TUI (`src/commands/ui.rs`) — Server's persistent read-only pool is the appropriate alternative for long-lived server process
- Command dispatch in `main.rs` — `Browser` variant follows exact same async pattern as other commands
- Additive only — new files: `src/commands/browser.rs`, production `build.rs`; no modifications to existing command logic

### Integration points
- `src/cli.rs` `Commands` enum — Add `Browser { port: Option<u16>, no_open: bool }` variant
- `Cargo.toml` `[features]` section — New `browser` feature enabling axum, rust-embed, axum-embed deps
- `build.rs` at repo root — Conditional: only run npm build when `browser` feature is active
- `.github/workflows/*.yml` — Add `--features browser` to cargo build commands in CI matrix

</code_context>

<specifics>
## Specific Ideas

- Port 3000 try-then-fallback: attempt `TcpListener::bind("127.0.0.1:3000")`, if `AddrInUse` error then bind to port 0 and report the actual assigned port
- Print the actual URL to terminal (including auto-selected port) so the user always knows where to connect
- The `/api/status` endpoint can return project name (from squad.yml), agent count (from DB), and server uptime — minimal but proves the full data pipeline
- Feature-gated subcommand: use `#[cfg(feature = "browser")]` on the command module import and match arm; the "not enabled" fallback is a simple non-gated arm that prints the message

</specifics>

<deferred>
## Deferred Ideas

None captured during discussion.

</deferred>

---

*Phase: 26-axum-server-and-cli-command*
*Context gathered: 2026-03-22*
