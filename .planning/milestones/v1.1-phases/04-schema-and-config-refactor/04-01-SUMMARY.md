---
phase: 04-schema-and-config-refactor
plan: "01"
subsystem: config
tags: [config, schema, tdd, refactor]
dependency_graph:
  requires: []
  provides: [SquadConfig.project:String, AgentConfig.tool, AgentConfig.model, AgentConfig.description, resolve_db_path, Register--tool]
  affects: [src/commands/init.rs, src/commands/register.rs, src/commands/status.rs, src/main.rs]
tech_stack:
  added: []
  patterns: [serde default attribute, SQUAD_STATION_DB env var for test DB injection]
key_files:
  created:
    - tests/test_config.rs
  modified:
    - src/config.rs
    - src/cli.rs
    - squad.yml
    - src/commands/init.rs
    - src/commands/register.rs
    - src/commands/status.rs
    - src/main.rs
    - tests/test_commands.rs
    - tests/test_integration.rs
    - tests/test_lifecycle.rs
    - tests/test_views.rs
    - tests/test_cli.rs
decisions:
  - "SQUAD_STATION_DB env var check moved into resolve_db_path so all commands benefit without individual changes"
  - "init.rs uses minimal stubs (TODO: Plan 03) — agent name auto-derived from project+tool+role pattern"
metrics:
  duration: 9m
  completed: 2026-03-08
  tasks_completed: 2
  files_modified: 12
---

# Phase 4 Plan 01: Config Schema Refactor Summary

**One-liner:** Config structs refactored — `project` is String, `AgentConfig` gains `tool`/`model`/`description`, loses `command`/`provider`; all 4 config unit tests green.

## What Was Built

- `src/config.rs`: Replaced `ProjectConfig` struct with a plain `String` on `SquadConfig.project`. Renamed `AgentConfig.provider` to `AgentConfig.tool`. Removed `AgentConfig.command`. Added `AgentConfig.model: Option<String>` and `AgentConfig.description: Option<String>`. Added `#[serde(default = "default_role")]` on role field. `resolve_db_path` now uses `config.project` directly and checks `SQUAD_STATION_DB` env var first.
- `src/cli.rs`: `Register` subcommand now uses `--tool` flag instead of `--provider`; `--command` removed.
- `squad.yml`: Updated to new format with `project: squad-station` (scalar string) and `tool:` fields.
- `tests/test_config.rs`: Four TDD unit tests covering CONF-01 through CONF-04, all passing.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Write failing config unit tests (TDD RED) | 2ceab07 |
| 2 | Refactor config.rs/cli.rs/squad.yml (TDD GREEN) | 9df77b6 |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated test suite to use new config format**
- **Found during:** Task 2 — when running `cargo test` after config struct changes
- **Issue:** `tests/test_commands.rs`, `tests/test_integration.rs`, `tests/test_lifecycle.rs`, `tests/test_views.rs`, and `tests/test_cli.rs` all contained inline YAML using the old `project.name` nested format, old `provider`/`command` fields, and old CLI flags (`--provider`, `--command`)
- **Fix:** Updated all test files to use new YAML format. Added `cmd_with_db(db_path)` helper to each test file that sets `SQUAD_STATION_DB` env var — this replaces the old `db_path` field in `ProjectConfig` that tests used to inject a custom DB path. `resolve_db_path` was also updated to check `SQUAD_STATION_DB` first, making all commands work with the test injection pattern.
- **Files modified:** `tests/test_commands.rs`, `tests/test_integration.rs`, `tests/test_lifecycle.rs`, `tests/test_views.rs`, `tests/test_cli.rs`
- **Commit:** 9df77b6

## Verification Results

- `cargo test --test test_config`: 4/4 tests pass (CONF-01 through CONF-04)
- `cargo check`: exits 0, no errors
- `cargo test` (full suite): all 117 tests pass across all test files
- `grep "pub project: String" src/config.rs`: confirms String type
- `grep "pub tool: String" src/config.rs`: confirms tool field
- `grep "project:" squad.yml`: confirms `project: squad-station` scalar format
- `ProjectConfig` struct: completely removed from codebase
- `AgentConfig.command`: completely removed from codebase
- `AgentConfig.provider`: completely removed, replaced by `tool`

## Self-Check: PASSED

All files found and commits verified:
- SUMMARY.md: FOUND
- tests/test_config.rs: FOUND
- src/config.rs: FOUND
- Commit 2ceab07 (RED phase): FOUND
- Commit 9df77b6 (GREEN phase): FOUND
