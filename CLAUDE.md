# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Squad Station is a stateless Rust CLI that routes messages between an AI orchestrator and N agents running in tmux sessions. It is provider-agnostic (Claude Code, Gemini CLI, etc.) and uses embedded SQLite (WAL mode) for persistence. Each project gets its own DB at `.squad/station.db` inside the project directory.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (binary at target/release/squad-station)
cargo test                     # Run all 164 unit + integration tests
cargo test test_name           # Run a single test by name
cargo test --test test_commands # Run a specific test file
./tests/e2e_cli.sh            # End-to-end CLI tests (requires release binary)
cargo check                    # Quick compilation check
```

**Local Binary Access:** A symlink at `~/.cargo/bin/squad-station` points to `target/release/squad-station`. After `cargo build --release`, the binary is immediately available via `squad-station` command—no manual installation needed.

## Architecture

**Layered design:** CLI commands → Database layer → SQLite (WAL)

- `src/main.rs` — Entry point: SIGPIPE handler, async runtime, command dispatch
- `src/cli.rs` — clap-based argument parsing (`Commands` enum defines all subcommands)
- `src/config.rs` — YAML config loading (`squad.yml`), DB path resolution
- `src/tmux.rs` — tmux session management: launch, send-keys, capture-pane, reconciliation
- `src/commands/` — One file per subcommand (init, send, signal, peek, list, register, agents, context, status, ui, view)
- `src/db/` — SQLite pool setup (`mod.rs`), agent CRUD (`agents.rs`), message CRUD (`messages.rs`)
- `src/db/migrations/` — SQL migration files, auto-applied via `sqlx::migrate!()`

**Key design decisions:**
- Single-writer SQLite pool (`max_connections=1`) with 5s busy_timeout
- `send-keys -l` (literal mode) to prevent shell injection via tmux
- Idempotent agent registration (`INSERT OR IGNORE`) and signal handling (most-recent-pending)
- TUI (`ratatui`) drops pool after each fetch to prevent WAL starvation
- Hook scripts in `hooks/` detect agent task completion per provider

## Testing

Tests use `tests/helpers.rs` → `setup_test_db()` which creates an isolated temp SQLite DB with migrations. All tests are async (tokio). Integration tests that need tmux are in `test_views.rs` and `test_lifecycle.rs`.

## Database Schema

Two tables: `agents` (id, name, provider, role, command, status) and `messages` (id, agent_name, task, status, priority, timestamps). Messages reference agents by name. Priority ordering: urgent > high > normal.

<skills_system priority="1">

## Available Skills

<!-- SKILLS_TABLE_START -->
<usage>
When users ask you to perform tasks, check if any of the available skills below can help complete the task more effectively. Skills provide specialized capabilities and domain knowledge.

How to use skills:
- Invoke: `skillkit read <skill-name>` or `npx skillkit read <skill-name>`
- The skill content will load with detailed instructions on how to complete the task
- Base directory provided in output for resolving bundled resources (references/, scripts/, assets/)

Usage notes:
- Only use skills listed in <available_skills> below
- Do not invoke a skill that is already loaded in your context
- Each skill invocation is stateless
</usage>

<available_skills>

<skill>
<name>qa-test-planner</name>
<description>Generate comprehensive test plans, manual test cases, regression test suites, and bug reports for QA engineers. Includes Figma MCP integration for design validation.</description>
<location>project</location>
</skill>

<skill>
<name>test-driven-development</name>
<description>Use when implementing any feature or bugfix, before writing implementation code</description>
<location>project</location>
</skill>

<skill>
<name>webapp-testing</name>
<description>Toolkit for interacting with and testing local web applications using Playwright. Supports verifying frontend functionality, debugging UI behavior, capturing browser screenshots, and viewing browser logs.</description>
<location>project</location>
</skill>

</available_skills>
<!-- SKILLS_TABLE_END -->

</skills_system>
