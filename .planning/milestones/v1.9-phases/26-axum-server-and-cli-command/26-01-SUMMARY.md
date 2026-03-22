---
phase: 26-axum-server-and-cli-command
plan: 01
subsystem: server
tags: [axum, rust-embed, axum-embed, websocket, spa, tower-http, open, sqlite, feature-flags]

# Dependency graph
requires:
  - phase: 25-architecture-research
    provides: axum + rust-embed + axum-embed patterns validated in spike; connect_readonly pool pattern; build.rs npm pipeline approach
provides:
  - Feature-gated `browser` cargo feature with axum, rust-embed, axum-embed, tower-http, open dependencies
  - build.rs npm pipeline: conditionally runs npm install + npm run build in web/ only when browser feature active
  - connect_readonly() in src/db/mod.rs: read-only multi-reader pool (5 connections, no migrate!, no journal_mode)
  - src/commands/browser.rs: full browser command implementation with server startup, port selection, SPA serving, WebSocket echo, /api/status endpoint, graceful shutdown
  - CLI Browser subcommand with --port and --no-open flags
  - cfg-gated Browser dispatch in main.rs with "not enabled" fallback message
affects: [27-websocket-streaming, future-browser-phases]

# Tech tracking
tech-stack:
  added:
    - axum 0.7 with ws feature (optional, browser feature gate)
    - rust-embed 8 with axum-ex and debug-embed features (optional)
    - axum-embed 0.1 (optional)
    - tower-http 0.5 with trace and timeout features (optional)
    - open 5 (optional, system browser launcher)
  patterns:
    - Feature-gated optional dependencies in Cargo.toml with dep: prefix syntax
    - build.rs early-exit pattern: check CARGO_FEATURE_BROWSER first, return if unset
    - connect_readonly: read_only(true), max_connections(5), busy_timeout(5s), no migrate!
    - Route ordering: explicit routes (/api/status, /ws) BEFORE nest_service("/") SPA fallback
    - Port fallback: no --port tries 3000 then random; explicit --port errors if taken (no fallback)
    - Graceful shutdown via tokio::select! on ctrl_c + SIGTERM

key-files:
  created:
    - build.rs (repo root) — conditional npm pipeline, runs only with browser feature
    - src/commands/browser.rs — full browser command: run(), bind_listener(), shutdown_signal(), ws_handler(), api_status(), FrontendAssets embed, AppState
  modified:
    - Cargo.toml — [features] section, optional browser deps, build = "build.rs"
    - src/db/mod.rs — added connect_readonly() function
    - src/cli.rs — added Browser variant with --port and --no-open
    - src/commands/mod.rs — added cfg-gated browser module
    - src/main.rs — added cfg-gated Browser dispatch arm with "not enabled" fallback

key-decisions:
  - "Route ordering in axum: explicit /api/status and /ws routes registered BEFORE nest_service('/') SPA fallback — ensures API routes take priority"
  - "Port fallback asymmetry: omitting --port falls back from 3000 to random; explicit --port fails hard if taken — per CONTEXT.md decision"
  - "DB graceful degradation: if config missing or DB unavailable, browser command continues with None pool rather than failing"
  - "connect_readonly is NOT feature-gated — it is a general utility in db/mod.rs available to any future consumer"

patterns-established:
  - "Feature gate pattern: cfg(feature = 'browser') on module declaration + both match arms (#[cfg] and #[cfg(not)]) in main.rs dispatch"
  - "Embedded SPA: #[derive(Embed)] #[folder = 'web/dist/'] with ServeEmbed as catch-all last route"

requirements-completed: [SRV-01, SRV-02, SRV-03, SRV-04, UI-01]

# Metrics
duration: 15min
completed: 2026-03-22
---

# Phase 26 Plan 01: Axum Server and CLI Command Summary

**Feature-gated axum server with rust-embed SPA serving, port fallback logic, WebSocket echo, /api/status JSON endpoint, and graceful shutdown behind `--features browser`**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T09:31:00Z
- **Completed:** 2026-03-22T09:46:52Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments
- cargo feature flag `browser` gates all optional axum/rust-embed/open dependencies — base build unaffected
- build.rs npm pipeline: auto-runs `npm install && npm run build` in `web/` only when browser feature active
- `squad-station browser` starts axum server on 127.0.0.1, prints URL, auto-opens system browser (or skips with --no-open)
- Port selection: default tries 3000 with random fallback; explicit --port errors clearly if taken
- GET /ws returns WebSocket echo (upgrades cleanly), GET /api/status returns JSON, GET / serves embedded SPA
- All 13 existing tests pass in both feature modes; "not enabled" message on non-feature build

## Task Commits

Each task was committed atomically:

1. **Task 1: Add feature-gated dependencies, build.rs, and connect_readonly** - `5ae6a3c` (feat)
2. **Task 2: Implement browser command, CLI wiring, and feature-gated dispatch** - `38501ad` (feat)

## Files Created/Modified
- `build.rs` (created) — conditional npm pipeline, early-exits without browser feature
- `src/commands/browser.rs` (created) — full browser command implementation (130+ lines)
- `Cargo.toml` (modified) — [features], optional browser deps, build = "build.rs"
- `src/db/mod.rs` (modified) — added connect_readonly() for read-only multi-reader pool
- `src/cli.rs` (modified) — Browser variant with --port and --no-open flags
- `src/commands/mod.rs` (modified) — cfg-gated browser module declaration
- `src/main.rs` (modified) — cfg-gated Browser dispatch with "not enabled" fallback

## Decisions Made
- Used `dep:axum` syntax in feature declaration (Rust 2021 edition optional dep syntax)
- connect_readonly is NOT feature-gated — it is a general db utility for any future use
- DB gracefully degrades to None if config or DB is unavailable (server still starts)
- Route ordering: /api/status and /ws are registered before nest_service("/") to ensure priority

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Browser server infrastructure complete — ready for Phase 27 WebSocket streaming
- FrontendAssets embeds from `web/dist/` — React Flow SPA will serve at GET /
- WebSocket echo at /ws is a placeholder — Phase 27 replaces with real event streaming
- /api/status endpoint functional with real DB agent count when squad.yml present

---
*Phase: 26-axum-server-and-cli-command*
*Completed: 2026-03-22*
