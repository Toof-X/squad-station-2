---
phase: 13
slug: safe-injection-and-documentation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-09
---

# Phase 13 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in + tokio-test (Cargo.toml dev-deps) |
| **Config file** | None — standard `cargo test` |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test && ./tests/e2e_cli.sh` |
| **Estimated runtime** | ~10 seconds (unit), ~30 seconds (full with e2e) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 13-01-01 | 01 | 0 | TMUX-01 | unit | `cargo test test_load_buffer_args` | ❌ W0 | ⬜ pending |
| 13-01-02 | 01 | 0 | TMUX-01 | unit | `cargo test test_paste_buffer_args` | ❌ W0 | ⬜ pending |
| 13-01-03 | 01 | 0 | TMUX-01 | unit | `cargo test test_inject_body_args` | ❌ W0 | ⬜ pending |
| 13-01-04 | 01 | 0 | TMUX-01 | unit | `cargo test test_inject_body_cleanup` | ❌ W0 | ⬜ pending |
| 13-01-05 | 01 | 1 | TMUX-02 | unit | `cargo test test_send_uses_inject_body` | ❌ W0 | ⬜ pending |
| 13-02-01 | 02 | 1 | DOCS-01 | manual | `test -f docs/PLAYBOOK.md` | ❌ W0 | ⬜ pending |
| 13-02-02 | 02 | 1 | DOCS-02 | manual | `grep -q "antigravity" docs/PLAYBOOK.md` | ❌ W0 | ⬜ pending |
| 13-02-03 | 02 | 1 | DOCS-03 | manual | `grep -q "Notification" docs/PLAYBOOK.md` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/tmux.rs` — add `load_buffer_args`, `paste_buffer_args`, `inject_body` + unit tests
- [ ] `src/commands/send.rs` — replace `send_keys_literal` call with `inject_body`
- [ ] `docs/PLAYBOOK.md` — create file (does not exist yet)

*No new test files needed — unit tests go in existing `#[cfg(test)]` block in `src/tmux.rs`.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| PLAYBOOK.md documents `signal $TMUX_PANE` | DOCS-01 | Documentation content cannot be auto-tested | Open `docs/PLAYBOOK.md`, verify hook setup section shows `squad-station signal $TMUX_PANE` as inline command, not shell script path |
| PLAYBOOK.md covers Antigravity provider | DOCS-02 | Documentation content cannot be auto-tested | Open `docs/PLAYBOOK.md`, verify Antigravity section exists with correct `tool: antigravity` squad.yml syntax |
| PLAYBOOK.md covers Notification hook | DOCS-03 | Documentation content cannot be auto-tested | Open `docs/PLAYBOOK.md`, verify Notification section shows `permission_prompt` matcher for Claude Code |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
