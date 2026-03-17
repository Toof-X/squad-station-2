---
phase: 16-tui-wizard
plan: 01
subsystem: ui
tags: [ratatui, crossterm, tui, wizard, form, rust]

# Dependency graph
requires: []
provides:
  - "src/commands/wizard.rs: complete TUI wizard module with types, validation, rendering, and event loop"
  - "WizardResult and AgentInput public API types for squad.yml generation"
affects: [17-init-integration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "3-zone TUI layout: header (step progress), content (page-specific form), footer (key hints)"
    - "TDD red-green for validation functions and data types before TUI rendering"
    - "State machine with WizardPage enum driving per-page render and key dispatch"
    - "TextInputState pattern: push/pop chars with inline error slot"

key-files:
  created:
    - src/commands/wizard.rs
  modified:
    - src/commands/mod.rs

key-decisions:
  - "Tool enum cycles through ClaudeCode -> GeminiCli -> Antigravity (matches VALID_PROVIDERS order)"
  - "TextInputState.push() and pop() clear the error field automatically (no stale errors)"
  - "Separate render_text_field and render_agent_page helpers for clean layout separation"
  - "frame.size() used instead of frame.area() for ratatui 0.26.3 compatibility"
  - "AgentDraft structs pre-allocated on AgentCount confirm; never popped on Esc (indexed reuse)"

patterns-established:
  - "Wizard pages: state enum variant drives both render function and handle_key dispatch"
  - "TUI module copies terminal setup/teardown pattern from src/commands/ui.rs"
  - "Panic hook installs before terminal setup to guarantee restore_terminal on crash"

requirements-completed: [INIT-01, INIT-02, INIT-03, INIT-06, INIT-07]

# Metrics
duration: 3min
completed: 2026-03-17
---

# Phase 16 Plan 01: TUI Wizard Core Summary

**Multi-page ratatui wizard in src/commands/wizard.rs collecting project name, agent count, and per-agent role/tool/model/description with inline validation and full keyboard navigation**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-17T04:01:11Z
- **Completed:** 2026-03-17T04:04:34Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Complete wizard module with 832 lines covering all data types, state machine, validation, rendering, and event loop
- 11 unit tests covering TextInputState, Tool cycling/as_str, validate_count, validate_role, validate_project_name
- TUI renders 4 page types (ProjectName, AgentCount, AgentConfig x N, Summary) with Cyan/Red border styling
- Ctrl+C returns Ok(None) cancel; Enter on Summary returns Ok(Some(WizardResult)) with trimmed values

## Task Commits

Each task was committed atomically:

1. **Task 1: Data types, validation functions, and unit tests** - `41b5749` (feat)
2. **Task 2: TUI rendering, event loop, and state machine** - `04f4f7a` (feat)

**Plan metadata:** (docs commit follows)

_Note: Task 1 used TDD — wrote tests first, verified GREEN with implementation._

## Files Created/Modified
- `src/commands/wizard.rs` - Complete TUI wizard: WizardResult, AgentInput, Tool, TextInputState, AgentDraft, WizardPage, WizardState, validation functions, setup/restore_terminal, render_page, handle_key, run()
- `src/commands/mod.rs` - Added `pub mod wizard;` registration

## Decisions Made
- Reused terminal setup/teardown pattern from ui.rs (enable_raw_mode, EnterAlternateScreen, panic hook)
- Tool cycling order matches config.rs VALID_PROVIDERS (antigravity/claude-code/gemini-cli) — all three variants verified in test
- AgentDraft pre-allocated as a vec on AgentCount confirm — Esc navigates by index not pop, preventing data loss when going back
- render_text_field helper shared by ProjectName and AgentCount pages (DRY)
- frame.size() not frame.area() — confirmed from ui.rs pattern (ratatui 0.26.3)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Wizard module is complete and compiles cleanly
- Phase 17 (init-integration) can import `commands::wizard::run()` and wire it into the `init` subcommand
- WizardResult and AgentInput types are public and ready for squad.yml serialization
- No blockers

---
*Phase: 16-tui-wizard*
*Completed: 2026-03-17*

## Self-Check: PASSED
- src/commands/wizard.rs: FOUND
- .planning/phases/16-tui-wizard/16-01-SUMMARY.md: FOUND
- commit 41b5749 (task 1): FOUND
- commit 04f4f7a (task 2): FOUND
