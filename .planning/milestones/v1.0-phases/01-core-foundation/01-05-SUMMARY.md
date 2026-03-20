---
phase: 01-core-foundation
plan: 05
subsystem: tests
tags: [rust, sqlx, sqlite, integration-tests, tdd, cargo, test-infrastructure]

requires:
  - phase: 01-core-foundation/01-01
    provides: "DB connect(), insert_agent, insert_message, update_status, list_messages, peek_message, tmux arg builders"
  - phase: 01-core-foundation/01-02
    provides: "register, send, signal commands"
  - phase: 01-core-foundation/01-03
    provides: "init command, config parsing, DB path resolution"
  - phase: 01-core-foundation/01-04
    provides: "list and peek commands"

provides:
  - "tests/helpers.rs: shared setup_test_db() using temp-file SQLite with WAL mode and migrations"
  - "tests/test_db.rs: 17 tests — agent CRUD, message CRUD, idempotency (INSERT OR IGNORE, UPDATE subquery), priority ordering"
  - "tests/test_commands.rs: 7 tests — SquadConfig YAML parsing, DB path resolution, SAFE-04 binary startup"
  - "src/lib.rs: library surface exposing all modules for integration test access"
  - "Phase gate: cargo test passes with 28 tests, zero failures — all 12 Phase 1 requirements verified"

affects: []

tech-stack:
  added: []
  patterns:
    - "Integration test isolation: each test creates its own named temp-file SQLite pool via setup_test_db()"
    - "lib.rs + main.rs split: binary references library via use squad_station::{cli, commands} — standard Rust pattern for testable binaries"
    - "CARGO_BIN_EXE_squad-station env var: test environment auto-sets binary path for subprocess tests"

key-files:
  created:
    - "tests/helpers.rs — shared setup_test_db() helper: temp-file SQLite with WAL mode + sqlx migrations"
    - "tests/test_db.rs — 17 DB layer tests: agent CRUD, message CRUD, idempotency, priority ordering (peek)"
    - "tests/test_commands.rs — 7 command tests: config parsing, DB path resolution, SIGPIPE binary startup"
    - "src/lib.rs — library surface re-exporting cli, commands, config, db, tmux modules"
  modified:
    - "src/main.rs — switched from inline mod declarations to use squad_station::{cli, commands}"
    - "src/db/messages.rs — [Rule 1] fixed update_status: rewrote UPDATE...ORDER BY...LIMIT as subquery"

key-decisions:
  - "Split into lib.rs + main.rs: expose internal modules as library so integration tests can import them without duplicating module declarations"
  - "Temp-file SQLite for test isolation: each test gets its own NamedTempFile DB to prevent cross-test pollution (in-memory pools share a single connection so WAL mode is unsupported)"
  - "update_status subquery fix: SQLite does not support UPDATE...ORDER BY...LIMIT without SQLITE_ENABLE_UPDATE_DELETE_LIMIT compile flag — rewrote using UPDATE WHERE id = (SELECT id ... LIMIT 1)"

requirements-completed:
  - SESS-01
  - SESS-02
  - MSG-01
  - MSG-02
  - MSG-03
  - MSG-04
  - MSG-05
  - MSG-06
  - SAFE-01
  - SAFE-02
  - SAFE-03
  - SAFE-04

duration: ~4min
completed: 2026-03-06
---

# Phase 1 Plan 05: Integration Test Suite Summary

**28 tests (4 unit + 17 DB + 7 command), zero failures — all 12 Phase 1 requirements have automated coverage. Phase gate passed.**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-03-06T05:25:09Z
- **Completed:** 2026-03-06T05:28:36Z
- **Tasks:** 2
- **Files created:** 4 (helpers.rs, test_db.rs, test_commands.rs, lib.rs)
- **Files modified:** 2 (main.rs, db/messages.rs)

## Accomplishments

- **Test infrastructure:** `tests/helpers.rs` provides `setup_test_db()` — creates an isolated temp-file SQLite pool per test, applies migrations, WAL mode enabled. Each test is fully independent.
- **DB layer tests (17):** Agent CRUD (insert, get, list, idempotency via INSERT OR IGNORE), message insert, update_status, list with all filter combinations, peek with priority ordering (urgent > high > normal). Covers SESS-01, SESS-02, MSG-01 through MSG-06, SAFE-01.
- **Command tests (7):** SquadConfig YAML deserialization (valid, multi-agent, custom db_path, missing required field returns error), DB path resolution (default `~/.agentic-squad/<name>/station.db`, custom path), SIGPIPE binary startup verification.
- **tmux unit tests (4, pre-existing):** send_keys_args always include `-l` (SAFE-02), Enter sent without `-l`, launch uses direct command (SAFE-03), special characters preserved with `-l`.

## Task Commits

1. **Task 1: Create test infrastructure and DB layer tests** — `e382569` (test)
2. **Task 2: Create command logic tests, verify tmux safety, and run full suite** — `a646f20` (test)

## Files Created/Modified

- `tests/helpers.rs` — `setup_test_db()` using `NamedTempFile` for WAL-compatible isolated per-test SQLite pools
- `tests/test_db.rs` — 17 integration tests covering full DB layer
- `tests/test_commands.rs` — 7 tests: config parsing, DB path resolution, binary startup (SAFE-04)
- `src/lib.rs` — re-exports all internal modules (cli, commands, config, db, tmux) for integration test access
- `src/main.rs` — updated to use lib crate exports via `use squad_station::{cli, commands}`
- `src/db/messages.rs` — fixed `update_status` SQL (see Deviations)

## Decisions Made

- **lib.rs + main.rs split:** Standard Rust pattern — lib.rs re-exports all modules, main.rs uses them via `use squad_station::`. Lets integration tests import library code without duplication.
- **Temp-file test DB:** `NamedTempFile` instead of `:memory:` — SQLite WAL mode requires a real file; in-memory pools don't support multi-connection WAL. Each test gets an isolated file, file is leaked for test duration (cleaned up on process exit).
- **CARGO_BIN_EXE_squad-station:** Used in `test_sigpipe_binary_starts` to get the cargo-compiled binary path automatically — no hardcoded paths.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added src/lib.rs to expose modules for integration tests**
- **Found during:** Task 1 (test_db.rs compilation)
- **Issue:** Integration tests in `tests/` cannot access internal `mod` declarations from `src/main.rs` (binary-only crate). `use squad_station::db` would fail with "use of undeclared crate" without a lib target.
- **Fix:** Created `src/lib.rs` re-exporting cli, commands, config, db, tmux. Updated `src/main.rs` to use lib exports via `use squad_station::{cli, commands}`.
- **Files modified:** src/lib.rs (created), src/main.rs (updated)
- **Commit:** e382569

**2. [Rule 1 - Bug] Fixed update_status SQL — SQLite rejects UPDATE...ORDER BY...LIMIT**
- **Found during:** Task 1 (test_update_status_* failures)
- **Issue:** `UPDATE messages SET ... WHERE ... ORDER BY created_at DESC LIMIT 1` fails with "near ORDER: syntax error". SQLite does not support ORDER BY/LIMIT in UPDATE without the `SQLITE_ENABLE_UPDATE_DELETE_LIMIT` compile-time flag, which bundled sqlx SQLite does not enable.
- **Fix:** Rewrote as subquery: `UPDATE messages SET ... WHERE id = (SELECT id FROM messages WHERE ... ORDER BY created_at DESC LIMIT 1)`. Semantically identical, universally supported.
- **Files modified:** src/db/messages.rs
- **Commit:** e382569

## Requirement Coverage

| Requirement | Test(s) | Status |
|-------------|---------|--------|
| SESS-01 | test_config_parse_valid_yaml, test_db_path_resolution_default | Covered |
| SESS-02 | test_insert_agent_idempotent | Covered |
| MSG-01 | test_insert_message, test_insert_message_with_priority | Covered |
| MSG-02 | test_update_status_completes_message | Covered |
| MSG-03 | test_update_status_idempotent, test_update_status_no_pending | Covered |
| MSG-04 | test_list_filter_by_agent, test_list_filter_by_status, test_list_with_limit, test_list_no_filters | Covered |
| MSG-05 | test_peek_priority_ordering | Covered |
| MSG-06 | test_peek_returns_pending, test_peek_no_pending, test_peek_nonexistent_agent | Covered |
| SAFE-01 | setup_test_db() uses WAL mode (SqliteJournalMode::Wal) | Covered |
| SAFE-02 | test_send_keys_args_have_literal_flag (unit, src/tmux.rs) | Covered |
| SAFE-03 | test_launch_args_use_direct_command (unit, src/tmux.rs) | Covered |
| SAFE-04 | test_sigpipe_binary_starts | Covered |

## Self-Check: PASSED

Files verified present:
- FOUND: tests/helpers.rs
- FOUND: tests/test_db.rs
- FOUND: tests/test_commands.rs
- FOUND: src/lib.rs

Commits verified:
- FOUND: e382569 (Task 1 — DB tests + lib.rs + update_status fix)
- FOUND: a646f20 (Task 2 — command tests)

---
*Phase: 01-core-foundation*
*Completed: 2026-03-06*
