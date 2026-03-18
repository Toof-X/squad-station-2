---
phase: 21-quick-guide-and-install-flow
plan: "01"
subsystem: welcome-tui
tags: [tui, ratatui, navigation, guide-page, state-machine]
dependency_graph:
  requires: [20-01]
  provides: [WELCOME-05]
  affects: [src/commands/welcome.rs]
tech_stack:
  added: []
  patterns: [TDD red-green, pure-function extraction, state-machine enum dispatch]
key_files:
  created: []
  modified:
    - src/commands/welcome.rs
decisions:
  - WelcomePage enum added in same file as WelcomeAction — no new files needed
  - hint_bar_text() tests converted from exact-equality to contains() — accommodates future formatting changes without brittleness
  - guide_content() footer kept as plain text within Min(0) chunk — avoids 5th layout constraint
  - Left arrow on title page is no-op (non-wrapping) per CONTEXT.md resolution
  - Tab/Left from guide preserves remaining countdown (does not reset)
metrics:
  duration_seconds: 168
  completed_date: "2026-03-18"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 1
requirements:
  - WELCOME-05
---

# Phase 21 Plan 01: Quick Guide Page Summary

**One-liner:** Added a second TUI page (Quick Guide) reachable via Tab/Right from the welcome title, with WelcomePage enum state machine, guide_routing_action/guide_content/guide_hint_bar_text pure functions, draw_guide() layout, and 5s countdown reset on guide entry.

## What Was Built

Extended `src/commands/welcome.rs` to implement WELCOME-05: a second page in the welcome TUI state machine.

### New Types

- `WelcomePage` enum (`Title`, `Guide`) — tracks current TUI page in `run_welcome_tui()`
- `WelcomeAction::ShowGuide` variant — returned by `routing_action()` on Tab/Right
- `WelcomeAction::ShowTitle` variant — returned by `guide_routing_action()` on Tab/Left

### New Pure Functions

- `guide_routing_action(key: KeyCode) -> Option<WelcomeAction>` — Tab/Left returns `ShowTitle`; Q/Esc returns `Quit`; all others `None`
- `guide_hint_bar_text() -> String` — returns `"○ ●  Tab/←: Back  Q: Quit"` with Unicode dot indicator
- `guide_content() -> String` — concept summary line + 3 numbered steps + footer line

### Updated Functions

- `routing_action()` — added `KeyCode::Tab | KeyCode::Right => Some(WelcomeAction::ShowGuide)` arm before wildcard
- `hint_bar_text()` — updated both branches with `● ○` dot indicator prefix and `Tab: Guide` insertion

### New Rendering

- `draw_guide(frame: &mut Frame)` — 4-constraint vertical layout: `Length(1)` header, `Length(1)` blank, `Min(0)` content, `Length(1)` hint bar. No BigText, no borders, no Color::Red.

### Event Loop Updates

- `run_welcome_tui()` now has `let mut page = WelcomePage::Title` state variable
- `deadline` made mutable to support 5s reset when entering guide
- `terminal.draw()` dispatches to `draw_welcome()` or `draw_guide()` based on page
- Key handling dispatches to `routing_action()` or `guide_routing_action()` based on page
- `ShowGuide` arm resets deadline to `Instant::now() + Duration::from_secs(5)`
- `ShowTitle` arm returns to title preserving remaining countdown time

## Tests

24 welcome module tests pass (0 failures):
- 3 existing `hint_bar_text` tests updated from exact-equality to `contains()` assertions
- 11 new tests added: `test_routing_action_tab_opens_guide`, `test_routing_action_right_opens_guide`, `test_routing_action_left_noop`, `test_guide_routing_tab_returns_title`, `test_guide_routing_left_returns_title`, `test_guide_routing_quit`, `test_guide_routing_esc_quit`, `test_guide_routing_enter_noop`, `test_guide_hint_bar_text`, `test_guide_content`, `test_hint_bar_text_includes_tab_guide`
- Full suite: 241 tests, 0 failures, 0 regressions

## Verification

- `cargo check` — clean compilation, no errors, no warnings from new code
- `cargo test welcome` — 24 passed, 0 failed
- `cargo test` — full suite green

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Task | Description | Hash |
|------|-------------|------|
| 1 | Add WelcomePage enum, guide pure functions, extend routing (TDD) | a1e155d |
| 2 | Add draw_guide() and wire WelcomePage state into event loop | ea37a21 |

## Self-Check: PASSED

- src/commands/welcome.rs: FOUND
- 21-01-SUMMARY.md: FOUND
- Commit a1e155d: FOUND
- Commit ea37a21: FOUND
