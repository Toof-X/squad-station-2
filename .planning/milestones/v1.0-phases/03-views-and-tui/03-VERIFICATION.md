---
phase: 03-views-and-tui
verified: 2026-03-06T00:00:00Z
status: human_needed
score: 12/12 must-haves verified
re_verification: false
human_verification:
  - test: "Run `squad-station ui` in a terminal with a squad.yml and registered agents"
    expected: "Two-panel ratatui dashboard renders with agent list (left, colored status) and messages (right); q/Esc exits and terminal is fully restored"
    why_human: "TUI rendering in alternate screen cannot be verified programmatically; terminal restoration requires live terminal"
  - test: "Leave TUI running for 6+ seconds and watch status column"
    expected: "Agent status updates every 3 seconds without freezing or crashing"
    why_human: "Polling interval and live DB refresh require a running process to observe"
  - test: "Run `squad-station view` with 3+ live tmux agent sessions active"
    expected: "`squad-view` window created with tiled grid layout; each pane shows the correct agent's terminal"
    why_human: "Requires a live tmux server with real sessions; cannot mock tmux attach behavior"
  - test: "Run `squad-station view` twice in succession"
    expected: "Second invocation kills and recreates the window without duplicates; only one `squad-view` window exists"
    why_human: "Requires live tmux server to observe idempotency"
  - test: "Kill one agent's tmux session and run `squad-station view`"
    expected: "Dead agent's session is absent from the tiled layout"
    why_human: "Requires live tmux server and a killable agent session"
---

# Phase 3: Views and TUI Verification Report

**Phase Goal:** Users can monitor the entire agent fleet at a glance via text commands, an interactive terminal dashboard, and a split tmux pane layout — without needing to query agents individually
**Verified:** 2026-03-06
**Status:** human_needed — all automated checks pass; 5 items require live terminal / tmux
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can run `squad-station status` and see project name, agent summary (counts by status), DB path, and per-agent lines with pending message counts | VERIFIED | `src/commands/status.rs` lines 21-97: loads config, reconciles tmux, counts pending via `list_messages(..., Some("pending"), 9999)`, prints `Project:`, `DB:`, `Agents: N -- N idle, N busy, N dead`, per-agent `name: status | N pending`; test `test_status_text_output` passes |
| 2 | User can run `squad-station status --json` and get valid JSON with project, db_path, and agents array including pending_messages | VERIFIED | `StatusOutput { project, db_path, agents: Vec<AgentStatusSummary> }` at lines 5-19 with `serde::Serialize`; JSON branch at lines 63-71; test `test_status_json_output` passes |
| 3 | User can run `squad-station view` and get a tmux window named squad-view with tiled panes showing each live agent session | VERIFIED (automated portion) | `src/commands/view.rs` calls `tmux::list_live_session_names()`, filters live agents, calls `tmux::kill_window("squad-view")` then `tmux::create_view_window("squad-view", ...)` (lines 13-43); tmux.rs `create_view_window` at line 124; live tmux portion is human-only |
| 4 | Running `squad-station view` twice does not create duplicate windows (idempotent) | VERIFIED (code path) | `view.rs` line 34: `tmux::kill_window("squad-view")?` called unconditionally before every create; human test needed to confirm in live tmux |
| 5 | VIEW-02 (`agents` command) remains working as-is from Phase 2 | VERIFIED | `src/main.rs` line 33: `Agents => commands::agents::run(cli.json).await` — dispatch arm unchanged; Phase 2 tests remain green (58 total, 0 failures per SUMMARY 03-02) |
| 6 | User can run `squad-station ui` and see a live ratatui dashboard with two panels: agent list (left) and messages for selected agent (right) | VERIFIED (code) / HUMAN (render) | `src/commands/ui.rs` 319 lines; `draw_ui()` at line 170 implements 35/65 horizontal split with `List` widget (left) and `Paragraph` widget (right); visual output requires human verification |
| 7 | TUI auto-refreshes agent status and messages on a 3-second polling interval without holding a persistent DB connection | VERIFIED (code) | Event loop lines 279-313: `if last_refresh.elapsed() >= refresh_interval` triggers `fetch_snapshot`; `fetch_snapshot` (lines 117-139) opens read-only pool, drops it explicitly after fetch; live behavior requires human verification |
| 8 | User can navigate agents with up/down/j/k keys, switch panels with Tab, and quit with q or Esc | VERIFIED | `handle_key()` lines 86-104 covers all bindings; 7 unit tests pass: `test_ui_quit_key_q`, `test_ui_quit_key_esc`, `test_ui_navigation_next`, `test_ui_navigation_prev`, `test_ui_toggle_focus` |
| 9 | TUI shows colored status indicators (green/yellow/red) consistent with the agents command | VERIFIED (code) | `status_color()` at lines 162-168: idle=Green, busy=Yellow, dead/unknown=Red using `ratatui::style::Color`; consistent with agents.rs owo-colors mapping |
| 10 | Terminal is always restored to normal state on exit, including on errors and panics | VERIFIED (code) / HUMAN (live) | `restore_terminal()` called at line 315 on normal exit; panic hook installed at lines 265-270 calls `disable_raw_mode()` + `LeaveAlternateScreen`; live verification requires human |
| 11 | `squad-station status`, `squad-station ui`, `squad-station view` all appear as CLI subcommands | VERIFIED | `cli.rs` lines 75-79: `Status`, `Ui`, `View` variants in `Commands` enum with doc comments; `main.rs` lines 35-37: all three dispatch arms wired |
| 12 | `squad-station status` with no agents prints "No agents registered." | VERIFIED | `status.rs` lines 30-33: early return with `"No agents registered."` message; test `test_status_empty_squad` passes |

**Score:** 12/12 truths verified (5 require additional human confirmation for live runtime behavior)

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/status.rs` | Status command with reconciliation, pending counts, text + JSON output (min 60 lines) | VERIFIED | 129 lines; full implementation with reconciliation loop, `AgentStatusSummary`, JSON + text branches |
| `src/commands/view.rs` | Tmux pane layout command with tiled grid (min 40 lines) | VERIFIED | 46 lines; loads config, fetches live sessions, kill+create window, empty-state handling |
| `src/commands/ui.rs` | Ratatui TUI dashboard with event loop, connect-per-refresh DB, two-panel layout (min 150 lines) | VERIFIED | 319 lines; full App struct, navigation, fetch_snapshot, draw_ui, event loop |
| `src/cli.rs` | Status, Ui, View subcommand variants added; contains "Status" | VERIFIED | Lines 75-79: all three variants present with doc strings |
| `src/commands/mod.rs` | `pub mod status; pub mod ui; pub mod view;` | VERIFIED | Lines 9-11 confirmed |
| `src/main.rs` | Dispatch arms for Status, Ui, View | VERIFIED | Lines 35-37 confirmed |
| `src/tmux.rs` | `list_live_session_names`, `kill_window`, `create_view_window` | VERIFIED | Lines 104, 118, 124 confirmed |
| `tests/test_views.rs` | Integration tests for status and view; unit tests for TUI logic; contains "test_ui_" (min 40 lines) | VERIFIED | 13 named test functions found: 4 status, 1 view, 7 ui state, 1 smoke test; no `#[ignore]` markers present |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/commands/status.rs` | dispatch arm for Status variant | WIRED | Line 35: `Status => commands::status::run(cli.json).await` |
| `src/main.rs` | `src/commands/view.rs` | dispatch arm for View variant | WIRED | Line 37: `View => commands::view::run(cli.json).await` |
| `src/main.rs` | `src/commands/ui.rs` | dispatch arm for Ui variant | WIRED | Line 36: `Ui => commands::ui::run().await` |
| `src/commands/status.rs` | `src/db/agents.rs` | `db::agents::list_agents` for reconciliation | WIRED | Lines 28, 46: `db::agents::list_agents(&pool)` called twice (before + after reconciliation); `update_agent_status` at lines 39, 41 |
| `src/commands/status.rs` | `src/db/messages.rs` | `db::messages::list_messages` for pending count | WIRED | Line 51: `db::messages::list_messages(&pool, Some(&agent.name), Some("pending"), 9999)` |
| `src/commands/view.rs` | `src/tmux.rs` | `tmux::list_live_session_names` for session enumeration | WIRED | Line 13: `let live_sessions = tmux::list_live_session_names();` |
| `src/commands/ui.rs` | `src/db/agents.rs` | `fetch_snapshot` opens read-only pool, calls `list_agents` | WIRED | Line 130: `let agents = db::agents::list_agents(&pool).await?;` inside `fetch_snapshot` |
| `src/commands/ui.rs` | `src/db/messages.rs` | `fetch_snapshot` calls `list_messages` for selected agent | WIRED | Lines 131-135: conditional `list_messages` call in `fetch_snapshot` |
| `src/commands/ui.rs` | `crossterm` | `enable_raw_mode`, `EnterAlternateScreen`, event poll/read | WIRED | Lines 1-5 imports; `enable_raw_mode()` at line 146; `event::poll` at line 302 |
| `src/commands/ui.rs` | `ratatui` | `Terminal`, `Frame`, `Layout`, `List`, `ListState`, `Paragraph` | WIRED | Lines 6-13 imports; `Terminal::new` at line 148; all widgets used in `draw_ui` |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| VIEW-01 | 03-01-PLAN.md | User can see squad overview via `squad-station status` (text output) | SATISFIED | `status.rs` full implementation; 4 integration tests pass (text, JSON, pending count, empty squad) |
| VIEW-02 | 03-01-PLAN.md | User can list agents and their status via `squad-station agents` | SATISFIED | `agents` command dispatch unchanged in `main.rs` line 33; Phase 2 tests remain green |
| VIEW-03 | 03-02-PLAN.md | User can view interactive TUI dashboard via `squad-station ui` (ratatui) | SATISFIED (code) / HUMAN (live render) | `ui.rs` 319 lines with full implementation; 7 app state unit tests pass; visual rendering requires human verification |
| VIEW-04 | 03-01-PLAN.md | User can view split tmux pane layout of all agents via `squad-station view` | SATISFIED (code) / HUMAN (live tmux) | `view.rs` + `tmux.rs` new functions; `test_view_no_live_sessions` passes; actual pane creation requires live tmux |

All 4 requirements declared in plan frontmatter are accounted for. No orphaned requirements detected (REQUIREMENTS.md shows VIEW-01 through VIEW-04 mapped to Phase 3 — all present in plans).

---

## Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| None | — | — | No TODO, FIXME, placeholder comments, empty returns, or stub implementations found in any phase 3 source files |

Checked: `status.rs`, `view.rs`, `ui.rs`, `cli.rs`, `main.rs`, `tmux.rs`. No `todo!()` macros remain. No `return null` / empty stub patterns.

---

## Human Verification Required

### 1. TUI Two-Panel Dashboard Renders

**Test:** Run `squad-station ui` in a terminal with a valid `squad.yml` and at least one registered agent
**Expected:** Alternate screen opens showing two panels — left panel lists agents with colored status indicators (green=idle, yellow=busy, red=dead), right panel shows messages for the selected agent; panels have borders and titles
**Why human:** Ratatui alternate screen rendering cannot be verified by grep or build checks

### 2. TUI Auto-Refresh and Terminal Restore

**Test:** Let TUI run for 6+ seconds; press q to exit
**Expected:** Agent status updates silently every ~3 seconds; after q, the terminal is fully restored (cursor visible, shell prompt returns, no raw mode artifacts)
**Why human:** Requires live process observation; terminal restoration only verifiable by inspecting the actual terminal state

### 3. Tmux Tiled View Window Created

**Test:** With 3+ active tmux agent sessions, run `squad-station view`
**Expected:** A `squad-view` tmux window appears with a tiled grid — one pane per live agent session, each showing `tmux attach-session -t {agent_name}` output
**Why human:** Requires a live tmux server with real agent sessions attached

### 4. View Command Idempotency

**Test:** Run `squad-station view` twice in succession
**Expected:** Only one `squad-view` window exists after the second run; no duplicate windows, no error
**Why human:** Requires `tmux list-windows` check on a live tmux server

### 5. View Skips Dead Agents

**Test:** Kill one agent's tmux session (`tmux kill-session -t {agent}`), then run `squad-station view`
**Expected:** The killed agent has no pane in `squad-view`; remaining live agents have panes
**Why human:** Requires controllable live tmux sessions

---

## Gaps Summary

No automated gaps detected. All source files are substantive (no stubs, no `todo!()` macros), all key links are wired, all 4 requirements are implemented and tested. The 5 human verification items are runtime/visual behaviors that are inherently untestable via static analysis — they do not represent defects but are standard manual checks for TUI and tmux-dependent features.

---

_Verified: 2026-03-06_
_Verifier: Claude (gsd-verifier)_
