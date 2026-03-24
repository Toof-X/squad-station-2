# Architecture Research

**Domain:** Rust CLI — stateless binary, embedded SQLite WAL, tmux integration, long-lived watchdog command (v2.0 Workflow Watchdog)
**Researched:** 2026-03-24
**Confidence:** HIGH — all findings derived from direct source inspection of the current codebase. No external sources required; the question is integration-only, not ecosystem discovery.

---

## Existing Architecture (v1.9 Baseline)

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Entry Point                                  │
│   main.rs → SIGPIPE handler → Cli::parse() → run(cli)               │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────────┐
│                       CLI Dispatch (src/cli.rs)                      │
│   Cli { command: Option<Commands> }                                  │
│   None  →  welcome TUI or print_welcome()                            │
│   Some  →  match to subcommand handler                               │
└──────────┬──────────────────────────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────────────────────────┐
│                   Commands Layer (src/commands/)                     │
│  welcome.rs  wizard.rs  init.rs   ui.rs    send.rs  signal.rs        │
│  agents.rs   context.rs status.rs view.rs  peek.rs  register.rs      │
│  list.rs     notify.rs  close.rs  reset.rs freeze.rs clean.rs        │
│  reconcile.rs clone.rs  fleet.rs  watch.rs [ACTIVE] browser.rs       │
└──────┬─────────────────────────┬───────────────────────────────────┘
       │                         │
┌──────▼──────────┐    ┌─────────▼─────────────────────────────────┐
│  src/tmux.rs    │    │  src/db/  (SQLite via sqlx 0.8)            │
│  send_keys      │    │  mod.rs → connect() / connect_readonly()   │
│  inject_body    │    │  agents.rs → insert/get/list/update        │
│  session_exists │    │  messages.rs → insert/list/update/count    │
│  launch_agent   │    │  migrations/ → auto-applied on connect     │
│  (arg builders) │    └────────────────────────────────────────────┘
└─────────────────┘
```

### Long-Lived Command Patterns Already Established

Two existing commands already run as long-lived processes, each with a distinct polling pattern. The watchdog is the third.

| Command | Pool type | Polling mechanism | Shutdown |
|---------|-----------|-------------------|---------|
| `ui` (ratatui TUI) | connect-per-refresh (new pool each tick, 3s interval) | `tokio::time::interval` inside event loop | Ctrl+C → raw mode teardown |
| `browser` (axum server) | Read-only pool (5 connections) kept open; short-lived write pool opened+dropped per reconcile | `tokio::spawn(poll_agents)` and `tokio::spawn(poll_messages)` at 1s / 500ms | `tokio::signal` via `with_graceful_shutdown()` |
| `watch` (watchdog) | **Single write pool kept open across ticks** | `tokio::time::sleep(1s)` loop with `AtomicBool` SHUTDOWN flag | `libc::signal(SIGTERM/SIGINT)` → `AtomicBool` |

The watchdog uses a different pool strategy than browser: it opens one `db::connect()` pool at startup and reuses it across all ticks. This is intentional — the watchdog is the only writer during its lifetime and the poll interval (30s default) is long enough that a held pool does not starve other CLI commands (which connect, write, and immediately close).

---

## Current `watch.rs` Implementation (v2.0 Work-in-Progress)

`src/commands/watch.rs` already exists with a full tick loop. The current implementation:

**Three-pass tick:**
1. **Pass 1 — Individual agent reconciliation:** Calls `reconcile::reconcile_agents(pool, false)` to detect busy-in-DB / idle-in-tmux mismatches and fix them. Logs each action taken.
2. **Pass 2 — Global stall detection:** If all non-dead agents are idle AND zero processing messages exist AND last DB activity was >= `stall_threshold_mins` ago, nudges the orchestrator via `tmux::send_keys_literal`. Uses `NudgeState` for 10-minute cooldown and 3-nudge maximum before going silent.
3. **Pass 3 — Prolonged busy detection:** Logs a WARN for any agent busy > 30 minutes.

**Daemon mode:** `--daemon` forks a background process using `std::process::Command::spawn()` (not `nix::unistd::fork()` — simpler, portable). PID written to `.squad/watch.pid`. `--stop` sends `SIGTERM` to PID.

**Signal handling:** Uses raw `libc::signal()` with an `extern "C"` trampoline setting a `static AtomicBool SHUTDOWN`. The main loop checks this flag every 1-second sleep increment.

**Logging:** Appends timestamped lines to `.squad/log/watch.log` via `log_watch(squad_dir, level, msg)`.

**What is NOT yet implemented:**
- Telegram alerting (PROJECT.md v2.0 target feature)
- Stall detection for the case where pending/processing messages exist but zero agents are busy (deadlock) — current Pass 2 only fires when all agents are idle AND zero processing messages, which is the "everything stopped" case, not the "tasks stuck, no workers" deadlock

---

## v2.0 Watchdog — What Needs to Be Added

### Gap 1: True Deadlock Detection (pending messages + zero busy agents)

**Current condition in watch.rs Pass 2:**
```rust
let all_idle = non_dead.iter().all(|a| a.status == "idle");
let processing_count = db::messages::count_processing_all(pool).await.unwrap_or(0);
if all_idle && processing_count == 0 { ... nudge after threshold }
```

This fires when the system is completely idle — no tasks queued. It does NOT detect the deadlock case: `processing_count > 0` but no agent is busy (e.g., all agents went dead or hung while a task was mid-processing).

**Fix:** Add a second stall condition in the same tick:
```rust
let has_stuck_tasks = processing_count > 0 && non_dead.iter().all(|a| a.status != "busy");
```
When `has_stuck_tasks == true`, alert immediately (no threshold timer needed — this is an active failure, not just inactivity).

This requires a separate `NudgeState` instance (or an enum-tagged nudge state) to avoid conflating idle-inactivity nudges with deadlock alerts.

### Gap 2: Telegram Alerting

**What PROJECT.md requires:** "notify via Telegram MCP plugin (if available)"

**Architecture decision — command-line dispatch, not native HTTP:**

The Telegram MCP plugin runs inside the Claude Code session as an MCP server. It is not a Rust HTTP client target. The correct integration point is via the orchestrator's tmux session (which has MCP access) or via a shell command that invokes a Telegram bot API directly.

Two integration options:

**Option A: Orchestrator-mediated (recommended):** The watchdog injects a structured stall alert into the orchestrator's tmux pane. The alert message includes a signal phrase like `[SQUAD STALL ALERT]`. The orchestrator's Claude Code session, which has the Telegram MCP plugin active, can be instructed (via its system prompt / squad-orchestrator.md) to forward `[SQUAD STALL ALERT]` messages to Telegram automatically. This requires zero new Rust code for Telegram — the alert channel is tmux injection, which already works.

**Option B: Direct Telegram Bot API call from Rust:** Add `reqwest` dependency (or use `std::process::Command` to `curl`) to POST to `https://api.telegram.org/bot<TOKEN>/sendMessage`. Token and chat_id read from environment variables (`SQUAD_TELEGRAM_TOKEN`, `SQUAD_TELEGRAM_CHAT_ID`). If vars are absent, skip silently — "if available" behavior. This is self-contained but adds a network dependency to the binary.

**Recommendation: Option A for MCP-aware Telegram, Option B as independent fallback.**

The implementation for Option B fits in a new `src/commands/alert.rs` module (or inline in `watch.rs`) with a single async function:

```rust
// src/commands/alert.rs
pub async fn send_telegram(message: &str) -> bool {
    let token = match std::env::var("SQUAD_TELEGRAM_TOKEN") {
        Ok(t) => t,
        Err(_) => return false, // Not configured — skip silently
    };
    let chat_id = match std::env::var("SQUAD_TELEGRAM_CHAT_ID") {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Use tokio::process::Command to curl — no new Rust dependency
    let payload = serde_json::json!({
        "chat_id": chat_id,
        "text": message,
        "parse_mode": "Markdown"
    });
    let status = tokio::process::Command::new("curl")
        .args(["-s", "-X", "POST",
               &format!("https://api.telegram.org/bot{}/sendMessage", token),
               "-H", "Content-Type: application/json",
               "-d", &payload.to_string()])
        .status()
        .await;
    status.map(|s| s.success()).unwrap_or(false)
}
```

Using `tokio::process::Command` to shell out to `curl` avoids adding `reqwest` to the dependency tree. `curl` is present on all target platforms (darwin, linux). This matches the existing pattern in `browser.rs` where `tokio::process::Command::new("tmux")` is used for all tmux interactions.

---

## Complete Component Map for v2.0

### Existing Components — No Change

| Component | Role in watchdog context |
|-----------|--------------------------|
| `src/db/agents.rs` | `list_agents()`, `get_orchestrator()` — already used by tick |
| `src/db/messages.rs` | `count_processing_all()`, `total_count()`, `last_activity_timestamp()` — already used by tick |
| `src/commands/reconcile.rs` | `reconcile_agents()` — already called in Pass 1 |
| `src/tmux.rs` | `send_keys_literal()`, `session_exists()` — already used for orchestrator nudge |
| `src/config.rs` | `load_config()`, `resolve_db_path()` — used at startup |
| `src/cli.rs` | `Watch { interval, stall_threshold, daemon, stop }` variant — already defined |
| `src/main.rs` | `Watch` match arm — already wired |
| `src/commands/mod.rs` | `pub mod watch;` — already present |

### Existing Components — Modify

| Component | What to Add |
|-----------|-------------|
| `src/commands/watch.rs` | (1) Deadlock detection branch in `tick()` — `processing > 0 && no busy agents`; (2) Call `alert::send_telegram()` from stall/deadlock nudge paths; (3) Separate `NudgeState` instance for deadlock vs idle-inactivity cases |
| `src/db/messages.rs` | No new functions needed — `count_processing_all()` already exists |

### New Components

| Component | What It Does |
|-----------|--------------|
| `src/commands/alert.rs` | `send_telegram(message: &str) -> bool` — reads `SQUAD_TELEGRAM_TOKEN` + `SQUAD_TELEGRAM_CHAT_ID` env vars, shells out to `curl`, returns `false` silently if vars absent or curl fails |

**File count delta: 1 new file (`alert.rs`), 1 modified file (`watch.rs`).**

No new CLI subcommand. No schema migration. No new Cargo dependencies.

---

## Recommended Project Structure (delta view)

```
src/
├── commands/
│   ├── watch.rs         # MODIFY: add deadlock detection + telegram call
│   ├── alert.rs         # NEW: telegram dispatch (curl shell-out)
│   └── mod.rs           # MODIFY: add pub mod alert;
└── (all other files unchanged)
```

---

## Data Flow

### Watchdog Tick Flow (complete, v2.0)

```
tokio::time::sleep(interval_secs) → tick() called
    │
    ├── Pass 1: reconcile::reconcile_agents(pool, false)
    │       → for each busy agent: check tmux pane
    │           idle pane + busy DB → complete messages + notify orchestrator
    │           no tmux session → mark dead
    │       → log each action to .squad/log/watch.log
    │
    ├── Pass 2a: Global inactivity stall
    │       conditions: all_idle=true AND processing_count=0 AND idle_mins >= threshold
    │       → if nudge_state_idle.should_nudge(now):
    │           tmux::send_keys_literal(orch.name, "[SQUAD WATCHDOG] System idle for Xm...")
    │           alert::send_telegram("[SQUAD WATCHDOG] ...")  ← NEW
    │           nudge_state_idle.record_nudge(now)
    │       → log NUDGE to watch.log
    │
    ├── Pass 2b: Deadlock detection  ← NEW
    │       conditions: processing_count > 0 AND no agent is busy (all idle/dead)
    │       → if nudge_state_deadlock.should_nudge(now):
    │           tmux::send_keys_literal(orch.name, "[SQUAD WATCHDOG DEADLOCK] X tasks stuck...")
    │           alert::send_telegram("[SQUAD WATCHDOG DEADLOCK] ...")
    │           nudge_state_deadlock.record_nudge(now)
    │       → log DEADLOCK to watch.log
    │
    └── Pass 3: Prolonged busy detection
            → for each agent with status="busy": check busy_mins
                busy_mins > 30 → log WARN (no alert — informational only)
```

### Alert Dispatch Flow

```
nudge / deadlock condition met
    │
    ├── tmux::send_keys_literal(orch.name, msg)   [primary channel — always attempted]
    │       orch is antigravity OR session dead → skip silently
    │
    └── alert::send_telegram(msg)                 [secondary channel — optional]
            SQUAD_TELEGRAM_TOKEN absent → return false immediately (no-op)
            SQUAD_TELEGRAM_TOKEN present:
                curl POST to api.telegram.org/bot{TOKEN}/sendMessage
                    success → return true
                    failure → return false, log to watch.log ("telegram: send failed: ...")
```

### Daemon Lifecycle Flow

```
squad-station watch --daemon
    │
    ├── Check .squad/watch.pid: if exists and process alive → bail with error
    ├── std::process::Command::new(current_exe).arg("watch").arg("--interval")...spawn()
    ├── Write child.id() to .squad/watch.pid
    └── Return "Watchdog daemon started (PID X)"

squad-station watch --stop
    │
    ├── Read .squad/watch.pid
    ├── libc::kill(pid, 0) — check alive
    ├── libc::kill(pid, SIGTERM) — graceful shutdown
    └── Remove .squad/watch.pid
```

---

## Architectural Patterns Applied

### Pattern 1: Command-per-file with shared library functions

`watch.rs` calls `reconcile::reconcile_agents()` from `reconcile.rs` rather than duplicating the per-agent idle-detection logic. The `alert.rs` module follows the same command-per-file convention with `pub async fn send_telegram()` as its single public function. The convention is: one pub fn per module for the "run this feature" entry point.

### Pattern 2: Shell-out for external tools (no new dependencies)

The codebase consistently uses `tokio::process::Command` to shell out to `tmux` rather than a Rust tmux library. `alert.rs` follows this: `curl` for Telegram instead of `reqwest`. This keeps the binary footprint small and CI cross-compilation simple (musl targets don't need TLS library linking).

### Pattern 3: AtomicBool shutdown flag for signal-safe background loops

The watchdog uses `static AtomicBool SHUTDOWN` set by an `extern "C"` signal trampoline. This is correct for a `#[tokio::main]` async context where you need Unix signal handling inside a background process. The `browser` command uses `tokio::signal` (cleaner for foreground axum server). The watchdog predates the browser command and uses raw libc signals because it needs to work in daemon mode where tokio runtime lifecycle is more constrained.

### Pattern 4: Two separate NudgeState instances for distinct alert conditions

`NudgeState` is a simple struct tracking `count`, `last_nudge_at`, cooldown, and `max_nudges`. The same struct works for both inactivity nudges and deadlock alerts, but they must be separate instances — an inactivity nudge firing should not consume a slot in the deadlock counter, and vice versa. Both instances are local to `watch::run()` and passed as `&mut` into `tick()`.

### Pattern 5: "If available" graceful degradation for optional features

The Telegram alert channel returns `bool` and is called with `let _ = alert::send_telegram(msg).await;`. Failures are logged to `watch.log` but do not stop the watchdog tick. The primary alert channel (tmux injection) is always attempted first. This mirrors the existing pattern in `reconcile.rs` where orchestrator notification failure is ignored (`let _ = tmux::send_keys_literal(...).await;`).

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Adding `reqwest` for Telegram

**What people do:** Add `reqwest` as a Cargo dependency for HTTP calls to the Telegram Bot API.
**Why it's wrong:** Adds TLS stack (ring/aws-lc-rs) to the binary — significant size increase, cross-compilation complications for musl targets, CI matrix changes. The bot API call is simple enough for `curl` shell-out.
**Do this instead:** `tokio::process::Command::new("curl")` with JSON payload. Same pattern as the entire tmux integration layer.

### Anti-Pattern 2: Holding the write pool continuously in a tight loop

**What people do:** Open `db::connect()` once at the top of `watch::run()` and hold it across all ticks indefinitely.
**Why it's wrong:** This already causes `squad-station send` to block under the 5-second `busy_timeout` when the watchdog poll interval is short and reconcile writes overlap with user commands. At 30s intervals this is acceptable; at shorter intervals it becomes a problem.
**Do this instead:** The current watch.rs correctly opens one pool at startup and holds it — this is fine for 30s intervals. If a shorter interval becomes desirable, adopt the browser pattern: open a short-lived write pool per reconcile tick and close it immediately after.

### Anti-Pattern 3: Merging inactivity nudge and deadlock detection into one NudgeState

**What people do:** Use the same `NudgeState` for both "system idle too long" and "stuck processing messages" conditions, to reduce code.
**Why it's wrong:** A deadlock can happen immediately after a nudge-for-inactivity fires. The shared counter would suppress the deadlock alert. The two conditions are semantically distinct: inactivity is advisory (orchestrator may be done), deadlock is an error condition requiring immediate action.
**Do this instead:** Two named `NudgeState` instances — `nudge_state_idle` and `nudge_state_deadlock`. Pass both into `tick()`. Deadlock state resets when `processing_count` drops to zero; idle state resets when `total_count` changes.

### Anti-Pattern 4: Spawning tokio tasks for the watchdog poll loop

**What people do:** `tokio::spawn(async { loop { ... } })` to run the watchdog as a background task inside a long-lived tokio runtime.
**Why it's wrong:** The watchdog is designed as a standalone process (fork to background, PID file management, SIGTERM handling). Spawning inside the same tokio runtime couples its lifecycle to the parent process runtime, undermining the daemon model. The browser command uses task spawning correctly because it owns the runtime throughout its lifetime.
**Do this instead:** `while is_running() { tick().await; sleep().await; }` — the existing linear loop model. It is correct for a dedicated daemon process.

### Anti-Pattern 5: Sending Telegram alert for every prolonged-busy detection (Pass 3)

**What people do:** Also call `send_telegram()` in Pass 3 when an agent has been busy > 30 minutes.
**Why it's wrong:** Busy agents are normal. A 30-minute task is not unusual for complex coding work. Alerting on this would create noise every watchdog tick for any non-trivial task.
**Do this instead:** Pass 3 remains log-only (WARN to watch.log). Telegram alerts are reserved for inactivity stall (system stopped doing anything) and deadlock (tasks queued but no workers picking them up). These are action-required conditions; prolonged busy is informational.

---

## Integration Boundaries

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `commands/watch.rs` ↔ `commands/reconcile.rs` | `reconcile_agents(&pool, dry_run)` — pub fn | No change; already wired in v2.0 WIP |
| `commands/watch.rs` ↔ `commands/alert.rs` | `send_telegram(msg)` — new pub async fn | One-way; watch calls alert, not vice versa |
| `commands/watch.rs` ↔ `db/agents.rs` | `list_agents()`, `get_orchestrator()` | Already used; no new DB functions needed |
| `commands/watch.rs` ↔ `db/messages.rs` | `count_processing_all()`, `total_count()`, `last_activity_timestamp()` | Already used; no new DB functions needed |
| `commands/watch.rs` ↔ `tmux.rs` | `send_keys_literal()`, `session_exists()` | Already used; no changes to tmux.rs |
| `commands/alert.rs` ↔ `tokio::process::Command` | Shell-out to `curl` | System dependency; present on all target platforms |

### External Surfaces

| Surface | v2.0 Change | Notes |
|---------|-------------|-------|
| `.squad/watch.pid` | No change | Existing daemon management file |
| `.squad/log/watch.log` | Add DEADLOCK level entries | `log_watch(squad_dir, "DEADLOCK", ...)` alongside existing INFO/RECONCILE/NUDGE/STALL/WARN |
| Orchestrator tmux pane | New `[SQUAD WATCHDOG DEADLOCK]` message format | Additive; existing `[SQUAD WATCHDOG]` messages unchanged |
| Telegram Bot API | New — POST to `api.telegram.org` via curl | Only if `SQUAD_TELEGRAM_TOKEN` env var set; silent no-op otherwise |
| `squad.yml` / DB schema | No change | Watchdog is read-only relative to DB (reconcile.rs writes, not watch.rs directly) |
| CLI surface | No change | `watch` subcommand already defined with all needed flags |

---

## Build Order for v2.0 Completion

Dependencies flow from the least-coupled to the most-coupled component.

### Step 1: `src/commands/alert.rs` (no dependencies on other v2.0 work)

New file. Single pub async fn `send_telegram()`. Pure shell-out logic.
Add `pub mod alert;` to `src/commands/mod.rs`.
Write unit tests using a mock: test that absent env vars return false without spawning curl.

This step has zero dependencies on other v2.0 changes. It can be written, tested, and merged first.

### Step 2: Deadlock detection in `src/commands/watch.rs` tick()

Add second `NudgeState` instance for deadlock condition.
Add Pass 2b branch: `processing_count > 0 && no busy agents`.
Write integration tests using `setup_test_db()` (from `tests/helpers.rs`): insert processing messages, set all agents to idle, verify deadlock condition triggers.

Depends on: Step 1 (to call `send_telegram`) — but can be developed in parallel and wired together at merge.

### Step 3: Connect alert to both nudge paths

In `watch.rs`, add `let _ = alert::send_telegram(&msg).await;` after each `tmux::send_keys_literal()` call in Pass 2a (inactivity) and Pass 2b (deadlock).

Depends on: Steps 1 and 2 complete.

### Step 4: Update `squad-orchestrator.md` with watchdog instructions (optional)

Add a "Watchdog Alerts" section to `build_orchestrator_md()` in `context.rs` explaining the `[SQUAD WATCHDOG DEADLOCK]` message format and how the orchestrator should respond (check `squad-station status`, dispatch to idle agents).

Depends on: None (documentation-only change). Can be done alongside any step.

---

## Sources

- Direct source inspection: `src/commands/watch.rs`, `src/commands/browser.rs`, `src/commands/reconcile.rs`, `src/commands/mod.rs`, `src/db/agents.rs`, `src/db/messages.rs`, `src/tmux.rs`, `src/cli.rs`, `src/main.rs`, `Cargo.toml`
- Project decisions and constraints: `.planning/PROJECT.md` (v2.0 milestone section)
- Established patterns: daemon fork in `browser.rs::run_detached()`; AtomicBool shutdown in `watch.rs`; shell-out pattern throughout `tmux.rs` and `browser.rs`; NudgeState cooldown logic in `watch.rs`

---

*Architecture research for: Squad Station v2.0 — Workflow Watchdog integration with existing Rust CLI architecture*
*Researched: 2026-03-24*
