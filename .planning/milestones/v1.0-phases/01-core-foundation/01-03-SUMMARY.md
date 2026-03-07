---
phase: 01-core-foundation
plan: 03
subsystem: api
tags: [rust, sqlx, sqlite, tmux, owo-colors, anyhow, serde_json]

requires:
  - plan: 01-01
    provides: "DB layer (insert_message, update_status, get_agent), tmux module (send_keys_literal, session_exists), config layer (load_config, resolve_db_path)"

provides:
  - "send command: validates agent in DB + tmux session alive, writes message with priority to DB, injects task via literal send-keys (SAFE-02)"
  - "signal command: idempotent completion update (MSG-03), structured orchestrator notification via [SIGNAL] format, graceful orchestrator-down handling"
  - "get_orchestrator() helper in db/agents.rs for role-based agent lookup"
  - "Both commands support --json output and --priority flag (send)"

affects:
  - 01-04
  - 01-05
  - 02-01

tech-stack:
  added: []
  patterns:
    - "DB path resolution: load squad.yml from cwd via config::load_config + config::resolve_db_path"
    - "Terminal color detection: std::io::IsTerminal::is_terminal() on stdout"
    - "Idempotent signal: UPDATE WHERE status='pending' LIMIT 1 — rows=0 is silent success, not error"
    - "Orchestrator lookup: get_orchestrator() queries role='orchestrator' LIMIT 1"
    - "Post-update task_id retrieval: SELECT id WHERE status='completed' ORDER BY updated_at DESC LIMIT 1"

key-files:
  created: []
  modified:
    - "src/commands/send.rs — full implementation replacing todo!() stub"
    - "src/commands/signal.rs — full implementation replacing todo!() stub"
    - "src/db/agents.rs — added get_orchestrator() helper"

key-decisions:
  - "Used std::io::IsTerminal trait (Rust stdlib) for terminal detection, not owo-colors stream API (which doesn't exist in v3)"
  - "signal retrieves task_id via SELECT after UPDATE rather than RETURNING clause — simpler, avoids SQLite RETURNING compatibility concerns"
  - "Orchestrator notification only fires when rows_affected > 0 — duplicate signals do not trigger spurious notifications"
  - "When orchestrator tmux session is down, signal is persisted in DB only — not an error per user decision (signals queue in DB)"

patterns-established:
  - "Command pattern: load squad.yml -> connect DB -> validate -> execute -> output (json or tty-aware text)"
  - "Idempotency gate: check rows_affected before side effects (notification, output differentiation)"
  - "Structured notification format: [SIGNAL] key=value key=value (no JSON, simple grep-friendly)"

requirements-completed:
  - MSG-01
  - MSG-02
  - MSG-03
  - MSG-05

duration: ~10min
completed: 2026-03-06
---

# Phase 1 Plan 03: Messaging Commands Summary

**send command routing tasks via DB + tmux literal injection (SAFE-02), signal command with idempotent MSG-03 completion update and structured [SIGNAL] orchestrator notification**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-06T05:11:43Z
- **Completed:** 2026-03-06T05:21:12Z
- **Tasks:** 2
- **Files modified:** 3 (0 created, 3 modified)

## Accomplishments
- send command fully validates agent existence in DB and tmux session liveness before writing, preventing stale-state sends
- signal command achieves true MSG-03 idempotency: duplicate signals return exit 0 with friendly message, no DB corruption
- Orchestrator notification uses structured key=value format [SIGNAL] agent=X status=completed task_id=Y, queryable from logs
- get_orchestrator() helper enables role-based agent discovery without hardcoding names

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement send command** - `e9850a1` (feat) — bundled with 01-02 init command by parallel execution agent; send.rs full implementation committed
2. **Task 2: Implement signal command + get_orchestrator()** - `fe8b238` (feat)

## Files Created/Modified
- `src/commands/send.rs` — Full implementation: config load, DB connect, agent validation, tmux session check, insert_message(), send_keys_literal(), json/tty output
- `src/commands/signal.rs` — Full implementation: config load, DB connect, agent validation, update_status() idempotency, task_id retrieval, get_orchestrator(), conditional tmux notification, json/tty output
- `src/db/agents.rs` — Added get_orchestrator() function querying role='orchestrator'

## Decisions Made
- Used `std::io::IsTerminal` from Rust stdlib rather than owo-colors stream API (owo-colors v3 does not expose a `stream` module)
- signal retrieves task_id via a SELECT query after the UPDATE rather than RETURNING clause — avoids SQLite RETURNING compatibility edge cases and is simpler
- Orchestrator notification only fires when rows_affected > 0 — prevents duplicate [SIGNAL] messages on idempotent calls
- When orchestrator tmux session is down, signal is persisted in DB only (no error) — consistent with user decision "signals queue in DB regardless of orchestrator availability"

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed owo-colors terminal detection API**
- **Found during:** Task 1 (send command implementation)
- **Issue:** Plan suggested `owo_colors::stream::IsTerminal::is_terminal()` but owo-colors v3 does not have a `stream` module — E0433 compile error
- **Fix:** Used `std::io::IsTerminal` trait (stdlib, Rust 1.70+) with `std::io::stdout().is_terminal()`
- **Files modified:** `src/commands/send.rs` (applied in both send.rs and signal.rs)
- **Verification:** `cargo check` passes with zero errors
- **Committed in:** `e9850a1` (Task 1 commit, bundled with 01-02 parallel execution)

---

**Total deviations:** 1 auto-fixed (Rule 1 — Bug)
**Impact on plan:** Fix was necessary for compilation. No scope creep. The std::io::IsTerminal approach is idiomatic Rust and works correctly.

## Issues Encountered
- The parallel execution of plan 01-02 also committed `send.rs` (it encountered the same owo-colors API bug and fixed it inline per Rule 1). Task 1 send.rs implementation is confirmed present and correct in commit `e9850a1`.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Core messaging path is complete: orchestrator can send tasks to agents, agents can signal completion back
- send + signal form the complete MSG-01 through MSG-03 + MSG-05 requirement chain
- list (01-04) and peek (01-04) commands are already implemented by parallel execution
- Phase 1 Wave 2 is functionally complete for the messaging path
- No blockers for Phase 2 (hook integration)

---
*Phase: 01-core-foundation*
*Completed: 2026-03-06*

## Self-Check: PASSED

All files verified present:
- FOUND: src/commands/send.rs
- FOUND: src/commands/signal.rs
- FOUND: src/db/agents.rs
- FOUND: .planning/phases/01-core-foundation/01-03-SUMMARY.md

All task commits verified:
- FOUND: e9850a1 (Task 1 — send command, bundled with 01-02 parallel execution)
- FOUND: fe8b238 (Task 2 — signal command + get_orchestrator)
