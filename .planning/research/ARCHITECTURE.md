# Architecture Reference

**Project:** Squad Station
**Domain:** Stateless Rust CLI — tmux message router with embedded SQLite
**Last updated:** 2026-03-08 (post-v1.1 rewrite — reflects actual implemented codebase)

---

## Overview

Squad Station is a **stateless Rust CLI binary** that routes messages between an AI orchestrator and N agents running in tmux sessions. Every invocation starts fresh, reads from SQLite, executes one action, and exits. There is no daemon, no shared memory, no long-running process.

- Provider-agnostic: works with Claude Code, Gemini CLI, or any AI tool that runs in a tmux session
- Uses **sqlx** (async) for SQLite access in WAL mode
- Each project gets its own DB at `~/.agentic-squad/<project-name>/station.db`
- Flat module files: src/tmux.rs is a single file (no src/tui/, src/orchestrator/, or src/tmux/ subdirectories)

---

## Module Layout

```
src/
├── main.rs           -- Entry point: SIGPIPE handler, tokio runtime, command dispatch
├── cli.rs            -- clap Commands enum: all subcommands with args
├── config.rs         -- SquadConfig / AgentConfig structs, load_config(), resolve_db_path()
├── tmux.rs           -- Direct tmux shell-out: session_exists(), launch_agent(), send_keys_literal()
├── lib.rs            -- Re-exports for integration tests
├── commands/
│   ├── mod.rs        -- mod declarations
│   ├── init.rs       -- Register agents + launch tmux sessions from squad.yml
│   ├── send.rs       -- Insert message, mark agent busy, inject into tmux
│   ├── signal.rs     -- Mark message completed, notify orchestrator, reset agent idle
│   ├── agents.rs     -- List agents with tmux reconciliation
│   ├── context.rs    -- Generate orchestrator Markdown context from live agent list
│   ├── list.rs       -- Query messages with filters
│   ├── peek.rs       -- Fetch highest-priority pending message for an agent
│   ├── register.rs   -- Runtime agent registration
│   ├── status.rs     -- Project + agent summary
│   ├── ui.rs         -- ratatui TUI event loop (read-only dashboard)
│   └── view.rs       -- tmux tiled view builder
└── db/
    ├── mod.rs         -- connect(): SqlitePool with WAL mode, single writer, sqlx::migrate!()
    ├── agents.rs      -- Agent struct (sqlx::FromRow), insert_agent(), get_agent(), list_agents(), get_orchestrator(), update_agent_status()
    ├── messages.rs    -- Message struct, insert_message(), update_status(), peek_message()
    └── migrations/
        ├── 0001_initial.sql    -- agents + messages base tables
        ├── 0002_agent_status.sql -- status_updated_at column
        └── 0003_v11.sql        -- v1.1 schema: tool rename, model/description/current_task, from_agent/to_agent/type/completed_at
```

---

## Key Dependencies

- **sqlx 0.7** — async SQLite pool, compile-time query checking, `sqlx::migrate!()`
- **clap** — argument parsing, `Commands` enum defines all subcommands
- **ratatui** — TUI dashboard in `src/commands/ui.rs`
- **anyhow** — error propagation throughout commands layer and `main.rs`
- **tokio** — async runtime, single-threaded
- **uuid** — message IDs
- **serde / serde_yaml** — `squad.yml` deserialization into `SquadConfig`
- **std::process::Command** — direct tmux CLI calls (no tmux crate wrapper)

---

## Database Layer

### Connection Pool

```rust
// src/db/mod.rs — connect() is async, returns SqlitePool
pub async fn connect(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)  // single writer — prevents async WAL deadlock
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;
    Ok(pool)
}
```

Key properties:
- WAL mode for concurrent reads
- `max_connections=1` — single writer to prevent WAL deadlock
- 5s `busy_timeout` — waits for locks instead of failing immediately
- Migrations applied automatically on every `connect()` call via `sqlx::migrate!()`

### Schema (post-migration 0003)

**agents table:**

```sql
CREATE TABLE agents (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,  -- <project>-<tool>-<role> convention
    tool            TEXT NOT NULL,          -- renamed from provider (AGNT-03)
    role            TEXT NOT NULL DEFAULT 'worker',
    command         TEXT NOT NULL,          -- legacy column, always '' (CONF-03 removed from config)
    created_at      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'idle',  -- idle|busy|dead
    status_updated_at TEXT NOT NULL,
    model           TEXT DEFAULT NULL,      -- AGNT-01
    description     TEXT DEFAULT NULL,      -- AGNT-01
    current_task    TEXT DEFAULT NULL       -- AGNT-02: FK to messages.id
);
```

**messages table:**

```sql
CREATE TABLE messages (
    id          TEXT PRIMARY KEY,
    agent_name  TEXT NOT NULL,              -- target agent name (legacy backcompat)
    task        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending|processing|completed|failed
    priority    TEXT NOT NULL DEFAULT 'normal',   -- normal|high|urgent
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    from_agent  TEXT DEFAULT NULL,          -- MSGS-01
    to_agent    TEXT DEFAULT NULL,          -- MSGS-01
    type        TEXT NOT NULL DEFAULT 'task_request',  -- MSGS-02: task_request|task_completed|notify
    completed_at TEXT DEFAULT NULL          -- MSGS-04
);
```

### Migration Files

- **0001_initial.sql** — `agents` + `messages` base tables
- **0002_agent_status.sql** — adds `status_updated_at` column to agents
- **0003_v11.sql** — v1.1 schema: `RENAME COLUMN provider TO tool`, adds `model`, `description`, `current_task` to agents; adds `from_agent`, `to_agent`, `type`, `completed_at` to messages

---

## Config Format

```yaml
# squad.yml — current valid format (source: src/config.rs SquadConfig + AgentConfig)

project: my-app                      # CONF-01: plain string, not nested struct

orchestrator:
  tool: claude-code                  # CONF-04: was 'provider'
  role: orchestrator
  model: claude-opus-4-5             # CONF-02: optional
  description: "Lead orchestrator"   # CONF-02: optional
  # NO command field (CONF-03: removed)

agents:
  - name: frontend                   # CLI-02: acts as role suffix
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Frontend UI specialist"
  - name: backend
    tool: gemini
    role: worker
```

Fields:
- `project` — plain string (not nested struct)
- `tool` — the AI provider/CLI tool (not `provider`)
- `model`, `description` — optional per-agent metadata
- No `command` field — removed in CONF-03
- No `session` field — sessions are derived from agent name

---

## Agent Naming

The `name` field in `squad.yml` acts as the **role suffix**. The full agent name is auto-prefixed in `src/commands/init.rs` using the pattern:

```
<project>-<tool>-<role_suffix>
```

Example: with `project: my-app`, `tool: claude-code`, `name: frontend` → full name is `my-app-claude-code-frontend`.

This full name is used as:
- The tmux session name
- The `name` column in the `agents` DB table
- The agent identifier in all CLI commands

---

## Key Command Flows

### send flow

```
CLI receives --body flag (named, not positional)
  └─> insert_message() into messages table
  └─> update agent status to 'busy' in agents table
  └─> tmux send-keys -l (literal mode) to agent's tmux session
```

`send-keys -l` (literal mode) prevents shell injection — the message is never interpreted by the shell.

### signal flow

```
signal command receives agent name + msg-id
  └─> update message status to 'completed' in messages table
  └─> update agent status to 'idle' in agents table
  └─> inject plain string into orchestrator's tmux session:
        "<agent> completed <msg-id>"
```

Signal format example: `my-app-claude-code-frontend completed 8c2e9e2f-1234-...`

This is NOT the old `[SIGNAL] agent=X status=completed task_id=Y` format.

### init flow

```
Load squad.yml → SquadConfig
  └─> derive full agent names: <project>-<tool>-<role_suffix>
  └─> INSERT OR IGNORE into agents table (idempotent)
  └─> launch tmux sessions for each agent
```

---

## Design Properties

| Property | Value |
|----------|-------|
| Execution model | Stateless — one invocation = one action, then exit |
| DB access | sqlx async pool, single writer, WAL mode |
| tmux integration | Direct `std::process::Command` calls (no crate wrapper) |
| Priority ordering | urgent > high > normal (messages table) |
| TUI behavior | `ratatui` drops pool after each fetch to prevent WAL starvation |
| Hook scripts | `hooks/` directory — detect agent task completion per provider |
| DB location | `~/.agentic-squad/<project-name>/station.db` |
| DB override | `SQUAD_STATION_DB` env var (checked in `resolve_db_path`) |
