---
phase: 05-feature-completion
plan: 02
subsystem: cli
tags: [cli, send, init, signal, context, tdd]
dependency_graph:
  requires: [05-01]
  provides: [CLI-01, CLI-02, CLI-03, SIG-01]
  affects: [src/cli.rs, src/commands/send.rs, src/main.rs, src/commands/init.rs, src/commands/signal.rs, src/commands/context.rs]
tech_stack:
  added: []
  patterns: [TDD-red-green, clap-named-flags, markdown-output]
key_files:
  created: []
  modified:
    - src/cli.rs
    - src/commands/send.rs
    - src/main.rs
    - src/commands/init.rs
    - src/commands/signal.rs
    - src/commands/context.rs
    - tests/test_cli.rs
    - tests/test_commands.rs
    - tests/test_lifecycle.rs
    - tests/test_integration.rs
    - tests/e2e_cli.sh
decisions:
  - "CLI-01: Send uses --body named flag (not positional) — required by SOLUTION-DESIGN.md for discoverability and shell safety"
  - "CLI-02: init.rs always produces <project>-<tool>-<role> names; name field in squad.yml acts as role suffix if provided"
  - "CLI-03: context output uses Markdown ## headings per agent, not table format"
  - "SIG-01: Notification format is '<agent> completed <msg-id>' — clean, pattern-matchable by orchestrator"
metrics:
  duration: 239s
  completed_date: "2026-03-08"
  tasks_completed: 2
  files_modified: 11
---

# Phase 5 Plan 02: CLI Compliance (CLI-01, CLI-02, CLI-03, SIG-01) Summary

Four surgical Rust changes closing all outstanding CLI and signal requirements against SOLUTION-DESIGN.md. `--body` flag replaces positional task arg, init naming enforces `<project>-<tool>-<role>`, context output uses Markdown heading format with model and description, and signal notification is plain `"<agent> completed <msg-id>"`.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | CLI-01 --body flag + CLI-02 init naming + SIG-01 signal format | 43e34ee |
| 2 | CLI-03 context output with model, description, --body syntax | dd1cc80 |

## Decisions Made

1. **CLI-01 --body flag**: The `Send` subcommand's positional `task` arg is replaced by `#[arg(long)] body: String`. All downstream callers (send.rs, main.rs dispatch) updated. Positional form now correctly rejected by clap.

2. **CLI-02 naming logic**: `init.rs` orchestrator uses `name` field as role suffix if provided, defaults to "orchestrator". Workers use `name` if provided, otherwise `role`. Result is always `<project>-<tool>-<suffix>`. Removed two TODO stubs from Plan 03.

3. **CLI-03 context format**: Replaced `| Agent | Role | Status | Send Command |` table with `## agentname (Model)` headings, description paragraph, `Role: X | Status: Y`, and `→ squad-station send agentname --body "..."`. Usage section also updated.

4. **SIG-01 signal format**: Changed from `[SIGNAL] agent=X status=completed task_id=Y` to `X completed Y`. The clean format is directly pattern-matchable by orchestrator hook scripts.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated integration tests using old positional send syntax**
- **Found during:** Task 1 GREEN phase
- **Issue:** `test_send_agent_not_found` and `test_send_no_tmux_session` in `test_integration.rs` called the binary with positional arg `send agent "do something"` — clap rejects this after CLI-01
- **Fix:** Updated both test invocations to use `--body` flag
- **Files modified:** tests/test_integration.rs
- **Commit:** 43e34ee

**2. [Rule 3 - Blocking] Updated e2e_cli.sh send invocations**
- **Found during:** Task 1 GREEN phase
- **Issue:** 7 invocations in `tests/e2e_cli.sh` used old positional syntax
- **Fix:** Updated all to use `--body` flag
- **Files modified:** tests/e2e_cli.sh
- **Commit:** 43e34ee

**3. [Rule 1 - Bug] Updated test_context_output_contains_agents**
- **Found during:** Task 2 GREEN phase
- **Issue:** Test checked for old `| Agent | Role | Status | Send Command |` table header which no longer exists in CLI-03 format
- **Fix:** Updated assertion to check for `## Available Agents` heading
- **Files modified:** tests/test_lifecycle.rs
- **Commit:** dd1cc80

## Verification

```
cargo test: all test suites — ok
cargo build: clean (no errors)
```

- `send agent --body "task"` accepted by clap
- `send agent "positional"` rejected by clap (exit non-zero)
- `init` with `name: backend` registers `<project>-<tool>-backend`
- `context` output: `## agentname (Model)`, description, `→ squad-station send agentname --body "..."`
- Signal notification: `"<agent> completed <msg-id>"` (no [SIGNAL] prefix)

## Self-Check: PASSED

Files verified present:
- src/cli.rs — `#[arg(long)] body: String` in Send variant
- src/commands/send.rs — `body: String` parameter
- src/commands/init.rs — `<project>-<tool>-<role>` naming
- src/commands/signal.rs — `format!("{} completed {}", agent, task_id_str)`
- src/commands/context.rs — `## {} ({})` heading format

Commits verified:
- 43e34ee — feat(05-02): CLI-01 --body flag, CLI-02 init naming, SIG-01 signal format
- dd1cc80 — feat(05-02): CLI-03 context output with model, description, and --body syntax
