# Phase 1: Core Foundation - Research

**Researched:** 2026-03-06
**Domain:** Rust CLI, SQLite (sqlx), tmux automation, SIGPIPE, YAML parsing
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**squad.yml Config Format:**
- Structured format: each agent has name, provider (label only), role, and explicit `command` field for launch
- Dedicated top-level `orchestrator:` section, separate from `agents:` list — structurally distinct from workers
- Include project-level config: project name, DB path, tmux settings alongside agent definitions
- Provider field is purely a label (e.g., "claude-code", "gemini") — no built-in provider-to-command mappings. Actual launch always via explicit `command` field

**CLI Output & Feedback:**
- Minimal by default: action + result only (e.g., `✓ Sent task to frontend-agent` or `✗ Agent not found: backend`)
- `--json` flag available on all commands for machine-readable structured output
- `list` command uses table format with aligned columns (like `docker ps` / `kubectl get pods`)
- Terminal colors with auto-detect: enabled by default, auto-disabled when piped or `NO_COLOR` env set

**Notification Delivery:**
- Signal notifies orchestrator via tmux send-keys into orchestrator session
- Notification format is structured: `[SIGNAL] agent=frontend status=completed task_id=42` — machine-parseable by orchestrator AI
- Completion event only — no output capture. Orchestrator uses `capture-pane` separately if it needs agent output
- Signals queue in DB regardless of orchestrator availability. If orchestrator session not running, notification is persisted and can be retrieved via `peek`/`list` on next check

**Error Handling:**
- Invalid targets (agent not found, dead tmux session) fail with clear error message and exit non-zero
- Simple exit codes: 0=success, 1=any error. Error type communicated via stderr message
- `init` partial failure: continue launching what works, report which agents failed and why. Exit 1 only if all agents failed
- `init` is idempotent: re-running skips already-running agents, only launches missing ones. Safe to retry after partial failures

### Claude's Discretion
- DB schema design and migration approach
- Rust module structure and code organization
- Exact table column layout for `list` output
- Color choices for status indicators
- Internal error types and error message wording
- Specific SQLite configuration details beyond WAL + busy_timeout

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SESS-01 | User can initialize squad from squad.yml — creates DB, registers agents, creates tmux sessions, launches AI tools | squad.yml YAML parsing (serde-saphyr), sqlx migrate! for DB creation, tmux new-session via std::process::Command, shell readiness pattern |
| SESS-02 | User can register new agent at runtime without editing squad.yml | SQL INSERT with conflict handling (INSERT OR IGNORE), clap subcommand `register` |
| MSG-01 | Orchestrator can send task to agent via `squad-station send` — writes to DB and injects prompt into agent tmux session | sqlx INSERT, tmux send-keys -l for literal injection, shell readiness check |
| MSG-02 | Hook can signal agent completion via `squad-station signal` — updates DB status and notifies orchestrator via tmux send-keys | sqlx UPDATE with idempotency check, orchestrator tmux send-keys for structured [SIGNAL] format |
| MSG-03 | Send and signal operations are idempotent — duplicate hook fires do not create duplicate messages or state corruption | DB-level idempotency: INSERT OR IGNORE / UPDATE WHERE status = 'pending', task_id uniqueness |
| MSG-04 | User can list messages with filters by agent, status, and limit | sqlx SELECT with WHERE clauses, table-format output (aligned columns), --json flag |
| MSG-05 | Messages support priority levels (normal, high, urgent) | DB column `priority` as TEXT or INTEGER, clap ValueEnum for priority flag |
| MSG-06 | Agent can peek for pending tasks via `squad-station peek` | sqlx SELECT WHERE agent=? AND status='pending' ORDER BY priority DESC LIMIT 1 |
| SAFE-01 | SQLite uses WAL mode with busy_timeout to handle concurrent writes from multiple agent signals | sqlx SqliteConnectOptions: journal_mode(Wal), busy_timeout(5s), max_connections(1) writer pool |
| SAFE-02 | tmux send-keys uses literal mode (-l) to prevent special character injection | std::process::Command args: ["send-keys", "-t", target, "-l", text] + Enter as separate send-keys |
| SAFE-03 | tmux send-keys waits for shell readiness before injecting prompt | tmux new-session with direct command arg OR sleep + capture-pane poll pattern |
| SAFE-04 | SIGPIPE handler installed at binary startup | unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) } or signal-hook crate |
</phase_requirements>

---

## Summary

This phase builds a stateless Rust CLI binary that wires together four subsystems: YAML config parsing (squad.yml), embedded SQLite for message and agent state, tmux session management for agent lifecycle and message delivery, and safety primitives that must be present from day one. The binary has no daemon — each invocation opens a connection, performs one operation, and exits.

The most critical technical decision is **sqlx vs rusqlite**. The Cargo.toml has sqlx 0.7, but STATE.md references rusqlite. For a stateless CLI that runs one write operation per invocation and exits, rusqlite (synchronous) is the simpler and safer choice — it eliminates the async write-contention footgun inherent in sqlx with SQLite. However, the Cargo.toml already locks sqlx, and changing it now means a rewrite of the dependency tree. The planner must make an explicit call: either stay with sqlx (use `max_connections(1)` write pool + explicit WAL config) or switch to rusqlite (simpler, no async overhead). Research recommends **staying with sqlx** given it is already in Cargo.toml, but using a single-connection write pool pattern to avoid contention.

YAML parsing has a clear winner: **serde-saphyr** is the actively-maintained replacement for the archived serde_yaml. Version 0.0.17 is current as of 2026-03. The STATE.md concern about community size is valid but manageable — serde_yml is a larger-community fallback if serde-saphyr causes issues.

**Primary recommendation:** Use sqlx 0.8 (upgrade from 0.7), single max_connections(1) pool with WAL + busy_timeout, serde-saphyr for YAML, std::process::Command for all tmux operations with -l literal flag, and libc SIGPIPE reset at startup.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 (in Cargo.toml) | CLI argument parsing with derive macros | De-facto standard for Rust CLIs; derive feature eliminates boilerplate |
| sqlx | 0.8.x (upgrade from 0.7) | Async SQLite driver with compile-time query checking | Already in Cargo.toml; 0.8 has security fix (RUSTSEC-2024-0363) |
| tokio | 1.37 (in Cargo.toml) | Async runtime required by sqlx | Required by sqlx; full features already specified |
| serde / serde_json | 1.0 (in Cargo.toml) | Serialization for --json output and YAML parsing | Standard serialization layer |
| serde-saphyr | 0.0.17 | YAML deserialization for squad.yml | Active replacement for archived serde_yaml; panic-free parsing |
| anyhow | 1.0 (in Cargo.toml) | Error propagation with context | Standard for application-level error handling |
| chrono | 0.4 (in Cargo.toml) | Timestamps for message records | Already in Cargo.toml |
| uuid | 1.8 (in Cargo.toml) | Unique IDs for messages | Already in Cargo.toml |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| owo-colors | latest (3.x) | Terminal color output with NO_COLOR support | Color status indicators in list output; auto-detects tty |
| libc | 0.2 | SIGPIPE reset at binary startup | One-time unsafe call in main() before any I/O |

**Note on ratatui/crossterm:** Already in Cargo.toml but OUT OF SCOPE for Phase 1. Do not wire in Phase 1.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| sqlx | rusqlite | rusqlite is synchronous (simpler for CLI), no async overhead, bundled SQLite. However Cargo.toml already has sqlx and changing means dependency tree rewrite. Stick with sqlx but use single-connection write pool. |
| serde-saphyr | serde_yml | serde_yml has larger community (~50k downloads/week vs serde-saphyr ~100k). Both are valid. serde-saphyr is faster and type-driven; serde_yml is a closer drop-in for old serde_yaml users. |
| owo-colors | termcolor | termcolor is BurntSushi's cross-platform solution. owo-colors handles NO_COLOR + tty detection in one crate. For this use case both work. |

**Installation (additions to Cargo.toml):**
```toml
serde-saphyr = "0.0.17"
owo-colors = { version = "3", features = ["supports-colors"] }
libc = "0.2"
```

**Version upgrade:**
```toml
# Change in Cargo.toml:
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "macros", "chrono", "uuid", "migrate"] }
```

---

## Architecture Patterns

### Recommended Project Structure
```
src/
├── main.rs          # Entry point: SIGPIPE, clap parse, tokio::main, dispatch
├── cli.rs           # Cli struct + Commands enum with clap derive
├── db/
│   ├── mod.rs       # Pool setup: connect_with WAL+busy_timeout, run migrations
│   ├── migrations/  # Embedded SQL migration files (sqlx::migrate!)
│   │   └── 0001_initial.sql
│   ├── agents.rs    # Agent CRUD (insert, get, list)
│   └── messages.rs  # Message CRUD (insert, update status, list, peek)
├── tmux.rs          # All tmux operations via std::process::Command
├── config.rs        # squad.yml structs + serde-saphyr deserialization
└── commands/
    ├── init.rs      # squad-station init
    ├── send.rs      # squad-station send
    ├── signal.rs    # squad-station signal
    ├── list.rs      # squad-station list
    ├── peek.rs      # squad-station peek
    └── register.rs  # squad-station register
```

### Pattern 1: CLI Structure with Global --json Flag
**What:** Top-level Cli struct with global `json` flag and Commands subcommand enum
**When to use:** All commands inherit `--json` without repeating it

```rust
// Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "squad-station", version, about)]
pub struct Cli {
    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize squad from squad.yml
    Init {
        #[arg(default_value = "squad.yml")]
        config: PathBuf,
    },
    /// Send task to agent
    Send {
        agent: String,
        task: String,
        #[arg(long, value_enum, default_value = "normal")]
        priority: Priority,
    },
    /// Signal agent completion
    Signal { agent: String },
    /// List messages
    List {
        #[arg(long)] agent: Option<String>,
        #[arg(long)] status: Option<String>,
        #[arg(long, default_value = "20")] limit: u32,
    },
    /// Peek for pending task
    Peek { agent: String },
    /// Register agent at runtime
    Register {
        name: String,
        #[arg(long)] command: String,
        #[arg(long, default_value = "worker")] role: String,
        #[arg(long, default_value = "unknown")] provider: String,
    },
}

#[derive(clap::ValueEnum, Clone, serde::Serialize)]
pub enum Priority { Normal, High, Urgent }
```

### Pattern 2: SQLite Connection with WAL + Single Writer
**What:** Single-connection write pool prevents async write-contention deadlocks
**When to use:** Every DB write operation in this stateless CLI

```rust
// Source: https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html
// and https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::time::Duration;

pub async fn connect(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)   // CRITICAL: single writer, no async deadlock
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;
    Ok(pool)
}
```

### Pattern 3: tmux send-keys with Literal Mode
**What:** All tmux key injection uses -l flag and separate Enter invocation
**When to use:** Every `send` and `signal` operation — never deviate

```rust
// Source: tmux manual + https://github.com/anthropics/claude-code/issues/23513
use std::process::Command;

pub fn send_keys_literal(target: &str, text: &str) -> anyhow::Result<()> {
    // Step 1: Send text as literal (no key name interpretation)
    let status = Command::new("tmux")
        .args(["send-keys", "-t", target, "-l", text])
        .status()?;
    if !status.success() {
        anyhow::bail!("tmux send-keys failed for target: {}", target);
    }
    // Step 2: Send Enter as separate key (NOT -l, so Enter key is recognized)
    Command::new("tmux")
        .args(["send-keys", "-t", target, "Enter"])
        .status()?;
    Ok(())
}

pub fn session_exists(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

### Pattern 4: tmux new-session for Agent Launch (Shell Readiness)
**What:** Pass command directly to `new-session` to avoid send-keys race condition
**When to use:** `squad-station init` when launching agent sessions

```rust
// Source: https://github.com/anthropics/claude-code/issues/23513
// Preferred: pass command as part of new-session — avoids shell init race condition
pub fn launch_agent(session_name: &str, command: &str) -> anyhow::Result<()> {
    let status = Command::new("tmux")
        .args([
            "new-session", "-d",       // detached
            "-s", session_name,        // session name = agent identity
            command,                   // command runs directly as pane process
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to create tmux session: {}", session_name);
    }
    Ok(())
}
```

### Pattern 5: SIGPIPE Handler at Binary Startup
**What:** Reset SIGPIPE to default so broken pipe causes immediate exit, not panic
**When to use:** First thing in main(), before any I/O

```rust
// Source: https://github.com/rust-lang/rust/issues/62569
// Rust sets SIGPIPE to SIG_IGN by default since 2014.
// For CLI tools, SIG_DFL (exit immediately on broken pipe) is correct.
fn main() {
    // Reset SIGPIPE to default behavior (required for CLI tools)
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    // ... rest of main
}
```

### Pattern 6: squad.yml Deserialization

```rust
// Source: https://github.com/bourumir-wyngs/serde-saphyr
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SquadConfig {
    pub project: ProjectConfig,
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

#[derive(Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub db_path: Option<String>,  // default: ~/.agentic-squad/<name>/station.db
}

#[derive(Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub provider: String,   // label only — no built-in mappings
    pub role: String,
    pub command: String,    // actual launch command
}

pub fn load_config(path: &Path) -> anyhow::Result<SquadConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: SquadConfig = serde_saphyr::from_str(&content)?;
    Ok(config)
}
```

### Pattern 7: Idempotency for MSG-03

```sql
-- Agents: INSERT OR IGNORE on name (natural key)
INSERT OR IGNORE INTO agents (id, name, provider, role, command, created_at)
VALUES (?, ?, ?, ?, ?, ?);

-- Messages: task_id is UUID assigned at send time — duplicate sends blocked at application layer
-- Signal: UPDATE only if status = 'pending' to prevent double-signal corruption
UPDATE messages
SET status = 'completed', updated_at = ?
WHERE id = ? AND status = 'pending';
-- Rows affected = 0 means already completed — not an error, silently succeed
```

### Anti-Patterns to Avoid

- **Multiple write connections to sqlx SQLite pool:** Causes SQLITE_BUSY errors under concurrent hook signals. Always `max_connections(1)` for writes.
- **Using `BEGIN IMMEDIATE` across await points with sqlx:** The async runtime may schedule another task while holding the write lock, causing deadlock. Use sqlx's `pool.begin()` which handles this correctly, or avoid explicit transaction management for simple single-statement writes.
- **tmux send-keys without -l flag:** Text containing `[`, `Enter`, `Escape`, or other key names will be interpreted as keybindings and corrupt the injected prompt.
- **Sending command as one send-keys call with text+Enter:** Split into two calls: one with `-l` for text, one without `-l` for `Enter`. Combining them in one `-l` call sends literal "Enter" text instead of the Enter key.
- **Shell invocation for tmux:** Never use `Command::new("sh").arg("-c").arg(format!("tmux send-keys ..."))`. Always call tmux directly with explicit args to prevent shell injection.
- **Hard-coding DB path:** Always derive from project config or `~/.agentic-squad/<project>/station.db`. Create parent directories with `std::fs::create_dir_all`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Manual argv parsing | clap 4.5 derive | Handles help, version, error messages, shell completions |
| SQL query building | String concatenation with `format!` | sqlx query macros | SQL injection surface; compile-time checking with sqlx::query! |
| DB schema versioning | Check-and-create table per run | sqlx::migrate! | Handles version tracking, forward-only migrations, embedded in binary |
| YAML parsing | Custom parser | serde-saphyr | Edge cases in YAML spec (anchors, multiline, special chars) are deep |
| JSON output serialization | Manual string building | serde_json | Correct escaping, nesting, and schema consistency |
| Terminal color detection | Manual TERM/tty checks | owo-colors with supports-colors feature | Handles NO_COLOR, FORCE_COLOR, tty detection, CI detection |

**Key insight:** tmux operations are the one area where hand-rolling is appropriate — use `std::process::Command` calling tmux directly (no tmux crate wrapper needed for this use case).

---

## Common Pitfalls

### Pitfall 1: sqlx Write Contention with SQLite
**What goes wrong:** Multiple concurrent `signal` calls from different agents hit SQLite simultaneously. Even with WAL mode, concurrent writers contend for the write lock and produce `SQLITE_BUSY` errors.
**Why it happens:** sqlx's async pool can schedule multiple write futures concurrently, each acquiring a connection and fighting for the SQLite write lock. `busy_timeout` helps but doesn't eliminate the problem.
**How to avoid:** Set `max_connections(1)` on the SqlitePool. This serializes writes at the application level — only one write can execute at a time. Reads can use a separate pool with higher concurrency if needed (not required for Phase 1).
**Warning signs:** `database is locked` errors in logs when multiple hooks fire simultaneously.

### Pitfall 2: tmux send-keys Race Condition on Session Launch
**What goes wrong:** `init` creates tmux session with `new-session -d -s name` and immediately runs `send-keys -l "command" && send-keys Enter`. With shells like zsh + oh-my-zsh, the shell is still initializing — the injected text appears in the pane but never executes.
**Why it happens:** `new-session -d` returns before the shell inside the pane is ready to accept input.
**How to avoid:** Pass the command directly to `new-session`: `tmux new-session -d -s name "command"`. This runs the command as the pane's initial process, bypassing shell initialization entirely. This is the preferred pattern for agent launch.
**Warning signs:** Command text visible in tmux pane but not executing; shell prompt appears with the command text as plain text.

### Pitfall 3: tmux send-keys Without -l Corrupts Prompts
**What goes wrong:** Task text containing `[`, semicolons, pipe characters, or words like `Enter`/`Escape` gets interpreted as tmux key bindings, truncating or garbling the prompt injected into the agent.
**Why it happens:** `tmux send-keys` interprets its argument as a key sequence by default. `[` starts escape sequences; `Enter` maps to the Enter key.
**How to avoid:** Always use `-l` flag. Always send `Enter` as a separate `send-keys` call without `-l`.
**Warning signs:** Agent receives partial or garbled prompt; brackets in code snippets cause truncation.

### Pitfall 4: sqlx 0.7 Security Vulnerability
**What goes wrong:** Cargo.toml specifies sqlx 0.7. RUSTSEC-2024-0363 was patched in 0.8.1. Building with 0.7 produces a binary with a known security vulnerability.
**Why it happens:** Cargo.toml was written before 0.8 was current.
**How to avoid:** Upgrade to sqlx 0.8 in Cargo.toml as the first step of Wave 0.
**Warning signs:** `cargo audit` reports RUSTSEC-2024-0363.

### Pitfall 5: sqlx migrate! Without build.rs
**What goes wrong:** Adding or modifying `.sql` migration files doesn't trigger a cargo rebuild, so the old embedded migrations run instead of the new ones.
**Why it happens:** Cargo's dependency tracking doesn't watch files referenced from proc-macros by default.
**How to avoid:** Add a `build.rs` with `println!("cargo:rerun-if-changed=src/db/migrations");` or run `sqlx migrate build-script` to generate it automatically.
**Warning signs:** Schema changes not reflected after `cargo build`; migration version mismatch errors.

### Pitfall 6: DB Path Directory Not Existing
**What goes wrong:** `~/.agentic-squad/<project>/station.db` fails to open because the parent directory doesn't exist yet.
**Why it happens:** sqlx's `create_if_missing(true)` creates the file but not parent directories.
**How to avoid:** Call `std::fs::create_dir_all(db_path.parent().unwrap())?` before connecting.
**Warning signs:** `No such file or directory` error on first `init` run.

---

## Code Examples

### Full main.rs skeleton with SIGPIPE + tokio + clap

```rust
// src/main.rs
use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod config;
mod db;
mod tmux;

#[tokio::main]
async fn main() {
    // SAFE-04: Reset SIGPIPE to default before any I/O
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = cli::Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: cli::Cli) -> Result<()> {
    use cli::Commands::*;
    match cli.command {
        Init { config } => commands::init::run(config, cli.json).await,
        Send { agent, task, priority } => commands::send::run(agent, task, priority, cli.json).await,
        Signal { agent } => commands::signal::run(agent, cli.json).await,
        List { agent, status, limit } => commands::list::run(agent, status, limit, cli.json).await,
        Peek { agent } => commands::peek::run(agent, cli.json).await,
        Register { name, command, role, provider } =>
            commands::register::run(name, command, role, provider, cli.json).await,
    }
}
```

### DB schema (migration 0001)

```sql
-- src/db/migrations/0001_initial.sql
CREATE TABLE IF NOT EXISTS agents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    provider    TEXT NOT NULL DEFAULT '',
    role        TEXT NOT NULL DEFAULT 'worker',
    command     TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    agent_name  TEXT NOT NULL,
    task        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',   -- pending, completed, failed
    priority    TEXT NOT NULL DEFAULT 'normal',    -- normal, high, urgent
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    FOREIGN KEY (agent_name) REFERENCES agents(name)
);

CREATE INDEX IF NOT EXISTS idx_messages_agent_status ON messages(agent_name, status);
CREATE INDEX IF NOT EXISTS idx_messages_priority ON messages(priority, created_at);
```

### Signal idempotency pattern

```rust
// src/commands/signal.rs — idempotent update for MSG-03
pub async fn run(agent: String, json: bool) -> anyhow::Result<()> {
    let pool = db::connect(&db_path).await?;

    // Idempotent: only update if currently pending
    let result = sqlx::query!(
        "UPDATE messages SET status = 'completed', updated_at = ?
         WHERE agent_name = ? AND status = 'pending'
         ORDER BY created_at DESC LIMIT 1",
        now, agent
    )
    .execute(&pool)
    .await?;

    // rows_affected == 0 means already completed — not an error
    if result.rows_affected() == 0 {
        // Silently succeed — duplicate signal is not an error
        return Ok(());
    }

    // Notify orchestrator if session exists
    if tmux::session_exists(&orchestrator_session) {
        let msg = format!("[SIGNAL] agent={} status=completed", agent);
        tmux::send_keys_literal(&orchestrator_session, &msg)?;
    }
    // Else: signal persisted in DB, orchestrator retrieves on next check
    Ok(())
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| serde_yaml | serde-saphyr or serde_yml | serde_yaml archived ~2023 | Must use replacement — serde_yaml no longer maintained |
| sqlx 0.7 | sqlx 0.8 | July 2024 | Security fix RUSTSEC-2024-0363; upgrade mandatory |
| `#[unix_sigpipe = "sig_dfl"]` nightly attr | unsafe libc::signal call or signal-hook | 2024 stabilization in progress | `unix_sigpipe` not yet stable on stable Rust; use libc for now |
| sqlx BEGIN IMMEDIATE for writes | Single max_connections(1) pool | Community guidance 2024 | Eliminates async deadlock footgun with SQLite |

**Deprecated/outdated:**
- `serde_yaml`: Archived, no longer maintained — use serde-saphyr or serde_yml
- `atty` crate: Deprecated — use `std::io::IsTerminal` trait (stable since Rust 1.70) or owo-colors built-in detection
- sqlx 0.7: Known security vulnerability RUSTSEC-2024-0363 — upgrade to 0.8

---

## Open Questions

1. **sqlx 0.7 vs 0.8 upgrade scope**
   - What we know: 0.8 has breaking API changes (GAT refactoring) that may require code updates beyond just changing the version string
   - What's unclear: Exact scope of changes needed in query macros and type annotations
   - Recommendation: Planner should treat the 0.7→0.8 upgrade as a discrete Wave 0 task with a cargo check step to surface breakages

2. **serde-saphyr version to pin**
   - What we know: Latest is 0.0.17 as of research date; API is at 0.x (pre-stable)
   - What's unclear: Whether the API will stabilize before this project ships
   - Recommendation: Pin to exact version (= "0.0.17") to prevent breaking changes from minor updates

3. **DB path resolution for SESS-01**
   - What we know: Path should be `~/.agentic-squad/<project>/station.db` per PROJECT.md
   - What's unclear: How to resolve `~` on different platforms in Rust (home_dir is deprecated)
   - Recommendation: Use `dirs` crate (`dirs::home_dir()`) or `std::env::var("HOME")` as fallback

4. **tmux session naming format**
   - What we know: Convention is `<project>-<provider>-<role>` from PROJECT.md
   - What's unclear: Whether the session name must be validated against tmux's character restrictions (tmux session names cannot contain `.`)
   - Recommendation: Validate session names at register time — replace `.` and spaces with `-`

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | tokio-test 0.4 (already in dev-dependencies) |
| Config file | none — see Wave 0 |
| Quick run command | `cargo test` |
| Full suite command | `cargo test -- --include-ignored` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SESS-01 | init from squad.yml creates DB + registers agents | integration | `cargo test test_init_creates_db` | ❌ Wave 0 |
| SESS-01 | init is idempotent (re-run skips existing agents) | integration | `cargo test test_init_idempotent` | ❌ Wave 0 |
| SESS-02 | register adds agent to DB without squad.yml | unit | `cargo test test_register_agent` | ❌ Wave 0 |
| MSG-01 | send writes message to DB | unit | `cargo test test_send_creates_message` | ❌ Wave 0 |
| MSG-02 | signal updates message status to completed | unit | `cargo test test_signal_updates_status` | ❌ Wave 0 |
| MSG-03 | duplicate signal does not corrupt state | unit | `cargo test test_signal_idempotent` | ❌ Wave 0 |
| MSG-04 | list filters by agent, status, limit | unit | `cargo test test_list_filters` | ❌ Wave 0 |
| MSG-05 | priority levels stored and ordered correctly | unit | `cargo test test_priority_ordering` | ❌ Wave 0 |
| MSG-06 | peek returns pending task for agent | unit | `cargo test test_peek_returns_pending` | ❌ Wave 0 |
| SAFE-01 | concurrent signals do not produce busy errors | integration | `cargo test test_concurrent_signals` | ❌ Wave 0 |
| SAFE-02 | send-keys uses -l flag (no injection) | unit/manual | `cargo test test_tmux_args_literal` | ❌ Wave 0 |
| SAFE-03 | agent launched via new-session direct command | unit | `cargo test test_launch_uses_new_session` | ❌ Wave 0 |
| SAFE-04 | SIGPIPE handler installed before I/O | unit | `cargo test test_sigpipe_installed` | ❌ Wave 0 |

**Note on tmux tests (SAFE-02, SAFE-03):** Unit tests should verify the `Command` args vector produced by tmux functions, not actually invoke tmux. Integration tests for SAFE-01 require a real SQLite file and can run with `cargo test --features integration`.

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test -- --include-ignored`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/` directory — create as Rust integration test directory
- [ ] `tests/helpers.rs` — shared test DB setup (in-memory SQLite pool)
- [ ] `tests/test_messages.rs` — covers MSG-01 through MSG-06
- [ ] `tests/test_concurrent.rs` — covers SAFE-01
- [ ] `src/tmux.rs` test module — unit tests for Command args (SAFE-02, SAFE-03)
- [ ] Framework install: none needed — tokio-test already in dev-dependencies

---

## Sources

### Primary (HIGH confidence)
- `https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html` — WAL mode, busy_timeout, create_if_missing API
- `https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html` — Parser derive, global flag, subcommand patterns
- `https://docs.rs/sqlx/latest/sqlx/macro.migrate.html` — migrate! macro, embedded migrations
- `https://sqlite.org/wal.html` — SQLite WAL mode official documentation

### Secondary (MEDIUM confidence)
- `https://emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite/` — sqlx SQLite write transaction footgun, single-connection pool solution (verified against sqlx docs)
- `https://github.com/anthropics/claude-code/issues/23513` — tmux send-keys race condition, new-session direct command solution (multiple user confirmations)
- `https://crates.io/crates/serde-saphyr` — active maintenance, 0.0.17 latest version
- `https://rust-cli-recommendations.sunshowers.io/managing-colors-in-rust.html` — owo-colors recommendation for NO_COLOR + tty detection

### Tertiary (LOW confidence)
- WebSearch results about SIGPIPE handling — cross-referenced with official Rust issue tracker
- WebSearch results about serde-saphyr vs serde_yml tradeoffs — community forum posts, not official benchmarks

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Cargo.toml already defines core deps; sqlx/clap/serde all have excellent docs
- Architecture: HIGH — Patterns verified against official sqlx and clap docs
- Pitfalls: HIGH for sqlx write contention (documented official issue) and tmux race (active GitHub issue); MEDIUM for serde-saphyr stability (pre-1.0 API)

**Research date:** 2026-03-06
**Valid until:** 2026-06-06 (stable ecosystem; serde-saphyr pre-1.0 status should be re-checked)
