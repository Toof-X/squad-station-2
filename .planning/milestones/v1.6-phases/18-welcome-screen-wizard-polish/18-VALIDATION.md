---
phase: 18
slug: welcome-screen-wizard-polish
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 18 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && ./tests/e2e_cli.sh` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test && ./tests/e2e_cli.sh`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 18-01-01 | 01 | 1 | WEL-01 | unit | `cargo test test_welcome` | ❌ W0 | ⬜ pending |
| 18-01-02 | 01 | 1 | WEL-02 | unit | `cargo test test_welcome_version` | ❌ W0 | ⬜ pending |
| 18-01-03 | 01 | 1 | WEL-03 | unit | `cargo test test_welcome_subcommands` | ❌ W0 | ⬜ pending |
| 18-01-04 | 01 | 1 | WEL-04 | integration | `cargo build --release && ./target/release/squad-station 2>&1 \| grep -q "init"` | ❌ W0 | ⬜ pending |
| 18-02-01 | 02 | 2 | WIZ-01 | unit | `cargo test test_wizard_model_options` | ❌ W0 | ⬜ pending |
| 18-02-02 | 02 | 2 | WIZ-02 | unit | `cargo test test_wizard_model_stored` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/test_welcome.rs` or unit tests in `src/commands/welcome.rs` — stubs for WEL-01, WEL-02, WEL-03, WEL-04
- [ ] Update existing wizard model tests in `src/commands/init.rs` to expect simplified names

*Note: Rust test framework is built-in — no install step needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ASCII art renders in red terminal color | WEL-01 | Color output requires TTY visual check | Run `squad-station` in a terminal that supports ANSI colors, verify "SQUAD-STATION" title appears in red |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
