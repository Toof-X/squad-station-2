# Phase 19: Agent Diagram - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Print an ASCII fleet diagram immediately after `squad-station init` completes. Shows orchestrator + worker agent boxes with directional arrows and live DB status. This is a one-time post-init print — not an interactive TUI, not a standalone subcommand (that is a future req DIAG-F01).

</domain>

<decisions>
## Implementation Decisions

### Layout & orientation
- Vertical stack: orchestrator box centered at top, worker boxes in a horizontal row below
- Workers laid out in a fixed-width row (~80 cols); when workers exceed the row width, wrap to a new row below
- Arrows run from the orchestrator box down to each worker box (▼ per worker column)

### Box content
- Agent name is also the tmux session name — show it once (no duplication)
- Orchestrator box: first line is bold/uppercase `ORCHESTRATOR` label, then name, then `tool: <tool>  model: <model>`, then `[status]`
- Worker boxes: name on first line, then `tool: <tool>  model: <model>`, then `[status]`
- Fields per box: name, role (implied by position/label), tool, model (if set), status

### Color & visual style
- Unicode box-drawing characters: `┌─┐`, `│`, `└─┘`, `▼` arrows
- Box borders are neutral (no color)
- Only the `[status]` badge is colored via owo_colors: green=idle, yellow=busy, red=dead
- Consistent with how `colorize_agent_status` works in helpers.rs

### Placement in init output
- Diagram printed as the final section after the "Get Started:" block
- Section header: `Agent Fleet:` (newline before)
- Suppressed when `--json` flag is active — same guard as hook instructions in init.rs
- Uses `if_supports_color(Stream::Stdout, ...)` for all color output

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — DIAG-01, DIAG-02, DIAG-03 define exact fields and behavior required

### Existing patterns to follow
- `src/commands/welcome.rs` — owo_colors pattern: `if_supports_color(Stream::Stdout, |s| s.red())` for conditional color
- `src/commands/init.rs` — Post-init output block structure and `--json` guard pattern
- `src/commands/helpers.rs` — `colorize_agent_status()` function for status badge coloring; `reconcile_agent_statuses()` for fresh status before display
- `src/db/agents.rs` — `Agent` struct fields: `name`, `tool`, `role`, `model`, `status`; `list_agents()` for fetching

No external specs — requirements are fully captured in decisions above and REQUIREMENTS.md.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `colorize_agent_status(status)` in helpers.rs: returns colored status string — use directly for `[status]` badges
- `reconcile_agent_statuses(&pool)` in helpers.rs: syncs DB status against live tmux panes — call before diagram to show accurate status
- `list_agents(&pool)` in db/agents.rs: returns `Vec<Agent>` ordered by name
- `owo_colors::OwoColorize` + `owo_colors::Stream` already imported in welcome.rs — same pattern for diagram

### Established Patterns
- `if_supports_color(Stream::Stdout, |s| s.green())` — use for all colored output (consistent with welcome.rs and init.rs)
- `--json` guard in init.rs: `if !json_mode { ... }` wraps hook instructions — wrap diagram print with same guard
- Unicode box borders already used for `══════` lines in init.rs; `┌─┐└─┘│` are the natural extension

### Integration Points
- `src/commands/init.rs` — diagram call goes after the "Get Started:" println block, before function return
- Diagram module: new file `src/commands/diagram.rs` with `pub fn print_diagram(agents: &[Agent])` — called from init.rs after agent registration and reconciliation
- The `pool` is already available at the end of init flow — pass agents slice directly to avoid a second DB fetch

</code_context>

<specifics>
## Specific Ideas

- The diagram from the discussion preview is the target visual:
  ```
  ┌───────────────────────────────┐
  │ ORCHESTRATOR                  │
  │ myproj-claude-code-orch       │
  │ tool: claude-code  [idle]     │
  └───────────────────────────────┘
          │            │
          ▼            ▼
  ┌─────────────┐  ┌─────────────┐
  │ worker1     │  │ worker2     │
  │ tool: cc    │  │ tool: cc    │
  │ [idle]      │  │ [busy]      │
  └─────────────┘  └─────────────┘
  ```
- Status badge format: `[idle]`, `[busy]`, `[dead]` — brackets included, colored text inside

</specifics>

<deferred>
## Deferred Ideas

- `squad-station diagram` standalone subcommand — DIAG-F01 in REQUIREMENTS.md, explicitly future
- Message queue depth per agent in diagram — DIAG-F02, explicitly future
- Animated/updating diagram — out of scope per REQUIREMENTS.md Out of Scope section

</deferred>

---

*Phase: 19-agent-diagram*
*Context gathered: 2026-03-17*
