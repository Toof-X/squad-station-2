---
phase: 21
slug: quick-guide-and-install-flow
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-18
---

# Phase 21 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (cargo test) |
| **Config file** | none — inline `#[cfg(test)]` modules |
| **Quick run command** | `cargo test welcome` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test welcome`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 21-01-01 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_routing_action_tab_opens_guide` | Wave 0 | ⬜ pending |
| 21-01-02 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_routing_action_right_opens_guide` | Wave 0 | ⬜ pending |
| 21-01-03 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_guide_routing_tab_returns_title` | Wave 0 | ⬜ pending |
| 21-01-04 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_guide_routing_left_returns_title` | Wave 0 | ⬜ pending |
| 21-01-05 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_guide_routing_quit` | Wave 0 | ⬜ pending |
| 21-01-06 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_guide_hint_bar_text` | Wave 0 | ⬜ pending |
| 21-01-07 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_hint_bar_text_includes_tab_guide` | Update existing | ⬜ pending |
| 21-01-08 | 01 | 0 | WELCOME-05 | unit | `cargo test welcome::tests::test_guide_content` | Wave 0 | ⬜ pending |
| 21-02-01 | 02 | 1 | INSTALL-01 | manual | Code inspection: `if (process.stdout.isTTY)` guard present in install() | N/A | ⬜ pending |
| 21-02-02 | 02 | 1 | INSTALL-02 | smoke | `echo "" \| sh install.sh 2>/dev/null; echo $?` exits 0 | N/A | ⬜ pending |
| 21-02-03 | 02 | 1 | INSTALL-03 | smoke | pipe test — no spurious output when piped | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/commands/welcome.rs` — add test stubs for:
  - `test_routing_action_tab_opens_guide`
  - `test_routing_action_right_opens_guide`
  - `test_guide_routing_tab_returns_title`
  - `test_guide_routing_left_returns_title`
  - `test_guide_routing_quit`
  - `test_guide_hint_bar_text`
  - `test_hint_bar_text_includes_tab_guide` (update existing test)
  - `test_guide_content`

*All tests live in `welcome.rs`'s existing `#[cfg(test)]` module — no new test files needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| npm `install()` does not call spawnSync when `isTTY` is false | INSTALL-01 | Node.js TTY property can't be mocked without a test framework | Code inspect: confirm `if (process.stdout.isTTY)` wraps `spawnSync` call |
| curl installer exits 0 non-interactively | INSTALL-02 | Shell script; no Rust unit test | `echo "" \| sh install.sh 2>/dev/null; echo $?` → 0 |
| Silent when piped | INSTALL-03 | Output visibility requires terminal | Confirm no extra output in pipe test above |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
