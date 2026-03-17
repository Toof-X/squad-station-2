---
phase: 20-tty-safe-welcome-tui-core
plan: 01
subsystem: welcome-tui
tags: [ratatui, crossterm, tui-big-text, welcome, tty, terminal]
dependency_graph:
  requires: []
  provides:
    - run_welcome_tui (src/commands/welcome.rs)
    - WelcomeAction enum (src/commands/welcome.rs)
    - hint_bar_text pure function (src/commands/welcome.rs)
    - commands_list pure function (src/commands/welcome.rs)
    - TTY guard in main.rs None arm
  affects:
    - src/main.rs (None arm routing)
    - src/commands/ui.rs (frame.area() migration)
    - src/commands/wizard.rs (frame.area() migration)
tech_stack:
  added:
    - ratatui 0.30
    - crossterm 0.29
    - tui-big-text 0.8
  patterns:
    - AlternateScreen + raw mode setup/teardown (mirrors ui.rs pattern)
    - Panic hook for terminal restore
    - TTY guard via std::io::IsTerminal
    - Countdown event loop with saturating_duration_since
key_files:
  created: []
  modified:
    - Cargo.toml
    - src/commands/welcome.rs
    - src/commands/wizard.rs
    - src/commands/ui.rs
    - src/main.rs
decisions:
  - "AlternateScreen chosen over main-buffer for welcome TUI (consistent with existing ui.rs pattern, preserves scrollback)"
  - "WelcomeAction routing deferred to Plan 20-02 (main.rs uses let _ = action to suppress warnings)"
  - "hint_bar_text() and commands_list() extracted as pure functions for unit testability without a terminal"
metrics:
  duration: "3 minutes"
  completed: "2026-03-17T13:31:41Z"
  tasks_completed: 2
  files_changed: 5
---

# Phase 20 Plan 01: TTY-Safe Welcome TUI Core Summary

**One-liner:** ratatui 0.30 + tui-big-text 0.8 upgrade with AlternateScreen welcome TUI featuring BigText pixel-font title, 5-second countdown, hint bar, and TTY guard in main.rs.

## What Was Built

**Task 1 — Dependency upgrade + frame.area() migration:**
- Upgraded `ratatui` from 0.26 to 0.30
- Upgraded `crossterm` from 0.27 to 0.29
- Added `tui-big-text = "0.8"` for pixel-font BigText widget
- Replaced deprecated `frame.size()` with `frame.area()` in `src/commands/ui.rs` (line 176) and `src/commands/wizard.rs` (line 755)
- All 164 pre-existing tests continued to pass

**Task 2 — Welcome TUI implementation:**
- Added `WelcomeAction` enum (`LaunchInit`, `LaunchDashboard`, `Quit`) for routing by Plan 20-02
- Implemented `run_welcome_tui(has_config: bool)` with full AlternateScreen TUI:
  - BigText pixel-font "SQUAD-STATION" title (HalfHeight, red)
  - Version string from `env!("CARGO_PKG_VERSION")`
  - Tagline: "Multi-agent orchestration for AI coding"
  - Commands table (all 11 subcommands)
  - Hint bar with countdown (`hint_bar_text()` pure function)
  - 5-second auto-exit (timeout = silent exit)
  - Enter key: routes to LaunchDashboard (has_config) or LaunchInit (no config)
  - Q/Esc: sets Quit action
- Added `setup_terminal()`, `restore_terminal()`, panic hook (exact pattern from ui.rs)
- Updated `src/main.rs` None arm with TTY guard (`std::io::IsTerminal`): TTY -> TUI, non-TTY -> `print_welcome()`
- Routing (`match action`) deferred to Plan 20-02 (`let _ = action` suppresses warnings)
- Added 4 new unit tests (hint_bar_text: 3 cases, commands_list subcommand coverage)
- All 168 tests pass (94 unit + 74 integration)

## Commits

| Hash    | Message                                                                  |
| ------- | ------------------------------------------------------------------------ |
| 9ce2f3b | chore(20-01): upgrade ratatui 0.30 + crossterm 0.29 + tui-big-text 0.8  |
| 3fa19c3 | feat(20-01): implement welcome TUI with BigText title, countdown, TTY guard |

## Deviations from Plan

None - plan executed exactly as written.

## Self-Check: PASSED

Files verified:
- FOUND: src/commands/welcome.rs (contains run_welcome_tui, WelcomeAction, hint_bar_text, BigText::builder(), PixelSize::HalfHeight)
- FOUND: src/main.rs (contains is_terminal(), run_welcome_tui, print_welcome())
- FOUND: Cargo.toml (contains ratatui = "0.30", crossterm = "0.29", tui-big-text = "0.8")
- FOUND: 9ce2f3b in git log
- FOUND: 3fa19c3 in git log
