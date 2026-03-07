---
phase: 02-lifecycle-and-hooks
plan: 01
subsystem: database
tags: [sqlite, sqlx, agent-status, lifecycle, hooks, tmux]

# Dependency graph
requires:
  - phase: 01-core-foundation
    provides: agents table, Agent struct, signal command, send command, db::connect, config module
provides:
  - SQLite migration adding status and status_updated_at columns to agents table
  - update_agent_status() function for idle/busy/dead lifecycle tracking
  - 4-layer guard logic in signal.rs preventing infinite loops and handling edge cases
  - Agent status transitions: idle -> busy (send) -> idle (signal)
affects: [03-context-and-output, signal-command, send-command, agent-lifecycle]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Guard-first pattern: cheapest checks first (env var before DB connection)"
    - "Exit-0 contract: hooks must never fail the provider (stderr warnings, always Ok(()))"
    - "Lifecycle state machine: idle -> busy (send) -> idle (signal)"

key-files:
  created:
    - src/db/migrations/0002_agent_status.sql
  modified:
    - src/db/agents.rs
    - src/commands/signal.rs
    - src/commands/send.rs

key-decisions:
  - "Guard order in signal.rs: TMUX_PANE check first (cheapest), then config/DB, then agent lookup, then role check"
  - "Unregistered agent in signal returns Ok(()) silently — bail! was wrong for hook context"
  - "Orchestrator role check (guard 4) prevents infinite loop where orchestrator AfterAgent hook signals itself"
  - "Config/DB errors in signal print to stderr but always exit 0 — hooks must never fail providers"

patterns-established:
  - "Guard-first signal pattern: all early exits via Ok(()) — hooks must never propagate errors to provider"
  - "Agent status updated in send (busy) and signal (idle) — status always reflects current work state"

requirements-completed: [SESS-03, HOOK-01, HOOK-03]

# Metrics
duration: 2min
completed: 2026-03-06
---

# Phase 2 Plan 01: Agent Status and Signal Guards Summary

**SQLite migration adding agent lifecycle columns, update_agent_status() function, and 4-layer guard logic in signal.rs preventing orchestrator infinite loops and handling unregistered/non-tmux invocations with exit 0**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-06T06:52:39Z
- **Completed:** 2026-03-06T06:54:36Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Created migration 0002_agent_status.sql adding `status` (idle/busy/dead) and `status_updated_at` columns to agents table
- Added `update_agent_status()` to agents.rs enabling lifecycle state transitions
- Refactored signal.rs with 4-layer guard logic: TMUX_PANE check, config/DB failure handling, unregistered agent silent exit, orchestrator self-signal prevention
- Updated send.rs to mark agents as "busy" when a task is dispatched, completing the send->signal lifecycle arc

## Task Commits

Each task was committed atomically:

1. **Task 1: Add agent status migration and update_agent_status function** - `6f322a9` (feat)
2. **Task 2: Refactor signal command with 4-layer guard logic** - `9303e76` (feat)

## Files Created/Modified

- `src/db/migrations/0002_agent_status.sql` - ALTER TABLE adding status and status_updated_at columns with NOT NULL DEFAULT
- `src/db/agents.rs` - Agent struct extended with status fields; update_agent_status() function added
- `src/commands/signal.rs` - Complete refactor with 4-layer guard logic replacing bail! with silent Ok(())
- `src/commands/send.rs` - Added update_agent_status("busy") call after message insert

## Decisions Made

- Guard ordering in signal.rs is intentional: TMUX_PANE env var check is cheapest (no I/O), placed first. Config/DB next (file I/O but needed for all subsequent work). Agent lookup third. Role check last (requires Agent struct already fetched).
- Replaced `bail!("Agent not found: {}", agent)` with silent `Ok(())` — the bail was correct for interactive CLI use but wrong for hook context where unregistered agents are expected (hook fires before any agent is registered during development).
- Config/DB errors in signal go to stderr (not silently swallowed) to aid debugging while still exiting 0 to comply with hook contract.
- send.rs status update placed after `insert_message` but before `send_keys_literal` — DB is source of truth; status is set before the task text is injected.

## Deviations from Plan

None — plan executed exactly as written. The send.rs update was included in the Task 2 plan spec ("Also update the send command to set agent status to busy").

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Agent status foundation is complete; downstream commands (agents list) can now display status column
- Signal guards prevent infinite loops — safe to configure AfterAgent hooks in Gemini CLI
- Status transitions (idle/busy/dead) are wired; Phase 2 remaining plans can build on this lifecycle model
- Ready for 02-02 (next plan in phase)

---
*Phase: 02-lifecycle-and-hooks*
*Completed: 2026-03-06*
