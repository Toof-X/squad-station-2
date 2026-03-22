# Phase 25: Architecture Research - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Spike all integration points for v1.9 Browser Visualization before any production code is written. The output is runnable proof-of-concept code in an isolated workspace member (`spike/`) plus architecture decisions recorded in PROJECT.md Key Decisions table. No production code ships in this phase — only validated patterns and locked decisions.

</domain>

<decisions>
## Implementation Decisions

### Spike format & output
- Produce runnable proof-of-concept code, committed to repo
- Spike code lives in a separate `spike/` directory as a Cargo workspace member — completely isolated from main crate, honoring the additive-only constraint
- Architecture decisions go directly into PROJECT.md Key Decisions table — no separate research document
- Pass/fail bar is "it compiles and works" — no formal test-first criteria, keep the spike fast and MVP-focused

### Build pipeline integration
- `build.rs` script in the Rust cargo project auto-runs `npm run build` for the frontend SPA whenever `cargo build` is executed — seamless single-command build
- If `dist/` doesn't exist (first clone, no node/npm installed), `cargo build` hard-fails with a clear error message telling the user what's missing
- CI/CD pipeline updates deferred to Phase 26 — no CI changes for the research spike
- React app has its own independent `web/package.json` inside its directory, separate from the existing `npm-package/` distribution wrapper

### Frontend project location & structure
- React app lives at `web/` at the repo root (`web/package.json`, `web/src/`, `web/dist/`)
- Use Vite's React-TS template with TypeScript to catch errors early
- Install and validate React Flow in the spike — prove the full chain (Vite -> React Flow -> dist -> rust-embed -> axum serve)
- Build artifacts not committed: `web/dist/` and `web/node_modules/` added to `.gitignore`

### Tokio runtime coexistence
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — v1.9 requirements (SRV-01 through UI-03); Phase 25 has no direct requirements (pre-implementation research)

### Roadmap
- `.planning/ROADMAP.md` — Phase 25 success criteria define the 4 validation points that must pass

### Existing patterns to understand
- `src/db/mod.rs` — Current `db::connect()` with single-writer pool (`max_connections(1)`) and WAL mode — server needs a separate read-only pool
- `src/main.rs` — Command dispatch pattern via `#[tokio::main]` — `browser` command will follow same pattern but block until Ctrl+C
- `src/commands/ui.rs` — Existing long-running TUI command — `browser` follows same terminal-blocking pattern
- `src/commands/watch.rs` — Another long-running command with daemon option — reference for Ctrl+C shutdown
- `Cargo.toml` — Current dependency set; spike crate will add axum, rust-embed, tower-http, tokio-tungstenite

### Build & distribution
- `npm-package/` — Existing npm distribution wrapper; completely separate from the new `web/` React app
- `.gitignore` — Needs `web/dist/` and `web/node_modules/` entries

### Prior milestone context
- `.planning/milestones/v1.8-phases/24-agent-role-templates-in-wizard/24-CONTEXT.md` — Most recent phase context; v1.8 fully shipped

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `db::connect()` in `src/db/mod.rs` — Connection pool factory; spike needs a variant or second call with read-only options for the server
- `db::agents::get_all_agents()` — Fetches agent list from DB; server will call this to build topology
- `db::messages::get_messages()` — Fetches messages with filters; server will call this for in-flight message state
- `config::load_config()` in `src/config.rs` — Loads squad.yml; server needs this for topology hierarchy

### Established Patterns
- Single-writer pool (`max_connections(1)`) — Production commands use this; server must NOT share this pool
- Connect-per-refresh in TUI (`src/commands/ui.rs`) — Prevents WAL starvation; server's read-only pool is a different solution to the same problem
- Command dispatch in `main.rs` — `Browser { port }` variant will be added to `Commands` enum in Phase 26
- `#[tokio::main]` async runtime — All commands share this; axum `.serve()` runs inside the same runtime

### Integration Points
- `cli.rs` `Commands` enum — Will need `Browser` variant (Phase 26, not spike)
- `Cargo.toml` — Spike crate has its own `Cargo.toml`; production deps added to root in Phase 26
- `.gitignore` — Needs `web/dist/` and `web/node_modules/` entries (can be added in spike phase)
- `build.rs` — New file at repo root; auto-runs `npm run build` in `web/` directory

</code_context>

<specifics>
## Specific Ideas

- The spike should validate all 4 integration points as a cohesive mini-app: axum serves rust-embed'd React Flow SPA with a WebSocket echo endpoint — proving the entire stack works together, not just individual pieces
- `build.rs` should check for `npm`/`node` availability and provide actionable error messages (e.g., "Install Node.js from https://nodejs.org to build the browser UI")
- The read-only connection pool can use `SqliteConnectOptions` with `read_only(true)` and a higher `max_connections` since reads don't contend

</specifics>

<deferred>
## Deferred Ideas

None captured during discussion.

</deferred>

---

*Phase: 25-architecture-research*
*Context gathered: 2026-03-22*
