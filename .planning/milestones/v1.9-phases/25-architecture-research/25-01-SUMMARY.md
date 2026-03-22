---
phase: 25-architecture-research
plan: 01
subsystem: infra
tags: [rust, axum, rust-embed, axum-embed, react, vite, react-flow, sqlite, websocket, spike]

# Dependency graph
requires: []
provides:
  - Cargo workspace with spike/ member crate
  - Vite React-TS frontend with @xyflow/react at web/
  - spike/build.rs npm build pipeline integration (npm install + npm run build)
  - spike/src/main.rs: axum server with rust-embed SPA, /ws WebSocket echo, read-only DB pool, graceful shutdown
  - web/dist/ artifacts: index.html + JS/CSS bundles from React Flow SPA
affects: [26-browser-command, 27-realtime-streaming, 28-production-polish]

# Tech tracking
tech-stack:
  added:
    - axum 0.7 (HTTP + WebSocket upgrade)
    - rust-embed 8 with axum-ex + debug-embed features
    - axum-embed 0.1 (ServeEmbed tower service)
    - tower-http 0.5 (trace + timeout middleware)
    - @xyflow/react 12 (React Flow node-graph component)
    - vite 8 (React-TS frontend build tool)
  patterns:
    - build.rs-driven npm pipeline: cargo build triggers npm install + npm run build
    - rust-embed #[folder] for compile-time SPA embedding
    - ServeEmbed<T>.nest_service pattern for SPA serving in axum
    - Read-only SqlitePool (max_connections=5, read_only=true) separate from main single-writer pool
    - Graceful shutdown via tokio::select! on ctrl_c + SIGTERM
    - WebSocket echo via WebSocketUpgrade extractor + on_upgrade(handler)

key-files:
  created:
    - spike/Cargo.toml
    - spike/build.rs
    - spike/src/main.rs
    - web/package.json
    - web/vite.config.ts
    - web/tsconfig.json
    - web/src/App.tsx
    - web/src/main.tsx
    - web/src/index.css
    - web/index.html
  modified:
    - Cargo.toml (added [workspace] section with members = ["spike"])
    - .gitignore (added web/dist/ and web/node_modules/)

key-decisions:
  - "spike/ is a Cargo workspace member with its own Cargo.toml — completely isolated from main crate"
  - "build.rs invokes npm install + npm run build in ../web — single cargo build command builds everything"
  - "rust-embed debug-embed feature forces embedding in debug builds — validates release behavior without release compilation"
  - "Read-only pool uses read_only(true) with max_connections(5) — never runs migrate!, never sets journal_mode"
  - "DB connection failure is non-fatal in spike — prints warning and continues (SPA+WS validation is primary goal)"
  - "axum-embed ServeEmbed handles ETag, compression, directory redirects — no hand-rolled MIME routing needed"

patterns-established:
  - "Read-only pool pattern: SqliteConnectOptions::new().read_only(true).busy_timeout() — separate from single-writer pool"
  - "build.rs npm pipeline: rerun-if-changed + npm detection + npm install + npm run build with current_dir(../web)"
  - "Graceful shutdown: tokio::select! on ctrl_c + terminate (unix cfg gate) passed to with_graceful_shutdown()"
  - "SPA serving: nest_service(\"/\", ServeEmbed::<EmbedStruct>::new()) — fallback to index.html for SPA routing"

requirements-completed: [SPIKE-1, SPIKE-2, SPIKE-4]

# Metrics
duration: 5min
completed: 2026-03-22
---

# Phase 25 Plan 01: Architecture Research Spike Summary

**axum server spike with rust-embed React Flow SPA, WebSocket echo on /ws, and read-only SQLite pool proving full v1.9 stack end-to-end**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-22T08:43:17Z
- **Completed:** 2026-03-22T08:48:59Z
- **Tasks:** 2
- **Files modified:** 14 (2 modified, 12 created)

## Accomplishments
- Cargo workspace extended with spike/ member crate — fully isolated from main squad-station crate
- Vite React-TS frontend with @xyflow/react scaffolded at web/ with Orchestrator/Worker node graph
- build.rs pipeline integrates npm build into cargo build — single command builds entire stack
- spike binary embeds React Flow SPA via rust-embed + axum-embed ServeEmbed, serves on port 3000
- WebSocket echo handler on /ws proves axum WS upgrade path (SPIKE-2)
- Read-only DB pool pattern established: separate from single-writer, no migrate!, no journal_mode (SPIKE-1)
- All 313 existing tests pass — workspace addition is non-breaking

## Task Commits

Each task was committed atomically:

1. **Task 1: Scaffold workspace, frontend project, and build pipeline** - `3fbef77` (feat)
2. **Task 2: Implement spike axum server with rust-embed SPA, WS echo, read-only DB pool, and graceful shutdown** - `4eb5316` (feat)

## Files Created/Modified
- `Cargo.toml` - Added [workspace] with members = ["spike"], resolver = "2"
- `.gitignore` - Added web/dist/ and web/node_modules/ entries
- `spike/Cargo.toml` - Spike crate: axum, rust-embed, axum-embed, sqlx, tokio deps
- `spike/build.rs` - npm detection, npm install, npm run build pipeline with cargo::rerun-if-changed
- `spike/src/main.rs` - Full axum server: SPA embed, /ws WebSocket, read-only pool, graceful shutdown
- `web/package.json` - React + Vite + @xyflow/react frontend project
- `web/vite.config.ts` - Vite configuration for React-TS build
- `web/tsconfig.json` + `web/tsconfig.app.json` + `web/tsconfig.node.json` - TypeScript configs
- `web/index.html` - Vite HTML entry point
- `web/src/App.tsx` - Minimal React Flow component: Orchestrator + Worker 1 + Worker 2 nodes
- `web/src/main.tsx` - React root entry (StrictMode + createRoot)
- `web/src/index.css` - Minimal CSS reset (React Flow manages its own styles)

## Decisions Made
- Used `axum-embed` (ServeEmbed) rather than hand-rolling MIME type routing — handles ETag, compression, fallback automatically
- `debug-embed` feature on rust-embed forces compile-time embedding in dev builds — validates exact release behavior without full release compilation
- Read-only pool pattern: `read_only(true)` with `max_connections(5)` — safe for concurrent reads alongside single-writer CLI pool under WAL mode
- DB connection failure is non-fatal in spike — warning printed, server continues (SPA + WS are the primary integration points)
- Workspace resolver = "2" specified explicitly for Cargo edition 2021 compatibility

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed TypeScript verbatimModuleSyntax strict import error in App.tsx**
- **Found during:** Task 1 (frontend build verification)
- **Issue:** Vite 8's react-ts template enables `verbatimModuleSyntax` in tsconfig; importing `Node` and `Edge` types as value imports causes TS error 1484
- **Fix:** Changed `import { ReactFlow, Node, Edge }` to `import { ReactFlow }` + `import type { Node, Edge }` (type-only import)
- **Files modified:** web/src/App.tsx
- **Verification:** `npm run build` in web/ exits 0, dist/ produced successfully
- **Committed in:** 3fbef77 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 bug)
**Impact on plan:** Necessary TypeScript strict-mode fix. No scope creep.

## Issues Encountered
- npm cache was root-owned (EACCES) — resolved by using `--cache /tmp/npm-cache` flag for all npm commands (no sudo required)

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SPIKE-1 (rust-embed + axum-embed SPA serving), SPIKE-2 (WebSocket echo), SPIKE-4 (Vite build pipeline) all validated
- SPIKE-3 (event detection strategy) and SPIKE-5 (PROJECT.md decisions) are separate plan items not covered in plan 01
- Phase 26 can now implement production `browser` command using established patterns from spike
- build.rs pattern for npm integration is proven — Phase 26 can adopt at root crate level
- Read-only pool pattern ready to use in production browser command
- axum-embed ServeEmbed pattern ready for production SPA serving

## Self-Check: PASSED

- spike/Cargo.toml: FOUND
- spike/build.rs: FOUND
- spike/src/main.rs: FOUND
- web/package.json: FOUND
- web/src/App.tsx: FOUND
- web/dist/index.html: FOUND
- 25-01-SUMMARY.md: FOUND
- Commit 3fbef77: FOUND
- Commit 4eb5316: FOUND

---
*Phase: 25-architecture-research*
*Completed: 2026-03-22*
