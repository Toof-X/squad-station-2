---
phase: 5
slug: feature-completion
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust test (cargo test) + async tokio |
| **Config file** | Cargo.toml (test configuration inline) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && ./tests/e2e_cli.sh` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && ./tests/e2e_cli.sh`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 5-01-01 | 01 | 1 | HOOK-01 | smoke (shell) | `bash -n hooks/claude-code-notify.sh` | ❌ Wave 0 | ⬜ pending |
| 5-01-02 | 01 | 1 | HOOK-02 | smoke (shell) | `bash -n hooks/gemini-cli-notify.sh` | ❌ Wave 0 | ⬜ pending |
| 5-02-01 | 02 | 2 | CLI-01 | unit (clap parse) | `cargo test --test test_cli test_cli_send_body_flag` | ❌ Wave 0 | ⬜ pending |
| 5-02-02 | 02 | 2 | CLI-02 | integration | `cargo test --test test_commands test_init_agent_name_prefix` | ❌ Wave 0 | ⬜ pending |
| 5-02-03 | 02 | 2 | CLI-03 | integration | `cargo test --test test_commands test_context_includes_model` | ❌ Wave 0 | ⬜ pending |
| 5-02-04 | 02 | 2 | SIG-01 | unit | `cargo test --test test_commands test_signal_notification_format` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_cli.rs` — add `test_cli_send_body_flag_accepted` and `test_cli_send_positional_rejected`; update `test_cli_send_priority_flag_accepts_valid_values` to use `--body`
- [ ] `tests/test_commands.rs` — `test_init_agent_name_prefix` covering CLI-02
- [ ] `tests/test_commands.rs` — `test_context_includes_model_and_description` covering CLI-03
- [ ] `tests/test_commands.rs` — `test_signal_notification_format` covering SIG-01
- [ ] Shell syntax check stubs for new hook scripts: `bash -n hooks/claude-code-notify.sh && bash -n hooks/gemini-cli-notify.sh`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Hook fires on actual permission prompt in Claude Code | HOOK-01 | Requires live Claude Code session with hooks configured | Register hook in `.claude/settings.json`, trigger a tool permission prompt, verify orchestrator receives notification |
| Hook fires on actual ToolPermission in Gemini CLI | HOOK-02 | Requires live Gemini CLI session with hooks configured | Register hook in `.gemini/settings.json`, trigger a tool permission prompt, verify orchestrator receives notification |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
