# Codebase Structure

**Analysis Date:** 2026-03-08

## Directory Layout

```
squad-station/
├── src/
│   ├── main.rs               # Binary entry point: SIGPIPE, tokio runtime, dispatch
│   ├── lib.rs                # Library crate: re-exports all modules for integration tests
│   ├── cli.rs                # clap CLI definitions: Cli, Commands enum, Priority enum
│   ├── config.rs             # squad.yml parsing, DB path resolution
│   ├── tmux.rs               # tmux subprocess wrapper (send-keys, launch, list-sessions, view)
│   ├── commands/
│   │   ├── mod.rs            # Re-exports all command modules
│   │   ├── init.rs           # `init` — parse config, register agents, launch tmux sessions
│   │   ├── send.rs           # `send` — write message to DB, inject task into tmux session
│   │   ├── signal.rs         # `signal` — mark message complete, notify orchestrator
│   │   ├── peek.rs           # `peek` — fetch highest-priority pending message for an agent
│   │   ├── list.rs           # `list` — list messages with optional agent/status/limit filters
│   │   ├── register.rs       # `register` — runtime agent registration (no tmux launch)
│   │   ├── agents.rs         # `agents` — list agents with tmux reconciliation
│   │   ├── status.rs         # `status` — project summary with pending message counts
│   │   ├── context.rs        # `context` — print Markdown agent roster for orchestrator context
│   │   ├── ui.rs             # `ui` — ratatui TUI dashboard (agents + messages panels)
│   │   └── view.rs           # `view` — tmux tiled window of all live agent sessions
│   └── db/
│       ├── mod.rs            # Pool setup: WAL mode, single connection, auto-migrate
│       ├── agents.rs         # Agent struct + CRUD (insert_agent, get_agent, list_agents, update_agent_status)
│       ├── messages.rs       # Message struct + CRUD (insert_message, update_status, list_messages, peek_message)
│       └── migrations/
│           ├── 0001_initial.sql      # agents + messages tables, indexes
│           └── 0002_agent_status.sql # status + status_updated_at columns on agents
├── tests/
│   ├── helpers.rs            # setup_test_db() — isolated temp SQLite DB with migrations
│   ├── test_commands.rs      # Unit tests for command logic (no tmux required)
│   ├── test_db.rs            # DB layer tests (CRUD, edge cases)
│   ├── test_integration.rs   # Integration tests for full command flows
│   ├── test_lifecycle.rs     # Agent lifecycle tests (requires tmux)
│   ├── test_views.rs         # TUI and view tests (requires tmux)
│   ├── test_cli.rs           # CLI argument parsing tests
│   └── e2e_cli.sh            # End-to-end shell tests against the release binary
├── hooks/
│   ├── claude-code.sh        # Claude Code Stop hook: detects session, calls `signal`
│   └── gemini-cli.sh         # Gemini CLI AfterAgent hook: same pattern
├── docs/                     # Project documentation and retrospectives
├── .planning/                # GSD planning artifacts (milestones, phases, codebase maps)
│   ├── codebase/             # This directory — codebase analysis documents
│   ├── milestones/           # Phase plans per milestone
│   ├── phases/               # Active phase tracking
│   └── research/             # Research documents
├── .claude/                  # Claude Code configuration and GSD tooling
├── .gemini/                  # Gemini CLI configuration
├── Cargo.toml                # Package manifest and dependencies
├── Cargo.lock                # Locked dependency versions
├── build.rs                  # Build script (sqlx compile-time query check support)
├── squad.yml                 # Example squad configuration for this project
└── CLAUDE.md                 # Project instructions for Claude Code
```

## Directory Purposes

**`src/commands/`:**
- Purpose: One file per CLI subcommand, each exposing a single `pub async fn run(...)` function
- Contains: Business logic for each operation; all follow the same pattern (load config → connect DB → validate → act → output)
- Key files: `send.rs` (primary write path), `signal.rs` (hook-triggered completion), `ui.rs` (TUI dashboard)

**`src/db/`:**
- Purpose: All database interaction — pool creation, migration, and typed CRUD operations
- Contains: `SqlitePool` type alias, `Agent` and `Message` structs derived with `sqlx::FromRow` and `serde::Serialize`
- Key files: `mod.rs` (connect function), `migrations/` (SQL applied automatically on connect)

**`src/db/migrations/`:**
- Purpose: Versioned SQL schema files applied via `sqlx::migrate!()`
- Contains: Sequential numbered `.sql` files
- Generated: No — hand-authored
- Committed: Yes — required for `sqlx::migrate!()` macro at compile time

**`tests/`:**
- Purpose: All test code — unit, integration, and e2e
- Contains: Rust test files importable as integration tests; `helpers.rs` shared test setup; `e2e_cli.sh` for binary-level tests
- Key files: `helpers.rs` (must be imported by every Rust test file), `test_integration.rs` (largest test file)

**`hooks/`:**
- Purpose: Shell scripts registered with AI providers to call `squad-station signal` on task completion
- Contains: Provider-specific hook scripts
- Key files: `claude-code.sh` (Claude Code Stop hook), `gemini-cli.sh` (Gemini AfterAgent hook)

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry; SIGPIPE setup, tokio runtime, CLI parse, command dispatch
- `src/lib.rs`: Library crate surface; re-exports for integration tests

**Configuration:**
- `squad.yml`: Project-local squad configuration (project name, orchestrator, worker agents)
- `src/config.rs`: Config struct definitions and `load_config()` / `resolve_db_path()`
- `Cargo.toml`: Rust dependencies
- `build.rs`: Build script for sqlx offline query verification

**Core Logic:**
- `src/commands/send.rs`: Task dispatch path (validates agent, writes DB, injects tmux)
- `src/commands/signal.rs`: Completion signal path (guards, DB update, orchestrator notification)
- `src/tmux.rs`: All tmux subprocess calls, with private argument-builder functions for testability
- `src/db/agents.rs`: Agent CRUD — `insert_agent` (INSERT OR IGNORE), `update_agent_status`
- `src/db/messages.rs`: Message CRUD — `peek_message` (priority-ordered), `update_status` (subquery idempotency)

**Testing:**
- `tests/helpers.rs`: `setup_test_db()` — creates isolated temp file SQLite DB per test call
- `tests/e2e_cli.sh`: Full CLI black-box tests requiring compiled release binary

**Data Storage:**
- `~/.agentic-squad/<project-name>/station.db`: Per-project SQLite database (runtime, not in repo)
- `src/db/migrations/`: Schema SQL (in repo, embedded at compile time)

## Naming Conventions

**Files:**
- Rust source: `snake_case.rs` matching the subcommand name (e.g., `send.rs` for `send` subcommand)
- SQL migrations: `NNNN_description.sql` (four-digit sequence number prefix)
- Shell hooks: `<provider-name>.sh` (e.g., `claude-code.sh`)
- Test files: `test_<area>.rs` (e.g., `test_db.rs`, `test_integration.rs`)

**Functions:**
- Command handlers: always named `run(...)` inside their module
- DB functions: `verb_noun` pattern (e.g., `insert_agent`, `get_agent`, `list_agents`, `update_agent_status`, `peek_message`)
- Tmux functions: verb describing the tmux action (e.g., `send_keys_literal`, `session_exists`, `launch_agent`)

**Types:**
- Structs: `PascalCase` (e.g., `SquadConfig`, `AgentConfig`, `Agent`, `Message`)
- Enums: `PascalCase` with `PascalCase` variants (e.g., `Commands::Init`, `Priority::Urgent`)

**Modules:**
- All module names are `snake_case`
- Command submodule names exactly match the CLI subcommand name

## Where to Add New Code

**New CLI Subcommand:**
1. Add variant to `Commands` enum in `src/cli.rs`
2. Create `src/commands/<name>.rs` with `pub async fn run(...) -> anyhow::Result<()>`
3. Add `pub mod <name>;` to `src/commands/mod.rs`
4. Add match arm in `src/main.rs` `run()` function
5. Add tests to `tests/test_commands.rs` or a new `tests/test_<name>.rs`

**New Database Column/Table:**
1. Create `src/db/migrations/NNNN_description.sql` (next sequential number)
2. Update relevant struct in `src/db/agents.rs` or `src/db/messages.rs`
3. Add/update CRUD functions as needed
4. Migrations run automatically on `db::connect()`

**New tmux Operation:**
1. Add private argument-builder function in `src/tmux.rs` returning `Vec<String>`
2. Add public API function calling `Command::new("tmux").args(...)`
3. Add unit test for the argument-builder (no tmux required)

**Shared Utilities:**
- DB helpers: `src/db/agents.rs` or `src/db/messages.rs`
- tmux helpers: `src/tmux.rs`
- Config helpers: `src/config.rs`
- No general-purpose `utils.rs` exists; keep helpers in their relevant domain module

## Special Directories

**`.planning/`:**
- Purpose: GSD workflow artifacts (milestones, phases, codebase analysis)
- Generated: No — hand-authored by planning commands
- Committed: Yes

**`.claude/` and `.gemini/`:**
- Purpose: Provider-specific configuration, GSD tooling scripts, and skill definitions
- Generated: Partially (GSD tooling is installed here)
- Committed: Yes

**`target/`:**
- Purpose: Cargo build output (debug and release binaries)
- Generated: Yes
- Committed: No (in `.gitignore`)

**`~/.agentic-squad/<project>/`:**
- Purpose: Per-project runtime data directory holding `station.db`
- Generated: Yes (created by `db::connect()` via `std::fs::create_dir_all`)
- Committed: No (lives outside the repo)

---

*Structure analysis: 2026-03-08*
