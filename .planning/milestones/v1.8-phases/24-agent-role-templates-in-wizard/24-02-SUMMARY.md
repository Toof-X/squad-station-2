---
phase: 24-agent-role-templates-in-wizard
plan: "02"
subsystem: wizard-ui
tags: [ratatui, tui, template-selector, routing-hints, agent-wizard]
dependency_graph:
  requires: [24-01]
  provides: [template-selector-ui, routing-hints-db-flow]
  affects: [src/commands/wizard.rs, src/commands/init.rs]
tech_stack:
  added: []
  patterns:
    - ratatui horizontal split layout (45%/55%) for two-pane template selector
    - radio list reuse (render_radio_list) for template left pane
    - Paragraph with Wrap for description preview right pane
    - HashMap routing hint carryover from wizard result to DB insert
key_files:
  created: []
  modified:
    - src/commands/wizard.rs
    - src/commands/init.rs
decisions:
  - "Use orch_role (raw wizard name) not orch_name (sanitized session name) as HashMap key for orchestrator routing hints lookup — these are different strings and must align"
  - "extract_routing_hints stores raw agent names as keys (matching role_suffix in the workers loop)"
  - "Template selector always shown — no conditional rendering based on whether user skipped templates; custom option provides escape hatch"
  - "Provider Esc now navigates back to Template field, completing the Name -> Template -> Provider -> Model -> Description chain"
metrics:
  duration_minutes: 5
  completed_date: "2026-03-19"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Phase 24 Plan 02: Wizard Template Selector UI and Routing Hints Wiring Summary

Template selector UI integrated into both orchestrator and worker wizard pages with split-pane layout (45% radio list / 55% description preview), auto-fill on template confirmation, and routing_hints propagated from wizard result through to the DB insert_agent call in init.rs.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Extend AgentField/AgentDraft + template key handling | 19a806e | src/commands/wizard.rs |
| 2 | Render split-pane layout + wire routing_hints through init.rs | 4085a65 | src/commands/wizard.rs, src/commands/init.rs |

## What Was Built

### Task 1 — AgentField/AgentDraft Extensions and Key Handling

**`src/commands/wizard.rs`:**

- Added `AgentField::Template` variant between `Name` and `Provider`
- Extended `AgentDraft` with three new fields:
  - `template_index: usize` — index into the template list (last index = Custom)
  - `is_orchestrator: bool` — selects ORCHESTRATOR_TEMPLATES vs WORKER_TEMPLATES
  - `routing_hints: Option<Vec<&'static str>>` — set by template selection, None for Custom
- `WizardState::new()` sets `orchestrator.is_orchestrator = true`; worker drafts default to `false`
- Added `AgentField::Template` arm in `handle_agent_key()`:
  - Up/Down: moves `draft.template_index` within bounds
  - Enter/Tab: applies auto-fill (name=slug, provider, model index, description, routing_hints) and advances to Provider
  - Enter/Tab on Custom: clears all fields to defaults, routing_hints=None
  - Esc: returns focus to Name field
- Updated `AgentField::Name` Enter/Tab to advance to `Template` (was Provider)
- Updated `AgentField::Provider` Esc to go back to `Template` (was Name)
- Navigation chain: Name -> Template -> Provider -> Model -> Description
- `draft_to_agent_input()` serializes `routing_hints` via `serde_json::to_string`
- Added "select template" footer hint for `AgentField::Template`

### Task 2 — Split-Pane Render + init.rs Routing Hints Wiring

**`src/commands/wizard.rs`:**

- `render_agent_page()` computes `template_h = (templates_list.len() + 1 + 2) as u16`
- Inserts `Constraint::Length(template_h)` at slot 2, shifting all subsequent slots by +1
- New slot 2 split into horizontal layout: 45% left (radio list), 55% right (description preview)
- Left pane: `render_radio_list()` with title `" Role Template "`, display_names = template display names + "Custom"
- Right pane: `Paragraph` with `Wrap { trim: true }`, title `" Preview "`, shows template description or "Enter role and description manually." for Custom
- Preview border Cyan when template focused, DarkGray otherwise

**`src/commands/init.rs`:**

- Declared `wizard_routing_hints: Option<HashMap<String, Option<String>>>` before conditional wizard branches
- In new-file wizard branch and Overwrite branch: populates via `extract_routing_hints(&result)` before result is consumed
- In AddAgents branch: builds HashMap from `new_workers` keyed by agent name (raw)
- Orchestrator `insert_agent` call: looks up by `orch_role` (the raw name) — NOT by `orch_name` (which is the sanitized session name)
- Worker `insert_agent` call: looks up by `role_suffix` (the raw name)
- Added `extract_routing_hints()` helper at bottom of file — stores raw agent names as keys

## Decisions Made

1. **orch_role vs orch_name for routing hints lookup:** The wizard result stores the raw name (e.g. "coder") while init.rs computes a sanitized session name (e.g. "myproject-coder"). The HashMap must be keyed by raw names — use `orch_role` (which equals `config.orchestrator.name.as_deref().unwrap_or("orchestrator")`) for the orchestrator lookup, and `role_suffix` for workers.

2. **extract_routing_hints raw-name keys:** This design ensures the key in the HashMap matches the variable used in the agents loop (`role_suffix`), avoiding a sanitization mismatch.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed as written with one design clarification.

**Clarification: orch_name vs orch_role for HashMap key**

The plan stated "use `&orch_name`" for the HashMap lookup but `orch_name` is the sanitized session name (e.g. "myproject-coder") while `extract_routing_hints` stores the raw wizard name as key (e.g. "coder"). Used `orch_role` instead, which is the raw name. This is a precision improvement over the plan wording — both point to the same correct implementation intent.

## Verification

- `cargo check`: Finished with 1 pre-existing warning (diagram.rs unused_assignments), 0 new warnings
- `cargo test`: All 290 tests pass (0 failures)
- Template selector visible in wizard layout: render_agent_page has Direction::Horizontal split
- Navigation flow: Name -> Template -> Provider -> Model -> Description
- Routing hints flow: wizard template selection -> AgentDraft.routing_hints -> draft_to_agent_input -> AgentInput.routing_hints -> init.rs extract_routing_hints -> insert_agent -> DB

## Self-Check: PASSED

Files created/modified:
- FOUND: src/commands/wizard.rs
- FOUND: src/commands/init.rs

Commits:
- 19a806e: feat(24-02): extend AgentField/AgentDraft with template selector + key handling
- 4085a65: feat(24-02): render template split-pane layout + wire routing_hints through init.rs
