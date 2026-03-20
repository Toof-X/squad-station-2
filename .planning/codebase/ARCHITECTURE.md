# Architecture

**Analysis Date:** 2026-03-08

## Pattern Overview

**Overall:** Stateless CLI with layered command dispatch — each invocation is fully independent, reads `squad.yml` from the current working directory, resolves the DB path, and exits.

**Key Characteristics:**
- No daemon or long-running server process; the binary is called on demand by the user or hook scripts
- Single source of truth is SQLite (WAL mode) at `~/.agentic-squad/<project-name>/station.db`
- Agent identity is the tmux session name; tmux is the runtime environment for agents, not a managed subprocess
- Provider-agnostic: the binary does not know which AI (Claude, Gemini, etc.) runs in a session — it only routes messages and injects text via tmux
- Dual output mode: every command supports `--json` (machine-readable) or plain-text (human-readable, with ANSI color when stdout is a terminal)

## Layers

**CLI Layer:**
- Purpose: Parse arguments and route to the correct command handler
- Location: `src/cli.rs`, `src/main.rs`
- Contains: `Cli` struct (clap `Parser`), `Commands` enum (clap `Subcommand`), `Priority` enum
- Depends on: nothing internal
- Used by: `src/main.rs` dispatch loop

**Command Layer:**
- Purpose: Orchestrate a single operation — load config, connect to DB, call tmux, produce output
- Location: `src/commands/` (one file per subcommand)
- Contains: one public `run(...)` async function per module
- Depends on: `config`, `db`, `tmux` modules
- Used by: `src/main.rs` `run()` function

**Config Layer:**
- Purpose: Parse `squad.yml` and resolve the per-project DB path
- Location: `src/config.rs`
- Contains: `SquadConfig`, `ProjectConfig`, `AgentConfig` structs; `load_config()`, `resolve_db_path()`
- Depends on: filesystem, `dirs` crate for `~`
- Used by: every command handler

**Database Layer:**
- Purpose: All SQLite reads and writes; schema migration on connect
- Location: `src/db/mod.rs` (pool setup), `src/db/agents.rs` (agent CRUD), `src/db/messages.rs` (message CRUD)
- Contains: `Pool` type alias, `connect()`, `Agent` struct, `Message` struct, CRUD functions
- Depends on: `sqlx`, migration SQL files in `src/db/migrations/`
- Used by: command layer only

**Tmux Layer:**
- Purpose: Wrap all `tmux` subprocess calls with a safe, testable API
- Location: `src/tmux.rs`
- Contains: `send_keys_literal()`, `session_exists()`, `list_live_session_names()`, `launch_agent()`, `create_view_window()`, `kill_window()`
- Depends on: `std::process::Command` (no crate dependencies)
- Used by: command layer; argument-builder functions are private and tested independently

**TUI Layer:**
- Purpose: Interactive terminal dashboard for monitoring agents and messages
- Location: `src/commands/ui.rs`
- Contains: `App` state struct, `draw_ui()`, `fetch_snapshot()` (read-only DB connection per refresh), event loop
- Depends on: `ratatui`, `crossterm`, `db` module
- Used by: `commands::ui::run()` entry point; drops pool after each fetch to avoid WAL starvation

## Data Flow

**Task Dispatch (orchestrator → worker):**
1. Orchestrator calls `squad-station send <agent> "<task>" [--priority urgent|high|normal]`
2. `commands::send::run()` loads `squad.yml` via `config::load_config()`
3. Resolves DB path via `config::resolve_db_path()` and connects with `db::connect()`
4. Validates agent exists in `agents` table via `db::agents::get_agent()`
5. Confirms agent tmux session is alive via `tmux::session_exists()`
6. Writes message to `messages` table with status `pending` via `db::messages::insert_message()`
7. Marks agent status `busy` via `db::agents::update_agent_status()`
8. Injects task text into agent tmux session via `tmux::send_keys_literal()` (literal `-l` flag prevents shell injection)
9. Outputs result as JSON or text

**Task Completion (worker → orchestrator):**
1. AI provider finishes a response; provider hook script fires (e.g., `hooks/claude-code.sh`)
2. Hook detects `TMUX_PANE`, resolves session name, calls `squad-station signal <agent>`
3. `commands::signal::run()` applies four guards (not in tmux, config failure, unregistered agent, orchestrator self-signal) — all exit `Ok(())` silently
4. Marks most-recent pending message `completed` via `db::messages::update_status()` (idempotent subquery)
5. Retrieves orchestrator via `db::agents::get_orchestrator()` and sends notification string to orchestrator tmux session via `tmux::send_keys_literal()`
6. Marks worker agent status back to `idle`

**Status Reconciliation:**
- Commands `agents`, `status`, and `context` reconcile DB agent status against live tmux sessions on every call
- If tmux session gone but status is not `dead`: update to `dead`
- If tmux session present but status is `dead`: auto-revive to `idle`

**State Management:**
- All persistent state lives in SQLite; nothing is held in process memory between CLI invocations
- Agent lifecycle status: `idle` → `busy` (on send) → `idle` (on signal) / `dead` (session gone)
- Message lifecycle: `pending` → `completed` (on signal)

## Key Abstractions

**Agent:**
- Purpose: Represents a registered AI provider session (orchestrator or worker)
- Examples: `src/db/agents.rs` (`Agent` struct), `src/config.rs` (`AgentConfig`)
- Pattern: Name is the primary key for agent lookup AND the tmux session name; UUID `id` is stored but name is used in all joins

**Message:**
- Purpose: A task routed to an agent, with priority and lifecycle status
- Examples: `src/db/messages.rs` (`Message` struct)
- Pattern: References agents by `agent_name` (TEXT FK). Priority ordering enforced in SQL: `urgent=1`, `high=2`, `normal=3`

**Priority:**
- Purpose: Three-tier task urgency (normal < high < urgent)
- Examples: `src/cli.rs` (`Priority` enum), `src/db/messages.rs` (`peek_message` SQL CASE expression)
- Pattern: Stored as lowercase text string in DB; enum implements `Display` for serialization

**Hook Script:**
- Purpose: Bridge between provider lifecycle events and the signal command
- Examples: `hooks/claude-code.sh`, `hooks/gemini-cli.sh`
- Pattern: Shell script registered as provider AfterAgent/Stop hook; always exits 0 to avoid blocking the AI provider; all guard logic is in Rust binary

## Entry Points

**Binary Entry:**
- Location: `src/main.rs`
- Triggers: CLI invocation (`squad-station <subcommand>`)
- Responsibilities: Install SIGPIPE handler, start tokio runtime, parse CLI, dispatch to command module

**Library Crate:**
- Location: `src/lib.rs`
- Triggers: Integration test imports (`use squad_station::...`)
- Responsibilities: Re-export all modules so `tests/` can import them without duplicating code

**Hook Entry:**
- Location: `hooks/claude-code.sh`, `hooks/gemini-cli.sh`
- Triggers: AI provider lifecycle event (Stop/AfterAgent)
- Responsibilities: Detect tmux session, call `squad-station signal <agent>`, always exit 0

## Error Handling

**Strategy:** `anyhow::Result<()>` propagated through all command handlers; fatal errors print to `stderr` and exit non-zero via `std::process::exit(1)` in `main.rs`.

**Patterns:**
- Signal command uses explicit guard pattern: four early-return `Ok(())` guards before any writes, ensuring hook invocations never fail the AI provider
- DB operations use `?` operator throughout; no silent swallowing except in signal guards
- TUI refresh errors keep stale data and continue the loop rather than crashing
- `init` continues on partial agent launch failure; only fails if ALL agents fail
- Tmux interactions use `bail!` on non-zero subprocess exit codes

## Cross-Cutting Concerns

**Logging:** No structured logging framework; warnings go to `stderr` with prefix `squad-station: warning:`, info output to `stdout`
**Validation:** Agent existence validated against DB before tmux or message operations; tmux session validated before send
**Authentication:** None — single-user local tool; DB file permissions are the only access control
**Idempotency:** Agent registration uses `INSERT OR IGNORE`; `init` skips already-running sessions; signal uses subquery-based UPDATE to avoid double-completion; agents command auto-reconciles stale status on every read

---

*Architecture analysis: 2026-03-08*
