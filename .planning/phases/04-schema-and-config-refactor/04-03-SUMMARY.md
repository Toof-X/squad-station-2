---
phase: 04-schema-and-config-refactor
plan: "03"
subsystem: database
tags: [sqlite, sqlx, rust, agents, schema, migration]

# Dependency graph
requires:
  - phase: 04-01
    provides: "SquadConfig with project String, AgentConfig with tool field, cli.rs Register using tool"
  - phase: 04-02
    provides: "Migration 0003_v11.sql with tool/model/description/current_task columns, insert_message new signature"
provides:
  - "Agent struct: tool, model, description, current_task fields (provider/command removed from active use)"
  - "insert_agent signature: (pool, name, tool, role, model, description)"
  - "send.rs sets current_task FK after insert_message"
  - "signal.rs clears current_task = NULL after update_status"
  - "agents command displays TOOL column (not PROVIDER)"
  - "list command displays FROM/TO columns with processing colorize"
  - "Full test suite passes: 124 tests, 0 failures"
affects: [05-tmux-lifecycle, 06-views-and-tui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "insert_agent always passes empty string '' for legacy command column (NOT NULL compat)"
    - "Agent.command kept as Option<String> dead_code for SELECT * compat"
    - "current_task FK set/cleared via raw sqlx::query in send/signal commands"

key-files:
  created: []
  modified:
    - src/db/agents.rs
    - src/commands/init.rs
    - src/commands/register.rs
    - src/commands/send.rs
    - src/commands/signal.rs
    - src/commands/agents.rs
    - src/commands/list.rs
    - tests/test_db.rs
    - tests/test_views.rs
    - tests/test_lifecycle.rs
    - tests/test_integration.rs

key-decisions:
  - "Agent.command kept as Option<String> with dead_code allow — SELECT * returns it, removing it would break FromRow"
  - "insert_agent passes '' for command column to satisfy NOT NULL constraint without ALTER TABLE"
  - "current_task set/cleared with raw sqlx::query inline in send/signal — not added to db::agents API"
  - "list.rs updated to FROM/TO columns, not just AGENT column — directional routing now visible in UI"

patterns-established:
  - "Legacy column compat: keep field in struct as Option<T> with #[allow(dead_code)] rather than explicit SELECT"
  - "FK lifecycle: set FK on write (send), clear FK on completion (signal) — direct sqlx in command"

requirements-completed: [AGNT-01, AGNT-02, AGNT-03]

# Metrics
duration: 25min
completed: 2026-03-08
---

# Phase 4 Plan 03: Schema Integration Wave Summary

**Agent DB layer fully migrated: tool/model/description/current_task fields live, current_task FK lifecycle wired in send and signal commands, 124 tests all green**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-08T00:00:00Z
- **Completed:** 2026-03-08
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Updated `Agent` struct: renamed `provider` → `tool`, added `model`/`description`/`current_task` optional fields, kept `command` as dead_code Option for legacy compat
- `insert_agent` signature now accepts `model: Option<&str>` and `description: Option<&str>`, passes `''` for legacy `command` NOT NULL column
- `send.rs` sets `current_task` FK via `UPDATE agents SET current_task = ? WHERE name = ?` after `insert_message`
- `signal.rs` clears `current_task = NULL` after `update_status` + status reset to idle
- `agents` command now displays `TOOL` column header, `agent.tool` field
- `list` command now shows `FROM` and `TO` columns; `colorize_status` handles `"processing"` (yellow)
- All 26 test_db tests pass including 4 new tests: `test_agent_has_tool_field`, `test_agent_stores_model_description`, `test_send_sets_current_task`, `test_signal_clears_current_task`
- Fixed all `insert_agent` call sites across test_db, test_lifecycle, test_integration, test_views

## Task Commits

Each task was committed atomically:

1. **Task 1: Update db/agents.rs struct, CRUD, and test_db.rs agent tests** - `7395de7` (feat)
2. **Task 2: Wire all command callers to new signatures** - `761c73f` (feat)

## Files Created/Modified
- `src/db/agents.rs` - Agent struct rewritten; insert_agent new signature with model/description
- `src/commands/init.rs` - insert_agent called with model/description from config
- `src/commands/register.rs` - insert_agent called with None/None for model/description
- `src/commands/send.rs` - sets current_task FK after insert_message
- `src/commands/signal.rs` - clears current_task = NULL after update_status
- `src/commands/agents.rs` - TOOL column header and agent.tool field
- `src/commands/list.rs` - FROM/TO columns; processing colorize added
- `tests/test_db.rs` - all call sites updated; 4 new AGNT-01/02/03 tests
- `tests/test_views.rs` - Agent struct construction updated to new fields
- `tests/test_lifecycle.rs` - all insert_agent call sites updated
- `tests/test_integration.rs` - all call sites updated; assertions for TO/tool

## Decisions Made
- `Agent.command` kept as `Option<String>` with `#[allow(dead_code)]` — using `SELECT *` means FromRow must include all columns; dropping it would require switching to explicit SELECT across 4 query functions
- `insert_agent` passes `''` (empty string literal) for the `command` column to satisfy its `NOT NULL` constraint without needing `ALTER TABLE`
- `current_task` FK lifecycle handled with raw `sqlx::query` inline in the send/signal command files rather than adding new functions to `db::agents` — keeps the DB API minimal for this single-use pattern
- `list.rs` updated from `AGENT` to `FROM`/`TO` columns — directional routing is now surfaced in the default table output, matching the schema's routing intent

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed Agent struct construction in test_views.rs**
- **Found during:** Task 2 (command caller wiring)
- **Issue:** `test_views.rs` used struct literal construction with `provider: "test".into()` and `command: "echo".into()` — broke with new field layout
- **Fix:** Updated `mock_agent()` to use `tool`, `command: None`, and add `model`, `description`, `current_task: None`
- **Files modified:** `tests/test_views.rs`
- **Committed in:** `761c73f` (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed test_integration.rs assertions for new column names**
- **Found during:** Task 2 (final test run)
- **Issue:** `test_list_text_output_with_messages` checked for `"AGENT"` column (renamed to `"TO"`); `test_agents_json_output` checked `agent["provider"]` (renamed to `agent["tool"]`)
- **Fix:** Updated both assertions to match new column/field names
- **Files modified:** `tests/test_integration.rs`
- **Committed in:** `761c73f` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 3 blocking)
**Impact on plan:** Both fixes required for compilation/test correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## Next Phase Readiness
- Phase 4 complete — all 3 plans done, full test suite passing (124 tests, 0 failures)
- Agent and message DB layers fully aligned with v1.1 solution design schema
- Ready for Phase 5 (tmux lifecycle improvements) or Phase 6 (views and TUI)

---
*Phase: 04-schema-and-config-refactor*
*Completed: 2026-03-08*
