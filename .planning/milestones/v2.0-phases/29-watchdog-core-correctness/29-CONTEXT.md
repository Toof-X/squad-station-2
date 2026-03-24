# Phase 29 Context: Watchdog Core Correctness

**Phase Goal:** Users can run `squad-station watch` and have the daemon reliably detect real stalls — deadlocks and prolonged-busy agents — without false positives, while safely coexisting with all other CLI commands

**Created:** 2026-03-24
**Status:** Ready for research/planning

## Prior Decisions (from v2.0 planning)

- Two separate NudgeState instances required — one for idle-inactivity stall, one for deadlock — merging them suppresses deadlock alerts after inactivity nudges fire
- Telegram dispatch deferred to Phase 30 — this phase focuses on tmux injection only
- `count_processing_all()` confirmed to count only `status = 'processing'` rows (verified in code)

## Existing Code Baseline

`src/commands/watch.rs` already implements:
- NudgeState struct (cooldown + max_nudges + reset on activity)
- PID file daemon (`--daemon` / `--stop`) with SIGTERM/SIGINT signal handlers
- Pass 1: Agent reconciliation via `reconcile::reconcile_agents()`
- Pass 2: Global idle stall detection (all idle + no processing → nudge orchestrator)
- Pass 3: Prolonged busy logging (>30 min busy → log only, no injection)
- `log_watch()` file logger to `.squad/log/watch.log`
- 5 unit tests for NudgeState + log_watch

### Gaps to Close

| Requirement | Gap |
|---|---|
| DETECT-01 (deadlock) | Missing — need "processing exists + zero busy agents" detection |
| DETECT-02 (debounce) | Missing — alerts fire immediately, no consecutive-cycle confirmation |
| DETECT-03 (message age) | Missing — needs per-message age check, not just global idle duration |
| DETECT-04 (prolonged busy) | Partial — logs only, needs tmux injection |
| ALERT-01 (tmux message) | Partial — only idle case covered, need deadlock-specific message |
| ALERT-02 (dedup + cooldown) | Partial — NudgeState exists for idle, need separate instance for deadlock |
| OPS-01 (--status) | Missing |
| OPS-02 (configurable flags) | Partial — interval + stall_threshold exist, need cooldown/debounce flags |
| OPS-03 (--dry-run) | Missing |

## Decisions

### 1. Deadlock Alert Message Content & Escalation

**Decision:** Deadlock alerts use escalating message tone, same pattern as idle nudges but with deadlock-specific content. Messages include stuck message IDs for debugging.

**Format per escalation level:**
- Nudge 1: `[SQUAD WATCHDOG] Deadlock detected — {N} processing message(s) but zero busy agents. Stuck: {msg_ids}. Idle for {duration}. Run: squad-station list --status processing`
- Nudge 2: `[SQUAD WATCHDOG] Deadlock persists — {N} stuck message(s): {msg_ids}. {duration} elapsed. Review and re-dispatch or complete manually.`
- Nudge 3 (final): `[SQUAD WATCHDOG] CRITICAL — deadlock unresolved for {duration}. Stuck: {msg_ids}. Watchdog stopping alerts. Manual intervention required.`

**Why:** Escalation conveys urgency progression. Including message IDs lets orchestrator (or user reviewing logs) immediately identify which tasks are stuck without running a separate command.

### 2. Debounce Cycle Count

**Decision:** 3 consecutive poll cycles must confirm a stall condition before the first alert fires.

**Why:** Prevents false positives from transient windows — e.g., slow agent environment loading, brief gap between agent finishing and signal propagating. At default 30s interval, this means ~90 seconds before first alert, which balances detection latency vs false positive rate.

**Implementation:** Add `debounce_count: u32` field to a new `DeadlockState` struct (or extend NudgeState). Increment on each tick where condition holds, reset to 0 when condition clears. Only proceed to nudge logic when `debounce_count >= 3`.

### 3. Dry-Run Behavior Scope

**Decision:**
- `--dry-run` MUST NOT inject anything into tmux panes
- `--dry-run` MUST NOT execute reconciliation actions (pass `dry_run=true` to `reconcile_agents`)
- `--dry-run` MUST still write to `watch.log` (with `[DRY-RUN]` prefix on action lines)
- `--dry-run` MUST still detect and log stall conditions normally

**Why:** Dry-run is for debugging and validation — operators need to see what the watchdog *would* do without affecting the running system. Log output is essential for this purpose.

### 4. --status Output Format

**Decision:** `watch --status` prints a detailed status report:

```
Watchdog Status
  PID:           12345
  Status:        alive
  Uptime:        2h 15m
  Stall State:   deadlock (3 processing, 0 busy)  |  clear
  Last Alert:    2026-03-24T10:30:00Z (deadlock nudge #2)
  Nudge Counts:  idle=1/3, deadlock=2/3
  Poll Interval: 30s
  Stall Threshold: 5m
```

**Implementation:** The status subcommand reads the PID file, checks process liveness, and reads a structured status file (`.squad/watch.status.json`) that the running watchdog updates each tick. This avoids IPC complexity — the status command just reads a file.

**Why:** Operators need to quickly assess whether the watchdog is healthy and what state it's tracking. The status file approach is simple and consistent with the stateless CLI architecture.

## Code Context

### Key files to modify:
- `src/commands/watch.rs` — Main implementation (all changes)
- `src/cli.rs` — Add `--dry-run`, `--status`, `--cooldown`, `--debounce` flags
- `src/db/messages.rs` — May need `list_processing_messages()` for stuck message IDs

### Key patterns to follow:
- NudgeState pattern for deadlock state tracking
- `log_watch()` for all logging
- `reconcile::reconcile_agents(pool, dry_run)` already accepts dry_run bool
- `tmux::send_keys_literal()` for orchestrator injection
- `db::agents::get_orchestrator()` for finding injection target

### Reusable assets:
- NudgeState struct — extend or duplicate for deadlock tracking
- Signal handler setup — already correct
- PID file management — already correct
- log_watch — reuse as-is

## Deferred Ideas

None captured during discussion.

---
*Context created: 2026-03-24*
