---
phase: 20
slug: tty-safe-welcome-tui-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-17
---

# Phase 20 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | none — inline `#[cfg(test)] mod tests` in each source file |
| **Quick run command** | `cargo test welcome` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test welcome`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 20-01-W0 | 01 | 0 | WELCOME-01,02,03,04,06 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-01-01 | 01 | 1 | WELCOME-02 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-01-02 | 01 | 1 | WELCOME-03 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-01-03 | 01 | 1 | WELCOME-04 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-01-04 | 01 | 1 | WELCOME-06 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-01-05 | 01 | 1 | WELCOME-07 | unit | `cargo test welcome` | ✅ existing | ⬜ pending |
| 20-02-01 | 02 | 2 | INIT-01 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-02-02 | 02 | 2 | INIT-02 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-02-03 | 02 | 2 | INIT-03 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |
| 20-02-04 | 02 | 2 | WELCOME-01 | unit | `cargo test welcome` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/commands/welcome.rs` — add unit tests for `hint_bar_text(has_config, remaining_secs)` pure function
- [ ] `src/commands/welcome.rs` — add unit test for `routing_action(key, has_config)` returning `Option<WelcomeAction>`
- [ ] `src/commands/welcome.rs` — update `test_welcome_content_has_init_hint` assertion (hint now keyboard-driven, not text-based)
- [ ] `cargo test` baseline must pass after ratatui 0.30 upgrade (frame.size() → frame.area() migration)

*All test stubs for WELCOME-01 through INIT-03 must exist before Wave 1 execution begins.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| BigText pixel-font renders visually in real terminal | WELCOME-02 | Requires real TTY + visual inspection | Run `squad-station` in terminal; verify large pixel-font title appears |
| Auto-exit countdown ticks down correctly | WELCOME-06 | Requires real TTY event loop | Run `squad-station`, wait 10s; verify countdown updates and TUI exits |
| TTY detection: piped output prints static text | WELCOME-01 | Requires piped stdin check | Run `squad-station \| cat`; verify no raw mode attempt, static text printed |
| Init wizard launches after Enter (no squad.yml) | INIT-01 | Requires real terminal + wizard | In fresh dir, run `squad-station`, press Enter; verify wizard starts |
| No re-init when squad.yml exists | INIT-02 | Requires config file + real terminal | With squad.yml present, run `squad-station`, press Enter; verify no wizard |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
