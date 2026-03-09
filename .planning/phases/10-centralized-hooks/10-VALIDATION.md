---
phase: 10
slug: centralized-hooks
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-09
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio-test (async) |
| **Config file** | Cargo.toml `[dev-dependencies]` — no separate config file |
| **Quick run command** | `cargo test signal` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test signal`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 10-01-01 | 01 | 1 | HOOK-01 | unit | `cargo test test_session_name_from_pane_args` | ❌ W0 | ⬜ pending |
| 10-01-02 | 01 | 1 | HOOK-01 | integration | `cargo test test_signal_no_args_no_tmux` | ❌ W0 | ⬜ pending |
| 10-01-03 | 01 | 1 | HOOK-01 | integration | `cargo test test_signal_via_tmux_pane` | ❌ W0 | ⬜ pending |
| 10-01-04 | 01 | 1 | HOOK-01 | integration | `cargo test test_signal_pane_id_as_arg` | ❌ W0 | ⬜ pending |
| 10-01-05 | 01 | 1 | HOOK-01 | integration | `cargo test test_signal_explicit_agent_still_works` | ✅ existing | ⬜ pending |
| 10-02-01 | 02 | 2 | HOOK-02 | manual | `grep -q DEPRECATED hooks/claude-code.sh` | ❌ W0 (file edit) | ⬜ pending |
| 10-02-02 | 02 | 2 | HOOK-02 | manual | `grep -q DEPRECATED hooks/gemini-cli.sh` | ❌ W0 (file edit) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_integration.rs` — add stubs for `test_signal_via_tmux_pane`, `test_signal_pane_id_as_arg`, `test_signal_no_args_no_tmux` (skip gracefully when tmux not running, per existing `test_send_no_tmux_session` pattern)
- [ ] `src/tmux.rs` — add `test_session_name_from_pane_args` unit test (arg array verification, no actual tmux needed)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `hooks/claude-code.sh` has DEPRECATED header | HOOK-02 | File header is a comment change with no runtime behavior | `grep -q DEPRECATED hooks/claude-code.sh && echo OK` |
| `hooks/gemini-cli.sh` has DEPRECATED header | HOOK-02 | File header is a comment change with no runtime behavior | `grep -q DEPRECATED hooks/gemini-cli.sh && echo OK` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
