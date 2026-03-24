---
phase: 29-watchdog-core-correctness
verified: 2026-03-24T07:30:00Z
status: passed
score: 5/5 must-haves verified
re_verification: false
---

# Phase 29: Watchdog Core Correctness Verification Report

**Phase Goal:** Users can run `squad-station watch` and have the daemon reliably detect real stalls -- deadlocks and prolonged-busy agents -- without false positives, while safely coexisting with all other CLI commands
**Verified:** 2026-03-24T07:30:00Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `watch --status` after `watch --daemon` prints daemon PID, alive status, and uptime | VERIFIED | `show_status()` at line 192 reads `watch.status.json`, prints PID (line 269), status alive/dead (line 271), uptime with hours/minutes (lines 241-252). Handles no-daemon (line 198), stale PID (line 221), and starting-up (line 232) cases. |
| 2 | A deadlock state (processing messages + zero busy agents) triggers tmux injection after N consecutive debounce cycles | VERIFIED | Pass 4 at line 612 checks `!processing_msgs.is_empty() && busy_agents.is_empty()`, filters by age (lines 618-627), calls `deadlock_state.record_tick()` (line 630), gates on `deadlock_state.should_nudge()` (line 644) which requires `is_confirmed()` (consecutive_ticks >= debounce_threshold). Escalating messages at lines 662-675 include message IDs, count, and stall duration. |
| 3 | A stall alert fires exactly once per stall event; subsequent polls do not re-inject until cooldown expires | VERIFIED | `DeadlockState::should_nudge()` at line 81 checks `self.count >= self.max_nudges` and cooldown via `(now - last).num_seconds() > self.cooldown_secs`. `record_nudge()` at line 94 updates `last_nudge_at`. Unit test `test_deadlock_state_cooldown_and_max` (line 838) validates this behavior. |
| 4 | Messages younger than configurable age threshold do not trigger stall alerts | VERIFIED | Lines 618-627 filter processing messages: `now.signed_duration_since(ts) >= threshold` where `threshold = chrono::Duration::minutes(stall_threshold_mins)`. Only stale messages proceed to deadlock detection. Young-only case clears ticks (line 704). |
| 5 | Running `watch --dry-run` logs stall detections without injecting into any tmux pane | VERIFIED | Three `if !dry_run` guards at lines 550, 594, 679 gate all `send_keys_literal` calls. Log entries use `"DRY-RUN"` level prefix (lines 555, 587, 684). Reconciliation also passes `dry_run` (line 490). |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/cli.rs` | Watch variant with dry_run, status, cooldown, debounce fields | VERIFIED | Lines 138-163: all 8 fields present with correct types and defaults (cooldown=600, debounce=3) |
| `src/main.rs` | Dispatch wiring all 8 Watch parameters | VERIFIED | Lines 87-96: destructures all 8 fields, passes to `commands::watch::run()` |
| `src/db/messages.rs` | `list_processing_messages()` returning `Vec<(String, String)>` | VERIFIED | Lines 149-156: queries `SELECT id, created_at FROM messages WHERE status = 'processing' ORDER BY created_at ASC` |
| `src/commands/watch.rs` | DeadlockState struct with debounce, deadlock detection pass, prolonged busy injection, dry-run gating, WatchStatus, show_status | VERIFIED | 881 lines. DeadlockState (line 48), WatchStatus (line 109), write_status (line 127), show_status (line 192), tick with Pass 3 (line 579) and Pass 4 (line 612). 9 unit tests passing. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/commands/watch.rs` | `watch::run()` call with 8 params | WIRED | Line 96: `commands::watch::run(interval, stall_threshold, daemon, stop, dry_run, status, cooldown, debounce).await` |
| `src/commands/watch.rs` | `src/db/messages.rs` | `list_processing_messages()` in deadlock detection | WIRED | Line 614: `db::messages::list_processing_messages(pool).await` |
| `src/commands/watch.rs` | `src/tmux.rs` | `send_keys_literal()` for deadlock and prolonged-busy injection | WIRED | Lines 551, 603, 680: `tmux::send_keys_literal(&orch.name, &msg).await` |
| `src/commands/watch.rs (tick)` | `.squad/watch.status.json` | `serde_json::to_string_pretty` write per tick | WIRED | Line 187-188: `serde_json::to_string_pretty(&status)` then `std::fs::write` |
| `src/commands/watch.rs (--status)` | `.squad/watch.status.json` | File read + deserialize | WIRED | Lines 236-238: `std::fs::read_to_string`, `serde_json::from_str` |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| DETECT-01 | 29-02 | Watchdog detects deadlock state -- processing messages exist but zero agents are busy | SATISFIED | Pass 4 (line 616): `!processing_msgs.is_empty() && busy_agents.is_empty()` |
| DETECT-02 | 29-02 | Watchdog debounces stall detection across N consecutive poll cycles | SATISFIED | `DeadlockState` with `consecutive_ticks` and `debounce_threshold`, `is_confirmed()` check |
| DETECT-03 | 29-02 | Watchdog respects configurable message age threshold | SATISFIED | Lines 618-627: filter by `stall_threshold_mins` age |
| DETECT-04 | 29-02 | Watchdog detects prolonged-busy and injects into orchestrator pane | SATISFIED | Pass 3 (lines 579-610): busy >30min triggers `send_keys_literal` injection |
| ALERT-01 | 29-02 | Stall notification with agent count, pending message count, stall duration | SATISFIED | Lines 662-675: 3 escalation levels with stale count, message IDs, oldest age |
| ALERT-02 | 29-02 | Alert deduplication with configurable cooldown | SATISFIED | `DeadlockState::should_nudge()` with cooldown_secs check; `--cooldown` CLI flag (default 600) |
| OPS-01 | 29-03 | `watch --status` reports daemon alive, PID, uptime | SATISFIED | `show_status()` prints PID, alive/dead, uptime calculation |
| OPS-02 | 29-01, 29-03 | Configurable poll interval, stall threshold, alert cooldown via CLI flags | SATISFIED | CLI: interval, stall_threshold, cooldown, debounce fields with defaults |
| OPS-03 | 29-01, 29-02 | `--dry-run` mode logs without sending alerts | SATISFIED | 3x `if !dry_run` guards; DRY-RUN log level; reconcile passes dry_run |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No anti-patterns found |

No TODO/FIXME/PLACEHOLDER/HACK comments. No empty implementations. No stub returns. All `let _ =` usages are intentional error suppression on file I/O and tmux sends (correct pattern for this codebase).

### Human Verification Required

### 1. Daemon Lifecycle

**Test:** Run `squad-station watch --daemon` then `squad-station watch --status`, then `squad-station watch --stop`
**Expected:** Status shows PID, alive, uptime. Stop kills daemon and cleans PID file.
**Why human:** Requires live daemon process and tmux environment.

### 2. Deadlock Detection End-to-End

**Test:** Create a processing message older than stall_threshold with no busy agents, run `squad-station watch --dry-run --debounce 1 --interval 5`
**Expected:** After 1 poll cycle, watch.log shows `DRY-RUN` deadlock detection with message IDs and duration
**Why human:** Requires live DB state, timing, and log file inspection.

### 3. Prolonged Busy Injection

**Test:** Set an agent to busy status for >30 minutes, run watch foreground
**Expected:** Orchestrator tmux pane receives `[SQUAD WATCHDOG] Agent 'X' busy for Ym` message
**Why human:** Requires tmux sessions and time-based state.

### Gaps Summary

No gaps found. All 5 success criteria verified against the actual codebase. All 9 requirements satisfied with concrete implementation evidence. All key links wired. All 13 tests pass (5 NudgeState + 4 DeadlockState + 4 integration). No anti-patterns detected.

---

_Verified: 2026-03-24T07:30:00Z_
_Verifier: Claude (gsd-verifier)_
