---
phase: 24
slug: agent-role-templates-in-wizard
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-19
---

# Phase 24 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test test_templates` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test test_templates`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 24-01-01 | 01 | 0 | TMPL-01 | unit | `cargo test test_templates` | ❌ W0 | ⬜ pending |
| 24-01-02 | 01 | 0 | TMPL-04 | unit | `cargo test test_templates` | ❌ W0 | ⬜ pending |
| 24-02-01 | 02 | 1 | TMPL-01 | integration | `cargo test test_templates` | ❌ W0 | ⬜ pending |
| 24-02-02 | 02 | 1 | TMPL-02 | integration | `cargo test test_templates` | ❌ W0 | ⬜ pending |
| 24-02-03 | 02 | 1 | TMPL-03 | integration | `cargo test test_templates` | ❌ W0 | ⬜ pending |
| 24-02-04 | 02 | 1 | TMPL-05 | integration | `cargo test test_context` | ❌ W0 | ⬜ pending |
| 24-02-05 | 02 | 1 | TMPL-06 | integration | `cargo test test_clone` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_templates.rs` — stubs for TMPL-01 through TMPL-06
- [ ] Template data unit tests (catalog completeness, field validity)
- [ ] Routing Matrix output tests in context generation

*Existing test infrastructure (cargo test, helpers.rs, setup_test_db) covers framework needs.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Template selector split-pane layout renders correctly | TMPL-01 | TUI visual rendering requires terminal | Run wizard, verify radio list on left + description preview on right |
| Auto-fill visually updates all fields | TMPL-02 | Visual confirmation of field population | Select template, verify Name/Provider/Model/Description fields update |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
