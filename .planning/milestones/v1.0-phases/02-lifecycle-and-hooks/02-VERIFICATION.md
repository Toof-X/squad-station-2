---
phase: 02-lifecycle-and-hooks
verified: 2026-03-06T07:15:00Z
status: passed
score: 11/11 must-haves verified
re_verification: false
gaps: []
human_verification:
  - test: "Run squad-station agents in a live tmux session with registered agents"
    expected: "Table shows colored status (idle=green, busy=yellow, dead=red) with duration like '5m' or '1h30m'. Dead agents revive when session reappears."
    why_human: "tmux reconciliation and ANSI color rendering require a running tmux environment"
  - test: "Register a Claude Code Stop hook and trigger it by ending a response"
    expected: "hooks/claude-code.sh fires, squad-station signal runs, agent status returns to idle. No error is raised in Claude Code."
    why_human: "Requires live Claude Code instance in a configured tmux session"
  - test: "Register a Gemini CLI AfterAgent hook and trigger it by ending a response"
    expected: "hooks/gemini-cli.sh fires, squad-station signal runs, agent status returns to idle. No error is raised in Gemini CLI."
    why_human: "Requires live Gemini CLI instance in a configured tmux session"
---

# Phase 02: Lifecycle and Hooks Verification Report

**Phase Goal:** Agent status is always accurate (reconciled against live tmux state), hook scripts handle both Claude Code and Gemini CLI, and the orchestrator never triggers an infinite loop
**Verified:** 2026-03-06T07:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Agent status column exists in DB with idle/busy/dead values | VERIFIED | `src/db/migrations/0002_agent_status.sql` contains two ALTER TABLE statements with `NOT NULL DEFAULT 'idle'` |
| 2 | `update_agent_status` writes correct status and timestamp to agents table | VERIFIED | `src/db/agents.rs` lines 64-79: SQL UPDATE with status + RFC3339 timestamp; `test_update_agent_status` and `test_update_agent_status_updates_timestamp` pass |
| 3 | Signal for unregistered agent returns Ok (exit 0), not an error | VERIFIED | `src/commands/signal.rs` lines 41-44: Guard 3 — `None => return Ok(())` silently. No `bail!` present in signal.rs |
| 4 | Signal from outside tmux (no TMUX_PANE) returns Ok silently | VERIFIED | `src/commands/signal.rs` lines 10-12: Guard 1 — `TMUX_PANE.is_err() => return Ok(())`. Confirmed by `test_signal_no_tmux_pane_exits_zero` subprocess test |
| 5 | Signal from orchestrator session returns Ok silently (no infinite loop) | VERIFIED | `src/commands/signal.rs` lines 48-50: Guard 4 — `agent_record.role == "orchestrator" => return Ok(())`. Tested in `test_orchestrator_has_orchestrator_role` |
| 6 | Signal completes messages and notifies orchestrator when all guards pass | VERIFIED | `src/commands/signal.rs` lines 56-99: updates message status, retrieves task_id, notifies orchestrator via tmux, sets agent to idle |
| 7 | `squad-station agents` reconciles status against live tmux on every invocation | VERIFIED | `src/commands/agents.rs` lines 20-30: reconciliation loop calls `tmux::session_exists` per agent, updates dead/idle appropriately |
| 8 | Dead agents auto-revive to idle when tmux session reappears | VERIFIED | `src/commands/agents.rs` line 25-27: `session_alive && agent.status == "dead" => update_agent_status("idle")`. Tested by `test_update_agent_status_dead_to_idle` |
| 9 | `squad-station context` outputs Markdown listing all agents with usage commands | VERIFIED | `src/commands/context.rs` 81 lines: full Markdown table with roster + usage guide sections |
| 10 | Hook scripts exit 0 in all code paths | VERIFIED | Both hooks: 4 explicit `exit 0` statements, zero non-zero exits. Both pass `bash -n` syntax check and are executable (chmod 755) |
| 11 | Full test suite passes with zero failures | VERIFIED | `cargo test`: 36 tests across all files, 0 failures (4 unit + 20 test_db + 5 test_lifecycle + 7 test_command) |

**Score:** 11/11 truths verified

---

## Required Artifacts

### Plan 02-01 Artifacts

| Artifact | Provided | Lines | Status | Details |
|----------|---------|-------|--------|---------|
| `src/db/migrations/0002_agent_status.sql` | ALTER TABLE for status + status_updated_at | 2 | VERIFIED | Both `NOT NULL DEFAULT 'idle'` and `NOT NULL DEFAULT (datetime('now'))` present |
| `src/db/agents.rs` | Agent struct with status fields; update_agent_status | 79 | VERIFIED | `pub status: String` and `pub status_updated_at: String` fields present; `update_agent_status` function exported |
| `src/commands/signal.rs` | 4-layer guard logic | 137 | VERIFIED | All 4 guards present in correct order; no `bail!` macro; update_agent_status("idle") called after rows > 0 |

### Plan 02-02 Artifacts

| Artifact | Provided | Lines | Status | Details |
|----------|---------|-------|--------|---------|
| `src/commands/agents.rs` | Agents command with tmux reconciliation + colored table | 95 | VERIFIED | Reconciliation loop, `format_status_with_duration`, `colorize_agent_status`, `pad_colored` — all present and substantive (min_lines: 50 — actual: 95) |
| `src/commands/context.rs` | Context command with Markdown roster + usage guide | 81 | VERIFIED | Full Markdown output with table, roster, and Notes section (min_lines: 30 — actual: 81) |
| `src/cli.rs` | Agents and Context variants in Commands enum | 92 | VERIFIED | Lines 71-73: `Agents` and `Context` variants present |
| `hooks/claude-code.sh` | Claude Code Stop hook wrapping squad-station signal | 30 | VERIFIED | Executable, TMUX_PANE check present, delegates to binary, exits 0 everywhere |
| `hooks/gemini-cli.sh` | Gemini CLI AfterAgent hook wrapping squad-station signal | 29 | VERIFIED | Executable, TMUX_PANE check present, delegates to binary, exits 0 everywhere |

### Plan 02-03 Artifacts

| Artifact | Provided | Lines | Status | Details |
|----------|---------|-------|--------|---------|
| `tests/test_lifecycle.rs` | Integration tests for lifecycle and guard logic | 90 | VERIFIED | 5 tests: signal guard subprocess, dead→idle revival, orchestrator detection (x2), list agents with status (min_lines: 80 — actual: 90) |
| `tests/test_db.rs` | Status-related DB tests | 319 | VERIFIED | Contains `test_update_agent_status`, `test_agent_default_status_is_idle`, `test_update_agent_status_updates_timestamp` — all present and passing |

---

## Key Link Verification

### Plan 02-01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/commands/signal.rs` | `src/db/agents.rs` | `get_agent()` for guard 3; `role` check for guard 4 | VERIFIED | Line 41: `db::agents::get_agent(&pool, &agent)`; Line 48: `agent_record.role == "orchestrator"` |
| `src/commands/signal.rs` | `src/db/agents.rs` | `update_agent_status("idle")` after successful signal | VERIFIED | Line 99: `db::agents::update_agent_status(&pool, &agent, "idle").await?` |
| `src/db/migrations/0002_agent_status.sql` | `src/db/agents.rs` | Agent struct includes status and status_updated_at fields | VERIFIED | `pub status: String` and `pub status_updated_at: String` match migration columns; `sqlx::migrate!` at `db/mod.rs` embeds all migrations |

### Plan 02-02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/commands/agents.rs` | `src/tmux.rs` | `session_exists()` call per agent in reconciliation loop | VERIFIED | Line 21: `let session_alive = tmux::session_exists(&agent.name)` |
| `src/commands/agents.rs` | `src/db/agents.rs` | `update_agent_status()` for dead/idle reconciliation | VERIFIED | Lines 24, 27: `db::agents::update_agent_status(...)` called on status mismatch |
| `src/commands/context.rs` | `src/db/agents.rs` | `list_agents()` for full agent roster | VERIFIED | Lines 10, 23: `db::agents::list_agents(&pool)` called twice (before and after reconciliation) |
| `src/main.rs` | `src/commands/agents.rs` | `Commands::Agents` match arm routes to `agents::run()` | VERIFIED | Line 33: `Agents => commands::agents::run(cli.json).await` |
| `hooks/claude-code.sh` | `squad-station signal` | Shell script calls binary with agent name | VERIFIED | Line 29: `"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 \| ...` |

### Plan 02-03 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `tests/test_lifecycle.rs` | `src/db/agents.rs` | Tests call `update_agent_status` and verify DB state | VERIFIED | Line 41, 45: `db::agents::update_agent_status(...)` — tests pass |
| `tests/test_db.rs` | `src/db/agents.rs` | Tests verify status column defaults and update behavior | VERIFIED | `test_update_agent_status` (line 290), matches `contains: "test_update_agent_status"` requirement |

---

## Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|---------------|-------------|--------|---------|
| SESS-03 | 02-01, 02-02, 02-03 | Station tracks agent status as idle, busy, or dead | SATISFIED | Migration adds status column with DEFAULT 'idle'; `update_agent_status` writes correct state; send.rs sets "busy", signal.rs sets "idle"; tests: `test_update_agent_status`, `test_agent_default_status_is_idle` |
| SESS-04 | 02-02, 02-03 | Station reconciles agent liveness by checking tmux session existence | SATISFIED | `agents.rs` reconciliation loop calls `session_exists` per agent; dead→idle auto-revive; tests: `test_update_agent_status_dead_to_idle`, `test_list_agents_includes_status` |
| SESS-05 | 02-02, 02-03 | Station auto-generates orchestrator context file listing available agents and usage commands | SATISFIED | `context.rs` outputs Markdown table with all agents, status, send commands, and full usage guide section |
| HOOK-01 | 02-01, 02-03 | Signal command skips orchestrator sessions (role=orchestrator) to prevent infinite loop | SATISFIED | `signal.rs` Guard 4 (line 48): `agent_record.role == "orchestrator" => return Ok(())`; tested by `test_orchestrator_has_orchestrator_role` |
| HOOK-02 | 02-02, 02-03 | Hook scripts work for both Claude Code (Stop event) and Gemini CLI (AfterAgent event) | SATISFIED | Both `hooks/claude-code.sh` and `hooks/gemini-cli.sh` exist, are executable, drain stdin, detect tmux session name, delegate to binary; HOOK-02 runtime behavior requires human verification |
| HOOK-03 | 02-01, 02-02, 02-03 | Hook gracefully exits when not in tmux or agent not registered (4-layer guard) | SATISFIED | Guard 1 (TMUX_PANE check), Guard 2 (config/DB error → stderr + Ok), Guard 3 (unregistered → Ok), Guard 4 (orchestrator → Ok); validated by `test_signal_no_tmux_pane_exits_zero` subprocess test |

**Orphaned requirements check:** REQUIREMENTS.md maps SESS-03, SESS-04, SESS-05, HOOK-01, HOOK-02, HOOK-03 to Phase 2. All 6 are claimed by plans 02-01, 02-02, and 02-03. No orphaned requirements.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | — | — | — | No stubs, placeholders, TODO/FIXME comments, or empty implementations found in any phase 2 files |

Scanned: `src/db/migrations/0002_agent_status.sql`, `src/db/agents.rs`, `src/commands/signal.rs`, `src/commands/agents.rs`, `src/commands/context.rs`, `src/cli.rs`, `src/commands/mod.rs`, `src/main.rs`, `hooks/claude-code.sh`, `hooks/gemini-cli.sh`, `tests/test_lifecycle.rs`, `tests/test_db.rs`

---

## Human Verification Required

### 1. Agent Status Display in Live tmux Session

**Test:** Open a tmux session named "frontend", run `squad-station register`, then run `squad-station agents`
**Expected:** Table renders with colored status (idle=green, busy=yellow, dead=red), duration column shows human-readable time (e.g., "idle 2m"), dead agents without sessions show "dead Xm"
**Why human:** ANSI color rendering and tmux session detection require a running tmux environment

### 2. Dead Agent Auto-Revive

**Test:** Register an agent in tmux, kill its session, run `squad-station agents` (should show dead), then recreate the session, run `squad-station agents` again
**Expected:** Status changes from "dead" back to "idle" on the second run
**Why human:** Requires live tmux session manipulation

### 3. Claude Code Hook Integration

**Test:** Configure `hooks/claude-code.sh` as a Stop hook in `.claude/settings.json`. Run Claude Code in an agent tmux session. Finish a response.
**Expected:** Hook fires, binary runs silently, agent status returns to idle. No Claude Code error or interruption.
**Why human:** Requires live Claude Code instance and configured hook registration

### 4. Gemini CLI Hook Integration

**Test:** Configure `hooks/gemini-cli.sh` as an AfterAgent hook in `.gemini/settings.json`. Run Gemini CLI in an agent tmux session. Finish a response.
**Expected:** Hook fires, binary runs silently, agent status returns to idle. No Gemini CLI retry triggered.
**Why human:** Requires live Gemini CLI instance and configured hook registration

---

## Test Suite Summary

| Test File | Tests | Result |
|-----------|-------|--------|
| Unit tests (lib) | 4 | All pass |
| `tests/test_command.rs` | 7 | All pass |
| `tests/test_db.rs` | 20 | All pass (includes 3 new SESS-03 status tests) |
| `tests/test_lifecycle.rs` | 5 | All pass (includes subprocess Guard 1 test) |
| **Total** | **36** | **0 failures** |

Key tests providing Phase 2 gate coverage:
- `test_signal_no_tmux_pane_exits_zero` — end-to-end Guard 1 (HOOK-03) via binary subprocess
- `test_update_agent_status` — SESS-03 status write
- `test_agent_default_status_is_idle` — SESS-03 default state
- `test_update_agent_status_updates_timestamp` — SESS-03 timestamp accuracy
- `test_update_agent_status_dead_to_idle` — SESS-04 revival
- `test_list_agents_includes_status` — SESS-04 status visibility
- `test_orchestrator_has_orchestrator_role` — HOOK-01 role detection
- `test_get_orchestrator_returns_none_when_no_orchestrator` — HOOK-01 edge case

---

## Build Status

`cargo build` — Finished `dev` profile, 0 errors. All 8 artifacts compile cleanly.

---

_Verified: 2026-03-06T07:15:00Z_
_Verifier: Claude (gsd-verifier)_
