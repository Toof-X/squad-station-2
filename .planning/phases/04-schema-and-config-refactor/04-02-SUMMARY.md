---
phase: 04-schema-and-config-refactor
plan: "02"
subsystem: database
tags: [sqlite, sqlx, migrations, messages, rust]

# Dependency graph
requires: []
provides:
  - "Migration 0003_v11.sql — all Phase 4 schema changes in one atomic migration"
  - "messages.rs with from_agent, to_agent, msg_type, completed_at fields"
  - "insert_message with new 6-arg signature storing directional routing and type"
  - "update_status sets completed_at timestamp, filters on status='processing'"
  - "peek_message filters on status='processing'"
affects:
  - "04-03 — agents.rs struct update depends on migration already applied"
  - "plan-03 — status command now correctly counts 'processing' messages"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "#[sqlx(rename)] for mapping reserved SQL keyword 'type' to Rust field msg_type"
    - "#[sqlx(rename = \"tool\")] on agents.provider for backward-compat field name during migration transition"
    - "agent_name set to to_agent value on insert for legacy query compat"

key-files:
  created:
    - src/db/migrations/0003_v11.sql
  modified:
    - src/db/messages.rs
    - src/db/agents.rs
    - src/commands/send.rs
    - src/commands/status.rs
    - tests/test_db.rs
    - tests/test_integration.rs
    - tests/test_views.rs

key-decisions:
  - "agent_name column set to to_agent value on INSERT for backward compat with peek_message and update_status subqueries"
  - "#[sqlx(rename = \"tool\")] added to agents.rs to bridge migration rename without breaking Rust API until Plan 03"
  - "All test call sites updated to new insert_message signature as part of this plan (blocking issue, Rule 3)"
  - "status 'pending' → 'processing' updated across test_integration.rs and test_views.rs as part of this plan"

patterns-established:
  - "Use #[sqlx(rename)] to map SQL reserved keywords and renamed columns to Rust-idiomatic field names"
  - "Set agent_name = to_agent on insert to maintain backward compat with single-column subqueries"

requirements-completed: [MSGS-01, MSGS-02, MSGS-03, MSGS-04]

# Metrics
duration: 25min
completed: 2026-03-08
---

# Phase 04 Plan 02: Schema Migration and Messages DB Layer Summary

**SQLite migration 0003_v11 adding from_agent/to_agent/type/completed_at columns, plus updated Rust message CRUD with directional routing and 'processing' status semantics**

## Performance

- **Duration:** 25 min
- **Started:** 2026-03-08T18:00:00Z
- **Completed:** 2026-03-08T18:25:00Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Migration 0003_v11.sql applies atomically: renames agents.provider→tool, adds model/description/current_task to agents, adds from_agent/to_agent/type/completed_at to messages
- Message struct updated with all new fields; insert_message now stores directional routing (MSGS-01), type (MSGS-02), and sets status='processing' not 'pending' (MSGS-03)
- update_status now sets completed_at timestamp (MSGS-04) and subquery filters on 'processing'
- All 22 test_db tests pass including 2 new tests: test_insert_message_stores_direction and test_update_status_sets_completed_at
- Full test suite: 119 tests, 0 failures

## Task Commits

Each task was committed atomically:

1. **Task 1: Write migration 0003_v11.sql** - `69df8c2` (chore)
2. **Task 2: Update db/messages.rs struct and CRUD, fix test_db.rs** - `bf62f2b` (feat)

## Files Created/Modified

- `src/db/migrations/0003_v11.sql` - All Phase 4 schema changes in one atomic migration
- `src/db/messages.rs` - Updated Message struct and CRUD with new fields and 'processing' status
- `src/db/agents.rs` - Added #[sqlx(rename = "tool")] and fixed INSERT column name (Rule 3 auto-fix)
- `src/commands/send.rs` - Updated insert_message call to new 6-arg signature (Rule 3 auto-fix)
- `src/commands/status.rs` - Updated pending count query to use 'processing' status (Rule 3 auto-fix)
- `tests/test_db.rs` - Updated all message call sites + added 2 new tests
- `tests/test_integration.rs` - Updated all insert_message calls and "pending"→"processing" assertions
- `tests/test_views.rs` - Updated insert_message call and raw SQL agent INSERTs (provider→tool)

## Decisions Made

- `agent_name` column is set to `to_agent` value on INSERT for backward compatibility with peek_message and update_status subqueries that filter by `agent_name`
- `#[sqlx(rename = "tool")]` added to `agents.rs` provider field as a bridge: migration renamed the SQL column but Plan 03 will complete the full Rust rename — this is the minimal fix needed for the test suite to pass now
- All test suite call sites updated in this plan since they were blocking compilation (Rule 3)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed agents.rs INSERT to use renamed 'tool' column**
- **Found during:** Task 2 (running tests after updating messages.rs)
- **Issue:** Migration 0003_v11 renamed `provider` to `tool` but `agents.rs` still used `provider` in its INSERT query, causing `table agents has no column named provider` errors across all 20 tests
- **Fix:** Added `#[sqlx(rename = "tool")]` to the `provider` field in Agent struct, changed INSERT SQL to use `tool` column name. Kept `provider` as the Rust field name for API stability until Plan 03 completes the full rename.
- **Files modified:** `src/db/agents.rs`
- **Verification:** All 22 test_db tests pass, full suite clean
- **Committed in:** `bf62f2b` (Task 2 commit)

**2. [Rule 3 - Blocking] Updated send.rs insert_message call to new signature**
- **Found during:** Task 2 (compilation)
- **Issue:** `send.rs` called `insert_message` with the old 4-arg signature; compilation failed
- **Fix:** Updated call to new 6-arg signature with `"orchestrator"` as from_agent
- **Files modified:** `src/commands/send.rs`
- **Committed in:** `bf62f2b` (Task 2 commit)

**3. [Rule 3 - Blocking] Updated all test files and status.rs to use 'processing' not 'pending'**
- **Found during:** Task 2 (test compilation and assertion failures)
- **Issue:** `test_integration.rs`, `test_views.rs`, and `status.rs` all used the old `insert_message` signature and asserted on `"pending"` status; raw SQL in test_views.rs used `provider` column name
- **Fix:** Replaced all call sites and assertions; updated raw SQL to use `tool` column name
- **Files modified:** `src/commands/status.rs`, `tests/test_integration.rs`, `tests/test_views.rs`
- **Committed in:** `bf62f2b` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (all Rule 3 — blocking issues caused directly by this plan's API changes)
**Impact on plan:** All fixes necessary for compilation and test correctness. No scope creep — all changes are direct consequences of the new insert_message signature and migration column rename.

## Issues Encountered

None beyond the auto-fixed blocking issues documented above.

## Next Phase Readiness

- Migration 0003_v11 is applied and all tests pass — Plan 03 can proceed with the agents.rs full rename (provider→tool Rust field) and command caller updates
- The `#[sqlx(rename = "tool")]` bridge in agents.rs will be cleaned up when Plan 03 does the full rename

---
*Phase: 04-schema-and-config-refactor*
*Completed: 2026-03-08*
