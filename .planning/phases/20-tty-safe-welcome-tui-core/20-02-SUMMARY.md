---
phase: 20-tty-safe-welcome-tui-core
plan: 02
subsystem: ui
tags: [rust, ratatui, crossterm, tui, welcome-screen, routing, onboarding]

# Dependency graph
requires:
  - phase: 20-tty-safe-welcome-tui-core plan 01
    provides: WelcomeAction enum, run_welcome_tui(), routing_action() pure function, BigText pixel-font title, TTY guard
provides:
  - WelcomeAction routing wired in main.rs: Enter -> init wizard (no squad.yml) or dashboard (squad.yml exists)
  - Q/Esc/timeout -> silent exit to shell
  - routing_action() pure function with 5 unit tests
  - Complete first-run onboarding flow from bare `squad-station` invocation
affects: [21-post-install-hints, any phase touching main.rs None arm or welcome flow]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure routing_action() function extracted from TUI event loop for unit testability without a terminal"
    - "WelcomeAction enum match in main.rs None arm dispatches to init::run() or ui::run() based on squad.yml presence"
    - "Quit maps to None return (silent shell exit), LaunchInit/LaunchDashboard map to Some(action)"

key-files:
  created: []
  modified:
    - src/main.rs
    - src/commands/welcome.rs

key-decisions:
  - "routing_action() extracted as pure function: WelcomeAction routing logic lives in welcome.rs (testable without TUI), not inlined in main.rs"
  - "Quit action maps to None return value from run_welcome_tui(): caller (main.rs) does nothing, returns silently to shell"
  - "Enter with no squad.yml routes to init::run(PathBuf::from('squad.yml'), false) — hardcoded path, consistent with init subcommand default"

patterns-established:
  - "Pure function extraction for TUI action routing: complex match logic extracted to testable pure functions before wiring into main.rs"

requirements-completed: [INIT-01, INIT-02, INIT-03]

# Metrics
duration: 20min
completed: 2026-03-17
---

# Phase 20 Plan 02: TTY-Safe Welcome TUI Core — Routing Summary

**WelcomeAction routing wired in main.rs: Enter launches init wizard (no squad.yml) or ratatui dashboard (squad.yml exists), Q/Esc/timeout exit silently, with 5 unit tests on pure routing_action() function**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-03-17T14:00:00Z
- **Completed:** 2026-03-17T14:20:00Z
- **Tasks:** 2 (1 auto + 1 human-verify checkpoint)
- **Files modified:** 2

## Accomplishments

- routing_action() pure function added to welcome.rs: maps KeyCode + has_config bool to WelcomeAction variant, fully unit-testable without a terminal
- main.rs None arm wired: WelcomeAction::LaunchInit dispatches to commands::init::run(), WelcomeAction::LaunchDashboard dispatches to commands::ui::run(), all other cases exit silently
- 5 unit tests covering all routing branches pass (enter/no-config, enter/with-config, q/no-config, esc/with-config, ignored-key)
- Human-verified in real terminal: pixel-font title, countdown, Enter routing, Q exit, and piped non-TTY fallback all confirmed working

## Task Commits

Each task was committed atomically:

1. **RED: Failing routing_action tests** - `d17a527` (test)
2. **Task 1: Wire WelcomeAction routing in main.rs and add routing_action()** - `d0fe3bc` (feat)
3. **Task 2: Human verification** - checkpoint approved, no additional code commit required

## Files Created/Modified

- `src/commands/welcome.rs` - Added routing_action() pure function and 5 unit tests for all routing branches
- `src/main.rs` - Replaced `let _ = action` placeholder with WelcomeAction match dispatching to init::run() or ui::run()

## Decisions Made

- routing_action() extracted as pure function in welcome.rs (not inlined in main.rs) so routing logic is unit-testable without spawning a terminal
- Quit variant maps to None return from run_welcome_tui() — main.rs receives None and exits silently to shell, avoiding an explicit "Quit" case in the dispatch match
- init::run() called with hardcoded PathBuf::from("squad.yml") matching the init subcommand's default behavior

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all tasks completed cleanly on first attempt.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Complete first-run onboarding flow is ready: bare `squad-station` invocation opens welcome TUI, Enter routes to init wizard or dashboard based on squad.yml presence
- Phase 21 (post-install hints) can build on this foundation
- No blockers or concerns

---
*Phase: 20-tty-safe-welcome-tui-core*
*Completed: 2026-03-17*

## Self-Check: PASSED

- FOUND: src/commands/welcome.rs
- FOUND: src/main.rs
- FOUND: .planning/phases/20-tty-safe-welcome-tui-core/20-02-SUMMARY.md
- FOUND: commit d0fe3bc (feat: wire WelcomeAction routing)
- FOUND: commit d17a527 (test: failing routing_action tests RED)
