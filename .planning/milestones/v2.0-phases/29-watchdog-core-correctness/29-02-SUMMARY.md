---
phase: 29-watchdog-core-correctness
plan: 02
subsystem: cli
tags: [watchdog, deadlock, debounce, dry-run, tmux-injection]

# Dependency graph
requires:
  - phase: 29-01
    provides: "Watch CLI flags (dry_run, status, cooldown, debounce) and list_processing_messages() DB query"
provides:
  - "DeadlockState struct with debounce, cooldown, and escalation logic"
  - "Pass 4: deadlock detection with message age filtering and 3-level escalating alerts"
  - "Pass 3: prolonged-busy agent injection into orchestrator tmux pane"
  - "Dry-run gating on all tmux injection (3 locations)"
affects: [29-03, 31]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Separate state structs for independent alert channels (NudgeState vs DeadlockState)"
    - "Debounce-then-escalate pattern: N ticks confirm, then 3-level nudge with cooldown"
    - "DRY-RUN log level prefix for non-destructive mode"

key-files:
  created: []
  modified:
    - src/commands/watch.rs

key-decisions:
  - "DeadlockState is separate from NudgeState to prevent idle nudges from suppressing deadlock alerts"
  - "Message age filtering uses stall_threshold_mins to exclude young processing messages from deadlock detection"
  - "Deadlock alert messages truncate to first 5 message IDs for readability with (+N more) suffix"

patterns-established:
  - "Dry-run gating: wrap send_keys_literal in if !dry_run, log with DRY-RUN level"
  - "Escalation tones: info (count=0), persist (count=1), critical (count>=2)"

requirements-completed: [DETECT-01, DETECT-02, DETECT-03, DETECT-04, ALERT-01, ALERT-02, OPS-03]

# Metrics
duration: 3min
completed: 2026-03-24
---

# Phase 29 Plan 02: Deadlock Detection and Dry-Run Summary

**Deadlock detection with 3-cycle debounce, message age filtering, escalating tmux injection, prolonged-busy orchestrator alerts, and dry-run gating across all passes**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-24T07:04:29Z
- **Completed:** 2026-03-24T07:07:42Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- DeadlockState struct with debounce counting, cooldown, max nudges, and full reset capability
- Pass 4 deadlock detection: processing messages + zero busy agents triggers after N debounce ticks, with 3-level escalating alerts
- Pass 3 prolonged-busy upgrade: agents busy >30min now inject warning into orchestrator tmux pane (not just log)
- Dry-run mode gates all tmux injection across 3 locations (idle nudge, deadlock, prolonged-busy)

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement DeadlockState struct with debounce and unit tests** - `5f2f797` (feat)
2. **Task 2: Implement deadlock detection pass, prolonged-busy injection, and dry-run gating** - `b2e40cd` (feat)

## Files Created/Modified
- `src/commands/watch.rs` - DeadlockState struct, Pass 4 deadlock detection, Pass 3 prolonged-busy injection, dry-run gating on all tmux sends

## Decisions Made
- DeadlockState kept separate from NudgeState per architectural decision in STATE.md (prevents idle nudges from suppressing deadlock alerts)
- Message age filtering uses same stall_threshold_mins as idle detection for consistency
- Deadlock alert message IDs truncated to 5 with (+N more) suffix for readability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All detection and alerting logic complete for Plan 03 (--status subcommand)
- Dry-run mode fully functional for integration testing in Phase 31
- 9 unit tests passing (5 NudgeState + 4 DeadlockState)

---
*Phase: 29-watchdog-core-correctness*
*Completed: 2026-03-24*

## Self-Check: PASSED
- src/commands/watch.rs exists on disk
- Commit 5f2f797 (Task 1) verified in git log
- Commit b2e40cd (Task 2) verified in git log
- DeadlockState struct present in watch.rs
- 3 occurrences of `if !dry_run` confirmed
- `[SQUAD WATCHDOG] Deadlock detected` present in watch.rs
- All 9 unit tests passing
