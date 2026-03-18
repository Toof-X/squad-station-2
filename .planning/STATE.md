---
gsd_state_version: 1.0
milestone: v1.8
milestone_name: Install & Live Status
status: defining_requirements
stopped_at: defining requirements
last_updated: "2026-03-18"
last_activity: 2026-03-18 — feat(init): add --tui flag to init command; npm bumped to v1.5.7 (binaryVersion 1.8)
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-18)

**Core value:** Routing messages reliably between Orchestrator and agents — stateless CLI, no daemon
**Current focus:** Defining requirements for v1.8

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-03-18 — feat(init): add --tui flag to init command; npm bumped to v1.5.7 (binaryVersion 1.8)

## Code Status

**Rust crate:** v0.5.4 (`Cargo.toml`)
**npm package:** v1.5.7 (`npm-package/package.json`, binaryVersion: 1.8)
**Last shipped milestone:** v1.7 First-Run Onboarding (2026-03-18)

### Recent changes (since v1.7 shipped)

- `feat(init): add --tui flag` — `init` now requires explicit `--tui` to enter the wizard; bare `init` reads existing `squad.yml` directly and notifies if missing. Re-init prompt only shown with `--tui`. Welcome TUI `LaunchInit` uses `tui=true`; `reset` relaunch uses `tui=false`.
- `fix(npm)`: removed install banner, updated step 3 hint to reference `--tui`
- `docs`: updated init quickstart for `--tui` flag
- npm bumped through v1.5.3 → v1.5.7; binaryVersion set to 1.8

## Accumulated Context

### Decisions

All decisions logged in PROJECT.md Key Decisions table.

### Pending Todos

None.

### Blockers/Concerns

None.
