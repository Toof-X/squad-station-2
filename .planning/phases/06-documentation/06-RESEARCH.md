# Phase 6: Documentation - Research

**Researched:** 2026-03-08
**Domain:** Documentation accuracy — Rust CLI codebase audit against existing markdown docs
**Confidence:** HIGH (all findings are direct source-code reads, no inference required)

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DOCS-01 | `.planning/research/ARCHITECTURE.md` reflects current sqlx + flat module structure | Full gap analysis below: ARCHITECTURE.md describes rusqlite + subdirectory module layout, actual code uses sqlx async + flat files |
| DOCS-02 | `docs/PLAYBOOK.md` reflects correct CLI syntax and config format post-refactor | Full gap analysis below: PLAYBOOK.md retains v1.0 `provider`/`command`/nested `project:` syntax and positional send arg |
</phase_requirements>

---

## Summary

Phase 6 is a pure documentation-update phase. No code changes are required. Both target documents contain verifiably stale content that predates the v1.1 refactor completed in Phases 4 and 5.

**ARCHITECTURE.md** was written during initial design research (2026-03-06) and describes the _planned_ architecture: `rusqlite` + `rusqlite_migration` with module subdirectories (`src/db/agent_repo.rs`, `src/tui/`, `src/orchestrator/`). The actual implementation uses **sqlx** (async, WAL mode pool) with `sqlx::migrate!()` and **flat module files** (`src/db/agents.rs`, `src/db/messages.rs`, `src/tmux.rs`). The planned `orchestrator` and `tui` subdirectory modules do not exist; their functionality lives in `src/commands/context.rs` and `src/commands/ui.rs`.

**PLAYBOOK.md** was written for v1.0 and still shows: the `provider` field (renamed to `tool` in CONF-04/AGNT-03), the `command` field (removed in CONF-03), the nested `project: name:` format (flattened to `project:` string in CONF-01), and the positional task argument `send <agent> "task"` (replaced by `--body` flag in CLI-01). The `context` command example output is also outdated, showing a table format instead of the Markdown section-per-agent format implemented in CLI-03.

**Primary recommendation:** Rewrite both files directly from the verified source code. No library research needed — the ground truth is in `src/`.

---

## Gap Analysis

### DOCS-01: ARCHITECTURE.md vs Actual Code

The following table catalogs every stale claim in `.planning/research/ARCHITECTURE.md` and its correct replacement:

| Section | Stale Claim | Correct Reality | Source |
|---------|-------------|-----------------|--------|
| DB Layer diagram | `rusqlite` label | `sqlx` (async pool) | `src/db/mod.rs` line 4 |
| Module 3: `db` | `rusqlite` dependency | `sqlx::SqlitePool`, `sqlx::migrate!()` | `src/db/mod.rs` |
| Module 3: `db` | `migrations.rs` embedded as `M::up(sql)` via `rusqlite_migration` | `src/db/migrations/` SQL files, applied via `sqlx::migrate!("./src/db/migrations")` | `src/db/mod.rs` line 24 |
| Module 3: `db` | Files: `agent_repo.rs`, `message_repo.rs`, `models.rs`, `migrations.rs` | Actual files: `agents.rs`, `messages.rs` (no models.rs, no migrations.rs module) | `src/db/` directory |
| Module 3: `db` | `message_log` table | Actual table name is `messages` | `src/db/migrations/0001_initial.sql` |
| Module 3: `db` | Schema: `session TEXT`, `registered_at TEXT`, `last_signal_at TEXT` | Actual schema: no `session` column; columns are `created_at`, `status_updated_at`, `model`, `description`, `current_task`, `tool` (not `provider`) | `src/db/agents.rs` struct + migration 0003 |
| Module 4: `tmux` | Subdirectory: `src/tmux/mod.rs`, `src/tmux/send.rs`, etc. | Single flat file: `src/tmux.rs` | `src/` directory |
| Module 4: `tmux` | `tmux_interface` crate | Direct `std::process::Command` calls to `tmux` CLI (no crate wrapper) | `src/tmux.rs` |
| Module 5: `config` | Subdirectory: `src/config/mod.rs`, `src/config/types.rs` | Single flat file: `src/config.rs` | `src/` directory |
| Module 5: `config` | squad.yml shows `provider` field | Field is `tool` (CONF-04) | `src/config.rs` struct |
| Module 5: `config` | squad.yml shows `session` field | No `session` field; sessions are derived from agent name | `src/config.rs` struct |
| Module 5: `config` | squad.yml shows `command` field | `command` field removed (CONF-03) | `src/config.rs` struct — absent |
| Module 6: `tui` | Subdirectory: `src/tui/mod.rs`, `src/tui/state.rs`, `src/tui/components/` | Single file: `src/commands/ui.rs` (tui lives in commands layer) | `src/commands/` directory |
| Module 7: `orchestrator` | Separate module `src/orchestrator/` | No separate module; context generation is `src/commands/context.rs` | `src/commands/` directory |
| Pattern 2 | `rusqlite::Connection`, `conn.transaction()` synchronous pattern | `sqlx::SqlitePool`, async functions (`await`), `sqlx::query().execute(&pool).await` | `src/db/agents.rs` |
| Pattern 4 (migration) | `migrations::run(&mut conn)` via `rusqlite_migration` | `sqlx::migrate!("./src/db/migrations").run(&pool).await` | `src/db/mod.rs` |
| Data Flow diagrams | `rusqlite SELECT →`, `rusqlite UPDATE` | `sqlx query_as → .fetch_optional(pool).await`, async throughout | `src/commands/send.rs`, `src/db/agents.rs` |
| Flow 2 (signal) | `orchestrator::generate_context()` on signal completion | No context file generation on signal; signal sends tmux notification string to orchestrator session | `src/commands/signal.rs` |
| Build Order | Phase 1: config module, Phase 2: tmux + orchestrator modules | Modules do not correspond — no build order document needed (it was planning scaffolding, not doc) | N/A |

**What ARCHITECTURE.md should become:** A description of the _actual_ implemented structure with the correct module layout, sqlx pool pattern, flat file structure, real DB schema, and signal flow.

---

### DOCS-02: PLAYBOOK.md vs Actual Code

| Section | Stale Content | Correct Content | Source |
|---------|--------------|-----------------|--------|
| Section 1 squad.yml | `project:\n  name: my-app` (nested struct) | `project: my-app` (flat string, CONF-01) | `src/config.rs` SquadConfig |
| Section 1 squad.yml | `provider: claude-code` field | `tool: claude-code` field (CONF-04) | `src/config.rs` AgentConfig |
| Section 1 squad.yml | `command: "claude --dangerously-skip-permissions"` | No `command` field (CONF-03 removed it) | `src/config.rs` AgentConfig |
| Section 1 fields table | `*.provider` required field | `*.tool` required field | `src/config.rs` |
| Section 1 fields table | `*.command` required field | Field does not exist | `src/config.rs` |
| Section 1 fields table | `project.db_path` optional | Field not in SquadConfig struct (DB path controlled by env var `SQUAD_STATION_DB` only) | `src/config.rs` resolve_db_path |
| Section 1 fields table | `*.name` is the tmux session name | `*.name` acts as role suffix; full agent name is auto-prefixed as `<project>-<tool>-<role>` (CLI-02) | `src/commands/init.rs` |
| Section 2 init output | `"launched": 3` JSON output format | Same format — still valid | `src/commands/init.rs` |
| Section 4 send syntax | `squad-station send frontend-dev "Build the login page"` (positional arg) | `squad-station send frontend-dev --body "Build the login page"` (named flag, CLI-01) | `src/cli.rs` Send variant |
| Section 4 send examples | All three examples use positional task | All must use `--body` flag | `src/cli.rs` |
| Section 8 register | `--provider claude-code` flag | `--tool claude-code` flag (CONF-04) | `src/cli.rs` Register variant |
| Section 9 context | Output shows a table: `| Agent | Role | Status | Send Command |` | Output is Markdown sections per agent: `## agentname (model)`, description, role/status, `→ squad-station send <name> --body "..."` (CLI-03) | `src/commands/context.rs` |
| Workflow diagram | `squad-station send agent "task"` | `squad-station send agent --body "task"` | `src/commands/send.rs` |
| Workflow diagram | `[SIGNAL] agent=X status=completed task_id=Y` signal format | `<agent> completed <msg-id>` signal format (SIG-01) | `src/commands/signal.rs` line 77 |
| Command Reference table | `send <agent> <task>` | `send <agent> --body <task>` | `src/cli.rs` |
| Command Reference table | `register <name>` with `--provider` flag | `register <name>` with `--tool` flag | `src/cli.rs` |

**squad.yml example that is NOW correct (for use in PLAYBOOK rewrite):**

```yaml
project: my-app

orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5           # optional
  description: "Lead orchestrator" # optional

agents:
  - name: frontend                  # role suffix; full name = my-app-claude-code-frontend
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5        # optional
    description: "Frontend specialist"
  - name: backend
    tool: gemini
    role: worker
```

**Context output format now correct (for PLAYBOOK section 9 rewrite):**

```
# Squad Station -- Agent Roster

## Available Agents

## my-app-claude-code-frontend (claude-sonnet-4-5)

Frontend specialist

Role: worker | Status: idle

→ squad-station send my-app-claude-code-frontend --body "..."

---

## Usage

Send a task to an agent:
```
squad-station send <agent> --body "<task description>"
```
```

---

## Architecture Patterns

### Actual Module Structure (what ARCHITECTURE.md should describe)

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

### Actual DB Schema (post-migration 0003)

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

### sqlx Pool Pattern (actual implementation)

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

### Signal Notification Format (SIG-01)

The signal notification injected into the orchestrator's tmux session is a plain string:

```
<agent> completed <msg-id>
```

Example: `my-app-claude-code-frontend completed 8c2e9e2f-...`

This is NOT `[SIGNAL] agent=X status=completed task_id=Y` (the old format shown in PLAYBOOK.md).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Stale content inventory | Manual diff approach | Read source files directly — they are the single source of truth |
| New squad.yml examples | Invented configs | Use fields exactly as in `src/config.rs` AgentConfig struct |
| Command syntax examples | Guessed flags | Use fields exactly as in `src/cli.rs` Commands enum |

---

## Common Pitfalls

### Pitfall 1: Residual `provider` References
**What goes wrong:** Updating most occurrences but missing some (e.g., the fields table in PLAYBOOK Section 1 or the register command flags).
**How to avoid:** Search PLAYBOOK.md for every occurrence of `provider` before closing — there should be zero remaining.

### Pitfall 2: ARCHITECTURE.md Module Layout
**What goes wrong:** Describing the planned subdirectory layout (`src/db/agent_repo.rs`, `src/tui/`) rather than the actual flat layout.
**How to avoid:** The flat layout is verified: `src/tmux.rs` is one file, `src/db/` has only `mod.rs`, `agents.rs`, `messages.rs`, and `migrations/`.

### Pitfall 3: `command` Field Confusion
**What goes wrong:** Documenting `command` as "optional" (since it's still a legacy column in the DB) rather than "removed from config entirely."
**Correct:** The `command` field does NOT appear in `squad.yml`. It was removed (CONF-03). The DB column still exists but is set to `''` programmatically and ignored at the application layer.

### Pitfall 4: Agent Naming Convention
**What goes wrong:** Describing the `name` field as "the final agent name" — but in v1.1 it is only the role suffix. The full name becomes `<project>-<tool>-<role_suffix>`.
**Correct:** squad.yml `name` field is optional and acts as the role portion. If absent, the `role` field is used. Full name is auto-prefixed in `init.rs`.

### Pitfall 5: Context Output Format
**What goes wrong:** Showing the old table format (`| Agent | Role | Status | Send Command |`) in PLAYBOOK Section 9.
**Correct:** Context output is Markdown sections (one `##` heading per agent) — see `src/commands/context.rs` for exact format.

---

## Code Examples

### squad.yml — Current Valid Format
```yaml
# Source: src/config.rs SquadConfig + AgentConfig structs

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

### send CLI Syntax — Current Valid
```bash
# Source: src/cli.rs Send variant, --body flag
squad-station send my-app-claude-code-frontend --body "Build the login page"
squad-station send my-app-gemini-backend --body "Fix auth endpoint" --priority urgent
squad-station send my-app-claude-code-frontend --body "Add validation" --priority high --json
```

### register CLI Syntax — Current Valid
```bash
# Source: src/cli.rs Register variant — --tool not --provider
squad-station register reviewer --tool claude-code --role reviewer
```

### Signal Format — Current Valid
```
# Source: src/commands/signal.rs line 77
# Format: "{agent} completed {task_id}"
my-app-claude-code-frontend completed 8c2e9e2f-1234-...
```

---

## Validation Architecture

`nyquist_validation` is `true` in config.json — include this section.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust test + tokio (cargo test) |
| Config file | Cargo.toml (`[dev-dependencies]`) |
| Quick run command | `cargo test -- --test-output immediate 2>&1 \| tail -5` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| DOCS-01 | ARCHITECTURE.md describes sqlx + flat module structure | manual | N/A — doc review | N/A |
| DOCS-02 | PLAYBOOK.md shows `--body` syntax, `tool`/`model`/`description`, correct naming | manual | N/A — doc review | N/A |

Documentation accuracy is verified by human review against source code — not by automated tests. No test files need creating.

### Sampling Rate
- **Per task commit:** `cargo check` — confirms no code was accidentally modified
- **Per wave merge:** `cargo test` — full suite green (ensures doc edits did not touch code)
- **Phase gate:** `cargo test` green + manual review of both docs before `/gsd:verify-work`

### Wave 0 Gaps
None — no new test infrastructure needed for documentation-only changes.

---

## Sources

### Primary (HIGH confidence)

All findings are direct reads of source files. No inference, no external sources needed.

| File | What Was Verified |
|------|-------------------|
| `src/cli.rs` | All command signatures: `--body` flag on `send`, `--tool` on `register`, full Commands enum |
| `src/config.rs` | SquadConfig struct: `project: String` (flat), AgentConfig fields: `tool`, `model`, `description`, no `command` |
| `src/db/mod.rs` | sqlx pool, WAL mode, `sqlx::migrate!()`, single-writer max_connections=1 |
| `src/db/agents.rs` | Agent struct fields: `tool`, `model`, `description`, `current_task`, `command` (dead) |
| `src/db/migrations/0001_initial.sql` | Base schema: `agents`, `messages` table names |
| `src/db/migrations/0003_v11.sql` | v1.1 changes: `RENAME COLUMN provider TO tool`, new columns |
| `src/commands/init.rs` | Agent naming: `format!("{}-{}-{}", project, tool, role_suffix)` |
| `src/commands/send.rs` | `body` parameter (not positional); `--body` flag confirmed |
| `src/commands/signal.rs` | Signal format: `"{} completed {}"` (agent, task_id_str) |
| `src/commands/context.rs` | Context output: Markdown `##` sections per agent, not table |
| `src/` directory listing | Flat module structure confirmed: no `src/tui/`, no `src/orchestrator/`, no `src/tmux/` subdirs |
| `src/db/` directory listing | Flat DB module: `agents.rs`, `messages.rs`, `mod.rs`, `migrations/` |
| `.planning/research/ARCHITECTURE.md` | All stale claims identified by comparison |
| `docs/PLAYBOOK.md` | All stale content identified by comparison |

---

## Metadata

**Confidence breakdown:**
- Gap inventory (DOCS-01): HIGH — direct source code comparison, no ambiguity
- Gap inventory (DOCS-02): HIGH — direct source code comparison, no ambiguity
- Replacement content: HIGH — pulled verbatim from struct definitions and command handlers

**Research date:** 2026-03-08
**Valid until:** End of Phase 6 — research is bound to current codebase state, not library versions
