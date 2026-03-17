---
phase: 17-init-flow-integration
plan: "02"
subsystem: init
tags: [re-init, wizard, prompt, append, yaml]
dependency_graph:
  requires: ["17-01"]
  provides: ["INIT-05"]
  affects: ["src/commands/init.rs"]
tech_stack:
  added: []
  patterns: ["crossterm raw mode keypress", "is_terminal() non-interactive guard", "string YAML append"]
key_files:
  created: []
  modified:
    - src/commands/init.rs
decisions:
  - "Non-interactive detection via std::io::IsTerminal — skip prompt_reinit() when stdin is not a TTY; falls through to load_config for backward-compatible behavior"
  - "Ctrl+C and Esc both map to ReinitChoice::Abort for consistent exit UX"
  - "append_workers_to_yaml is pure string manipulation (not serde_yaml) — consistent with generate_squad_yml approach from Plan 01"
  - "is_terminal() guard preserves all existing integration tests without modification"
metrics:
  duration: "~12 minutes"
  completed: "2026-03-17"
  tasks_completed: 1
  tasks_total: 2
  files_modified: 1
---

# Phase 17 Plan 02: Re-init Prompt (Overwrite / Add Agents / Abort) Summary

Re-init handling for `squad-station init` when squad.yml already exists: interactive prompt with three choices (overwrite, add agents, abort) plus non-interactive fallback.

## What Was Built

### ReinitChoice enum

```rust
enum ReinitChoice { Overwrite, AddAgents, Abort }
```

### prompt_reinit()

Uses crossterm raw mode to capture a single keypress. Displays three options and maps:
- `o` → Overwrite (run full wizard, replace squad.yml)
- `a` → AddAgents (run worker-only wizard, append to squad.yml)
- `q` / Esc / Ctrl+C → Abort (exit without changes)

### append_workers_to_yaml()

Pure string append: takes existing YAML content and a slice of `AgentInput`, returns new content with each worker appended as YAML list entries. Preserves all existing content unchanged. Returns original if workers slice is empty.

### init::run() restructured

```
if !config_path.exists()
  → first-time: run full wizard, write squad.yml
else if stdin.is_terminal()
  → re-init: prompt_reinit() → Overwrite / AddAgents / Abort
// else: non-interactive (test env) — fall through to load_config
```

The `is_terminal()` guard ensures all existing integration tests continue passing without modification.

## Task Status

| Task | Name | Status | Commit |
|------|------|--------|--------|
| 1 | Implement re-init prompt | Complete | ba01fce |
| 2 | Human E2E verification | Paused at checkpoint | — |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Integration tests broke due to prompt_reinit() in non-TTY environment**
- **Found during:** Task 1 verification (GREEN phase)
- **Issue:** `crossterm::terminal::enable_raw_mode()` returns "Device not configured (os error 6)" when stdin is not a TTY (integration test subprocess). 4 existing tests failed.
- **Fix:** Added `else if std::io::stdin().is_terminal()` guard. Non-interactive runs skip prompt_reinit() and fall through directly to load_config — matching pre-change behavior.
- **Files modified:** src/commands/init.rs
- **Commit:** ba01fce (included in Task 1 commit)

## Self-Check

- [x] src/commands/init.rs modified with ReinitChoice, prompt_reinit(), append_workers_to_yaml()
- [x] Task 1 commit ba01fce exists
- [x] All 6 new tests pass
- [x] Full test suite passes (0 failures across all test files)
- [x] cargo build --release succeeds

## Self-Check: PASSED
