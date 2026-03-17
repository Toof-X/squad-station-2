---
phase: 16-tui-wizard
plan: 01
subsystem: ui
tags: [ratatui, crossterm, tui, wizard, form, rust]

# Dependency graph
requires: []
provides:
  - "src/commands/wizard.rs: complete TUI wizard module with types, validation, rendering, and event loop"
  - "WizardResult, AgentInput, SddWorkflow, Provider, Role, ModelSelector public API types for squad.yml generation"
affects: [17-init-integration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "5-page TUI: Project (with SDD sub-field), OrchestratorConfig, WorkerCount, WorkerConfig x N, Summary"
    - "TDD red-green for validation functions and data types before TUI rendering"
    - "State machine with WizardPage enum driving per-page render and key dispatch"
    - "TextInputState with cursor position: push/pop insert at cursor, cursor_left/right move, display(active) renders with '|' marker"
    - "render_radio_list helper renders visible list with ●/○ markers and Cyan highlight for selected option"
    - "handle_agent_key shared helper handles Name/Provider/Model/Description fields for both orchestrator and worker pages"
    - "PageTransition enum returned by handle_agent_key to decouple field logic from page routing"
    - "ModelSelector: per-provider model list with index cycling, reset on provider change, 'other' triggers custom_model text input"

key-files:
  created:
    - src/commands/wizard.rs
  modified:
    - src/commands/mod.rs

key-decisions:
  - "Provider enum (not Tool) cycles ClaudeCode -> GeminiCli -> Antigravity (matches VALID_PROVIDERS order)"
  - "Orchestrator configured on its own dedicated page (OrchestratorConfig), separate from worker pages"
  - "SddWorkflow enum (Bmad/GetShitDone/Superpower) added to Project page as radio list sub-field"
  - "WizardResult holds project, sdd, orchestrator, and agents (workers) — not a flat agents list"
  - "AgentInput has name (optional identifier), role (implicit), provider, model, description"
  - "AgentDraft has name (TextInputState), provider (Provider enum), model (ModelSelector), custom_model (TextInputState), description"
  - "AgentField enum: Name/Provider/Model/Description — role is implicit from which page, not a field"
  - "TextInputState cursor support: insert at cursor position, left/right arrow moves cursor, display() renders '|' at cursor"
  - "ModelSelector: per-provider static option lists; 'other' option unlocks custom_model text input"
  - "Antigravity provider skips Model field entirely (goes Name -> Provider -> Description)"
  - "frame.size() used instead of frame.area() for ratatui 0.26.3 compatibility"
  - "Workers pre-allocated as vec on WorkerCount confirm; Esc navigates by index (no data loss)"
  - "validate_project_name and validate_role removed — name is optional, role is implicit from page context"

patterns-established:
  - "Wizard pages: state enum variant drives both render function and handle_key dispatch"
  - "TUI module copies terminal setup/teardown pattern from src/commands/ui.rs"
  - "Panic hook installs before terminal setup to guarantee restore_terminal on crash"
  - "render_radio_list: shared helper for any enum-selector field (SDD workflow, Provider, Model)"
  - "PageTransition enum: handle_agent_key returns Stay or Go(page) — page routing in caller"

requirements-completed: [INIT-01, INIT-02, INIT-03, INIT-06, INIT-07]

# Metrics
duration: 3min
completed: 2026-03-17
---

# Phase 16 Plan 01: TUI Wizard Core Summary

**Multi-page ratatui wizard in src/commands/wizard.rs: 5 pages (Project+SDD, OrchestratorConfig, WorkerCount, WorkerConfig×N, Summary) collecting project name, SDD workflow, orchestrator config, and per-worker config (name, provider, model, description) with cursor-aware text inputs and radio-list selectors.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-17T04:01:11Z
- **Completed:** 2026-03-17T04:04:34Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Complete wizard module (1362 lines) covering all data types, state machine, validation, rendering, and event loop
- 20+ unit tests covering TextInputState (push/pop/cursor/display), Provider cycling/as_str, Role cycling/as_str, ModelSelector (claude/gemini/antigravity options, is_other, reset), validate_count
- 5 TUI page types rendered: Project (Name + SDD radio), OrchestratorConfig, WorkerCount, WorkerConfig, Summary
- Cyan/Red/DarkGray border styling; radio lists with ●/○ markers; cursor `|` in active text fields
- Ctrl+C returns Ok(None) cancel; Enter on Summary returns Ok(Some(WizardResult)) with trimmed values

## Actual Types Built

```rust
pub struct WizardResult {
    pub project: String,
    pub sdd: SddWorkflow,
    pub orchestrator: AgentInput,
    pub agents: Vec<AgentInput>, // workers
}

pub struct AgentInput {
    pub name: String,
    pub role: String,     // "orchestrator" | "worker"
    pub provider: String, // "claude-code" | "gemini-cli" | "antigravity"
    pub model: Option<String>,
    pub description: Option<String>,
}

pub enum SddWorkflow { Bmad, GetShitDone, Superpower }
pub enum Provider { ClaudeCode, GeminiCli, Antigravity }
pub enum Role { Orchestrator, Worker }
pub struct ModelSelector { index: usize }  // per-provider model list with cycling
```

## Task Commits

1. **Task 1: Data types, validation functions, and unit tests** - `41b5749` (feat)
2. **Task 2: TUI rendering, event loop, and state machine** - `04f4f7a` (feat)

## Files Created/Modified

- `src/commands/wizard.rs` — Complete TUI wizard: WizardResult, AgentInput, SddWorkflow, Provider, Role, ModelSelector, TextInputState (with cursor), AgentDraft, AgentField, WizardPage, ProjectField, WizardState, PageTransition, KeyAction, validation, setup/restore_terminal, render_page, render_project_page, render_agent_page, render_text_input, render_radio_list, render_summary_page, handle_key, handle_agent_key, draft_to_agent_input, draft_summary_line, run()
- `src/commands/mod.rs` — Added `pub mod wizard;` registration

## Decisions Made

- Renamed `Tool` → `Provider`; added `name` field to `AgentInput`; `WizardResult` now has separate `sdd`, `orchestrator`, `agents` fields (not a flat list)
- Orchestrator gets its own page (OrchestratorConfig) — makes role implicit, reduces form complexity
- SddWorkflow (Bmad/GetShitDone/Superpower) added to Project page as radio sub-field
- TextInputState gained cursor position (`cursor: usize`), `cursor_left/right`, and `display(active)` — supports mid-string editing
- ModelSelector provides per-provider model lists; "other" option unlocks a free-text `custom_model` field
- Antigravity provider skips Model step (goes Name → Provider → Description)
- `validate_project_name` and `validate_role` were NOT implemented — name is optional (identifier only), role is implicit from page context
- `PageTransition` enum returned from `handle_agent_key` to keep field logic decoupled from page routing
- `render_radio_list` shared helper used for SDD, Provider, and Model selectors

## Deviations from Plan

- **WizardResult structure changed:** `{ project, agents }` → `{ project, sdd, orchestrator, agents }` — orchestrator separated out; SDD workflow added
- **AgentInput changed:** `tool: String` → `provider: String`; added `name: String`
- **Tool enum renamed to Provider**
- **New types added:** `SddWorkflow`, `Role`, `ModelSelector`
- **TextInputState got cursor support** (not planned — push/pop-only in plan)
- **AgentField changed:** `Role` → `Name`, `Tool` → `Provider`
- **5 pages instead of 4:** Project (with SDD sub-field) + OrchestratorConfig added; AgentCount → WorkerCount; AgentConfig → WorkerConfig
- **`validate_project_name` and `validate_role` not implemented** — name optional, role implicit
- **20+ unit tests** instead of planned 11 — cursor movement, ModelSelector, Role/Provider added

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Wizard module is complete and compiles cleanly
- Phase 17 (init-integration) imports `commands::wizard::run()` and uses `WizardResult` with fields: `project`, `sdd`, `orchestrator`, `agents`
- `AgentInput.provider` (not `.tool`) and `AgentInput.name` are available for squad.yml generation
- No blockers

---
*Phase: 16-tui-wizard*
*Completed: 2026-03-17*
*Updated: 2026-03-17 — corrected to match actual implementation*

## Self-Check: PASSED
- src/commands/wizard.rs: FOUND (1362 lines)
- .planning/phases/16-tui-wizard/16-01-SUMMARY.md: FOUND
- commit 41b5749 (task 1): FOUND
- commit 04f4f7a (task 2): FOUND
