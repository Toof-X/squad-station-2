---
phase: 12
slug: ide-context-hook-setup
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-09
---

# Phase 12 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + tokio-test (via `#[tokio::test]`) |
| **Config file** | `Cargo.toml` (no separate config file) |
| **Quick run command** | `cargo test test_context` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test test_context`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 12-01-01 | 01 | 0 | AGNT-04 | integration | `cargo test test_context_generates_delegate_file` | ❌ W0 | ⬜ pending |
| 12-01-02 | 01 | 0 | AGNT-04 | integration | `cargo test test_context_delegate_content` | ❌ W0 | ⬜ pending |
| 12-01-03 | 01 | 0 | AGNT-05 | integration | `cargo test test_context_generates_monitor_file` | ❌ W0 | ⬜ pending |
| 12-01-04 | 01 | 0 | AGNT-05 | integration | `cargo test test_context_monitor_content` | ❌ W0 | ⬜ pending |
| 12-01-05 | 01 | 0 | AGNT-06 | integration | `cargo test test_context_generates_roster_file` | ❌ W0 | ⬜ pending |
| 12-01-06 | 01 | 0 | AGNT-06 | integration | `cargo test test_context_roster_content` | ❌ W0 | ⬜ pending |
| 12-02-01 | 02 | 0 | HOOK-03 | integration | `cargo test test_init_hook_merge_creates_backup` | ❌ W0 | ⬜ pending |
| 12-02-02 | 02 | 0 | HOOK-03 | integration | `cargo test test_init_hook_merge_adds_entry` | ❌ W0 | ⬜ pending |
| 12-02-03 | 02 | 0 | HOOK-03 | integration | `cargo test test_init_hook_merge_idempotent` | ❌ W0 | ⬜ pending |
| 12-02-04 | 02 | 0 | HOOK-04 | integration | `cargo test test_init_hook_instructions_no_settings` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_integration.rs` — new tests for AGNT-04, AGNT-05, AGNT-06 (file generation: delegate, monitor, roster)
- [ ] `tests/test_integration.rs` — new tests for HOOK-03 (backup creation, hook merge, idempotency), HOOK-04 (instructions on no settings.json)
- [ ] Update existing `test_context_lists_registered_agents` — change from stdout assertions to file content assertions

*Existing tokio + tempfile + cmd_with_db infrastructure covers all tests — no new framework needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Hook fires in live Claude Code session | HOOK-03 | Requires real tmux + Claude Code session | Run `squad-station init`, open a new Claude Code session in the project, verify `squad-station signal` is called when session stops |
| Hook fires in live Gemini CLI session | HOOK-03 | Requires Gemini CLI installed and running | Run `squad-station init`, run Gemini CLI in project, verify `AfterAgent` hook fires |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
