---
phase: 4
slug: schema-and-config-refactor
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in + tokio-test (via `#[tokio::test]`) |
| **Config file** | `Cargo.toml` `[dev-dependencies]` section |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01-W0 | 01 | 0 | CONF-01..04 | unit | `cargo test test_config` | ❌ W0 | ⬜ pending |
| 04-01-01 | 01 | 1 | CONF-01 | unit | `cargo test test_config` | ✅ W0 | ⬜ pending |
| 04-01-02 | 01 | 1 | CONF-02 | unit | `cargo test test_config` | ✅ W0 | ⬜ pending |
| 04-01-03 | 01 | 1 | CONF-03 | unit | `cargo test test_config` | ✅ W0 | ⬜ pending |
| 04-01-04 | 01 | 1 | CONF-04 | unit | `cargo test test_config` | ✅ W0 | ⬜ pending |
| 04-02-W0 | 02 | 0 | MSGS-01..04 | unit | `cargo test test_db` | ✅ (update) | ⬜ pending |
| 04-02-01 | 02 | 1 | MSGS-01 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-02-02 | 02 | 1 | MSGS-02 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-02-03 | 02 | 1 | MSGS-03 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-02-04 | 02 | 1 | MSGS-04 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-03-W0 | 03 | 0 | AGNT-02 | unit | `cargo test test_db` | ❌ W0 | ⬜ pending |
| 04-03-01 | 03 | 1 | AGNT-01 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-03-02 | 03 | 1 | AGNT-02 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |
| 04-03-03 | 03 | 1 | AGNT-03 | unit | `cargo test test_db` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_config.rs` — stubs for CONF-01, CONF-02, CONF-03, CONF-04 (config parsing unit tests)
- [ ] Add `test_send_sets_current_task` to `tests/test_db.rs` — covers AGNT-02
- [ ] Update all 17+ existing tests in `tests/test_db.rs` asserting `status == "pending"` to `status == "processing"` — covers MSGS-03
- [ ] Update `insert_agent` call sites in `tests/test_db.rs` to new signature (drop `command`, add `model`/`description`, rename `provider` → `tool`)
- [ ] Update `insert_message` call sites in `tests/test_db.rs` to new signature (`from_agent`, `to_agent`, `type`, `body`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Migration runs on existing v1.0 DB without data loss | MSGS-01..04, AGNT-01..03 | Requires real DB file from prior version | Run `cargo test` against a copy of an existing v1.0 `station.db`; verify row counts match before/after |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
