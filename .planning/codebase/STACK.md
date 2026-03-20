# Technology Stack

**Analysis Date:** 2026-03-08

## Languages

**Primary:**
- Rust (edition 2021) - All application logic, CLI, database layer, TUI

**Secondary:**
- Bash - Hook scripts in `hooks/` for provider-specific completion detection
- SQL - Database migrations in `src/db/migrations/`

## Runtime

**Environment:**
- Native binary (no runtime VM)
- Async runtime: Tokio 1.37 (full features)

**Platform:**
- Tested on macOS (aarch64-apple-darwin, stable-aarch64)
- Requires tmux to be installed on the host system
- Requires `squad-station` binary on PATH for hooks

## Toolchain

**Rust Version:** 1.93.1 (stable-aarch64-apple-darwin)
**Package Manager:** Cargo
- Lockfile: `Cargo.lock` (committed)
- No explicit `rust-toolchain.toml` — uses system stable

**Build System:**
- `build.rs` — Triggers rebuild when migrations directory changes
  ```rust
  println!("cargo:rerun-if-changed=src/db/migrations");
  ```

## Frameworks

**Core:**
- `clap` 4.5 (derive feature) — CLI argument parsing, subcommand dispatch
- `tokio` 1.37 (full) — Async runtime for all I/O operations
- `sqlx` 0.8 (runtime-tokio-rustls, sqlite, macros, chrono, uuid, migrate) — Async SQLite with WAL mode

**TUI:**
- `ratatui` 0.26 — Terminal UI dashboard (`src/commands/ui.rs`)
- `crossterm` 0.27 — Cross-platform terminal input/output backend for ratatui

**Serialization:**
- `serde` 1.0 (derive) — Serialization/deserialization traits
- `serde_json` 1.0 — JSON output mode (`--json` flag)
- `serde-saphyr` 0.0.17 — YAML deserialization for `squad.yml` config

**Testing:**
- `tokio-test` 0.4 — Async test utilities (dev dependency)
- `tempfile` 3 — Temporary SQLite DB creation for isolated tests (dev dependency)

## Key Dependencies

**Critical:**
- `sqlx` 0.8 — Database access layer; embedded migrations via `sqlx::migrate!()`; WAL mode; single-writer pool (max_connections=1)
- `clap` 4.5 — All CLI surface area; auto-generates help text and version info
- `tokio` 1.37 — All async operations; SIGPIPE reset via `libc` before runtime init

**Infrastructure:**
- `anyhow` 1.0 — Unified error handling with context chains throughout all modules
- `chrono` 0.4 (serde feature) — Timestamp handling for `created_at` / `updated_at` fields
- `uuid` 1.8 (v4, serde features) — Primary keys for `agents` and `messages` tables
- `owo-colors` 3 (supports-colors feature) — Terminal color output for human-readable CLI responses
- `dirs` 5 — Resolves `~/.agentic-squad/<project>/station.db` default DB path
- `libc` 0.2 — SIGPIPE signal reset (`SAFE-04`) before async runtime starts

## Configuration

**Project Config:**
- Format: YAML (`squad.yml` at project root)
- Parser: `serde-saphyr` via `config::load_config()`
- Default path: `squad.yml` (overridable via `init` subcommand argument)
- Schema defined in `src/config.rs`: `SquadConfig`, `ProjectConfig`, `AgentConfig`

**Database:**
- Default path: `~/.agentic-squad/<project.name>/station.db`
- Optional override: `project.db_path` field in `squad.yml`
- Auto-creates parent directories on first connect
- Migrations auto-applied via `sqlx::migrate!("./src/db/migrations")`

**Build:**
- Release binary target: `target/release/squad-station`

## Platform Requirements

**Development:**
- Rust stable toolchain (1.93.1 tested)
- Cargo (bundled with Rust)
- tmux (required for agent session management at runtime)

**Production:**
- `squad-station` binary on PATH (or set `SQUAD_STATION_BIN` env var for hooks)
- tmux installed on host
- `~/.agentic-squad/` writable directory for DB storage

---

*Stack analysis: 2026-03-08*
