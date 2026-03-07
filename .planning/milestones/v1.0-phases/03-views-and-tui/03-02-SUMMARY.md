---
phase: 03-views-and-tui
plan: "02"
subsystem: tui-dashboard
tags: [tui, ratatui, crossterm, view, monitoring]
dependency_graph:
  requires: [03-01]
  provides: [ui-command, tui-dashboard, connect-per-refresh]
  affects: [src/commands/ui.rs, tests/test_views.rs]
tech_stack:
  added: []
  patterns: [connect-per-refresh, panic-hook-terminal-safety, tdd-state-logic]
key_files:
  created: []
  modified:
    - src/commands/ui.rs
    - tests/test_views.rs
decisions:
  - "frame.size() used instead of frame.area() — ratatui 0.26 API (area() added in 0.27+)"
  - "fetch_snapshot silently retains stale data on DB error — TUI continues running rather than crashing"
  - "connect-per-refresh strategy: read-only SqlitePool, max_connections(1), dropped immediately after fetch to release WAL reader lock"
  - "Panic hook installed before entering raw mode; original hook restored after normal exit"
metrics:
  duration: "~2 min"
  completed_date: "2026-03-06"
  tasks_completed: 2
  files_changed: 2
---

# Phase 3 Plan 02: Ratatui TUI Dashboard Summary

Ratatui TUI dashboard with two-panel layout, connect-per-refresh DB strategy, panic-safe terminal teardown, and 7 unit tests for app state logic.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Implement ratatui TUI dashboard (VIEW-03) | 2c98673 | src/commands/ui.rs |
| 2 | Unit tests for TUI app state logic | 174279a | tests/test_views.rs |

## What Was Built

**TUI dashboard (VIEW-03) — src/commands/ui.rs (319 lines):**

- `FocusPanel` enum: `AgentPanel` / `MessagePanel`
- `App` struct with `agents`, `messages`, `agent_list_state` (ratatui `ListState`), `focus`, `quit`
- Navigation: `select_next()` / `select_previous()` — both wrap around; no-op on empty list
- `toggle_focus()` — alternates between panels
- `handle_key()` — q/Esc quit, Down/j next, Up/k prev, Tab toggle, Home/End jump
- `fetch_snapshot()` — opens read-only `SqlitePool` (max 1 connection), fetches agents + messages for selected agent, drops pool before returning (WAL checkpoint starvation prevention)
- `setup_terminal()` / `restore_terminal()` — raw mode + alternate screen lifecycle
- Panic hook installed before entering raw mode; calls `disable_raw_mode()` + `LeaveAlternateScreen` on panic
- `draw_ui()` — 35/65 horizontal split; agent list with colored status indicators (green=idle, yellow=busy, red=dead/unknown); messages panel with Paragraph
- Event loop: 3-second polling interval, 250ms crossterm poll timeout; auto-selects first agent on first refresh; silently keeps stale data on DB error

**Unit tests — tests/test_views.rs (+7 tests):**

- `test_ui_app_new` — defaults: empty agents, quit=false, no selection, AgentPanel focus
- `test_ui_navigation_next` — 3 agents: cycles None->0->1->2->0
- `test_ui_navigation_prev` — 3 agents: from 0 wraps to 2, then 1, then 0
- `test_ui_quit_key_q` — handle_key('q') sets quit=true
- `test_ui_quit_key_esc` — handle_key(Esc) sets quit=true
- `test_ui_toggle_focus` — AgentPanel -> MessagePanel -> AgentPanel
- `test_ui_navigation_empty` — no panic with 0 agents; selection stays None

## Test Results

58 total tests, 0 failures, 0 ignored. New tests: 7.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `frame.area()` not available in ratatui 0.26**
- **Found during:** Task 1 (first build)
- **Issue:** Plan code used `frame.area()` which was added in ratatui 0.27+; project uses 0.26
- **Fix:** Changed to `frame.size()` — the equivalent method in 0.26
- **Files modified:** src/commands/ui.rs
- **Commit:** 2c98673

**2. [Rule 2 - Resilience] DB error handling in event loop**
- **Found during:** Task 1 (code review)
- **Issue:** Plan used `?` on `fetch_snapshot` which would crash the TUI on any transient DB error
- **Fix:** Changed to `match` — on error, TUI retains stale data and continues running
- **Files modified:** src/commands/ui.rs
- **Commit:** 2c98673

## Self-Check: PASSED

- src/commands/ui.rs exists (319 lines, > min_lines 150)
- tests/test_views.rs exists with 7 `test_ui_` tests
- Commit 2c98673 (feat: TUI dashboard) verified
- Commit 174279a (test: unit tests) verified
- `cargo build` clean
- `cargo test` — 58 tests, 0 failures
