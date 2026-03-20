# Release Plan: Squad Station v0.6.0

**Codename:** Signal Reliability
**Status:** Draft
**Date:** 2026-03-20
**Previous version:** v0.5.8

## Problem Statement

In production multi-agent deployments (kindle-ai-export, competitoriq), agents get permanently stuck in `processing` status because completion signals are silently lost. The root cause is a fragile, single-point-of-failure notification chain with no self-recovery mechanism. The system has zero visibility into signal failures and zero ability to self-heal.

Observed failure: 4 tasks stuck on `kindle-implement` for 3+ hours. All real tasks after the first `/clear` cycle permanently orphaned. Manual intervention required.

## Root Cause Analysis

The current signal chain has 7 links, any of which can silently fail:

```
Claude Code Stop event
  → shell hook execution
    → tmux display-message (resolve agent name)
      → squad-station signal (Rust binary)
        → SQLite update (FIFO-based)
          → tmux send-keys (orchestrator notification)
            → orchestrator processes notification
```

Three root causes identified:

1. **FIFO signal queue race with /clear** — `/clear` triggers a Stop event in Claude Code. If a real task's DB record exists at that moment, the /clear signal prematurely completes it. Alternatively, /clear may disrupt internal hook state, preventing subsequent Stop events from firing hooks.

2. **Fragile agent name resolution** — `tmux display-message -p '#S'` is a client command that can fail silently in subprocess contexts. No fallback, no logging.

3. **No self-recovery** — When a signal is lost, the task stays `processing` forever. No reconciliation, no watchdog, no detection.

---

## Changes

### 1. Bug Fixes

#### 1.1 Signal uses `current_task` instead of FIFO queue
**Priority:** P0 — Root cause fix
**Files:** `src/commands/signal.rs`, `src/db/messages.rs`

Replace the FIFO-based `update_status` (which completes the oldest processing message) with `current_task`-targeted completion:

```
BEFORE: signal → find oldest processing message → complete it
AFTER:  signal → read agent.current_task → complete that specific message
```

The `current_task` field already exists on the `agents` table and is already maintained by `send.rs`. The signal should use it instead of guessing which message to complete.

**What this fixes:**
- /clear's Stop event cannot steal signals from real tasks (current_task is NULL or points to the /clear message which is already auto-completed)
- Multiple processing messages no longer create ambiguity
- Signal always completes the task that was actually assigned

**Implementation:**
- In `signal.rs`: replace `db::messages::update_status(&pool, &agent)` with `db::messages::complete_by_id(&pool, &agent_record.current_task)`
- Add `complete_by_id` function to `db/messages.rs`
- Guard: if `current_task` is NULL, signal is a noop (agent has no assigned task)
- Guard: if `current_task` message is already completed, signal is a noop (duplicate signal)
- After completion: check remaining processing messages, update `current_task` to next task or NULL

**Breaking change:** None. `update_status` is an internal function. External behavior (signal CLI) is unchanged.

#### 1.2 Fire-and-forget must not set `current_task`
**Priority:** P0 — Companion to 1.1
**Files:** `src/commands/send.rs`

Currently, `send.rs` line 68 sets `current_task` to the message ID for ALL messages, including fire-and-forget. Then lines 84-113 auto-complete and restore `current_task`. This creates a brief window where `current_task` points to a /clear message.

**Fix:** Move the `current_task` assignment AFTER the `is_fire_and_forget` check:

```rust
if is_fire_and_forget(&body) {
    // Auto-complete the message, do NOT touch current_task
    // ... existing auto-complete logic ...
} else {
    // Only set current_task for real tasks
    sqlx::query("UPDATE agents SET current_task = ? WHERE name = ?")
        .bind(&msg_id)
        .bind(&agent)
        .execute(&pool)
        .await?;
    db::agents::update_agent_status(&pool, &agent, "busy").await?;
}
```

#### 1.3 Retain `update_status` as fallback
**Priority:** P1
**Files:** `src/commands/signal.rs`

If `current_task` is NULL when a signal fires (edge case: task was sent but current_task wasn't set due to a race), fall back to the existing FIFO `update_status` behavior. Log a warning when this fallback is used.

```rust
if let Some(task_id) = &agent_record.current_task {
    complete_by_id(&pool, task_id).await?;
} else {
    log_warning("current_task is NULL, falling back to FIFO");
    update_status(&pool, &agent).await?;
}
```

---

### 2. Architecture Improvements

#### 2.1 `$SQUAD_AGENT_NAME` environment variable
**Priority:** P0 — Eliminates tmux resolution fragility
**Files:** `src/tmux.rs`, `src/commands/init.rs`, `src/commands/context.rs`

Set a `SQUAD_AGENT_NAME` environment variable in each agent's tmux session at launch time. Hook commands use this instead of `tmux display-message`.

**Agent launch change in `tmux.rs`:**
```rust
pub fn launch_agent(session_name: &str, command: &str) -> Result<()> {
    let wrapped = format!(
        "export SQUAD_AGENT_NAME='{}'; {}",
        session_name, command
    );
    let args = launch_args(session_name, &wrapped);
    // ...
}
```

**Process isolation guarantee:** Each tmux session is a separate process tree. `export` runs inside the session's shell before Claude Code starts. Claude Code inherits it. Hook subprocesses inherit it from Claude Code. Unix fork/exec guarantees per-process environment isolation. Two Claude Code instances on the same project cannot cross-contaminate.

**Generated `.claude/settings.json` hook:**
```json
{
  "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; [ -n \"$AGENT\" ] && squad-station signal \"$AGENT\" 2>>.squad/log/signal.log || true"
}
```

- Primary: `$SQUAD_AGENT_NAME` (set at launch, deterministic)
- Fallback: `$TMUX_PANE` + `list-panes` (server command, more reliable than `display-message`)
- Guard: `[ -n "$AGENT" ]` prevents running with empty agent name
- Logging: errors append to `.squad/log/signal.log`
- Exit: always 0 (never crash the provider)

#### 2.2 Hybrid push + pull signal model
**Priority:** P1 — Architectural safety net

The push path (Stop hook → signal) remains the fast path. A pull path (reconciliation) is added as a safety net that catches all missed signals regardless of cause.

```
PUSH (existing, best-effort, fast):
  Stop hook → squad-station signal → DB update → orchestrator notification

PULL (new, guaranteed, slow):
  Periodic check → detect idle pane + busy DB → auto-reconcile → notify
```

The pull path is implemented via the reconcile command (3.2) and watchdog daemon (3.3).

#### 2.3 Consolidated provider module
**Priority:** P1 — Preparatory refactor
**Files:** `src/providers.rs` (new)

Flat module with functions — no trait, no dynamic dispatch. Centralizes all provider-specific facts into a single file so other modules reference `providers::*` instead of hardcoding provider strings.

```rust
// src/providers.rs

/// Patterns that indicate the provider's TUI is idle and waiting for input.
pub fn idle_patterns(provider: &str) -> Option<&'static [&'static str]> {
    match provider {
        "claude-code" => Some(&["❯"]),
        "gemini-cli" => Some(&["Type your message"]),
        _ => None,
    }
}

/// Whether /clear triggers the completion hook (Stop/AfterAgent).
/// Claude Code: yes (Stop fires) — root cause of FIFO race.
/// Gemini CLI: no (AfterAgent does not fire on /clear).
pub fn clear_triggers_completion_hook(provider: &str) -> bool {
    match provider {
        "claude-code" => true,
        _ => false,
    }
}

/// Provider settings file path relative to project root.
pub fn settings_path(provider: &str) -> Option<&'static str> {
    match provider {
        "claude-code" => Some(".claude/settings.json"),
        "gemini-cli" => Some(".gemini/settings.json"),
        _ => None,
    }
}

/// Whether the provider uses an alternate screen buffer (full-screen TUI).
/// Affects tmux capture-pane strategy: need -a flag for alternate buffer.
pub fn uses_alternate_buffer(provider: &str) -> bool {
    match provider {
        "gemini-cli" => true,
        _ => false,
    }
}

/// Hook event name for task completion signal.
pub fn completion_hook_event(provider: &str) -> Option<&'static str> {
    match provider {
        "claude-code" => Some("Stop"),
        "gemini-cli" => Some("AfterAgent"),
        _ => None,
    }
}

/// Whether hook stdout must be valid JSON.
/// Gemini CLI golden rule: stdout must be JSON only.
pub fn hook_requires_json_stdout(provider: &str) -> bool {
    match provider {
        "gemini-cli" => true,
        _ => false,
    }
}

/// Commands that execute instantly without producing a provider response turn.
/// These never trigger the completion hook, so DB messages must be auto-completed.
pub fn fire_and_forget_prefixes(provider: &str) -> &'static [&'static str] {
    match provider {
        "claude-code" => &["/clear"],
        "gemini-cli" => &["/clear"],
        _ => &["/clear"],  // safe default
    }
}
```

Other modules consume this instead of hardcoding:
- `reconcile.rs` / `watch.rs`: `providers::idle_patterns(agent.tool)` and `providers::uses_alternate_buffer(agent.tool)`
- `init.rs`: `providers::settings_path(provider)`, `providers::completion_hook_event(provider)`, `providers::hook_requires_json_stdout(provider)`
- `send.rs`: `providers::fire_and_forget_prefixes(provider)` (future: make `is_fire_and_forget` provider-aware)
- `signal.rs`: `providers::clear_triggers_completion_hook(provider)` (for logging/diagnostics)

**v0.7.0 consideration:** If a 3rd provider is added (e.g., Cursor, Windsurf, aider), refactor `providers.rs` from flat functions to a `Provider` trait with per-provider modules (`src/providers/claude_code.rs`, `src/providers/gemini_cli.rs`). Two providers don't justify a trait; three do.

---

### 3. New Features

#### 3.1 Project-scoped signal logging
**Priority:** P0 — Makes failures visible
**Files:** `src/commands/signal.rs`, `src/config.rs`

Add structured logging to `.squad/log/signal.log` inside each project directory. The signal command already resolves the project root via `find_project_root()`.

**What gets logged:**
```
2026-03-20T10:15:32Z OK    agent=kindle-implement task=82b47c49 rows=1 notified=true
2026-03-20T10:15:35Z OK    agent=kindle-implement task=none rows=0 notified=false reason=no_pending_task
2026-03-20T10:15:40Z GUARD agent=kindle-implement reason=config_not_found cwd=/Users/x/tmp
2026-03-20T10:15:45Z GUARD agent=kindle-orchestrator reason=orchestrator_self_signal
```

Every guard exit that currently does `return Ok(())` silently will now log the reason. This is the single most important change for debuggability.

**Log rotation:** Truncate to last 500 lines when file exceeds 1MB. Checked once per signal invocation.

**Implementation:**
```rust
fn log_signal(project_root: &Path, level: &str, agent: &str, msg: &str) {
    let log_dir = project_root.join(".squad").join("log");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("signal.log");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_file) {
        let _ = writeln!(f, "{} {:<5} agent={} {}",
            Utc::now().to_rfc3339(), level, agent, msg);
    }
}
```

Logging is best-effort (ignores write failures). It must never cause the signal command to fail.

#### 3.2 `squad-station reconcile` command
**Priority:** P1 — Self-healing for missed signals
**Files:** `src/commands/reconcile.rs` (new), `src/cli.rs`, `src/main.rs`

Standalone command that detects and fixes stuck agents:

```bash
squad-station reconcile              # One-shot reconciliation
squad-station reconcile --dry-run    # Show what would be fixed without changing DB
squad-station reconcile --json       # Machine-readable output
```

**Algorithm:**
```
for each agent where status = 'busy':
    stale_minutes = now - status_updated_at
    if stale_minutes < 2: skip  (probably still working)

    pane_state = capture last 3 lines of tmux pane
    if pane shows idle prompt (❯ or >):
        complete all processing messages for this agent
        set current_task = NULL
        set status = idle
        notify orchestrator: "[SQUAD RECONCILE] agent completed (signal was lost)"
        log: "RECONCILE agent=X tasks=N reason=idle_pane"
    elif pane does not exist:
        set status = dead
        log: "DEAD agent=X reason=no_tmux_session"
    else:
        skip  (pane shows active output, agent is working)
```

**Idle prompt detection (provider-aware):**
```rust
fn pane_looks_idle(session_name: &str) -> bool {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p", "-l", "3"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let last_line = text.lines().rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");
            // Claude Code: ❯    Gemini CLI: >
            last_line.contains("❯") || last_line.trim().ends_with(">")
        }
        _ => false,  // Can't determine — assume not idle (safe default)
    }
}
```

**Embedded reconciliation (zero-config):**

Also integrate reconciliation as a side-effect in existing commands:

- `squad-station status` — reconcile stale agents on each status check
- `squad-station send` — pre-flight reconcile before creating new message

This ensures reconciliation happens even without explicit `reconcile` calls or a running watchdog.

#### 3.3 `squad-station watch` — Watchdog daemon
**Priority:** P2 — Unconditional liveness guarantee
**Files:** `src/commands/watch.rs` (new), `src/cli.rs`, `src/main.rs`

Persistent background process that polls and auto-repairs:

```bash
squad-station watch                  # Foreground (for tmux pane / terminal tab)
squad-station watch --interval 30    # Custom poll interval in seconds (default: 30)
squad-station watch --daemon         # Fork to background, write PID to .squad/watch.pid
squad-station watch --stop           # Kill running daemon via PID file
```

**Each tick runs three detection passes:**

**Pass 1 — Individual agent reconciliation (Class 1 + Class 2):**
Same algorithm as `squad-station reconcile` (section 3.2). Detects agents that are `busy` in DB but idle in their tmux pane — meaning the Stop hook signal was lost or the task was never delivered.

**Pass 2 — Global stall detection with orchestrator nudge (Class 3):**

Detects the exact deadlock pattern from the kindle incident: all agents idle, orchestrator idle, no work happening, but work should be happening because a signal notification was lost.

```
global_stall_check:
    all_agents_idle = every non-dead agent has status = 'idle'
    no_processing    = count(messages WHERE status = 'processing') == 0
    orch_idle        = orchestrator tmux pane shows idle prompt (❯ or >)
    last_activity    = max(messages.updated_at)
    idle_duration    = now - last_activity

    if all_agents_idle AND no_processing AND orch_idle
       AND idle_duration > STALL_THRESHOLD:
        → GLOBAL STALL detected
        → Nudge orchestrator
```

`STALL_THRESHOLD` default: 5 minutes. Configurable via `--stall-threshold` flag.

**Orchestrator nudge mechanism:**
```rust
fn nudge_orchestrator(pool: &SqlitePool, orch_session: &str, idle_minutes: u64) -> Result<()> {
    // Build context-rich nudge message
    let last_completed = db::messages::last_completed(pool).await?;
    let agent_summary = db::agents::idle_summary(pool).await?;

    let message = format!(
        "[SQUAD WATCHDOG] System idle for {}m — all agents idle, no pending tasks.\n\
         Last completed: {} ({}m ago)\n\
         Agents: {}\n\
         Action: Run 'squad-station status' to review and dispatch next work.",
        idle_minutes,
        last_completed.task.chars().take(60).collect::<String>(),
        idle_minutes,
        agent_summary,
    );

    tmux::send_keys_literal(orch_session, &message)?;
    Ok(())
}
```

**Nudge cooldown and escalation:**

| Nudge # | Timing | Message |
|---------|--------|---------|
| 1st | After `STALL_THRESHOLD` (5m) | Standard nudge with status summary |
| 2nd | +10 minutes after 1st | Escalation: "System still idle after nudge" |
| 3rd | +10 minutes after 2nd | Final: "Watchdog stopping nudges — manual review required" |
| Stop | After 3rd | No more nudges. Log `STALL_UNRESOLVED`. Avoid infinite nudge loops. |

Nudge counter resets to 0 when any new message activity is detected (a task is sent or completed).

```rust
struct NudgeState {
    count: u32,
    last_nudge_at: Option<DateTime<Utc>>,
    cooldown_secs: u64,  // 600 (10 minutes)
    max_nudges: u32,     // 3
}

impl NudgeState {
    fn should_nudge(&self, now: DateTime<Utc>) -> bool {
        if self.count >= self.max_nudges {
            return false;  // Stop after max nudges
        }
        match self.last_nudge_at {
            None => true,  // First nudge
            Some(last) => (now - last).num_seconds() > self.cooldown_secs as i64,
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_nudge_at = None;
    }
}
```

**Pass 3 — Prolonged busy detection (stagnation warning):**
If an agent has been `busy` for >30 minutes AND its tmux pane shows active output (not idle), log a warning. This catches stalled AI responses or infinite loops without taking automated action (the agent might legitimately be doing long work).

```
2026-03-20T10:45:00Z WARN  agent=kindle-implement busy_minutes=45 pane=active reason=prolonged_busy
```

**Daemon lifecycle:**
- `squad-station init` starts watchdog automatically (with `--daemon`)
- PID written to `.squad/watch.pid`
- `squad-station watch --stop` or `squad-station clean` kills it
- Graceful shutdown on SIGTERM/SIGINT
- Duplicate launch prevention: checks existing PID file, refuses to start if daemon is already running

**CLI:**
```bash
squad-station watch                     # Foreground mode
squad-station watch --interval 30       # Custom poll interval (default: 30s)
squad-station watch --stall-threshold 5 # Minutes before global stall nudge (default: 5)
squad-station watch --daemon            # Fork to background
squad-station watch --stop              # Kill running daemon via PID file
```

**Resource overhead:** One SQLite read + N tmux captures per tick. At 30s interval: negligible CPU, <1MB memory.

**Deadlock detection coverage:**

| Class | Failure | Detection | Recovery | Automated? |
|-------|---------|-----------|----------|------------|
| 1 | Lost signal | busy DB + idle pane | Reconcile: complete + notify | Yes |
| 2 | Lost delivery | busy DB + idle pane | Reconcile: complete + notify with context | Yes |
| 3 | Lost notification | all idle + stall timer | Nudge orchestrator (3x with escalation) | Yes |
| 4 | Circular wait | busy DB + busy pane + no output change | Log warning (future: P3) | Alert only |

#### 3.4 `clean` and `reset` — Log preservation and watchdog lifecycle
**Priority:** P1 — Lifecycle correctness
**Files:** `src/commands/clean.rs`, `src/cli.rs`

**Design principle:** Logs are evidence, not runtime state. They must survive `clean` and `reset` by default.

**`.squad/` directory structure after v0.6.0:**
```
.squad/
  station.db          ← runtime state (deleted by clean)
  watch.pid           ← watchdog PID file (deleted by clean)
  log/
    signal.log        ← signal audit trail (preserved by clean)
    watch.log         ← watchdog audit trail (preserved by clean)
```

**What `clean` deletes by default vs `--all`:**

| Asset | `clean` | `clean --all` | Rationale |
|-------|---------|---------------|-----------|
| tmux sessions | Kill | Kill | Runtime process |
| watchdog daemon | Stop (via PID) | Stop (via PID) | Runtime process |
| `station.db` | Delete | Delete | Runtime state |
| `watch.pid` | Delete | Delete | Stale after daemon stop |
| `.squad/log/` | **Preserve** | Delete | Evidence for post-mortem |

**Watchdog shutdown during clean is mandatory, not optional:**
A running watchdog with a deleted DB would either crash on every tick or recreate DB state that shouldn't exist. Clean must stop the watchdog BEFORE deleting the DB.

**Shutdown sequence in `clean`:**
```
1. Stop watchdog daemon (read .squad/watch.pid → kill PID → delete PID file)
2. Kill all tmux sessions (existing behavior)
3. Delete station.db (existing behavior)
4. If --all: delete .squad/log/ directory
```

**CLI changes:**
```bash
squad-station clean                # Kill sessions + stop watchdog + delete DB (logs preserved)
squad-station clean --all          # Same + delete .squad/log/
squad-station clean -y             # Skip confirmation (existing flag)
squad-station clean --all -y       # Full teardown without confirmation
```

**Updated confirmation prompt:**
```
Kill all squad sessions, stop watchdog, and delete station.db? [y/N]:
```
With `--all`:
```
Kill all squad sessions, stop watchdog, delete station.db AND logs? [y/N]:
```

**`reset` behavior:** `reset` calls `clean` internally then re-inits. It inherits `clean`'s log preservation — logs from the failed run survive the reset cycle, allowing comparison of pre-reset and post-reset signal behavior.

**JSON output update:**
```json
{
  "project": "kindle",
  "killed": 4,
  "watchdog_stopped": true,
  "db_deleted": true,
  "logs_deleted": false
}
```

---

### 4. Hook System Improvements

#### 4.1 Updated `.claude/settings.json` template
**Priority:** P0 — Part of init changes
**Files:** `src/commands/init.rs`

Generated during `squad-station init`:

```json
{
  "hooks": {
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; [ -n \"$AGENT\" ] && squad-station signal \"$AGENT\" 2>>.squad/log/signal.log || true"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; [ -n \"$AGENT\" ] && squad-station notify --body 'Agent needs input' --agent \"$AGENT\" || true"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; [ -n \"$AGENT\" ] && squad-station notify --body 'Agent needs input' --agent \"$AGENT\" || true"
          }
        ]
      }
    ]
  }
}
```

#### 4.2 Updated `.gemini/settings.json` template
**Priority:** P0
**Files:** `src/commands/init.rs`

Generated during `squad-station init` for Gemini CLI agents:

```json
{
  "hooks": {
    "AfterAgent": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; squad-station signal \"${AGENT:-__none__}\" >>.squad/log/signal.log 2>&1; printf '{}'",
            "name": "squad-signal",
            "description": "Signal task completion to squad-station",
            "timeout": 30000
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t \"$TMUX_PANE\" -F '#S' 2>/dev/null | head -1)}; squad-station notify --body 'Agent needs input' --agent \"${AGENT:-__none__}\" >>.squad/log/signal.log 2>&1; printf '{}'",
            "name": "squad-notify",
            "description": "Forward permission prompt to orchestrator",
            "timeout": 30000
          }
        ]
      }
    ]
  }
}
```

**Critical differences from Claude Code template:**

| Aspect | Claude Code hook | Gemini CLI hook |
|--------|-----------------|----------------|
| Completion event | `Stop` | `AfterAgent` |
| Stdout requirement | Ignored (can be plain text) | **MUST be valid JSON** (golden rule) |
| Stdout handling | `squad-station signal` output goes to terminal | All output redirected to log; `printf '{}'` outputs empty JSON |
| Notification matching | `"permission_prompt"` (specific) | `""` (match all — Gemini uses different type identifiers) |
| PostToolUse/AfterTool | `AskUserQuestion` matcher | Not used — no equivalent selective matcher |
| Optional fields | None | `name`, `description`, `timeout` |
| Exit code 2 meaning | Blocking error (stops) | **Automatic retry** (catastrophic) |

**The JSON stdout rule is the most important Gemini CLI difference.** Claude Code ignores hook stdout (or treats it as optional). Gemini CLI **requires** valid JSON on stdout and interprets it as hook response instructions. Plain text stdout causes parse errors.

The Gemini CLI hook command pattern:
```bash
# 1. Resolve agent name (same as Claude Code)
AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -1)}

# 2. Run signal, redirect ALL output (stdout+stderr) to log file
squad-station signal "${AGENT:-__none__}" >>.squad/log/signal.log 2>&1

# 3. Output valid JSON to stdout (Gemini CLI requirement)
# Empty object {} = "no special instructions" — Gemini continues normally
printf '{}'
```

Why `${AGENT:-__none__}` instead of `[ -n "$AGENT" ] && ...`: The conditional `&&` pattern from the Claude Code hook would skip `printf '{}'` if the test fails (due to shell short-circuit). Using `${AGENT:-__none__}` ensures the signal command always runs (with a dummy name that hits GUARD 3 in signal.rs), the log file captures the attempt, and `printf '{}'` always executes.

**AfterAgent fires on every response turn:** This is the same behavior as Claude Code's Stop — both fire once per AI response, not just on task completion. The signal command is idempotent: if `current_task` is NULL or already completed, signal is a noop. No architectural concern.

---

## Provider Compatibility Matrix

### v0.6.0 changes by provider

| Change | Claude Code | Gemini CLI | Notes |
|--------|------------|-----------|-------|
| **1.1** Signal uses `current_task` | Fixes /clear race | Correct but less urgent | Gemini /clear doesn't trigger AfterAgent |
| **1.2** Fire-and-forget skips `current_task` | Critical fix | Housekeeping | Gemini /clear = SessionStart, not AfterAgent |
| **1.3** FIFO fallback | Both | Both | Provider-agnostic DB logic |
| **2.1** `$SQUAD_AGENT_NAME` | Works | Works | Both inherit parent env in hook subprocesses |
| **3.1** Signal logging | Both | Both | Provider-agnostic |
| **3.2** Reconcile | Both | Both | Provider-aware idle detection (see below) |
| **3.3** Watchdog | Both | Both | Provider-aware idle detection (see below) |
| **3.4** Clean/reset | Both | Both | Provider-agnostic |
| **4.1** Hook template | `.claude/settings.json` | — | Claude-specific |
| **4.2** Hook template | — | `.gemini/settings.json` | Gemini-specific (JSON stdout, AfterAgent, name/timeout fields) |

### Hook system comparison

| Aspect | Claude Code | Gemini CLI |
|--------|------------|-----------|
| **Completion event** | `Stop` | `AfterAgent` |
| **When it fires** | After each AI response | After each agent turn (all tools + final response) |
| **Stdin** | JSON with session context | JSON with `prompt`, `prompt_response`, `stop_hook_active` |
| **Stdout requirement** | Optional (ignored if not JSON) | **Mandatory valid JSON** — no plain text allowed |
| **Stdout semantics** | None | `{}` = continue; `{"decision":"deny"}` = retry turn |
| **Stderr** | Warning output | Log/debug output only |
| **Exit 0** | Success | Success, parse stdout as JSON |
| **Exit 2** | Blocking error (stops) | **Triggers automatic retry** (agent re-runs entire turn) |
| **Exit other** | Non-blocking error | Warning, continue |
| **Notification events** | `permission_prompt`, `elicitation_dialog` | Generic (match all with `""`) |
| **Tool completion** | `PostToolUse` (selective: `AskUserQuestion`) | `AfterTool` (fires for ALL tools, no selective matching) |
| **Context injection** | `SessionStart` | `SessionStart` (also fires after /clear with `source: "clear"`) |
| **Config merging** | All levels merged, hooks run in parallel | Project takes precedence over user-level |
| **Extra hook fields** | None | `name`, `description`, `timeout` (ms), `sequential` (bool) |
| **Available events** | Stop, Notification, PostToolUse, SessionStart, PreToolUse | SessionStart, SessionEnd, BeforeAgent, AfterAgent, BeforeTool, AfterTool, BeforeModel, AfterModel, BeforeToolSelection, Notification, PreCompress |

### Critical provider difference: /clear behavior

| Behavior | Claude Code | Gemini CLI |
|----------|------------|-----------|
| `/clear` triggers completion hook? | **YES** (`Stop` fires) | **NO** (`AfterAgent` does not fire) |
| `/clear` triggers other hooks? | Unclear (may disrupt hook state) | `SessionEnd` (reason: "clear") then `SessionStart` (source: "clear") |
| `/clear` race condition exists? | **YES** — root cause of kindle deadlock | **NO** — architecturally immune |
| `is_fire_and_forget` needed? | Critical (prevents FIFO queue blocking) | Housekeeping only (keeps DB clean) |
| Context re-injection after /clear? | Depends on SessionStart hook | Automatic — SessionStart fires with source "clear" |

### Environment variable inheritance

| Aspect | Claude Code | Gemini CLI |
|--------|------------|-----------|
| Parent env inherited by hooks? | Yes | Yes (docs confirm) |
| `$SQUAD_AGENT_NAME` safe? | Yes | Yes (not matched by redaction patterns KEY/TOKEN/SECRET) |
| Provider-injected vars | `CLAUDE_PROJECT_DIR` | `GEMINI_PROJECT_DIR`, `GEMINI_SESSION_ID`, `GEMINI_CWD` |
| Env redaction | None | Optional system (OFF by default), strips vars matching KEY/TOKEN/SECRET patterns |

### Hook command pattern comparison

**Claude Code** — stdout is ignored, errors to log:
```bash
AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -1)}
[ -n "$AGENT" ] && squad-station signal "$AGENT" 2>>.squad/log/signal.log || true
```

**Gemini CLI** — stdout MUST be valid JSON, all signal output to log:
```bash
AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -1)}
squad-station signal "${AGENT:-__none__}" >>.squad/log/signal.log 2>&1
printf '{}'
```

Key differences in the Gemini CLI command:
1. **`>>.squad/log/signal.log 2>&1`** — redirects BOTH stdout and stderr from `squad-station signal` to the log file. Nothing from signal reaches Gemini CLI's stdout parser.
2. **`printf '{}'`** — outputs exactly `{}` (valid JSON empty object) as the last thing on stdout. Gemini CLI parses this as "no special instructions, continue normally."
3. **`${AGENT:-__none__}`** instead of `[ -n ] && ... || true` — avoids shell short-circuit that would skip `printf '{}'`. The `__none__` dummy name harmlessly hits GUARD 3 (agent not found) in signal.rs.
4. **No `|| true` needed** — `printf '{}'` always succeeds and is the last command, so exit code is 0. If signal fails, the semicolon continues to printf regardless.

### Idle prompt detection (provider-aware)

The reconcile and watchdog idle detection must be provider-aware. Each provider has a different prompt pattern and terminal mode:

| Provider | Idle prompt | Terminal mode | Detection strategy |
|----------|------------|--------------|-------------------|
| Claude Code | `❯` (Unicode) | Normal scrollback | Match `❯` in last non-empty line |
| Gemini CLI | `> Type your message` | **Alternate screen buffer** | Match `Type your message` in captured output; fall back to alternate buffer capture |

**Implementation:**
```rust
/// Detect if an agent's tmux pane shows an idle prompt.
/// Provider-aware: each provider has different prompt patterns and terminal modes.
fn pane_looks_idle(session_name: &str, provider: &str) -> bool {
    // First attempt: standard capture
    let text = capture_pane(session_name);

    // If capture is empty, try alternate screen buffer (Gemini CLI uses full-screen TUI)
    let text = if text.trim().is_empty() {
        capture_pane_alternate(session_name)
    } else {
        text
    };

    let last_line = text.lines().rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");

    match provider {
        "claude-code" => last_line.contains('❯'),
        "gemini-cli" => {
            let t = last_line.trim();
            // "Type your message" is the definitive Gemini CLI idle indicator
            // Bare ">" alone is NOT sufficient (too many false positives)
            t.contains("Type your message") || t == ">"  && text.contains("gemini")
        }
        _ => false, // Unknown provider: cannot detect idle (safe default — skip reconcile)
    }
}

fn capture_pane(session: &str) -> String {
    Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p", "-l", "5"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Capture from alternate screen buffer (for full-screen TUI apps like Gemini CLI)
fn capture_pane_alternate(session: &str) -> String {
    Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p", "-a", "-l", "5"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}
```

**The `-a` flag** tells tmux to capture the alternate screen buffer instead of the normal scrollback. This is required for Gemini CLI because its TUI renders in the alternate buffer.

**The agent record already stores the provider** in the `tool` column (`agents.tool`), so the reconcile/watchdog can look up the provider for each agent and call `pane_looks_idle` with the correct provider string.

---

## Implementation Priority

### Wave 1: Root Cause Fixes (P0)
Must ship together. Fixes the actual bug.

| # | Change | Files | Est. Complexity |
|---|--------|-------|-----------------|
| 1.1 | Signal uses `current_task` | signal.rs, messages.rs | Medium |
| 1.2 | Fire-and-forget skips `current_task` | send.rs | Small |
| 2.1 | `$SQUAD_AGENT_NAME` in tmux launch | tmux.rs, init.rs | Small |
| 3.1 | Project-scoped signal logging | signal.rs | Small |
| 4.1 | Updated hook templates | init.rs | Small |

### Wave 2: Safety Net (P1)
Defense in depth. Catches any remaining edge cases.

| # | Change | Files | Est. Complexity |
|---|--------|-------|-----------------|
| 1.3 | FIFO fallback in signal | signal.rs | Small |
| 2.3 | Consolidated `providers.rs` module | providers.rs (new), init.rs, reconcile.rs | Small |
| 3.2 | `squad-station reconcile` | reconcile.rs (new), cli.rs | Medium |
| 3.4 | `clean`/`reset` log preservation + watchdog shutdown | clean.rs, cli.rs | Small |
| — | Embed reconcile in status/send | status.rs, send.rs | Small |

### Wave 3: Unconditional Liveness (P2)
Insurance layer. Operates independently of all other commands.

| # | Change | Files | Est. Complexity |
|---|--------|-------|-----------------|
| 3.3 | `squad-station watch` daemon | watch.rs (new), cli.rs | Medium |
| — | Auto-start watchdog in init | init.rs | Small |

---

## Breaking Changes

**None.** All changes are backward-compatible:

- `squad-station signal <agent>` — same CLI interface, different internal completion strategy
- `squad-station send` — same CLI interface, fire-and-forget behavior refined internally
- `.claude/settings.json` — existing projects continue to work. New hook format is generated on next `init`. Old format (`tmux display-message`) still works but is less reliable.
- `$SQUAD_AGENT_NAME` — optional enhancement. If unset, hooks fall back to tmux resolution.
- New commands (`reconcile`, `watch`) are additive.

## Migration for Existing Projects

No migration required. To opt into the new hook format for an already-running project:

```bash
# 1. Set env var in existing sessions (takes effect after Claude Code restart)
tmux set-environment -t kindle-implement SQUAD_AGENT_NAME kindle-implement
tmux set-environment -t kindle-brainstorm SQUAD_AGENT_NAME kindle-brainstorm
tmux set-environment -t kindle-orchestrator SQUAD_AGENT_NAME kindle-orchestrator

# 2. Update .claude/settings.json with new hook command (from section 4.1)

# 3. Restart Claude Code in each agent session (/exit then relaunch)

# OR: just re-run squad-station init (regenerates everything)
```

## Test Plan

### Unit Tests
- `test_complete_by_id` — new function completes specific message by ID
- `test_signal_uses_current_task` — signal completes current_task, not FIFO
- `test_signal_current_task_null` — signal noops when current_task is NULL
- `test_signal_current_task_already_completed` — duplicate signal is idempotent
- `test_fire_and_forget_does_not_set_current_task` — /clear leaves current_task unchanged
- `test_reconcile_detects_idle_agent` — busy agent with idle pane gets reconciled
- `test_reconcile_skips_active_agent` — busy agent with active pane is not touched
- `test_reconcile_marks_dead_agent` — missing tmux session → status dead
- `test_signal_logging` — log entries written to .squad/log/signal.log
- `test_pane_idle_claude_code` — detects `❯` as idle for claude-code provider
- `test_pane_idle_gemini_cli` — detects `Type your message` as idle for gemini-cli provider
- `test_pane_idle_rejects_bare_gt` — bare `>` in non-gemini context is NOT idle
- `test_pane_idle_unknown_provider` — unknown provider returns false (safe default)
- `test_install_gemini_hooks_json_stdout` — Gemini hook command ends with `printf '{}'`
- `test_install_gemini_hooks_no_plain_stdout` — Gemini hook redirects signal stdout to log, not terminal
- `test_install_claude_hooks_no_json_requirement` — Claude hook does NOT add `printf '{}'` (not needed)
- `test_gemini_hook_uses_afteragent` — Gemini template uses AfterAgent, not Stop
- `test_gemini_hook_has_name_and_timeout` — Gemini template includes name, description, timeout fields
- `test_nudge_state_cooldown` — nudge fires once, then respects cooldown timer
- `test_nudge_state_max_nudges` — stops nudging after 3 attempts
- `test_nudge_state_reset_on_activity` — nudge counter resets when new message activity detected
- `test_watchdog_pid_lock` — refuses to start if daemon already running (PID alive)

### Integration Tests
- `test_clear_then_task_signal_flow` — send /clear, send task, verify signal completes correct task
- `test_multiple_processing_tasks_signal_order` — verify current_task-based completion is correct
- `test_reconcile_embedded_in_status` — status command reconciles stale agent as side-effect
- `test_reconcile_embedded_in_send` — send command reconciles before creating new message
- `test_clean_preserves_logs` — clean deletes DB and PID but `.squad/log/` survives
- `test_clean_all_deletes_logs` — `clean --all` removes `.squad/log/` directory
- `test_clean_stops_watchdog` — clean reads PID file and kills watchdog process before DB deletion
- `test_reset_preserves_logs` — reset cycle preserves logs across clean + re-init

### Integration Tests (watchdog)
- `test_global_stall_detection` — all agents idle + no processing tasks + idle > threshold → stall detected
- `test_global_stall_not_triggered_during_work` — busy agents prevent stall detection
- `test_nudge_sends_to_orchestrator` — stall triggers tmux send-keys to orchestrator session
- `test_nudge_escalation_sequence` — 1st nudge → cooldown → 2nd nudge → cooldown → 3rd nudge → stop

### Manual E2E Tests
- Verify `$SQUAD_AGENT_NAME` propagates through tmux → Claude Code → hook subprocess
- Verify `$SQUAD_AGENT_NAME` propagates through tmux → Gemini CLI → hook subprocess
- Verify Gemini CLI hook outputs valid JSON to stdout (capture hook output, parse as JSON)
- Verify Gemini CLI hook does NOT output plain text to stdout (would cause parse error)
- Run two agents on same project, verify no env cross-contamination
- Kill a hook mid-execution, verify watchdog catches the stuck task
- Run full orchestrator cycle with /clear between tasks, verify zero stuck tasks (Claude Code)
- Verify Gemini CLI /clear does NOT trigger AfterAgent (confirm no /clear race for Gemini agents)
- Simulate kindle deadlock: complete agent task, block orchestrator notification, verify watchdog nudges orchestrator within 5 minutes
- Verify Gemini CLI alternate buffer capture: reconcile correctly detects idle Gemini CLI agent

## Success Criteria

1. Zero stuck `processing` tasks after a 50-task orchestrator session with frequent `/clear` usage
2. Signal log shows every signal invocation with outcome (OK/GUARD/FALLBACK)
3. Watchdog detects and repairs stuck agents within 60 seconds (2 poll cycles)
4. No manual intervention required for any signal-related failure
5. All fixes work for both Claude Code and Gemini CLI agents in the same project
6. Idle detection correctly identifies idle prompts for both providers (including Gemini CLI alternate buffer)
