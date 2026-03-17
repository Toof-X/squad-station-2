---
phase: 16
slug: tui-wizard
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 16 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `cargo test` (tokio async runtime via `#[tokio::test]`) |
| **Config file** | None — standard cargo test discovery |
| **Quick run command** | `cargo test wizard` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test wizard`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 16-01-01 | 01 | 0 | INIT-01 | unit | `cargo test wizard::tests::test_text_input` | ❌ W0 | ⬜ pending |
| 16-01-02 | 01 | 0 | INIT-02 | unit | `cargo test wizard::tests::test_validate_count` | ❌ W0 | ⬜ pending |
| 16-01-03 | 01 | 0 | INIT-03 | unit | `cargo test wizard::tests::test_validate_role` | ❌ W0 | ⬜ pending |
| 16-01-04 | 01 | 0 | INIT-03 | unit | `cargo test wizard::tests::test_tool_cycle` | ❌ W0 | ⬜ pending |
| 16-01-05 | 01 | 0 | INIT-06 | compile | `cargo check` | ❌ W0 | ⬜ pending |
| 16-01-06 | 01 | 0 | INIT-07 | unit | `cargo test wizard::tests::test_validation_error_cleared` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/commands/wizard.rs` — module must be created; inner `#[cfg(test)]` block covers all 5 requirement mappings
- [ ] No new test file in `tests/` — wizard logic tests live inside the module as unit tests

*Existing test infrastructure (`cargo test`) covers all phase requirements once the module exists.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Full TUI wizard renders correctly in terminal | INIT-06 | ratatui rendering requires a PTY; cannot be automated without a terminal emulator | Run `cargo build --release && squad-station init` in a dir without squad.yml; verify pages render |
| Key navigation (Enter/Esc/Ctrl+C) works end-to-end | INIT-06, INIT-07 | Key event dispatch requires interactive terminal | Navigate through all wizard pages manually; verify Esc goes back, Ctrl+C exits cleanly, Enter advances |
| Terminal restored after Ctrl+C | INIT-06 | Requires interactive terminal session | Run wizard, press Ctrl+C mid-flow; verify shell prompt returns in normal mode |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
