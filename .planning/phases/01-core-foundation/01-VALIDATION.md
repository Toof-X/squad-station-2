---
phase: 1
slug: core-foundation
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-06
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (tokio-test 0.4 in dev-dependencies) |
| **Config file** | Cargo.toml (dev-dependencies section) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test -- --include-ignored` |
| **Estimated runtime** | ~1 second |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test -- --include-ignored`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 1 second

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 0 | SESS-01 | integration | `cargo test test_config_parse_valid_yaml` | ✅ | ✅ green |
| 01-01-02 | 01 | 0 | SESS-01 | integration | `cargo test test_db_path_resolution_default` | ✅ | ✅ green |
| 01-01-03 | 01 | 0 | SESS-02 | unit | `cargo test test_insert_and_get_agent` | ✅ | ✅ green |
| 01-01-04 | 01 | 0 | MSG-01 | unit | `cargo test test_insert_message` | ✅ | ✅ green |
| 01-01-05 | 01 | 0 | MSG-02 | unit | `cargo test test_update_status_completes_message` | ✅ | ✅ green |
| 01-01-06 | 01 | 0 | MSG-03 | unit | `cargo test test_update_status_idempotent` | ✅ | ✅ green |
| 01-01-07 | 01 | 0 | MSG-04 | unit | `cargo test test_list_filter_by_agent` | ✅ | ✅ green |
| 01-01-08 | 01 | 0 | MSG-05 | unit | `cargo test test_peek_priority_ordering` | ✅ | ✅ green |
| 01-01-09 | 01 | 0 | MSG-06 | unit | `cargo test test_peek_returns_pending` | ✅ | ✅ green |
| 01-01-10 | 01 | 0 | SAFE-01 | integration | `cargo test test_insert_message` (WAL via setup_test_db) | ✅ | ✅ green |
| 01-01-11 | 01 | 0 | SAFE-02 | unit | `cargo test test_send_keys_args_have_literal_flag` | ✅ | ✅ green |
| 01-01-12 | 01 | 0 | SAFE-03 | unit | `cargo test test_launch_args_use_direct_command` | ✅ | ✅ green |
| 01-01-13 | 01 | 0 | SAFE-04 | unit | `cargo test test_sigpipe_binary_starts` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/` directory — Rust integration test directory
- [x] `tests/helpers.rs` — shared setup_test_db() (temp-file SQLite pool with WAL)
- [x] `tests/test_db.rs` — 20 tests covering MSG-01 through MSG-06, SESS-01, SESS-02, SAFE-01
- [x] `tests/test_commands.rs` — 7 tests covering config parsing, DB path resolution, SAFE-04
- [x] `tests/test_lifecycle.rs` — 5 tests covering lifecycle and guard integration
- [x] `src/tmux.rs` test module — 4 unit tests for SAFE-02, SAFE-03
- [x] Framework install: none needed — tokio-test already in dev-dependencies

*Existing infrastructure covers all phase requirements.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| tmux session actually launches agent | SESS-01 | Requires real tmux | Run `squad-station init` with test squad.yml, verify `tmux ls` shows sessions |
| send-keys delivers prompt to tmux pane | MSG-01 | Requires real tmux | Run `squad-station send <agent> "test"`, verify text appears in pane |
| Signal notification appears in orchestrator pane | MSG-02 | Requires real tmux | Run `squad-station signal <agent>`, verify notification in orchestrator pane |

*All core logic paths have automated verification. Manual tests only for tmux integration.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 1s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-03-06

---

## Validation Audit 2026-03-06

| Metric | Count |
|--------|-------|
| Requirements audited | 12 |
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| Total tests | 36 |
| Tests passing | 36 |
