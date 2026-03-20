---
phase: 6
slug: documentation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-08
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Manual review + `cargo test` for regression |
| **Config file** | none |
| **Quick run command** | `cargo check` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo check`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | DOCS-01 | manual | `cat .planning/research/ARCHITECTURE.md` | ✅ | ⬜ pending |
| 06-01-02 | 01 | 1 | DOCS-01 | manual | `grep -c "sqlx" .planning/research/ARCHITECTURE.md` | ✅ | ⬜ pending |
| 06-02-01 | 02 | 1 | DOCS-02 | manual | `grep "send --body" docs/PLAYBOOK.md` | ✅ | ⬜ pending |
| 06-02-02 | 02 | 1 | DOCS-02 | manual | `grep "tool:" docs/PLAYBOOK.md` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

*Existing infrastructure covers all phase requirements. This phase only modifies documentation files — no new test infrastructure needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| ARCHITECTURE.md uses sqlx, not rusqlite | DOCS-01 | Document review | Open `.planning/research/ARCHITECTURE.md`, confirm no `rusqlite` references, all DB uses `sqlx` |
| ARCHITECTURE.md flat module structure | DOCS-01 | Document review | Confirm no `src/tui/`, `src/orchestrator/`, `src/db/agent_repo.rs` references — should show flat `src/commands/` layout |
| PLAYBOOK.md uses `--body` flag | DOCS-02 | Document review | Search for `send --body` pattern, no positional `send <task>` syntax |
| PLAYBOOK.md squad.yml has `tool`/`model`/`description` | DOCS-02 | Document review | Confirm no `provider:` or `command:` fields in yaml examples |
| PLAYBOOK.md agent naming convention | DOCS-02 | Document review | Confirm `<project>-<tool>-<role_suffix>` naming pattern is documented |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
