---
phase: 2
slug: lifecycle-and-hooks
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-03-06
audited: 2026-03-06
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust test (`cargo test`) + tokio-test 0.4 |
| **Config file** | Cargo.toml `[dev-dependencies]` — no separate config |
| **Quick run command** | `cargo test --test test_lifecycle` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test test_db`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | SESS-03 | unit | `cargo test test_update_agent_status` | ✅ | ✅ green |
| 02-01-02 | 01 | 1 | SESS-03 | smoke | `cargo test test_agents_command_shows_status_with_duration` | ✅ | ✅ green |
| 02-01-03 | 01 | 1 | SESS-04 | unit | `cargo test test_update_agent_status_dead_to_idle` | ✅ | ✅ green |
| 02-01-04 | 01 | 1 | SESS-04 | unit | `cargo test test_list_agents_includes_status` | ✅ | ✅ green |
| 02-02-01 | 02 | 1 | SESS-05 | smoke | `cargo test test_context_output_contains_agents` | ✅ | ✅ green |
| 02-02-02 | 02 | 1 | SESS-05 | smoke | `cargo test test_context_output_has_usage` | ✅ | ✅ green |
| 02-03-01 | 03 | 2 | HOOK-01 | unit | `cargo test test_orchestrator_has_orchestrator_role` | ✅ | ✅ green |
| 02-03-02 | 03 | 2 | HOOK-03 | smoke | `cargo test test_signal_no_tmux_pane_exits_zero` | ✅ | ✅ green |
| 02-03-03 | 03 | 2 | HOOK-03 | unit | `cargo test test_get_orchestrator_returns_none_when_no_orchestrator` | ✅ | ✅ green |
| 02-03-04 | 03 | 2 | HOOK-03 | smoke | `cargo test test_signal_guard_db_error_exits_zero_with_warning` | ✅ | ✅ green |
| 02-04-01 | 04 | 2 | HOOK-02 | shell/manual | Manual — shell script testing | N/A | ⬜ pending |
| 02-04-02 | 04 | 2 | HOOK-02 | shell/manual | Manual — shell script testing | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/test_db.rs` — `test_update_agent_status`, `test_agent_default_status_is_idle`, `test_update_agent_status_updates_timestamp` added
- [x] `tests/test_lifecycle.rs` — reconcile tests, signal guard tests (Guard 1 + Guard 2), context output tests, agents duration test added
- [ ] HOOK-02 shell scripts: manual verification only (tmux dependency — cannot automate)

*Existing infrastructure (`setup_test_db()`, `#[tokio::test]`, `tempfile`) covers all framework needs.*

---

## Nyquist Audit Notes (2026-03-06)

Four gaps filled by Nyquist auditor:

| Gap | Test Added | Approach |
|-----|------------|----------|
| SESS-05 context roster heading | `test_context_output_contains_agents` | subprocess + temp dir + squad.yml with custom db_path |
| SESS-05 context usage section | `test_context_output_has_usage` | subprocess + temp dir + squad.yml with custom db_path |
| SESS-03 format_status_with_duration (private fn) | `test_agents_command_shows_status_with_duration` | subprocess agents command — private fn tested via observable output |
| HOOK-03 Guard 2 (config/DB error → stderr + exit 0) | `test_signal_guard_db_error_exits_zero_with_warning` | subprocess with TMUX_PANE set, no squad.yml in CWD |

Full suite result after audit: **40 tests, 0 failures**.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| claude-code.sh exits 0 in all cases | HOOK-02 | Shell script with tmux dependency | Run script outside tmux, verify exit 0; run inside tmux with unregistered agent, verify exit 0 |
| gemini-cli.sh exits 0 in all cases | HOOK-02 | Shell script with tmux dependency | Same as above for gemini script |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** 2026-03-06 — Nyquist auditor — 40/40 tests green (10 automated, 2 manual-only HOOK-02)
