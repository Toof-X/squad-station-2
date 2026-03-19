---
phase: 23-dynamic-agent-cloning
plan: 01
subsystem: cli
tags: [rust, sqlite, tmux, clone, agent-management]

# Dependency graph
requires:
  - phase: 22-orchestrator-intelligence-data
    provides: context::run() for auto-regeneration after clone

provides:
  - squad-station clone subcommand with full CLONE-01 through CLONE-06 coverage
  - delete_agent_by_name DB function for safe clone rollback
  - Auto-incremented naming logic (strip_clone_suffix, extract_clone_number)

affects:
  - 23-dynamic-agent-cloning
  - 24-agent-templates

# Tech tracking
tech-stack:
  added: []
  patterns:
    - DB-first with tmux rollback — insert DB record before tmux launch, delete on failure
    - Best-effort context regeneration — clone succeeds even if context::run() fails
    - Name collision prevention — scan both DB and live tmux sessions before generating name

key-files:
  created:
    - src/commands/clone.rs
  modified:
    - src/db/agents.rs
    - src/commands/mod.rs
    - src/cli.rs
    - src/main.rs

key-decisions:
  - "strip_clone_suffix only strips -N where N >= 2, so agent names like worker-1 are not mistaken for clones"
  - "Name generation scans both DB and live tmux sessions to avoid collisions with orphaned sessions from re-init"
  - "Antigravity agents skipped for tmux launch — DB-only registration matches register command behavior"
  - "Context regeneration is best-effort: warns on failure, does not fail the clone operation"
  - "CLONE-06 (TUI visibility) satisfied by existing list_agents query — no TUI changes required"

patterns-established:
  - "DB-first with rollback: insert before external operation, clean up on failure"
  - "Best-effort side effects: warn but do not fail on non-critical post-clone operations"

requirements-completed: [CLONE-01, CLONE-02, CLONE-03, CLONE-04, CLONE-05, CLONE-06]

# Metrics
duration: 15min
completed: 2026-03-19
---

# Phase 23 Plan 01: Dynamic Agent Cloning Summary

**`squad-station clone <agent>` command with DB-first insert, tmux rollback, auto-incremented naming, and best-effort context regeneration covering all six CLONE requirements**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-19T06:00:00Z
- **Completed:** 2026-03-19T06:15:00Z
- **Tasks:** 2 of 2
- **Files modified:** 5

## Accomplishments
- Added `delete_agent_by_name` to DB layer enabling safe rollback if tmux launch fails after DB insert
- Implemented full `clone` command covering all six CLONE requirements in a single new file
- Wired `Clone` subcommand into CLI (cli.rs variant, mod.rs module, main.rs dispatch arm)
- Added 6 unit tests covering name-stripping and launch-command logic

## Task Commits

Each task was committed atomically:

1. **Task 1: Add delete_agent_by_name to DB layer** - `9840973` (feat)
2. **Task 2: Implement clone command with CLI wiring** - `1923c8b` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/db/agents.rs` - Added `delete_agent_by_name(pool, name)` for clone rollback
- `src/commands/clone.rs` - Full clone implementation: run(), generate_clone_name(), strip_clone_suffix(), extract_clone_number(), get_launch_command()
- `src/commands/mod.rs` - Added `pub mod clone;` alphabetically
- `src/cli.rs` - Added `Clone { agent: String }` variant to Commands enum
- `src/main.rs` - Added `Clone { agent } => commands::clone::run(agent, cli.json).await` match arm

## Decisions Made
- Strip `-N` suffix only when N >= 2: cloning `worker-3` yields `worker-4`; a name like `worker-1` is treated as a base name to avoid accidental stripping of legitimate names
- Name uniqueness check covers both DB agents and live tmux sessions to prevent collision with orphaned sessions left from previous `init` runs
- `antigravity` tool agents skip tmux launch — consistent with `register` behavior for DB-only agents
- Context regeneration failure warns but does not abort clone — operator can run `squad-station context` manually

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The symlink at `~/.cargo/bin/squad-station` points to a different workspace, but the freshly-built binary at `target/release/squad-station` in this project correctly shows `clone --help` output.

## Next Phase Readiness

- `clone` command is fully functional and tested
- All six CLONE requirements have corresponding code paths
- Ready for Phase 23 plan 02 (if exists) or Phase 24 agent templates

---
*Phase: 23-dynamic-agent-cloning*
*Completed: 2026-03-19*

## Self-Check: PASSED

- clone.rs: FOUND
- agents.rs with delete_agent_by_name: FOUND
- SUMMARY.md: FOUND
- Task 1 commit 9840973: FOUND
- Task 2 commit 1923c8b: FOUND
