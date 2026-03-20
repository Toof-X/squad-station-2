# Phase 3: Views and TUI - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Text status commands, interactive ratatui dashboard, and split tmux pane layout for fleet monitoring. Users can see all agents at a glance without querying individually. VIEW-02 (`agents` command) is already satisfied by Phase 2 — this phase delivers VIEW-01 (status), VIEW-03 (ui), and VIEW-04 (view).

Requirements: VIEW-01, VIEW-03, VIEW-04
Already complete: VIEW-02 (agents command from Phase 2)

</domain>

<decisions>
## Implementation Decisions

### Status command (VIEW-01)
- Shows summary header: project name, total agents, counts by status (e.g., "Agents: 5 — 3 idle, 1 busy, 1 dead"), DB path
- Compact inline list per agent: one line each like "frontend: idle 5m | backend: busy 2m"
- Per-agent pending message counts shown alongside status
- Always reconciles tmux state (like `agents` command does) — accuracy over speed
- Supports `--json` flag for machine-readable output (consistent with all other commands)

### TUI dashboard (VIEW-03)
- Two-panel split layout: left panel = agent list, right panel = messages for selected agent
- Auto-refresh on interval (polling DB periodically)
- Read-only monitoring — no send/signal actions from within TUI
- Keyboard navigation: up/down to select, tab to switch panels, q to quit
- Colored status indicators consistent with `agents` command (green/yellow/red)

### Tmux pane layout (VIEW-04)
- Auto grid layout adapting to agent count (like `tmux select-layout tiled`)
- Include all sessions including orchestrator — full fleet visibility
- Skip dead agents (no tmux session) — only show live sessions
- Clean up on re-run: idempotent behavior if view already exists

### VIEW-02 (agents command)
- Already complete from Phase 2 — no changes needed
- Current output: NAME, ROLE, STATUS (with duration), PROVIDER in colored aligned table

### Claude's Discretion
- DB connection strategy for TUI (connect-per-refresh vs read-only persistent connection) — must satisfy ROADMAP success criteria about WAL checkpoint starvation
- Tmux pane creation approach (new window with linked panes vs rearrange existing sessions)
- TUI auto-refresh interval (2-5 seconds range)
- Exact status command layout and formatting
- TUI keybindings beyond the basics (q, arrows, tab)

</decisions>

<specifics>
## Specific Ideas

- Status command should be the quick "how's my squad doing?" check — denser than `agents`, shows message queue depth
- TUI inspired by lazygit/k9s two-panel model — agents left, context right
- Grid layout for `view` — adapts from 2 columns to NxM based on agent count

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `db::agents::list_agents()`: Returns all agents with status — feeds status, TUI, and view commands
- `db::messages::list_messages()`: Filtered message queries — feeds TUI message panel
- `tmux::session_exists()`: Check if session is alive — used by reconciliation and view pane logic
- `agents.rs` reconciliation loop: Pattern for tmux status reconciliation — reusable in status command
- `colorize_agent_status()` / `format_status_with_duration()`: Coloring helpers from agents.rs
- `pad_colored()`: ANSI-safe column padding

### Established Patterns
- Stateless CLI: every command connects DB, works, exits — status/view follow this model
- Single-writer pool (`max_connections(1)`): TUI must not hold a write connection
- `--json` global flag: status command should support it
- Terminal-aware output: `std::io::IsTerminal` for colored vs plain output

### Integration Points
- `cli.rs`: Add `Status`, `Ui`, `View` subcommand variants
- `main.rs`: Add dispatch for new commands
- `commands/`: New `status.rs`, `ui.rs`, `view.rs` modules
- `tmux.rs`: May need new pane management functions for `view` command
- `Cargo.toml`: `ratatui` and `crossterm` already declared

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-views-and-tui*
*Context gathered: 2026-03-06*
