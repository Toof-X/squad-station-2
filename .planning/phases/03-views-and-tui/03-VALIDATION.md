---
phase: 3
slug: views-and-tui
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio-test 0.4 |
| **Config file** | No separate config — Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | VIEW-01 | integration | `cargo test test_status_` | ❌ W0 | ⬜ pending |
| 03-01-02 | 01 | 1 | VIEW-01 | integration | `cargo test test_status_json` | ❌ W0 | ⬜ pending |
| 03-01-03 | 01 | 1 | VIEW-01 | integration | `cargo test test_status_pending_count` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 1 | VIEW-04 | integration | `cargo test test_view_no_agents` | ❌ W0 | ⬜ pending |
| 03-02-02 | 02 | 1 | VIEW-04 | unit | `cargo test test_view_filters_dead` | ❌ W0 | ⬜ pending |
| 03-03-01 | 03 | 2 | VIEW-03 | unit | `cargo test test_ui_readonly_pool` | ❌ W0 | ⬜ pending |
| 03-03-02 | 03 | 2 | VIEW-03 | unit | `cargo test test_ui_quit_key` | ❌ W0 | ⬜ pending |
| 03-03-03 | 03 | 2 | VIEW-03 | unit | `cargo test test_ui_navigation` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_views.rs` — stubs for VIEW-01 status command tests (subprocess integration)
- [ ] `src/commands/status.rs` — VIEW-01 implementation module
- [ ] `src/commands/ui.rs` — VIEW-03 implementation module
- [ ] `src/commands/view.rs` — VIEW-04 implementation module
- [ ] `src/commands/mod.rs` — add `pub mod status; pub mod ui; pub mod view;`
- [ ] `cli.rs` — add `Status`, `Ui`, `View` variants to `Commands` enum
- [ ] `main.rs` — add dispatch arms for new commands

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TUI renders correctly with color | VIEW-03 | Requires live terminal with ratatui alternate screen | Run `squad-station ui` in a terminal with registered agents; verify two-panel layout renders, colors match agents command |
| TUI auto-refresh updates status | VIEW-03 | Requires live agents changing state during TUI session | Run `squad-station ui`, trigger a signal from another shell, verify status changes within refresh interval |
| `view` creates tmux pane grid | VIEW-04 | Requires live tmux server with agent sessions | Run `squad-station view` with 3+ live agents; verify tiled grid layout in tmux |
| `view` skips dead agents | VIEW-04 | Requires killing an agent session then running view | Kill one agent session, run `squad-station view`; verify dead agent has no pane |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
