---
phase: 03-views-and-tui
plan: "01"
subsystem: cli-commands
tags: [status, view, tmux, cli, tui-stub]
dependency_graph:
  requires: []
  provides: [status-command, view-command, ui-stub, tmux-view-helpers]
  affects: [src/cli.rs, src/main.rs, src/commands/mod.rs, src/tmux.rs]
tech_stack:
  added: []
  patterns: [reconciliation-loop-duplication, arg-builder-testability, file-db-integration-tests]
key_files:
  created:
    - src/commands/status.rs
    - src/commands/view.rs
    - src/commands/ui.rs
    - tests/test_views.rs
  modified:
    - src/cli.rs
    - src/main.rs
    - src/commands/mod.rs
    - src/tmux.rs
decisions:
  - "Reconciliation loop duplicated in status.rs (same pattern as agents.rs) — consistent with Phase 2 decision to avoid coupling independent command files"
  - "Status text output: project name + DB path header + agent counts + per-agent pending count line"
  - "View command filters live sessions using list_live_session_names — dead agents not in live sessions are skipped automatically"
  - "ui.rs is a stub with todo! — filled in by Plan 03-02"
  - "Integration tests use file-based SQLite (not in-memory) so binary subprocess reads same DB"
metrics:
  duration: "~7 min"
  completed_date: "2026-03-06"
  tasks_completed: 3
  files_changed: 8
---

# Phase 3 Plan 01: Views and CLI Wiring Summary

Wire all three Phase 3 commands (Status, Ui, View) into the CLI, fully implement `status` (VIEW-01) and `view` (VIEW-04), and stub `ui` for Plan 02. All tests green.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Wire CLI skeleton + test scaffolding | 0f78a1d | src/cli.rs, src/main.rs, src/commands/mod.rs, src/commands/{status,view,ui}.rs, tests/test_views.rs |
| 2 | Implement status command with tests (VIEW-01) | 2a55812 | src/commands/status.rs, tests/test_views.rs |
| 3 | Implement view command with tmux helpers and tests (VIEW-04) | 660690e | src/commands/view.rs, src/tmux.rs, tests/test_views.rs |

## What Was Built

**status command (VIEW-01):**
- Loads config + resolves DB path + connects
- Runs tmux reconciliation loop (dead/idle transitions)
- Counts pending messages per agent via `list_messages(..., Some("pending"), 9999)`
- Text output: project name, DB path, aggregate counts line (N idle, N busy, N dead), per-agent status with ANSI-safe duration + pending count
- JSON output: `StatusOutput { project, db_path, agents: Vec<AgentStatusSummary> }` with `pending_messages` field
- Empty squad: prints "No agents registered."

**view command (VIEW-04):**
- Lists live tmux sessions via new `list_live_session_names()`
- Filters DB agents to those with live sessions (dead agents skipped automatically)
- Kills existing `squad-view` window for idempotency
- Creates new window with tiled layout: first pane via `new-window`, additional panes via `split-window`, applies `select-layout tiled`
- Empty state: prints "No live agent sessions to display."

**ui stub:**
- `todo!("TUI dashboard — implemented in Plan 03-02")`

**tmux.rs additions:**
- `list_live_session_names() -> Vec<String>`
- `kill_window(window_name) -> Result<()>` (idempotent)
- `create_view_window(window_name, sessions) -> Result<()>`
- Private arg builders: `list_sessions_args`, `kill_window_args`, `new_window_args`, `split_window_args`, `select_layout_args`
- Unit tests for all 5 new arg builders

## Test Results

All tests green. New tests added:
- `test_status_text_output` — exit 0, contains project name + "idle" + "dead"
- `test_status_json_output` — valid JSON with project, db_path, agents with pending_messages
- `test_status_pending_count` — 3 pending messages inserted, status reports 3
- `test_status_empty_squad` — "No agents registered." output
- `test_view_no_live_sessions` — "No live agent sessions to display."
- `test_views_module_compiles` — smoke test
- 5 tmux arg builder unit tests in tmux.rs

Total test suite: 51 tests, 0 failures, 0 ignored.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Integration tests needed file-based SQLite, not in-memory pool**
- **Found during:** Task 2
- **Issue:** Binary subprocess reads from a file path in squad.yml; in-memory pools from `setup_test_db()` cannot be shared with subprocess
- **Fix:** Added `setup_file_db(path)` helper in test_views.rs that creates a real SQLite file and runs migrations
- **Files modified:** tests/test_views.rs
- **Commit:** 2a55812

**2. [Rule 1 - Bug] `Output` type does not implement `Default` — `unwrap_or_default()` failed**
- **Found during:** Task 3
- **Issue:** `Command::output().unwrap_or_default()` does not compile because `std::process::Output` has no `Default` impl
- **Fix:** Changed to `match ... { Ok(o) => o, Err(_) => return Vec::new() }`
- **Files modified:** src/tmux.rs
- **Commit:** 660690e

## Self-Check: PASSED

All created files exist. All three task commits verified.
