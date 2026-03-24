---
phase: 29-watchdog-core-correctness
plan: 01
subsystem: cli
tags: [clap, sqlite, watchdog, cli-flags]

# Dependency graph
requires: []
provides:
  - "Watch CLI variant with --dry-run, --status, --cooldown, --debounce flags"
  - "list_processing_messages() DB query returning (id, created_at) tuples"
  - "Updated main.rs dispatch wiring all 8 Watch parameters"
affects: [29-02, 29-03]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Suppress unused params with let _ = var for forward-declared interfaces"

key-files:
  created: []
  modified:
    - src/cli.rs
    - src/main.rs
    - src/commands/watch.rs
    - src/commands/init.rs
    - src/db/messages.rs

key-decisions:
  - "Cooldown default 600s (10min) and debounce default 3 cycles match plan specification"
  - "New params suppressed with let _ = to avoid unused warnings until Plan 02 implements logic"

patterns-established:
  - "Interface-first: define CLI flags and DB queries before implementing behavior"

requirements-completed: [OPS-02, OPS-03]

# Metrics
duration: 2min
completed: 2026-03-24
---

# Phase 29 Plan 01: Watch CLI Flags and DB Query Summary

**Watch CLI extended with --dry-run, --status, --cooldown, --debounce flags plus list_processing_messages() DB query for deadlock detection**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-24T06:59:54Z
- **Completed:** 2026-03-24T07:02:03Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Extended Watch CLI variant with 4 new flags (dry_run, status, cooldown, debounce) with proper defaults
- Added list_processing_messages() query returning processing message IDs and timestamps ordered by age
- Updated main.rs dispatch to wire all 8 parameters through to watch::run()
- Forward new flags in daemon spawn for background process consistency

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Watch CLI flags and update main.rs dispatch** - `df6a836` (feat)
2. **Task 2: Add list_processing_messages DB query** - `8602a05` (feat)

## Files Created/Modified
- `src/cli.rs` - Added dry_run, status, cooldown, debounce fields to Watch variant
- `src/main.rs` - Updated Watch destructure and dispatch to pass all 8 params
- `src/commands/watch.rs` - Extended run() signature, forward new flags in daemon spawn
- `src/commands/init.rs` - Updated two watch::run() call sites to match new signature
- `src/db/messages.rs` - Added list_processing_messages() function

## Decisions Made
- Cooldown defaults to 600s (10min) and debounce defaults to 3 cycles per plan specification
- New parameters suppressed with `let _ =` until Plan 02 implements the behavioral logic

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed watch::run() callers in init.rs**
- **Found during:** Task 1 (CLI flags and dispatch wiring)
- **Issue:** Two call sites in src/commands/init.rs called watch::run() with 4 args, now needs 8
- **Fix:** Updated both calls to pass default values for new params (false, false, 600, 3)
- **Files modified:** src/commands/init.rs
- **Verification:** cargo check passes
- **Committed in:** df6a836 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary fix for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CLI interface contracts established for Plan 02 (watchdog behavior implementation)
- DB query ready for deadlock detection in Plan 02/03
- All existing tests pass (13/13)

---
*Phase: 29-watchdog-core-correctness*
*Completed: 2026-03-24*

## Self-Check: PASSED
- All 5 modified files exist on disk
- Both task commits verified in git log (df6a836, 8602a05)
- dry_run field present in cli.rs, list_processing_messages present in messages.rs
