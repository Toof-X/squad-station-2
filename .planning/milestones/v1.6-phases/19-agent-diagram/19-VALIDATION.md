---
phase: 19
slug: agent-diagram
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 19 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`) |
| **Config file** | none — uses `#[cfg(test)]` inline and `tests/` integration files |
| **Quick run command** | `cargo test diagram` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test diagram`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 19-01-01 | 01 | 0 | DIAG-01 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |
| 19-01-02 | 01 | 0 | DIAG-02 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |
| 19-01-03 | 01 | 0 | DIAG-03 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |
| 19-01-04 | 01 | 1 | DIAG-01 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |
| 19-01-05 | 01 | 1 | DIAG-02 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |
| 19-01-06 | 01 | 1 | DIAG-03 | unit | `cargo test diagram` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/commands/diagram.rs` — module with `pub fn print_diagram()` and inline `#[cfg(test)]` tests covering:
  - Box border characters present (`┌`, `─`, `┐`, `│`, `└`, `┘`)
  - Agent name present in output
  - Status badge present (idle/busy/dead)
  - `▼` arrow present for worker count ≥ 1
  - `ORCHESTRATOR` label present in orchestrator box
  - No output (or graceful empty state) when agents list is empty

*No new test infrastructure needed — existing `#[cfg(test)]` pattern in commands modules is the standard.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Colors render correctly in terminal | DIAG-03 | ANSI colors can't be asserted in unit tests without stripping | Run `squad-station init` in a real terminal; verify idle=green, busy=yellow, dead=red |
| Diagram layout looks correct at 80-col terminal | DIAG-01 | Visual layout requires human inspection | Run `squad-station init` with 3+ workers and inspect box alignment |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
