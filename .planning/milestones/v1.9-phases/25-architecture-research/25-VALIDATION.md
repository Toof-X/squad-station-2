---
phase: 25
slug: architecture-research
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-22
---

# Phase 25 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | none (spike is standalone) |
| **Quick run command** | `cargo build -p spike` |
| **Full suite command** | `cargo test -p spike` |
| **Estimated runtime** | ~30 seconds (includes npm build via build.rs) |

---

## Sampling Rate

- **After every task commit:** Run `cargo build -p spike`
- **After every plan wave:** Run `cargo build -p spike` + manual browser smoke test
- **Before `/gsd:verify-work`:** All 4 spike validations pass
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 25-01-01 | 01 | 1 | SPIKE-1 | build | `cargo build -p spike` | ❌ W0 | ⬜ pending |
| 25-01-02 | 01 | 1 | SPIKE-2 | smoke | manual: `wscat -c ws://localhost:3000/ws` | ❌ W0 | ⬜ pending |
| 25-01-03 | 01 | 1 | SPIKE-3 | manual | n/a (documented in PROJECT.md) | ❌ W0 | ⬜ pending |
| 25-01-04 | 01 | 1 | SPIKE-4 | build | `cargo build -p spike` (includes build.rs) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `spike/` directory — create Cargo workspace member
- [ ] `spike/build.rs` — frontend build integration
- [ ] `spike/src/main.rs` — cohesive mini-app
- [ ] `web/` — Vite React-TS project with @xyflow/react
- [ ] Root `Cargo.toml` — add `[workspace]` with `members = ["spike"]`
- [ ] `.gitignore` — add `web/dist/` and `web/node_modules/` entries

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| WS echo handler echoes messages | SPIKE-2 | Requires running server + WS client | 1. Run `cargo run -p spike` 2. Open `wscat -c ws://localhost:3000/ws` 3. Send a message 4. Verify echo |
| React Flow renders in browser | SPIKE-4 | Visual verification | 1. Run `cargo run -p spike` 2. Open http://localhost:3000 3. Verify React Flow graph renders with nodes |
| Event detection strategy documented | SPIKE-3 | Documentation review | 1. Check PROJECT.md Key Decisions table 2. Verify entries for polling interval, change detection, debounce |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
