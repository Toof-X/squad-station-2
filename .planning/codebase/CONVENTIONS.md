# Coding Conventions

**Analysis Date:** 2026-03-08

## Naming Patterns

**Files:**
- `snake_case` for all Rust source files: `send.rs`, `agents.rs`, `mod.rs`
- One file per subcommand under `src/commands/`
- Test files named `test_<area>.rs` under `tests/`: `test_db.rs`, `test_cli.rs`, `test_integration.rs`, `test_lifecycle.rs`, `test_views.rs`
- Shared test utilities in `tests/helpers.rs`

**Functions:**
- `snake_case` for all functions: `insert_agent`, `list_messages`, `send_keys_literal`, `setup_test_db`
- Public command handlers always named `run(...)` in every `src/commands/*.rs` module
- Private helpers use descriptive `snake_case`: `format_status_with_duration`, `colorize_agent_status`, `pad_colored`
- Private argument builders use `_args` suffix: `send_keys_args`, `enter_args`, `launch_args`

**Variables:**
- `snake_case` for all variables: `agent_record`, `db_path`, `priority_str`, `rows_affected`
- Boolean flags use descriptive names: `session_alive`, `json`

**Types and Structs:**
- `PascalCase` for all types: `Agent`, `Message`, `SquadConfig`, `AgentConfig`, `StatusOutput`
- Derive macros listed in consistent order: `Debug` first, then trait impls: `#[derive(Debug, sqlx::FromRow, serde::Serialize)]`
- Local-scope output structs (e.g., `StatusOutput`, `AgentStatusSummary`) are private (`struct`, not `pub struct`) and defined at module top

**Enums:**
- `PascalCase` for enum names: `Priority`, `Commands`, `FocusPanel`
- `PascalCase` for variants: `Normal`, `High`, `Urgent`, `AgentPanel`, `MessagePanel`

## Code Style

**Formatting:**
- Standard `rustfmt` (no custom `.rustfmt.toml` — defaults apply)
- 4-space indentation
- Trailing commas in multi-line structs and match arms

**Linting:**
- No `.clippy.toml` — standard clippy defaults apply
- No `#![allow(...)]` suppressions detected in source files

**Imports:**
- Grouped at file top with no blank lines within the group
- `use crate::{module1, module2}` brace-grouped for internal imports
- Standard library and external crates separated from `crate::` imports by a blank line

## Import Organization

**Order:**
1. External crates: `use anyhow::...`, `use owo_colors::...`
2. Internal crates: `use crate::{cli, config, db, tmux}`

**Examples from `src/commands/send.rs`:**
```rust
use anyhow::bail;
use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{cli, config, db, tmux};
```

**Examples from `src/commands/agents.rs`:**
```rust
use crate::{config, db, tmux};
use owo_colors::OwoColorize;
use owo_colors::Stream;
```

**Path Aliases:**
- None used. All paths are absolute module paths.

## Error Handling

**Strategy:** Propagate with `anyhow::Result<()>` — all command `run()` functions return `anyhow::Result<()>`.

**Patterns:**
- Use `?` operator to propagate errors up to `main()`, which prints with `eprintln!("Error: {e:#}")` and exits 1
- Use `bail!("message")` for early-exit error conditions: `bail!("Agent not found: {}", agent)`
- Use `anyhow!("message")` to construct errors inline: `anyhow!("Cannot determine home directory")`
- Never `panic!` or `.unwrap()` in production code paths — only in test code via `.expect("description")` and `.unwrap()`
- `signal` command is a special case: it exits 0 silently on guard conditions (never errors) to avoid breaking provider hooks

**Error Output:**
- Errors always go to `stderr` via `eprintln!`
- Normal output goes to `stdout` via `println!`

## Logging

**Framework:** None. No logging crate (no `tracing`, `log`, or `env_logger`).

**Patterns:**
- `println!` for all user-facing output
- `eprintln!` for error messages (prefixed with `"Error: "` in `main.rs`) and warnings (signal guards)
- No debug/trace logging in production code

## Comments

**When to Comment:**
- Doc comments (`///`) on all public functions in `src/db/`: explains contract, edge cases, and design decisions
- Inline `//` comments on non-obvious logic, especially SQL decisions and safety rationale
- Safety codes embedded in comments: `// SAFE-02`, `// SAFE-04`, `// MSG-03`, `// SESS-01` — these reference the requirements tracker
- Step-numbered comments in command `run()` functions: `// 1. Resolve DB path`, `// 2. Connect to DB`, etc.

**Example pattern from `src/db/messages.rs`:**
```rust
/// Mark the most recent pending message for this agent as completed.
/// Returns the number of rows affected (0 = already completed, not an error — MSG-03 idempotency).
///
/// Uses a subquery to identify the target row because SQLite does not support
/// `UPDATE ... ORDER BY ... LIMIT` without a compile-time flag (SQLITE_ENABLE_UPDATE_DELETE_LIMIT).
pub async fn update_status(pool: &SqlitePool, agent_name: &str) -> anyhow::Result<u64> {
```

## Function Design

**Size:** Command `run()` functions are 30–60 lines. Helper functions are 5–20 lines. The step-comment pattern keeps long functions scannable.

**Parameters:** DB functions take `&SqlitePool` as first argument, then string slices (`&str`). Command functions take owned `String` and primitives matching the CLI arg types.

**Return Values:**
- `anyhow::Result<()>` for command handlers and DB mutation functions
- `anyhow::Result<T>` for DB query functions: `anyhow::Result<Option<Agent>>`, `anyhow::Result<Vec<Message>>`, `anyhow::Result<u64>`
- `bool` for infallible checks: `tmux::session_exists()`
- `Vec<String>` for infallible list operations: `tmux::list_live_session_names()`

## Output Pattern

All command `run()` functions implement a dual output mode:

```rust
if json {
    println!("{}", serde_json::to_string_pretty(&data)?);
    return Ok(());
}
// Human-readable table/text output below
```

**Terminal detection** for colorized vs. plain output:
```rust
if std::io::stdout().is_terminal() {
    println!("{} Sent task to {}", "✓".green(), agent);
} else {
    println!("Sent task to {}", agent);
}
```

**Color handling:** `owo-colors` with `if_supports_color(Stream::Stdout, ...)` for terminal-aware coloring. ANSI padding is handled manually via `pad_colored(raw, colored, width)` because format string padding counts escape bytes.

## Module Design

**Exports:**
- `src/lib.rs` re-exports all internal modules so the binary and integration tests share code: `pub mod cli; pub mod commands; pub mod config; pub mod db; pub mod tmux;`
- `src/commands/mod.rs` re-exports submodules: `pub mod init; pub mod send; ...`
- `src/db/mod.rs` re-exports `pub mod agents; pub mod messages;` and exposes `Pool` type alias and `connect()` function

**Barrel Files:**
- `src/commands/mod.rs` acts as a barrel, listing all command submodules
- `src/db/mod.rs` acts as a barrel for DB submodules plus the pool setup
- No `src/lib.rs` re-export of nested items — consumers use full paths: `db::agents::insert_agent`, `db::messages::list_messages`

---

*Convention analysis: 2026-03-08*
