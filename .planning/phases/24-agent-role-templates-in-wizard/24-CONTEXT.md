# Phase 24: Agent Role Templates in Wizard - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

The init wizard presents pre-built role packages so users configure agents with correct descriptions, model suggestions, and routing hints in seconds rather than typing from scratch. Templates auto-fill wizard fields; routing hints persist in DB and appear in squad-orchestrator.md as a Routing Matrix. No reordering by SDD workflow — static template order for all workflows.

</domain>

<decisions>
## Implementation Decisions

### Template catalog — Worker templates
- 8 worker templates in frequency-of-use order: coder, solution-architect, qa-engineer, devops-engineer, code-reviewer, technical-writer, data-engineer, security-engineer
- Plus "Custom" at the bottom of the list — selecting Custom clears all fields (name, model, description) to blank
- "coder" is a broad umbrella role covering frontend, backend, mobile, fullstack development
- "solution-architect" covers tech lead, architect, solution design
- Remaining 6 are specialized roles

### Template catalog — Orchestrator templates
- 3 orchestrator templates: order is Claude's discretion (project-manager, tech-lead, scrum-master)
- Plus "Custom" at the bottom
- Template selector appears on OrchestratorConfig page too, not just WorkerConfig

### Template data structure
- Each template contains: role slug, display name, description text (2-3 sentences), per-provider model mapping, routing hint keywords (list), and default provider
- Descriptions are detailed (2-3 sentences) — not one-liners
- Model suggestions are per-provider: each template maps to a specific model per provider (e.g. solution-architect → opus for Claude / gemini-2.5-pro for Gemini)
- Template also pre-selects Provider (user can override)
- Routing hints are keyword lists (e.g. ["frontend", "ui", "css", "react", "component"]) — consistent with Phase 22 word-intersection alignment approach

### Wizard UX flow
- Template selector appears AFTER Name field, BEFORE Provider: Name → Role Template → Provider → Model → Description
- Selecting a template auto-fills: Name (full role slug e.g. "frontend-engineer"), Provider, Model, Description
- All auto-filled fields are editable — user can override any field after template selection
- Template selector rendered as scrollable list with description preview on the right side (split layout: radio list left, selected template description right)
- Routing hints are hidden from wizard — internal metadata only visible in squad-orchestrator.md
- Worker-only wizard path (add-agents via re-init) also shows the template selector — same UX regardless of entry point

### Routing hints in squad-orchestrator.md
- New "Routing Matrix" section placed AFTER Session Routing
- Markdown table with columns: Keyword | Route to
- Only agents WITH routing hints appear in the matrix — agents without hints are omitted
- If NO agents have routing hints, section still renders with a placeholder message: "No routing hints configured — use templates during init for keyword-based routing"
- `build_orchestrator_md()` remains pure (INTEL-05) — routing hints passed as data from DB

### DB migration (0006)
- New `routing_hints` TEXT column on agents table — nullable
- Stores JSON array format (e.g. `["frontend","ui","css"]`) — requires serde_json for parse/serialize
- Existing agents get NULL (no hints) — NULL means "no template used"

### Template data storage in code
- New `src/commands/templates.rs` module — separate from wizard.rs (which is already 1566 lines)
- Template struct uses `&'static str` slices — zero allocation, compiled into binary
- Wizard imports template data from templates.rs

### Register/Clone integration
- `clone` inherits routing_hints from source agent — clone is a duplicate with same routing
- `register` always produces agents with NULL routing_hints — no --hints flag, keeps register simple
- If user wants hints on a registered agent, use the wizard

### Claude's Discretion
- Orchestrator template ordering (project-manager, tech-lead, scrum-master)
- Exact description text for each of the 11 templates (8 worker + 3 orchestrator)
- Exact keyword lists for each template's routing hints
- Per-provider model mapping details for each template
- Template struct field naming and internal design
- How the scrollable list + preview panel layout is implemented in ratatui

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — TMPL-01 through TMPL-06 define the six template requirements

### Existing wizard implementation
- `src/commands/wizard.rs` — Full wizard code: WizardState, AgentDraft, handle_key(), render_agent_page(), render_radio_list(), run(), run_worker_only()
- `src/commands/wizard.rs` lines 360-396 — AgentDraft struct and AgentField enum (template selector inserts between Name and Provider)
- `src/commands/wizard.rs` lines 676-761 — handle_agent_key() shared handler (needs new template field handling)
- `src/commands/wizard.rs` lines 955-1048 — render_agent_page() (needs split layout with template preview)

### Orchestrator context generation
- `src/commands/context.rs` lines 102-107 — `build_orchestrator_md()` signature (needs routing hints parameter, INTEL-05 purity)
- `src/commands/context.rs` lines 196-207 — Session Routing section (Routing Matrix goes after this)

### DB layer
- `src/db/agents.rs` — Agent struct, `insert_agent()` (needs routing_hints parameter)
- `src/db/migrations/` — Migration files directory (add 0006 for routing_hints column)

### Clone integration
- `src/commands/clone.rs` — Clone command (needs to copy routing_hints from source agent)

### Prior phase context
- `.planning/phases/22-orchestrator-intelligence-data/22-CONTEXT.md` — `build_orchestrator_md()` purity constraint (INTEL-05)
- `.planning/phases/23-dynamic-agent-cloning/23-CONTEXT.md` — Clone copies agent config including new routing_hints

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `render_radio_list()` in wizard.rs — existing radio list renderer, can be extended or paralleled for template selector with preview
- `ModelSelector` pattern — cycling selector with provider-aware options, similar pattern for template selector
- `AgentDraft` struct — directly receives template auto-fill values (name, provider, model index, description, custom_model)
- `handle_agent_key()` — shared key handler for agent pages, needs new `AgentField::Template` variant
- `draft_to_agent_input()` — converts draft to AgentInput, needs to carry routing_hints through

### Established Patterns
- `Provider::cycle_next/prev()` + `index()` + `as_str()` — enum cycling pattern to reuse for template selection
- `SddWorkflow` enum with `ALL` const array — pattern for listing options in radio selector
- Workers filtered via `agents.iter().filter(|a| a.role != "orchestrator")` — same filter for Routing Matrix
- `include_str!()` used for SDD playbooks — could be used if templates move to data files (decided against: templates.rs module instead)

### Integration Points
- `AgentField` enum — add `Template` variant before `Provider`
- `WizardPage::WorkerConfig` and `WizardPage::OrchestratorConfig` — both need template selector
- `insert_agent()` in db/agents.rs — needs new `routing_hints: Option<String>` parameter
- `Agent` struct — needs `routing_hints: Option<String>` field
- `build_orchestrator_md()` — needs routing hints slice parameter for Routing Matrix section
- `context.rs::run()` — needs to fetch routing_hints from DB and pass to builder

</code_context>

<specifics>
## Specific Ideas

- Broad umbrella roles preferred: "coder" covers frontend/backend/mobile/fullstack; "solution-architect" covers tech-lead/architect/solution-design
- Template selector should show a description preview panel on the right side when navigating options (like a split-pane layout)
- Routing Matrix always visible in squad-orchestrator.md even if empty (placeholder message encourages template usage)

</specifics>

<deferred>
## Deferred Ideas

- User-defined local template registry for custom team-specific role packages — tracked as TMPL-07 in REQUIREMENTS.md v2 section
- Template reordering by SDD workflow (TMPL-06 satisfied minimally with static order) — could be enhanced later if workflow-specific ordering proves valuable

</deferred>

---

*Phase: 24-agent-role-templates-in-wizard*
*Context gathered: 2026-03-19*
