---
phase: 11
slug: antigravity-provider-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-09
---

# Phase 11 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test test_antigravity` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test test_antigravity`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 11-01-01 | 01 | 1 | AGNT-01 | unit | `cargo test test_antigravity_config` | ✅ | ⬜ pending |
| 11-01-02 | 01 | 1 | AGNT-01 | integration | `cargo test test_antigravity_parse` | ✅ | ⬜ pending |
| 11-02-01 | 02 | 1 | AGNT-02 | integration | `cargo test test_antigravity_signal` | ✅ | ⬜ pending |
| 11-02-02 | 02 | 1 | AGNT-03 | integration | `cargo test test_antigravity_init` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

- `tests/test_config.rs` — exists, add `test_antigravity_config` and `test_antigravity_parse` tests
- `tests/test_integration.rs` — exists, add `test_antigravity_signal` and `test_antigravity_init` tests

*No new test infrastructure needed — all test patterns already in use.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Log message clarity for db-only registration | AGNT-03 | UX quality judgement | Run `cargo run -- init --config squad.yml` with `tool: antigravity`, verify message clearly explains DB-only mode |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
