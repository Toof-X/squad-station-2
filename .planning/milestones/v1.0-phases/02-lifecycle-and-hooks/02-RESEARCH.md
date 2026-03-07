# Phase 02: Lifecycle and Hooks - Research

**Researched:** 2026-03-06
**Domain:** Rust CLI lifecycle management, shell hook scripts, tmux session detection, SQLite migrations
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Hook delivery method:** Bundled shell scripts that wrap `squad-station signal`. Users reference these scripts from provider hook config (Claude Code hooks.json, Gemini CLI settings). Scripts contain the 4-layer guard logic (HOOK-03): not-in-tmux check, agent-registered check, orchestrator-skip check, then signal call.
- **Hook edge case behavior:**
  - Unregistered agent: silent exit 0 (agent might be outside the squad)
  - Not in tmux: silent exit 0 (can't be a managed agent)
  - Real errors (binary not found, DB connection failure): stderr warning + exit 0 (debuggable but never fails the provider)
  - Orchestrator self-signal (HOOK-01): silently exit 0 to prevent infinite loop
- **Context file design:**
  - Output format: Markdown structured for pasting into AI orchestrator prompts
  - Output destination: stdout only (user pipes/redirects as needed)
  - Content: agent roster (name, role, current status, per-agent usage commands) plus general squad-station usage guide
  - Self-contained: orchestrator should understand the full system from this file alone
- **Status reconciliation:**
  - Agent status includes duration (e.g., "idle 5m", "busy 2m", "dead since 10:30")
  - Dead agents auto-revive to idle when their tmux session reappears on next reconciliation

### Claude's Discretion
- Hook script location (repo root `hooks/` dir vs generated at init time)
- Single universal hook script vs separate per-provider scripts — based on how different the provider interfaces actually are
- Guard logic placement: shell script vs Rust binary — based on testability and reliability
- Orchestrator detection method: tmux session name check vs DB role lookup
- Status reconciliation timing: on every read command (eager) vs on agents command only (lazy)
- Agent status DB model: dedicated status column vs derived from messages + tmux state

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SESS-03 | Station tracks agent status as idle, busy, or dead based on current activity | Status column migration, reconciliation logic in `agents` command, chrono for duration calculation |
| SESS-04 | Station reconciles agent liveness by checking tmux session existence | `tmux::session_exists()` already exists; reconciliation = call it for each agent in `list_agents()` loop |
| SESS-05 | Station auto-generates orchestrator context file listing available agents and usage commands | New `Context` subcommand, stdout Markdown rendering, `list_agents()` + status join |
| HOOK-01 | Signal command skips orchestrator sessions (role=orchestrator) to prevent infinite loop | `get_orchestrator()` exists; guard: if `get_agent(name).role == "orchestrator"` then exit 0 |
| HOOK-02 | Hook scripts work for both Claude Code (Stop event) and Gemini CLI (AfterAgent event) | Provider interface schemas verified — see Architecture Patterns section |
| HOOK-03 | Hook gracefully exits when not in tmux or agent not registered (4-layer guard) | `$TMUX_PANE` env var for tmux detection; `squad-station signal` exit codes for guard ordering |
</phase_requirements>

## Summary

Phase 2 extends an already solid Phase 1 Rust binary with three categories of work: (1) agent status tracking via a new DB column and reconciliation against live tmux state, (2) hook scripts for Claude Code and Gemini CLI that call `squad-station signal` with 4-layer guards, and (3) two new CLI subcommands (`agents` and `context`).

The codebase is well-structured for this work. `tmux::session_exists()` already exists, `db::agents::list_agents()` and `db::agents::get_orchestrator()` are ready to use, and the existing test infrastructure (`setup_test_db()`, `tokio::test`, `tempfile`) requires no additions. The main new surfaces are: a SQLite migration adding a `status` column and `status_updated_at` timestamp to the agents table, two new `async fn run()` command files, and two shell scripts under `hooks/`.

The primary design decision for Claude's discretion is **guard logic placement**: placing guards in the Rust binary (rather than pure shell) makes them unit-testable and reliable. The shell scripts become thin wrappers that just pass the agent name and exit. This is the recommended approach given the established pattern of extracting testable logic into Rust (see `tmux.rs` arg-builder helpers).

**Primary recommendation:** Add status reconciliation as part of the `agents` command only (lazy), use a dedicated `status` + `status_updated_at` column in the agents table, implement guards in the Rust `signal` command, and ship two thin shell scripts (one per provider) in `hooks/`.

## Standard Stack

### Core (all already in Cargo.toml — no new dependencies needed)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sqlx | 0.8 | Status column migration, status queries | Already used; `sqlx::migrate!` pattern established |
| chrono | 0.4 | Duration calculation for "idle 5m", timestamps | Already used for `created_at`/`updated_at` |
| tokio | 1.37 | Async runtime for new command handlers | Already used |
| clap | 4.5 | New `Agents` and `Context` subcommands | Already used |
| owo-colors | 3 | Colored status output (idle=green, busy=yellow, dead=red) | Already used; `IsTerminal` pattern established |
| anyhow | 1.0 | Error handling in new commands | Already used |

### Supporting (shell scripting — no Rust additions needed)
| Tool | Purpose | Notes |
|------|---------|-------|
| bash | Hook script interpreter | POSIX sh compatible; avoid bash-isms for portability |
| jq | Parse JSON stdin from Claude Code Stop event | Must be available; script should check and warn if missing |

### No New Dependencies Required
All necessary libraries are already in `Cargo.toml`. The phase adds code, not dependencies.

## Architecture Patterns

### Recommended Project Structure Changes

```
src/
├── commands/
│   ├── agents.rs        # NEW: SESS-03, SESS-04 — list agents with reconciled status
│   ├── context.rs       # NEW: SESS-05 — generate orchestrator context file
│   ├── signal.rs        # MODIFIED: add 4-layer guard (HOOK-01, HOOK-03)
│   └── mod.rs           # MODIFIED: add agents, context exports
├── db/
│   ├── agents.rs        # MODIFIED: add status queries, update_agent_status
│   └── migrations/
│       └── 0002_agent_status.sql  # NEW: adds status + status_updated_at columns
├── cli.rs               # MODIFIED: add Agents and Context variants
└── main.rs              # MODIFIED: route new commands
hooks/
├── claude-code.sh       # NEW: HOOK-02 wrapper for Claude Code Stop event
└── gemini-cli.sh        # NEW: HOOK-02 wrapper for Gemini CLI AfterAgent event
```

### Pattern 1: Agent Status DB Model (Dedicated Column)

**What:** Add `status TEXT NOT NULL DEFAULT 'idle'` and `status_updated_at TEXT NOT NULL` to the agents table via a new migration. Do NOT derive status from messages — that's fragile and slow.

**When to use:** Always. Dedicated column is simple, fast to query, and easy to reconcile.

**Migration:**
```sql
-- src/db/migrations/0002_agent_status.sql
ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE agents ADD COLUMN status_updated_at TEXT NOT NULL DEFAULT (datetime('now'));
```

**Status update in agents.rs:**
```rust
// Source: established project pattern (sqlx, chrono)
pub async fn update_agent_status(
    pool: &SqlitePool,
    name: &str,
    status: &str,  // "idle" | "busy" | "dead"
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE agents SET status = ?, status_updated_at = ? WHERE name = ?"
    )
    .bind(status)
    .bind(&now)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(())
}
```

### Pattern 2: Status Reconciliation (Lazy — On `agents` Command)

**What:** In `commands/agents.rs`, for each agent returned by `list_agents()`, call `tmux::session_exists(agent.name)` and update DB status if it has changed.

**When to use:** Reconcile on the `agents` command only, not on every read. This is "lazy" reconciliation — the CONTEXT.md decision says "reconciles against live tmux session existence, not just DB cache", which is satisfied by the `agents` command doing it.

**Reconciliation logic:**
```rust
// Source: established pattern from init.rs + tmux.rs
pub async fn run(json: bool) -> anyhow::Result<()> {
    let pool = /* connect */;
    let agents = db::agents::list_agents(&pool).await?;

    for agent in &agents {
        let session_alive = tmux::session_exists(&agent.name);
        let expected_status = if session_alive {
            // Session exists: preserve busy/idle, only fix "dead"
            if agent.status == "dead" { "idle" } else { &agent.status }
        } else {
            "dead"
        };
        if expected_status != agent.status {
            db::agents::update_agent_status(&pool, &agent.name, expected_status).await?;
        }
    }

    // Re-fetch after reconciliation for accurate display
    let agents = db::agents::list_agents(&pool).await?;
    // ... render table
}
```

**Status + duration display:**
```rust
// Source: established pad_colored() pattern from commands/list.rs
fn format_status_with_duration(status: &str, status_updated_at: &str) -> String {
    let since = chrono::DateTime::parse_from_rfc3339(status_updated_at)
        .ok()
        .map(|t| {
            let dur = chrono::Utc::now().signed_duration_since(t);
            let mins = dur.num_minutes();
            if mins < 60 { format!("{}m", mins) }
            else { format!("{}h{}m", mins / 60, mins % 60) }
        })
        .unwrap_or_else(|| "?".to_string());
    format!("{} {}", status, since)
    // e.g., "idle 5m", "busy 12m", "dead 2h30m"
}
```

### Pattern 3: Guard Logic in Rust Binary (signal.rs)

**What:** The 4-layer guard lives in `commands/signal.rs` BEFORE the existing signal flow. Guards are in Rust, not shell, for testability.

**When to use:** Always — matches project pattern of extracting logic into Rust for unit testing.

**Guard order (HOOK-03 four-layer spec):**

```rust
// Source: CONTEXT.md guard spec + existing signal.rs structure
pub async fn run(agent: String, json: bool) -> anyhow::Result<()> {
    // GUARD 1: Not in tmux — silent exit 0
    if std::env::var("TMUX_PANE").is_err() {
        return Ok(());  // Silent: not a managed agent environment
    }

    // GUARD 2: squad.yml / DB connection
    let config_path = std::path::Path::new("squad.yml");
    let config = match config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("squad-station: warning: {e}");
            return Ok(());  // stderr + exit 0 per edge case decision
        }
    };
    let db_path = config::resolve_db_path(&config)?;
    let pool = match db::connect(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("squad-station: warning: DB connection failed: {e}");
            return Ok(());
        }
    };

    // GUARD 3: Agent not registered — silent exit 0
    let agent_record = db::agents::get_agent(&pool, &agent).await?;
    let agent_record = match agent_record {
        Some(r) => r,
        None => return Ok(()),  // Unregistered: silent exit 0
    };

    // GUARD 4: Orchestrator self-signal — silent exit 0 (HOOK-01)
    if agent_record.role == "orchestrator" {
        return Ok(());
    }

    // Existing signal flow follows...
    // update_agent_status to "idle" after successful signal
}
```

**Note:** After a successful signal, also call `update_agent_status(&pool, &agent, "idle")` to keep status accurate.

### Pattern 4: Provider Hook Scripts

**What:** Two thin shell scripts in `hooks/` that call `squad-station signal "$AGENT_NAME"`. All guard logic is in the Rust binary — the scripts only handle provider-specific environment setup (reading stdin, extracting agent name from tmux).

**When to use:** Users add one line to their provider config pointing to these scripts.

#### Claude Code hook script (hooks/claude-code.sh)

**Provider interface:**
- Config file: `~/.claude/settings.json` or `.claude/settings.json`
- Hook event: `Stop`
- JSON payload arrives on **stdin**: `{ "session_id": "...", "transcript_path": "...", "cwd": "...", "permission_mode": "...", "hook_event_name": "Stop", "stop_hook_active": true/false, "last_assistant_message": "..." }`
- The hook must exit 0 — exit 2 would prevent Claude from stopping (dangerous for this use case)
- No env vars from Claude Code identify the tmux session — script must detect it from tmux itself

```bash
#!/bin/bash
# hooks/claude-code.sh — Signal squad-station when Claude Code finishes
# Registered under Stop event in .claude/settings.json or ~/.claude/settings.json
#
# Claude Code passes JSON via stdin (we discard it — we only need the tmux session name)

# Drain stdin to avoid broken pipe signal to Claude Code
cat > /dev/null

# Detect agent name from current tmux session (Guard 1+2 handled in Rust binary)
if [ -z "$TMUX_PANE" ]; then
    exit 0  # Not in tmux — not a managed agent
fi

AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0  # Cannot determine session name
fi

# Delegate all other guards and signal logic to the binary
# Errors are captured as warnings to stderr — never fail the provider
SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    echo "squad-station: warning: binary not found at '$SQUAD_BIN'" >&2
    exit 0
fi

"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 | (grep -i "warning\|error" >&2 || true)
exit 0
```

**Configuration example for users:**
```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/squad-station/hooks/claude-code.sh"
          }
        ]
      }
    ]
  }
}
```

#### Gemini CLI hook script (hooks/gemini-cli.sh)

**Provider interface:**
- Config file: `.gemini/settings.json` or `~/.gemini/settings.json`
- Hook event: `AfterAgent`
- JSON payload arrives on **stdin**: `{ "session_id": "...", "transcript_path": "...", "cwd": "...", "hook_event_name": "AfterAgent", "timestamp": "...", "prompt": "...", "prompt_response": "...", "stop_hook_active": false }`
- Exit 0 = success. Exit 2 = retry (forces automatic retry — do NOT exit 2 for signal purposes). Other exits = warning (non-fatal).

```bash
#!/bin/bash
# hooks/gemini-cli.sh — Signal squad-station when Gemini CLI finishes
# Registered under AfterAgent event in .gemini/settings.json
#
# Gemini CLI passes JSON via stdin (we discard it — we only need the tmux session name)

# Drain stdin
cat > /dev/null

# Detect agent name from current tmux session
if [ -z "$TMUX_PANE" ]; then
    exit 0
fi

AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0
fi

SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    echo "squad-station: warning: binary not found at '$SQUAD_BIN'" >&2
    exit 0
fi

"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 | (grep -i "warning\|error" >&2 || true)
exit 0
```

**Configuration example for users (.gemini/settings.json):**
```json
{
  "hooks": {
    "AfterAgent": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/squad-station/hooks/gemini-cli.sh"
          }
        ]
      }
    ]
  }
}
```

**Key insight on provider script similarity:** Both providers use identical logic (stdin drain, tmux session detection, binary delegation). A single universal script would work but two named scripts provide clearer documentation and easier debugging. The CONTEXT.md discretion question resolves to: **two separate files** for clarity, since the interface is near-identical.

### Pattern 5: context Command (SESS-05)

**What:** New `commands/context.rs` that fetches all agents (with reconciled status) and renders Markdown to stdout.

**When to use:** User runs `squad-station context > agent-context.md` or pipes directly into orchestrator.

```rust
// Source: established project pattern (list.rs output, IsTerminal)
pub async fn run() -> anyhow::Result<()> {
    let pool = /* connect */;
    // Reconcile status (same logic as agents command)
    let agents = db::agents::list_agents(&pool).await?;

    // Render Markdown (always plain text — this is for pasting, not TTY display)
    println!("# Squad Station — Agent Roster");
    println!();
    println!("## Available Agents");
    println!();
    println!("| Agent | Role | Status | Send Command |");
    println!("|-------|------|--------|--------------|");
    for agent in &agents {
        println!(
            "| {} | {} | {} | `squad-station send {} \"<task>\"` |",
            agent.name, agent.role, agent.status, agent.name
        );
    }
    println!();
    println!("## Usage");
    println!();
    println!("Send a task:  `squad-station send <agent> \"<task description>\"`");
    println!("Check status: `squad-station agents`");
    println!("Signal done:  `squad-station signal <agent>`  (called by hook scripts)");
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Deriving status from messages table:** Slow, fragile — a pending message doesn't mean the agent is busy (message could be stale). Use the dedicated `status` column.
- **Exit code 2 in hook scripts for signal purposes:** Claude Code Stop hook exit 2 prevents Claude from stopping — catastrophic for agents. Always exit 0 from hook scripts.
- **Gemini CLI hook exit 2 for signal:** Exit 2 triggers automatic retry — equally catastrophic. Always exit 0.
- **Shell-only guard logic:** Pure shell guards can't be unit tested. Rust binary guards follow the testable-args pattern established in `tmux.rs`.
- **`bail!()` in signal when agent not found:** Phase 1 uses `bail!` for not-found, but HOOK-03 says unregistered agents must be **silent exit 0**. The guard must return `Ok(())`, not error.
- **Reconciling status on every `send`/`peek`:** Performance cost without benefit — reconcile only on `agents` and `context` commands.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tmux session detection | Custom pane parsing | `tmux::session_exists()` (already exists) | Already handles all error cases |
| Current session name in shell | Manual `$TMUX` parsing | `tmux list-panes -t "$TMUX_PANE" -F '#S'` | Official tmux format string, handles nested sessions |
| SQLite migrations | Manual `CREATE TABLE IF NOT EXISTS` | `sqlx::migrate!` (already used) | Transactional, numbered, idempotent |
| Duration formatting | Custom time math | `chrono::Utc::now().signed_duration_since()` | Already in Cargo.toml |
| Colored terminal output | Raw ANSI codes | `owo-colors` (already used) | Terminal detection already wired |
| Markdown table formatting | Custom alignment | Simple `println!` with `|` separators | Context output is plain Markdown, not aligned TTY table |

**Key insight:** All required tools are already in `Cargo.toml` and the existing codebase. This phase is purely additive.

## Common Pitfalls

### Pitfall 1: signal.rs Currently Uses `bail!` for Unregistered Agent — Must Change
**What goes wrong:** Current `signal.rs` line 18-20 calls `bail!("Agent not found: {}", agent)` which exits non-zero. HOOK-03 requires silent exit 0 for unregistered agents.
**Why it happens:** Phase 1 `signal` was designed as a direct CLI tool. Phase 2 adds hook context where the agent might legitimately be unregistered.
**How to avoid:** Replace `bail!` with `return Ok(())` for the unregistered-agent case in the guard. The bail path should become unreachable once guards are in place.
**Warning signs:** Integration test for `squad-station signal unknown-agent` that expects exit 1 — must be updated to expect exit 0.

### Pitfall 2: Stop Hook Exit Code Kills Agent's Ability to Stop
**What goes wrong:** Hook script exits 2 or non-zero → Claude Code interprets this as "block Claude from stopping" → agent loops forever.
**Why it happens:** Claude Code Stop hook `exit 2` semantics are "prevent stop, continue conversation" — exactly the wrong behavior for a notification hook.
**How to avoid:** Every code path in hook scripts ends with `exit 0`. Errors go to stderr as warnings.
**Warning signs:** Agent stuck in loop, Claude not returning control.

### Pitfall 3: `TMUX_PANE` vs `$TMUX` for Tmux Detection
**What goes wrong:** Checking `$TMUX` (the socket path) instead of `$TMUX_PANE`. `$TMUX` is set in nested tmux but `$TMUX_PANE` is the reliable pane identifier.
**Why it happens:** Common documentation error.
**How to avoid:** Use `[ -z "$TMUX_PANE" ]` to detect "not in tmux pane". `$TMUX_PANE` is always set to the pane's ID (e.g., `%3`) when inside a tmux pane.
**Warning signs:** Guard bypassed when user runs hook manually from terminal (TMUX is set, TMUX_PANE may not be).

### Pitfall 4: ALTER TABLE in SQLite — Column Ordering
**What goes wrong:** `ALTER TABLE agents ADD COLUMN status` without `DEFAULT` or `NOT NULL DEFAULT 'idle'` fails if there are existing rows without a value.
**Why it happens:** SQLite requires `DEFAULT` when adding `NOT NULL` columns to existing tables.
**How to avoid:** Migration must use `ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'idle'`. The `DEFAULT` clause is required for the migration to succeed against existing data.
**Warning signs:** `sqlx::migrate!` panics at startup for existing databases.

### Pitfall 5: status_updated_at Tracking for Duration
**What goes wrong:** Using `updated_at` from messages table to derive "how long has agent been idle" — it reflects message timestamps, not agent status transitions.
**Why it happens:** No dedicated tracking field for status change time.
**How to avoid:** Add `status_updated_at TEXT NOT NULL DEFAULT (datetime('now'))` to agents table. Update it in `update_agent_status()`.
**Warning signs:** Duration shows "0m" or wrong value after agent status changes.

### Pitfall 6: Context Command Ignores `--json` Flag
**What goes wrong:** `context` command renders human Markdown even with `--json`, breaking machine-readable tooling.
**Why it happens:** Markdown IS the intended output for this command — but the `--json` global flag should still be respected.
**How to avoid:** When `json == true`, emit a JSON envelope `{ "content": "<markdown string>" }` so tools can handle both modes. OR: document that `context` always emits Markdown (no JSON mode needed since the content IS structured).
**Decision required (Claude's discretion):** Given context is consumed by AI orchestrators that paste it directly, Markdown-only is defensible. The planner should decide.

### Pitfall 7: Gemini CLI AfterAgent `stop_hook_active` Field
**What goes wrong:** Hook fires twice during a retry sequence (when `stop_hook_active: true`). Double-signaling is idempotent per MSG-03, so this is not a bug — but worth knowing.
**Why it happens:** Gemini CLI sets `stop_hook_active: true` when the hook is running as part of a retry.
**How to avoid:** No action needed — MSG-03 idempotency handles it. Shell scripts drain stdin so this field is ignored anyway.

## Code Examples

Verified patterns from official sources:

### tmux Session Name from Inside a Pane
```bash
# Source: tmux documentation, confirmed via tmux list-panes man page
# TMUX_PANE is set by tmux in all child processes of a pane (e.g., %3)
if [ -n "$TMUX_PANE" ]; then
    AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
fi
```

### Claude Code Stop Hook JSON Input (Confirmed from Official Docs)
```json
{
  "session_id": "abc123",
  "transcript_path": "~/.claude/projects/.../00893aaf.jsonl",
  "cwd": "/Users/...",
  "permission_mode": "default",
  "hook_event_name": "Stop",
  "stop_hook_active": true,
  "last_assistant_message": "I've completed the task..."
}
```

### Gemini CLI AfterAgent Hook JSON Input (Confirmed from Official Docs)
```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string",
  "hook_event_name": "AfterAgent",
  "timestamp": "ISO 8601 string",
  "prompt": "the user's original request",
  "prompt_response": "the final text generated by the agent",
  "stop_hook_active": false
}
```

### Claude Code hooks.json Configuration Location
```
~/.claude/settings.json          # Global (all projects)
.claude/settings.json            # Per-project (committable)
.claude/settings.local.json      # Per-project local (gitignored)
```

### Gemini CLI settings.json Configuration Location
```
.gemini/settings.json            # Per-project
~/.gemini/settings.json          # Global (inferred from project pattern)
```

### SQLite Migration Pattern (ALTER TABLE with DEFAULT)
```sql
-- src/db/migrations/0002_agent_status.sql
-- Source: SQLite ALTER TABLE docs — NOT NULL columns require DEFAULT for existing rows
ALTER TABLE agents ADD COLUMN status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE agents ADD COLUMN status_updated_at TEXT NOT NULL DEFAULT (datetime('now'));
```

### Rust Guard Using TMUX_PANE
```rust
// Source: established project pattern (std::env, anyhow)
// Guard 1: Must be running inside a tmux pane
if std::env::var("TMUX_PANE").is_err() {
    return Ok(());  // Silent exit 0 — not in a managed tmux session
}
```

### Drain stdin in Bash (Avoid Broken Pipe to Provider)
```bash
# Always drain stdin when Claude Code / Gemini CLI sends JSON
# Prevents broken pipe signal that might confuse the provider
cat > /dev/null
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Claude Code `PostToolUse` for completion signal | `Stop` event hook | Recent (2025) | Stop fires when entire response is done, not after each tool |
| Gemini CLI manual signal | `AfterAgent` event hook | 2025 | AfterAgent fires after full agent response, correct for completion signals |
| Shell-only hook logic | Guards in binary, thin shell wrapper | This phase | Enables unit testing of guard logic |

**Deprecated/outdated:**
- `CLAUDE_TOOL_NAME` / `CLAUDE_FILE_PATH` env vars: old Claude Code hook pattern — current hooks use **stdin JSON**, not environment variables
- Exit code 2 for "hook error": now means "block action" in Claude Code, not "hook failed" — exit 0 with stderr for all error cases

## Open Questions

1. **`SQUAD_STATION_DB` env var fallback in hook scripts**
   - What we know: `register` command uses `SQUAD_STATION_DB` as fallback when no squad.yml exists (from Phase 1 decisions)
   - What's unclear: Should hook scripts export `SQUAD_STATION_DB` before calling the binary, or rely on the binary to find squad.yml in `cwd`?
   - Recommendation: Rely on the binary's `squad.yml` discovery (cwd-based). Hook scripts run in the project directory so cwd should be correct. If not, users can set `SQUAD_STATION_DB` in their shell profile.

2. **`--json` flag for `context` command**
   - What we know: All existing commands support `--json`. Context output is Markdown for orchestrator prompts.
   - What's unclear: Should `context` support `--json` at all? The output IS structured data (Markdown), not a rendered view.
   - Recommendation: Planner decides. Simplest: skip `--json` for `context` since the Markdown format is already machine-friendly. Document this exception in help text.

3. **`agents` command and status update race condition**
   - What we know: Single-writer pool (`max_connections(1)`) prevents write conflicts. Reconciliation is a quick loop of `tmux has-session` checks.
   - What's unclear: Performance when there are many agents (10+) each requiring a `tmux has-session` syscall.
   - Recommendation: Not a concern for v1 (squads are small). No optimization needed yet.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust test (`cargo test`) + tokio-test 0.4 |
| Config file | Cargo.toml `[dev-dependencies]` — no separate config |
| Quick run command | `cargo test --test test_db` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SESS-03 | `update_agent_status` writes correct status + timestamp | unit | `cargo test test_update_agent_status` | ❌ Wave 0 |
| SESS-03 | Status display includes duration string | unit | `cargo test test_format_status_with_duration` | ❌ Wave 0 |
| SESS-04 | Reconcile sets dead when session absent | unit (mock tmux) | `cargo test test_reconcile_dead_agent` | ❌ Wave 0 |
| SESS-04 | Reconcile revives idle when session reappears | unit (mock tmux) | `cargo test test_reconcile_revived_agent` | ❌ Wave 0 |
| SESS-05 | Context output contains all registered agents | unit | `cargo test test_context_output_contains_agents` | ❌ Wave 0 |
| SESS-05 | Context output contains usage commands | unit | `cargo test test_context_output_has_usage` | ❌ Wave 0 |
| HOOK-01 | Signal on orchestrator role returns Ok without signaling | unit | `cargo test test_signal_skips_orchestrator` | ❌ Wave 0 |
| HOOK-03 | Signal without TMUX_PANE returns Ok silently | unit | `cargo test test_signal_guard_no_tmux` | ❌ Wave 0 |
| HOOK-03 | Signal for unregistered agent returns Ok silently | unit | `cargo test test_signal_guard_unregistered` | ❌ Wave 0 |
| HOOK-03 | Signal with DB error emits stderr warning, exits 0 | unit | `cargo test test_signal_guard_db_error` | ❌ Wave 0 |
| HOOK-02 | claude-code.sh exits 0 in all cases | shell/manual | Manual — shell script testing | N/A |
| HOOK-02 | gemini-cli.sh exits 0 in all cases | shell/manual | Manual — shell script testing | N/A |

### Sampling Rate
- **Per task commit:** `cargo test --test test_db` (DB layer only, <5s)
- **Per wave merge:** `cargo test` (full suite including integration)
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
All test functions above need to be added to `tests/test_db.rs` (DB layer) or a new `tests/test_lifecycle.rs` (agents command logic). The test infrastructure (`setup_test_db()`, `#[tokio::test]`, `tempfile`) is fully established — no framework setup needed.

- [ ] `tests/test_db.rs` — add `test_update_agent_status`, `test_format_status_with_duration`
- [ ] `tests/test_lifecycle.rs` — new file: reconcile tests, signal guard tests, context output tests
- [ ] HOOK-02 shell scripts: manual verification (cannot be unit tested without shell test framework)

## Sources

### Primary (HIGH confidence)
- Official Claude Code Hooks Reference — `https://code.claude.com/docs/en/hooks` — Stop event JSON schema, configuration locations, exit code semantics verified
- Official Gemini CLI Hooks Reference — `https://geminicli.com/docs/hooks/reference/` — AfterAgent event JSON schema verified
- Project codebase (`src/`) — direct inspection: signal.rs, tmux.rs, db/agents.rs, migrations, Cargo.toml
- CONTEXT.md — locked decisions and discretion areas

### Secondary (MEDIUM confidence)
- tmux `list-panes` + `$TMUX_PANE` pattern — `https://man.man7.org/linux/man-pages/man1/tmux.1.html` — confirmed via WebSearch with multiple corroborating sources
- SQLite `ALTER TABLE ... ADD COLUMN ... DEFAULT` constraint — standard SQLite behavior, confirmed by sqlx migration pattern already in use

### Tertiary (LOW confidence)
- `SQUAD_STATION_BIN` env var fallback in hook scripts — project convention, not from official docs
- Gemini CLI `~/.gemini/settings.json` global config location — inferred from `.gemini/settings.json` project pattern; not explicitly documented in fetched reference

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in Cargo.toml, no new libraries
- Architecture: HIGH — direct inspection of existing codebase + official provider docs
- Provider hook schemas: HIGH — fetched from official docs (Claude Code) and official geminicli.com reference (Gemini CLI)
- Pitfalls: HIGH — directly identified from code inspection (signal.rs bail! issue) and official docs (exit code semantics)

**Research date:** 2026-03-06
**Valid until:** 2026-06-06 for Rust patterns (stable), 2026-04-06 for provider hook schemas (fast-moving)
