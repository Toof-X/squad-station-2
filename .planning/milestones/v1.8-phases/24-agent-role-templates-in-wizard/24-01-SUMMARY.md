---
phase: 24-agent-role-templates-in-wizard
plan: "01"
subsystem: templates-data-layer
tags: [templates, db-migration, agent-struct, routing-hints]
dependency_graph:
  requires: []
  provides:
    - src/commands/templates.rs (AgentTemplate struct, WORKER_TEMPLATES, ORCHESTRATOR_TEMPLATES, sentinel constants)
    - src/db/migrations/0005_routing_hints.sql (routing_hints column)
    - src/db/agents.rs (Agent.routing_hints field, updated insert_agent signature)
    - src/commands/wizard.rs (AgentInput.routing_hints field)
  affects:
    - src/commands/clone.rs
    - src/commands/register.rs
    - src/commands/init.rs
    - src/commands/diagram.rs
    - tests/* (all insert_agent callers)
tech_stack:
  added: []
  patterns:
    - Static const arrays of structs for embedded template data
    - SQLite ALTER TABLE migration for additive schema change
    - Option<String> field on existing struct with None defaults for backwards compatibility
key_files:
  created:
    - src/commands/templates.rs
    - src/db/migrations/0005_routing_hints.sql
  modified:
    - src/commands/mod.rs
    - src/db/agents.rs
    - src/commands/clone.rs
    - src/commands/register.rs
    - src/commands/init.rs
    - src/commands/wizard.rs
    - src/commands/diagram.rs
    - tests/test_commands.rs
    - tests/test_db.rs
    - tests/test_integration.rs
    - tests/test_lifecycle.rs
    - tests/test_clone.rs
    - tests/test_views.rs
decisions:
  - "[Phase 24-01] All 11 templates use default_provider=claude-code; per-provider model mapping stored in template struct not resolved at runtime"
  - "[Phase 24-01] routing_hints stored as JSON string (Option<String>) in DB and AgentInput; serialization to JSON array deferred to Plan 02 when template selection wires the data"
metrics:
  duration_minutes: 25
  completed_date: "2026-03-19"
  tasks_completed: 2
  files_changed: 14
---

# Phase 24 Plan 01: Templates Data Module and routing_hints Foundation Summary

**One-liner:** Static AgentTemplate data module with 8 worker + 3 orchestrator role templates, DB migration 0005 adding routing_hints column, and updated Agent/insert_agent/AgentInput contracts across all callers.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Create templates.rs data module | 1189d56 | src/commands/templates.rs, src/commands/mod.rs |
| 2 | DB migration, Agent struct, insert_agent, all callers | eb6829a | src/db/migrations/0005_routing_hints.sql, src/db/agents.rs, src/commands/clone.rs, src/commands/register.rs, src/commands/init.rs, src/commands/wizard.rs, src/commands/diagram.rs, tests/* |

## What Was Built

### Task 1: templates.rs data module

Created `src/commands/templates.rs` with:

- `AgentTemplate` struct: slug, display_name, description (2-3 sentences), default_provider, claude_model, gemini_model, routing_hints (&[&str])
- `WORKER_TEMPLATES: &[AgentTemplate]` with 8 entries in order: coder, solution-architect, qa-engineer, devops-engineer, code-reviewer, technical-writer, data-engineer, security-engineer
- `ORCHESTRATOR_TEMPLATES: &[AgentTemplate]` with 3 entries: project-manager, tech-lead, scrum-master
- `CUSTOM_IDX_WORKER: usize = 8` — sentinel for "Custom" option in worker selector
- `CUSTOM_IDX_ORCHESTRATOR: usize = 3` — sentinel for "Custom" option in orchestrator selector
- All templates default to `default_provider = "claude-code"`
- Per-provider model mappings: claude_model ("sonnet" or "opus") and gemini_model ("gemini-2.5-pro" or "gemini-2.5-flash")
- Module registered as `pub mod templates;` in `src/commands/mod.rs`

### Task 2: DB migration and contract updates

- **Migration 0005:** `ALTER TABLE agents ADD COLUMN routing_hints TEXT DEFAULT NULL` — purely additive, safe for existing DBs
- **Agent struct:** Added `pub routing_hints: Option<String>` field after `current_task`
- **insert_agent():** Added `routing_hints: Option<&str>` as 7th parameter; SQL updated to include routing_hints in INSERT values and ON CONFLICT UPDATE clause
- **AgentInput struct (wizard.rs):** Added `pub routing_hints: Option<String>` field
- **draft_to_agent_input():** Added `routing_hints: None` (actual template wiring deferred to Plan 02)
- **All callers updated:** clone.rs (passes source.routing_hints.as_deref()), register.rs (None), init.rs both calls (None)
- **All test helpers updated:** diagram.rs, init.rs test helpers, and all test files (test_commands.rs, test_db.rs, test_integration.rs, test_lifecycle.rs, test_clone.rs, test_views.rs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] diagram.rs test helper also constructs Agent struct**
- **Found during:** Task 2 (first cargo test run)
- **Issue:** `src/commands/diagram.rs` has a test helper `make_agent()` that constructs `Agent { ... }` directly. Adding `routing_hints` field to the struct caused a compile error there too.
- **Fix:** Added `routing_hints: None` to the Agent literal in diagram.rs `make_agent()` test helper
- **Files modified:** src/commands/diagram.rs
- **Commit:** eb6829a (included in Task 2 commit)

**2. [Rule 3 - Blocking] test_views.rs constructs Agent struct directly**
- **Found during:** Task 2 (second cargo test run after fixing diagram.rs)
- **Issue:** `tests/test_views.rs` `mock_agent()` helper constructs `Agent { ... }` without the new `routing_hints` field
- **Fix:** Added `routing_hints: None` to the struct literal
- **Files modified:** tests/test_views.rs
- **Commit:** eb6829a

**3. [Rule 3 - Blocking] All test files calling insert_agent with 6 args**
- **Found during:** Task 2 — many test files call insert_agent with 6 arguments (pool, name, tool, role, model, description)
- **Issue:** Changing insert_agent to 7 parameters broke all 60+ test call sites
- **Fix:** Used sed for single-line calls and a Python script for multiline calls to add `None` as the 7th routing_hints argument across test_commands.rs, test_db.rs, test_integration.rs, test_lifecycle.rs, test_clone.rs
- **Files modified:** All 5 test files listed above
- **Commit:** eb6829a

## Verification

```
cargo check   -> OK (1 pre-existing warning, no errors)
cargo test    -> 290 tests pass, 0 failures
ls src/db/migrations/ -> 0001 through 0005 present
grep -c "routing_hints" src/db/agents.rs -> 5 occurrences
```

## Self-Check: PASSED

- src/commands/templates.rs exists: FOUND
- src/db/migrations/0005_routing_hints.sql exists: FOUND
- WORKER_TEMPLATES has 8 entries: VERIFIED (counted in source)
- ORCHESTRATOR_TEMPLATES has 3 entries: VERIFIED (counted in source)
- Commits 1189d56 and eb6829a: FOUND in git log
- All tests pass: VERIFIED (cargo test output shows 0 failures)
