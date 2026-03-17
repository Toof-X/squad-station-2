---
phase: 19-agent-diagram
plan: "01"
subsystem: commands/diagram
tags: [ascii-diagram, agent-fleet, init, ux]
dependency_graph:
  requires: []
  provides: [diagram-module, init-diagram-output]
  affects: [src/commands/init.rs]
tech_stack:
  added: []
  patterns: [owo-colors for colored status badges, box-drawing Unicode chars, TDD with render function for testability]
key_files:
  created:
    - src/commands/diagram.rs
  modified:
    - src/commands/mod.rs
    - src/commands/init.rs
decisions:
  - render_diagram returns String so tests can assert without capturing stdout; print_diagram calls it
  - flush-left orchestrator box (no centering) accepted per plan spec
  - Workers wrap to new rows when cumulative width + gap exceeds 80 chars
  - Full-path crate::commands::helpers::reconcile_agent_statuses used — no new use import needed in init.rs
metrics:
  duration: "~3 minutes"
  completed: "2026-03-17"
  tasks_completed: 2
  files_changed: 3
---

# Phase 19 Plan 01: Agent Fleet Diagram Summary

ASCII fleet diagram that prints after `squad-station init`, showing orchestrator and worker agents as labeled boxes with box-drawing borders, directional arrows, and colored status badges.

## What Was Built

- **`src/commands/diagram.rs`** — New module with:
  - `pub fn print_diagram(agents: &[Agent])` — prints diagram to stdout
  - `pub fn render_diagram(agents: &[Agent]) -> String` — returns diagram as string (enables unit testing)
  - `fn render_agent_box(agent, is_orchestrator)` — builds Unicode box with ORCHESTRATOR header (when orch), name, tool + optional model, colored status badge
  - `fn render_arrow_row(worker_boxes, gap)` — builds `│` / `▼` connector lines between orchestrator and worker row
  - `fn visible_len(s)` — strips ANSI escape codes to compute visible display width
  - `fn build_content_lines(agent, is_orchestrator)` — returns (raw, colored) pairs for width measurement and display
  - 10 unit tests covering all specified behaviors

- **`src/commands/mod.rs`** — Added `pub mod diagram;` between `context` and `freeze` (alphabetical order)

- **`src/commands/init.rs`** — Inside `if !json { ... }` block, after "Get Started:" section:
  - Calls `reconcile_agent_statuses(&pool).await?` to refresh status from tmux
  - Calls `list_agents(&pool).await?` to fetch agents
  - Calls `diagram::print_diagram(&agents)` — suppressed in JSON mode by the surrounding guard

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

### Files

- [x] src/commands/diagram.rs exists
- [x] src/commands/mod.rs contains `pub mod diagram;`
- [x] src/commands/init.rs contains `diagram::print_diagram`

### Tests

- [x] `cargo test diagram` — 10 passed, 0 failed
- [x] `cargo check` — clean
- [x] `cargo test` (full suite) — all tests pass, no regressions

### Commits

- 2539bb1: feat(19-01): add diagram.rs module with print_diagram and render_diagram
- c151aaa: feat(19-01): integrate diagram into init post-setup output

## Self-Check: PASSED
