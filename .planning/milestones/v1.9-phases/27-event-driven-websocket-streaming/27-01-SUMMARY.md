---
phase: 27-event-driven-websocket-streaming
plan: 01
subsystem: api
tags: [axum, websocket, tokio, broadcast, sqlite, streaming, polling]

# Dependency graph
requires:
  - phase: 26-axum-server-and-cli-command
    provides: AppState with db pool, axum server with /ws echo handler, feature-gated browser module

provides:
  - broadcast::Sender<String> wired into AppState and spawned polling tasks
  - build_snapshot() returning full JSON agents+messages frame on WS connect
  - agents_changed() and messages_changed() delta detection functions
  - reconcile_for_server() adapting tmux session check pattern for server context
  - poll_agents() background task at 500ms with reconciliation + broadcast on change
  - poll_messages() background task at 200ms with broadcast on change
  - Real WS handler replacing echo: subscribes before snapshot, handles lagged receivers
  - Writable DB pool wired into run() for reconciliation

affects:
  - 27-02 (frontend WS integration will consume these events)
  - 28-react-flow-dynamic (React Flow nodes driven by WS snapshot/update events)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - tokio::sync::broadcast channel (capacity 32) for multi-client WS fan-out
    - Subscribe to broadcast BEFORE building initial snapshot to prevent race conditions
    - Delta detection via field-level comparison (not hash) for agents and messages
    - serde_json::json! macro for serialization without Clone on DB structs
    - Separate read-only pool (polling reads) and writable pool (reconciliation writes)
    - Graceful lagged-receiver handling (continue skip, not disconnect)

key-files:
  created: []
  modified:
    - src/commands/browser.rs

key-decisions:
  - "Use serde_json::json! macro for WS event serialization instead of WsEvent enum — Agent/Message do not derive Clone so owned enum variants would require Clone or lifetime annotations; json! works from references with no change to existing DB structs"
  - "Subscribe to broadcast channel BEFORE building snapshot in ws_handler to prevent missing events during snapshot query"
  - "reconcile_for_server() is a standalone fn (not calling helpers::reconcile_agent_statuses) to avoid coupling server code to CLI helpers"
  - "broadcast channel capacity 32 with Lagged error handling — lagged clients skip missed events since next poll will push current state"

patterns-established:
  - "Event type discriminator: serde_json::json!({'type': 'snapshot'|'agent_update'|'message_update', ...})"
  - "Polling loop: interval.tick().await -> fetch -> compare against cached -> broadcast if changed -> update cache (move)"
  - "DB pools in AppState: db (read-only) for polling reads, db_write (writable) for reconciliation writes"

requirements-completed: [RT-01, RT-02, RT-03]

# Metrics
duration: 15min
completed: 2026-03-22
---

# Phase 27 Plan 01: Event-Driven WebSocket Streaming Summary

**tokio::sync::broadcast WS streaming with 500ms agent and 200ms message polling, delta detection, and full snapshot on connect — replacing the Phase 26 echo handler**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T10:10:00Z
- **Completed:** 2026-03-22T10:25:00Z
- **Tasks:** 2 (implemented together in one cohesive file rewrite)
- **Files modified:** 1

## Accomplishments

- Replaced the Phase 26 echo WebSocket handler with real event-driven streaming infrastructure
- Implemented delta detection for agents (name, status, status_updated_at, current_task fields) and messages (id, status, updated_at fields) — broadcasts only on actual change
- Wired broadcast channel (capacity 32), separate writable DB pool, and two background polling tasks into run() before server start
- WS handler subscribes to broadcast BEFORE building initial snapshot (Pitfall 4 from research — prevents missing events during snapshot query)

## Task Commits

1. **Task 1 + Task 2: Add WsEvent types, polling tasks, replace WS handler, wire run()** - `9a21ad6` (feat)

## Files Created/Modified

- `src/commands/browser.rs` — Added broadcast channel, AppState db_write field, build_snapshot(), agents_changed(), messages_changed(), reconcile_for_server(), poll_agents(), poll_messages(), replaced echo handler with real streaming, wired polling tasks into run()

## Decisions Made

- Used `serde_json::json!` macro for WS event serialization instead of a `WsEvent` enum with owned data — Agent/Message don't derive Clone, and modifying existing DB structs would violate the v1.9 "additive only" constraint. The json! macro serializes from references directly.
- Subscribe to broadcast BEFORE building initial snapshot in ws_handler to prevent a race where new events are missed while the snapshot DB query is running.
- `reconcile_for_server()` is a standalone async fn duplicating the helpers.rs pattern rather than calling `helpers::reconcile_agent_statuses()` directly — this avoids coupling the browser server code to CLI helpers and keeps the server module self-contained.
- Broadcast channel capacity 32 with `RecvError::Lagged` handled via `continue` — lagged clients simply skip to the next event, which will contain current state since the poll loop always reads from DB.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `futures::SinkExt` import for SplitSink.send()**
- **Found during:** Task 2 (ws_handler / handle_socket implementation)
- **Issue:** `socket.split()` returns `SplitSink` which requires `SinkExt` trait in scope for `.send()` method. The plan specified `use futures::stream::StreamExt` but `SinkExt` was not mentioned.
- **Fix:** Changed `use futures::StreamExt;` to `use futures::{SinkExt, StreamExt};`
- **Files modified:** src/commands/browser.rs
- **Verification:** `cargo check --features browser` passes clean
- **Committed in:** 9a21ad6 (task commit)

---

**Total deviations:** 1 auto-fixed (1 blocking — missing trait import)
**Impact on plan:** Fix was necessary for compilation. No scope creep.

## Issues Encountered

None beyond the SinkExt trait import above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Server-side WS streaming is fully functional and ready for frontend integration
- Frontend (web/src/) needs to be updated in Plan 27-02 to consume snapshot/agent_update/message_update events and update React state
- The `ConnectionStatus.tsx` component already handles connect/disconnect/reconnect — it just needs to process incoming JSON messages
- All 313 tests pass; cargo check --features browser is clean

## Self-Check: PASSED

All files found: browser.rs, 27-01-SUMMARY.md. Commit 9a21ad6 verified.

---
*Phase: 27-event-driven-websocket-streaming*
*Completed: 2026-03-22*
