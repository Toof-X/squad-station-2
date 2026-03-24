# Pitfalls Research

**Domain:** Stateless CLI with SQLite WAL + tmux — adding a long-lived watchdog process for stall detection and multi-channel alerting (Telegram)
**Researched:** 2026-03-24
**Confidence:** HIGH — findings grounded in direct codebase inspection (`db/mod.rs`, `commands/monitor.rs`, `commands/browser.rs`, `commands/signal.rs`, `tmux.rs`, `db/agents.rs`, `db/messages.rs`) plus verified external sources for SQLite WAL checkpoint starvation, Telegram Bot API reliability, and tokio graceful shutdown patterns.

---

## Critical Pitfalls

### Pitfall 1: Watchdog Holds a Long-Lived Write Pool — Starves All Other CLI Commands

**What goes wrong:**
The watchdog polls every N seconds. If it creates a `db::connect()` pool at startup and holds it alive for the process lifetime, it owns the single-writer SQLite connection permanently. Every other CLI command that calls `db::connect()` — `send`, `signal`, `peek`, `register`, `clone` — blocks waiting for the 5-second acquire timeout, then errors with "connection pool timed out." The squad becomes unusable while the watchdog runs.

This is a concrete risk because `db::connect()` uses `max_connections(1)` by design (SAFE-01: prevents async write-contention deadlock). There is no second write slot available for concurrent CLI callers.

**Why it happens:**
Developers building daemon-style processes habitually create a pool at startup and reuse it throughout the process lifetime. This is correct for web servers with a read-write pool. For Squad Station's single-writer pattern, it is catastrophic — the watchdog becomes the sole holder of the write lock indefinitely.

**How to avoid:**
Apply the same connect-per-refresh pattern already used in `monitor.rs` (3-second refresh cycle, fresh `db::connect()` each time, pool dropped after use) and `browser.rs` (write pool opened, used, `.close().await` called, then dropped every 2 seconds). The watchdog must:
1. Open a `db::connect_readonly()` pool at startup and hold it only for reads (no WAL starvation risk from a read-only pool).
2. For any write operations (if any), call `db::connect()`, write, then explicitly `.close().await` before the next poll tick.
3. Never hold a write pool across poll intervals.

**Warning signs:**
- `squad-station send` hangs for 5+ seconds while watchdog is running
- "connection pool timed out" errors from CLI commands during watchdog session
- `db::connect()` call at the top of `watchdog::run()` with no corresponding `.close().await` in the poll loop

**Phase to address:**
Phase 1 (watchdog core implementation). Pool lifecycle must be the first architectural decision, before any polling logic is written.

---

### Pitfall 2: False Positive Stall — Transient Window Between Signal Completion and Agent Status Update

**What goes wrong:**
The stall condition is: `pending/processing messages exist AND zero agents are busy`. However, between the moment an agent completes a task and the moment `signal.rs` updates both the message status (to `completed`) and the agent status (to `idle`), there is a brief window where the message is still `processing` but the agent's status may already have been updated to `idle` (or not yet updated). The watchdog polling at that exact instant reads: message = processing, all agents = idle → declares a stall → fires an alert → injects noise into the orchestrator's tmux pane.

More critically: the `signal` command itself has a busy_timeout of 3 seconds. If the DB write is delayed (contention from the watchdog's own reads), the agent status update is delayed, widening this window.

**Why it happens:**
The watchdog's poll snapshot is not atomic with the signal write transaction. Two separate processes reading/writing the DB at overlapping moments will always encounter transient inconsistency at transaction boundaries. SQLite WAL mode allows readers to see the last committed state, but the state machine transition (processing → completed, busy → idle) involves two sequential writes that are not grouped in a single transaction.

**How to avoid:**
Implement a debounce threshold: the stall condition must persist for at least 2–3 consecutive poll cycles (e.g., at 5-second poll interval, require 15 seconds of continuous stall) before triggering an alert. A single snapshot showing "no busy agents, pending messages" is insufficient. Only declare a stall when the condition is stable across N consecutive reads. Additionally, examine `updated_at` on pending messages: if `now - updated_at < 30 seconds`, the message is likely in-flight and not a true stall.

**Warning signs:**
- Alert fires immediately on first detection without debounce
- Alert fires when a single agent just completed a task (within 5 seconds of signal)
- No `consecutive_stall_count` or equivalent state variable in the watchdog's detection loop
- No minimum age check on `processing` messages before declaring stall

**Phase to address:**
Phase 1 (stall detection logic). Debounce and message age threshold must be specified as explicit acceptance criteria before any detection code is written.

---

### Pitfall 3: False Positive Stall — Agents Busy in tmux But DB Status Is Stale

**What goes wrong:**
Agent status in the DB (`busy`/`idle`) is written by `signal.rs` when the agent's Stop hook fires. If an agent is actively running a long task and has not yet fired its Stop hook, the DB shows `idle` (the last known state before the task started — unless `send` sets it to `busy` explicitly). The watchdog reads DB-only and sees: message = processing, agent = idle → false stall alert.

The v2.0 design inherits the same "DB is source of truth" constraint as all other commands. But DB state can lag real tmux state for long-running tasks, especially if the `send` command does not set agent status to `busy` at send time.

**Why it happens:**
In the current schema, agent status transitions are driven by `signal` (completion) and reconciliation (session-aliveness check). There is no guaranteed "set to busy on receive" step because the agent's AI tool does not call `squad-station` when it picks up a task — only when it completes one (via Stop hook). The DB may correctly show `idle` for an agent that is actively working on a multi-minute task.

**How to avoid:**
The watchdog's stall detection must combine both conditions before alerting:
1. DB check: `processing` messages exist AND no agents show `busy` status.
2. tmux check: cross-reference with `tmux::list_live_session_names()` to confirm the target agents have active sessions. An agent with a live tmux session is likely running even if its DB status shows `idle`.
3. Apply the message age threshold (see Pitfall 2): only flag messages older than a configurable minimum (e.g., 5 minutes) as potentially stalled.

Note: tmux capture-pane can be used to check if the agent's session is showing a prompt (idle) vs. active output (busy), but this is fragile. Prefer the age threshold approach over pane content parsing.

**Warning signs:**
- Watchdog fires alert for agents running multi-minute tasks
- Stall detection logic only queries the `agents` table with no tmux session cross-check
- No configurable `min_stall_age_seconds` parameter in watchdog options
- Alert fires within 2 minutes of a `send` command for a known long-running task

**Phase to address:**
Phase 1 (stall detection logic). The compound detection condition (DB + tmux session check + message age) must be the stated detection algorithm.

---

### Pitfall 4: Duplicate Alert Spam — Watchdog Fires Every Poll Cycle After Stall Detected

**What goes wrong:**
Once a stall is detected, every subsequent poll cycle continues to observe the same condition (no busy agents, pending messages) and fires another alert. The orchestrator's tmux pane receives one stall injection every N seconds indefinitely. The Telegram chat receives a flood of identical messages until the stall is manually resolved. The user mutes notifications or stops trusting them.

**Why it happens:**
The watchdog is stateless between poll cycles if implemented naively. It has no memory of whether an alert has already been sent for the current stall event. Each cycle independently evaluates the condition and independently decides to alert.

**How to avoid:**
The watchdog must be stateful across poll cycles for alert deduplication:
1. Track a `stall_alerted_at: Option<Instant>` field in the watchdog's in-memory state.
2. When a stall is first detected (after debounce), fire the alert and set `stall_alerted_at = Some(Instant::now())`.
3. Do not fire another alert unless either: (a) the stall was resolved (condition cleared) and a new stall begins, OR (b) a configurable re-alert interval has elapsed (e.g., 30 minutes) for persistent stalls.
4. When the stall resolves (all pending messages completed or an agent becomes busy), clear `stall_alerted_at`.

This is the standard alert deduplication pattern used by Prometheus Alertmanager and Datadog Watchdog.

**Warning signs:**
- Watchdog state struct has no `last_alerted_at` or `stall_start` field
- Alert dispatch is called directly from the condition check with no cooldown guard
- Telegram chat shows identical stall messages arriving every N seconds
- No "stall resolved" state transition in the detection loop

**Phase to address:**
Phase 1 (watchdog core implementation). Alert deduplication state must be designed alongside the detection logic, not added later as a patch.

---

### Pitfall 5: Telegram MCP Plugin Unavailability Crashes the Watchdog

**What goes wrong:**
The watchdog is specified to "notify via Telegram MCP plugin (if available)." If the Telegram alert path is implemented as a blocking call with no error isolation, a network timeout, MCP server restart, or Telegram API 429 rate-limit response causes the entire watchdog process to crash or hang, stopping all future stall detection.

**Why it happens:**
External API calls (HTTP, MCP inter-process) can fail or hang. Without a timeout and fallback, the Rust async runtime's `await` on a Telegram send can block indefinitely if the underlying TCP connection stalls. Additionally, the Telegram Bot API returns HTTP 429 with a `retry_after` field when rate-limited; treating 429 as a generic error and retrying immediately produces a retry storm that gets the bot banned.

**How to avoid:**
1. Wrap every Telegram alert call in a `tokio::time::timeout(Duration::from_secs(10), telegram_send(...))`. If it times out, log a warning and continue — the watchdog must not stop monitoring because Telegram is slow.
2. Check Telegram availability with a capability guard: only attempt Telegram alerts if the MCP plugin responds to a health check at startup. If unavailable, log once and skip silently on subsequent poll cycles.
3. For 429 responses: read the `retry_after` header, store the retry timestamp in watchdog state, and skip Telegram alerts until that timestamp passes.
4. The tmux injection alert path and Telegram alert path are independent. A Telegram failure must never prevent the tmux injection.

**Warning signs:**
- `telegram_alert()` call has no surrounding `tokio::time::timeout()`
- A single `?` propagates Telegram errors up to the watchdog's main loop
- No capability guard — watchdog attempts Telegram on every cycle without checking availability
- No `retry_after` handling for 429 responses

**Phase to address:**
Phase implementing Telegram integration. Error isolation must be a stated requirement before the Telegram call site is written.

---

### Pitfall 6: tmux Injection Alert Into Orchestrator Fires When Orchestrator Session Is Dead

**What goes wrong:**
The stall alert injects a notification into the orchestrator's tmux pane via `send_keys`. If the orchestrator session is dead (the user closed it, or the AI provider crashed), `tmux send-keys -t <session-name>` fails silently (exit 1, no output) or returns an error. The alert was "sent" from the watchdog's perspective, but the orchestrator never saw it. The watchdog marks the alert as delivered and stops retrying. The stall goes unnoticed.

**Why it happens:**
The existing pattern in `signal.rs` and `send.rs` calls `tmux send-keys` without checking whether the target session is alive first (the liveness check is done by `reconcile_agents`, a separate step). The watchdog, running as a background process, does not have a synchronous reconciliation step before alert injection.

**How to avoid:**
Before injecting the stall alert into the orchestrator's tmux pane:
1. Call `tmux::session_exists(orchestrator_name)` (wraps `tmux has-session`). If false, skip the tmux injection and log a warning.
2. Escalate to Telegram only if the tmux injection fails. This inverts the typical priority: tmux is the primary channel; Telegram is the escalation when tmux is unreachable.
3. Log the alert attempt outcome (`ALERT sent to tmux: OK`, `ALERT tmux unavailable: escalated to Telegram`) for post-mortem visibility.

**Warning signs:**
- No `session_exists` check before `send_keys` in the alert dispatch
- Watchdog logs "alert sent" even when the orchestrator session is not visible in `tmux ls`
- No escalation logic — Telegram and tmux alerts are always fired independently, not as fallback chain

**Phase to address:**
Phase 1 (tmux alert dispatch). The session-alive check must precede the `send_keys` call. The fallback chain (tmux → Telegram) must be an explicit design decision.

---

### Pitfall 7: WAL Checkpoint Starvation From Continuous Read-Only Pool

**What goes wrong:**
A watchdog holding a long-lived `connect_readonly()` pool with 5 connections polling every second maintains at least one open read transaction continuously. SQLite WAL checkpoint cannot reset the WAL file while any reader holds an open read transaction. If the watchdog's polling loop never closes its read transaction between ticks, the WAL file grows indefinitely: every write from `send`, `signal`, and other CLI commands appends to the WAL without the WAL ever being checkpointed back to the main DB file. Over hours of watchdog operation, WAL file size grows to hundreds of MB, read performance degrades quadratically, and eventually SQLite begins returning `SQLITE_BUSY` even from readers.

**Why it happens:**
The `connect_readonly()` pool is designed for concurrent reading (the browser server uses it for WebSocket push). In the browser server, the read pool is used for short bursts during client-initiated snapshots. In a watchdog polling every second, the read pool is under continuous load with no idle periods. SQLite autocommit mode opens and closes transactions per query, but connection pooling with `max_connections > 1` can keep connections alive with open shared-cache read locks between queries.

**How to avoid:**
Apply the same connect-per-refresh pattern confirmed correct for the TUI (`monitor.rs`):
1. Do not hold a `connect_readonly()` pool across poll ticks.
2. Open a fresh `connect_readonly()` connection at the start of each poll cycle, run the queries, then drop it before sleeping until the next tick.
3. Alternatively: hold one `connect_readonly()` pool with `max_connections(1)` and use short timeouts to ensure the connection is released between ticks.
4. Set a watchdog poll interval no faster than 5 seconds — this provides natural reader gaps for WAL checkpointing.

**Warning signs:**
- `connect_readonly()` called once at watchdog startup, pool held across the entire process lifetime
- WAL file size growing while watchdog runs: `ls -lh .squad/station.db-wal`
- CLI commands (`send`, `signal`) becoming progressively slower while watchdog runs
- Poll interval set to sub-second values

**Phase to address:**
Phase 1 (watchdog core implementation). Connection lifecycle and poll interval must be specified as explicit constraints before any polling code is written.

---

### Pitfall 8: Watchdog Cannot Be Stopped Cleanly — Orphaned Process on Ctrl+C

**What goes wrong:**
The watchdog is the first long-lived background command in Squad Station. If it ignores `SIGTERM`/`SIGINT` or does not implement graceful shutdown, Ctrl+C terminates it abruptly mid-poll. If the watchdog holds any file handles (log files, WAL/SHM), the abrupt exit can leave the DB in a state that triggers WAL recovery on next connect (the existing `try_wal_recovery` path in `db/mod.rs` handles this, but it requires `lsof` and is slower than a clean shutdown). If the watchdog is run in the background via `squad-station watchdog &`, there is no established `squad-station watchdog stop` command — the user must `kill` the PID manually, which is poor UX.

**Why it happens:**
Rust's default process termination on SIGINT closes file descriptors, but any in-flight async operations (pending DB write, pending tmux send) are abandoned without rollback. The existing binary is stateless — every command exits cleanly because it runs one operation then exits. A long-lived process requires explicit signal handling that the codebase has no precedent for (the `browser` command uses `tokio::signal` for graceful shutdown, but only for SIGINT/SIGTERM, not for the watchdog's specific cleanup needs).

**How to avoid:**
1. Use `tokio::signal::ctrl_c()` and `tokio::signal::unix::signal(SignalKind::terminate())` in a `tokio::select!` against the poll loop, identical to the pattern in `browser.rs`'s graceful shutdown handler.
2. On shutdown signal: finish the current poll tick, close all DB pools explicitly with `.close().await`, flush any pending log writes, then exit.
3. Write a PID file to `.squad/watchdog.pid` at startup; remove it on clean shutdown. This enables `squad-station watchdog stop` to send SIGTERM to the recorded PID.
4. Log shutdown event: `"watchdog: received SIGTERM, shutting down after current tick"`.

**Warning signs:**
- No `tokio::select!` combining the poll loop with a shutdown signal in `watchdog::run()`
- No `.close().await` on DB pools before process exit
- No PID file written on startup
- `browser.rs` graceful shutdown code not used as a reference for the same pattern

**Phase to address:**
Phase 1 (watchdog process lifecycle). Graceful shutdown is not optional — it is the minimum viable implementation for a long-lived process.

---

### Pitfall 9: Stall Detection Triggers on Legitimate Idle Periods

**What goes wrong:**
A valid workflow state is: orchestrator sent all tasks, all agents completed, orchestrator is deciding next steps. During this thinking period, the DB shows no `processing` messages and no busy agents. This is not a stall — it is normal orchestrator cognition. However, if the orchestrator takes longer than the watchdog's stall threshold (e.g., 5 minutes of thinking on a complex architecture decision), the watchdog fires a false alert and injects a "stall detected" message into the orchestrator's pane, interrupting its reasoning mid-thought.

**Why it happens:**
The stall condition "no busy agents AND pending/processing messages exist" is necessary but not sufficient to distinguish a real deadlock from an intentional idle. An idle orchestrator with no pending messages dispatched yet looks identical to a post-deadlock state from the DB's perspective.

**How to avoid:**
Refine the stall condition to only trigger when `processing` (not just `pending`) messages exist with no busy agents. `pending` messages may not yet have been dispatched by the orchestrator and do not constitute a stall by themselves. Only `processing` messages assigned to an agent that has been idle for longer than a configurable threshold constitute a stall. Recommended default: 10 minutes for `processing` message age before stall alert.

Additionally: if no messages are in `processing` state at all, do not alert regardless of `pending` count. The orchestrator is thinking, not stuck.

**Warning signs:**
- Stall detection query checks `status IN ('pending', 'processing')` without distinguishing between the two
- No minimum age on `processing` messages before alerting
- Alert fires during normal inter-task orchestrator pause periods (< 10 minutes)
- No configurable threshold for stall sensitivity

**Phase to address:**
Phase 1 (stall detection logic). The exact SQL condition and age threshold must be specified as explicit acceptance criteria.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hold a write pool for the watchdog's process lifetime | Simpler code (no per-tick connect) | Starves all CLI commands from writing; squad becomes unusable during watchdog run | Never |
| Fire alert on first detection, no debounce | Simpler detection loop | False positive spam on transient signal/status update windows | Never |
| Alert every poll cycle after stall detected | Always notifies | Floods orchestrator tmux pane and Telegram with identical messages; alert fatigue | Never |
| No timeout on Telegram API call | Simpler async code | One Telegram network stall hangs the watchdog indefinitely; monitoring stops | Never |
| Treat 429 from Telegram as generic error, sleep 5s | Easy to implement | Over-sleeping in light load, potentially banned at peak | Never — read `retry_after` |
| Alert only via Telegram, not tmux injection | Single channel simpler | User must check phone; if Telegram is down, alerts are lost entirely | Never — tmux is the primary channel |
| Long-lived `connect_readonly()` pool held continuously | Fewer connection open/close calls | WAL grows indefinitely; CLI commands slow over hours | Never |
| No graceful shutdown | Zero extra code | Abrupt exit may leave WAL in recovery state; PID tracking impossible | Never for a production watchdog |
| Check DB status only, not tmux session liveness | No tmux subprocess overhead | False positives for agents running long tasks not yet reflected in DB | Acceptable only with a long age threshold (>10 min) as compensation |

---

## Integration Gotchas

Common mistakes when connecting the watchdog to existing systems.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Watchdog → SQLite write pool | Open `db::connect()` once and hold it | Use connect-per-refresh: open, use, `.close().await`, drop — same as `monitor.rs` |
| Watchdog → SQLite read pool | Hold `connect_readonly()` across all ticks | Drop read pool between ticks, or use `max_connections(1)` with short idle timeout |
| Watchdog → stall detection | Check `status = 'processing'` only | Also check message `updated_at` age AND tmux session liveness before alerting |
| Watchdog → tmux injection | Call `send_keys` without checking session alive | Call `session_exists()` first; if dead, skip tmux and escalate to Telegram |
| Watchdog → Telegram MCP | Await Telegram send directly in poll loop | Wrap in `tokio::time::timeout(10s, ...)`; errors must not stop the poll loop |
| Watchdog → Telegram 429 | Treat 429 as generic error, retry immediately | Read `retry_after`, store next-allowed-send timestamp, skip until it passes |
| Watchdog → alert deduplication | Fire alert in every cycle that matches condition | Track `stall_alerted_at` in watchdog state; deduplicate within configurable cooldown window |
| Watchdog → graceful shutdown | No SIGTERM/SIGINT handling | Use `tokio::select!` with `tokio::signal` same as `browser.rs`; write/remove PID file |
| Watchdog → `browser` server coexistence | Both hold `connect_readonly()` pools long-lived | Both must use short-lived connections; two concurrent long-lived read pools magnify WAL starvation |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Sub-second poll interval (e.g., 500ms) | WAL never checkpoints; WAL file grows unbounded; DB reads slow | Minimum 5-second poll interval for watchdog; leave reader gaps for checkpointing | Hours of runtime with any write activity |
| Watchdog + browser server both polling at high frequency | WAL starvation doubled; `squad-station send` begins timing out | Coordinate poll intervals; ensure both use connect-per-refresh | Immediately if both run simultaneously with <5s intervals |
| Telegram call inside the poll tick (blocking the loop) | Poll tick extends to the Telegram API round-trip time | Fire Telegram alerts in a detached `tokio::spawn` task; poll loop does not await the result | Every time Telegram is slow (frequent in practice) |
| tmux `capture-pane` called for all agents during stall check | O(N) subprocess spawns per tick | Stall detection should not use `capture-pane`; use DB + `list_live_session_names()` (single subprocess) | 5+ agents |
| Accumulating stall event history in memory | Memory grows if watchdog runs for days | Keep only last alert timestamp; no unbounded event history | Days of continuous operation |

---

## Security Mistakes

Domain-specific security issues.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Watchdog injects arbitrary alert text into orchestrator tmux pane without sanitization | If the stall message is constructed from DB content (agent names, task bodies), malicious task body content could inject shell commands via tmux | Alert message must be a static string (e.g., "WATCHDOG: stall detected — N messages processing, no busy agents") with no DB content interpolated into the tmux injection |
| Telegram bot token stored in plaintext in squad.yml or watchdog config | Token leaked in repo history if squad.yml is committed | Read token from environment variable only (`SQUAD_TELEGRAM_TOKEN`); never from config file |
| Watchdog PID file writable by other users on shared system | Malicious process writes fake PID, `watchdog stop` sends SIGTERM to wrong process | PID file permissions: 0600; verify PID belongs to a `squad-station` process before sending SIGTERM |
| Alert message truncation if task body is very long | Message content truncation causes confusing alerts | Truncate task body in alert to 80 chars with `...` suffix; never send the full task body |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Watchdog floods orchestrator pane with multi-line alert injection | Interrupts orchestrator mid-reasoning with a wall of text | Inject a single-line alert: `[WATCHDOG] Stall detected: N messages stuck (age: Xm). Run squad-station status` |
| Watchdog has no way to be stopped gracefully | User must `kill $(cat .squad/watchdog.pid)` manually | Implement `squad-station watchdog stop` that reads `.squad/watchdog.pid` and sends SIGTERM |
| Watchdog provides no status indication while running | User cannot tell if watchdog is healthy or has silently failed | Write periodic heartbeat to `.squad/watchdog.heartbeat` (timestamp); `watchdog status` reads it and reports |
| Watchdog alerts for stalls that the user intentionally created (paused workflow) | False alerts on paused projects | Implement `squad-station watchdog pause [N minutes]` that temporarily suppresses alerts |
| Telegram alert contains no actionable information | User sees "stall detected" but does not know which agent or message | Include: number of stuck messages, oldest stuck message age, list of agent names and their DB status |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Connection lifecycle:** Is there a `db::connect()` call at watchdog startup that is never closed? Verify the poll loop does connect-per-refresh, not a long-held pool.
- [ ] **Debounce:** Is there a consecutive-cycle counter before the first alert fires? Verify that a single-cycle stall observation does not trigger an alert.
- [ ] **Message age threshold:** Is there a minimum age check on `processing` messages? Verify that a message 30 seconds old does not trigger a stall alert.
- [ ] **Alert deduplication:** Is there a `stall_alerted_at` field in watchdog state? Verify that a persisting stall does not produce a new alert on every poll cycle.
- [ ] **Telegram timeout:** Is every Telegram API call wrapped in `tokio::time::timeout()`? Verify that a Telegram network failure does not hang the poll loop.
- [ ] **tmux session check:** Is `session_exists()` called before `send_keys` alert injection? Verify with a test where the orchestrator session is stopped mid-watchdog-run.
- [ ] **Graceful shutdown:** Does Ctrl+C produce clean log output and exit 0? Verify that DB pools are closed before process exit.
- [ ] **PID file:** Is `.squad/watchdog.pid` created on startup and removed on clean shutdown? Verify the file does not persist after the process exits.
- [ ] **Stall vs. idle distinction:** Does the detection only flag `processing` messages (not `pending`)? Verify that a queue of pending messages with no dispatched tasks does not trigger a stall alert.
- [ ] **Telegram 429 handling:** Is the `retry_after` value read and respected? Verify that a simulated 429 response does not trigger a retry storm.
- [ ] **WAL health:** After 30+ minutes of watchdog running alongside active `send`/`signal` calls, does `.squad/station.db-wal` remain a reasonable size (< 10 MB)? If growing unbounded, the connection pattern needs fixing.

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Watchdog blocking CLI commands (write pool held) | HIGH | Kill watchdog PID; `squad-station send` unblocks immediately; re-deploy watchdog with fixed connection lifecycle |
| Alert spam (no deduplication) | MEDIUM | Kill watchdog; inform orchestrator the alerts were false; re-deploy with deduplication state |
| Telegram bot banned (429 retry storm) | MEDIUM | Wait for Telegram's ban window to expire (usually 1–24 hours); re-deploy with proper `retry_after` handling |
| WAL file grown unbounded | MEDIUM | Stop watchdog; run `sqlite3 .squad/station.db "PRAGMA wal_checkpoint(TRUNCATE);"` to force checkpoint; re-deploy with corrected connection pattern |
| Watchdog stuck (Telegram hung, no shutdown possible) | LOW | `kill -9 $(cat .squad/watchdog.pid)` or `pkill squad-station`; WAL recovery handles the abrupt exit via existing `try_wal_recovery` path |
| False positive alert injected into orchestrator | LOW | Send a follow-up message to orchestrator: "Previous WATCHDOG alert was a false positive — ignore"; the orchestrator's AI will adapt |
| Stall never detected (debounce too conservative) | LOW | Reduce debounce threshold; re-deploy |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Long-held write pool starves CLI commands | Phase 1: watchdog core | Integration test: run `squad-station send` while watchdog is running; verify < 1s completion |
| False positive from transient signal window | Phase 1: stall detection | Unit test: DB shows processing + idle but message is 10s old; verify no alert fires |
| False positive from stale agent DB status | Phase 1: stall detection | Unit test: `processing` message, agent idle in DB, but live tmux session; verify no alert fires |
| Duplicate alert spam | Phase 1: watchdog state | Unit test: condition persists for 5 cycles; verify alert fires only once |
| Telegram failure crashes watchdog | Phase 2: Telegram integration | Unit test: mock Telegram endpoint returns 500; verify watchdog continues polling |
| Telegram 429 retry storm | Phase 2: Telegram integration | Unit test: mock returns 429 with `retry_after: 60`; verify no retry for 60 seconds |
| tmux injection to dead orchestrator | Phase 1: tmux alert dispatch | Unit test: orchestrator session does not exist; verify tmux send skipped, Telegram escalation triggered |
| WAL checkpoint starvation | Phase 1: watchdog core | Manual: run watchdog 30 min + active sends; verify WAL file stays < 5 MB |
| No graceful shutdown | Phase 1: process lifecycle | Manual: Ctrl+C during poll; verify clean log output, DB pools closed, PID file removed |
| Stall alert during intentional idle | Phase 1: stall detection | Unit test: all messages `pending` (not `processing`), all agents idle; verify no alert fires |

---

## Sources

- `src/db/mod.rs` — single-writer pool design (`max_connections(1)`), `connect_readonly()` pattern, `try_wal_recovery` path, timeout layering (BUSY_TIMEOUT 3s, ACQUIRE_TIMEOUT 5s, CONNECT_TIMEOUT 8s)
- `src/commands/monitor.rs` — connect-per-refresh pattern: `db::connect()` called inside refresh loop, pool dropped after each use — the established correct pattern for long-lived polling
- `src/commands/browser.rs` lines 376–411 — write pool opened, used, `.close().await`, dropped every 2 seconds in the reconcile tick; read pool held for polling (correct for browser's use case)
- `src/db/agents.rs` — `status_updated_at` semantics, `get_orchestrator` lookup, agent status values (`busy`/`idle`/`dead`/`frozen`)
- `src/db/messages.rs` — message status values (`processing`/`completed`), `updated_at` timestamp available for age checks
- `src/commands/signal.rs` — GUARD 3 (missing agent = silent exit), signal write sequence (message status update + agent status update = two sequential writes, not a single transaction)
- `src/tmux.rs` — `send_keys_args` uses `-l` (literal), `list_live_session_names()` for bulk session detection
- [SQLite WAL — Write-Ahead Logging](https://sqlite.org/wal.html) — checkpoint starvation: WAL cannot reset while any reader holds a read transaction; reader gaps required
- [SQLite Forum: Checkpoint Starvation](https://sqlite.org/forum/info/7da967e0141c7a1466755f8659f7cb5e38ddbdb9aec8c78df5cb0fea22f75cf6) — concrete strategies: short transactions, reader gaps, RESTART/TRUNCATE checkpoint modes
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown) — CancellationToken pattern, `tokio::signal::ctrl_c()`, `tokio::signal::unix::signal(SignalKind::terminate())`
- [GramIO: Telegram Rate Limits](https://gramio.dev/rate-limits) — `retry_after` field in 429 response, token-bucket algorithm, per-chat limits
- [Telegram Bot API Rate Limits Explained](https://hfeu-telegram.com/news/telegram-bot-api-rate-limits-explained-856782827/) — 1 msg/sec/chat soft limit, 20 msg/min in groups, adaptive throttling
- [Fixing 429 Errors: Practical Retry Policies](https://telegramhpc.com/news/574/) — retry storm prevention, exponential backoff, `retry_after` compliance
- [Prometheus Alertmanager Issue #2429](https://github.com/prometheus/alertmanager/issues/2429) — alert deduplication and suppress-repeat patterns
- [tmux send-keys race condition — claude-code Issue #23513](https://github.com/anthropics/claude-code/issues/23513) — real-world evidence of tmux injection timing issues in agent workflows
- `.planning/PROJECT.md` — "stateless CLI" constraint, connect-per-refresh TUI pattern decision, `max_connections(1)` write pool rationale, browser server architecture

---

*Pitfalls research for: Squad Station v2.0 — workflow watchdog with stall detection and multi-channel alerting*
*Researched: 2026-03-24*
