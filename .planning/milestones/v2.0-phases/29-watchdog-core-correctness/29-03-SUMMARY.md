---
phase: 29-watchdog-core-correctness
plan: 03
subsystem: cli
tags: [watchdog, status, json, serde, operator-tooling]

# Dependency graph
requires:
  - phase: 29-02
    provides: "DeadlockState and NudgeState structs with cooldown, debounce, and escalation logic"
provides:
  - "WatchStatus struct with serde serialization for JSON status persistence"
  - "Per-tick watch.status.json file written to .squad/ directory"
  - "--status subcommand reading status file and displaying formatted output"
  - "Stale PID file cleanup in show_status"
affects: [30, 31]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "File-based IPC: status JSON written per tick, read by CLI subcommand — no daemon socket needed"
    - "Graceful degradation: missing PID, stale PID, missing status file all handled with clear messages"

key-files:
  created: []
  modified:
    - src/commands/watch.rs

key-decisions:
  - "Status file uses serde_json for structured serialization rather than plain text — enables future tooling"
  - "show_status resolves config/db path independently since it returns before the main loop config resolution"

patterns-established:
  - "File-based status reporting: write JSON per tick, read on demand with CLI flag"

requirements-completed: [OPS-01, OPS-02]

# Metrics
duration: 2min
completed: 2026-03-24
---

# Phase 29 Plan 03: Status Subcommand Summary

**Per-tick watch.status.json persistence and --status subcommand displaying PID, uptime, stall state, nudge counts, and configuration**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-24T07:10:09Z
- **Completed:** 2026-03-24T07:12:26Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- WatchStatus struct with full serde Serialize/Deserialize for JSON round-trip
- Per-tick status file written to .squad/watch.status.json with PID, timestamps, nudge counts, debounce state, stall state
- --status subcommand reads status file and displays formatted output with uptime calculation
- Graceful handling of missing PID file, stale daemon PID, and missing status file
- Status file cleaned up on graceful shutdown alongside PID file

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement WatchStatus struct and per-tick status file writing** - `2e8cd4c` (feat)
2. **Task 2: Implement --status subcommand reading and formatted output** - `694f48a` (feat)

## Files Created/Modified
- `src/commands/watch.rs` - WatchStatus struct, write_status(), show_status(), status file cleanup on shutdown

## Decisions Made
- show_status() resolves config path independently (early return before main loop setup) to avoid duplicating config loading logic
- Status file uses serde_json structured format rather than plain text for future tooling compatibility

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All Phase 29 plans complete (3/3) — watchdog core correctness fully implemented
- Ready for Phase 30 (Telegram notifications) and Phase 31 (integration testing)
- Full test suite passes (13 tests including NudgeState, DeadlockState, and integration tests)

---
*Phase: 29-watchdog-core-correctness*
*Completed: 2026-03-24*

## Self-Check: PASSED
- src/commands/watch.rs exists on disk
- Commit 2e8cd4c (Task 1) verified in git log
- Commit 694f48a (Task 2) verified in git log
- WatchStatus struct present with serde derive
- write_status() and show_status() functions present
- watch.status.json referenced for write and cleanup
- All 13 tests passing, cargo build --release succeeds
