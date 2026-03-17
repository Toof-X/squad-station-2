# Phase 16: TUI Wizard - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Build a ratatui TUI form that collects project name, agent count, and per-agent config (role, tool, model, description) with inline validation — returning collected values to the calling code without writing any files. File writing and re-init handling are Phase 17.

</domain>

<decisions>
## Implementation Decisions

### Form layout
- Multi-page / step-by-step: each section gets its own screen
  - Page 1: project name
  - Page 2: agent count (integer input)
  - Pages 3..N+2: per-agent config (one page per agent)
  - Final page: summary/review screen
- Progress indicator in header (e.g. "Step 2 of 5")
- Summary/review screen after all agents filled — shows project name + all agents as a list. User confirms with Enter or goes back with Esc.

### Agent entry flow
- User types explicit agent count (integer) on page 2, then presses Enter
- Sequential per-agent pages follow: "Agent 1 of N", "Agent 2 of N", etc.
- Required fields per agent: role, tool
- Optional fields per agent: model, description (can be left blank, stored as None/empty)

### Validation UX
- Errors appear inline below the offending field in red text (⚠ message)
- Offending field border turns red on error
- Cursor stays in the field — user must fix before advancing
- Validation fires on Enter (submit attempt), not on field exit
- Tool field is NOT a text input — it's an enum selector (Left/Right arrow or Tab to cycle through: claude-code → gemini-cli → antigravity)
- Role field: validate non-empty string on Enter
- Agent count field: validate positive integer (≥ 1) on Enter

### Key bindings & navigation
- Enter: submit current field / advance to next page
- Esc: go back one page (not cancel — navigates backward through wizard steps)
- Ctrl+C: cancel wizard entirely, return None/error to caller (no files written)
- Left/Right arrows (or Tab): cycle tool selector values on the tool field
- Backspace: delete last character in text fields

### Claude's Discretion
- Exact color scheme for focused vs unfocused fields
- Border width, padding, text field width
- Whether to show a "hint" line below optional fields (e.g. "optional — press Enter to skip")

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

No external specs — requirements are fully captured in decisions above.

Requirements reference: `.planning/REQUIREMENTS.md` — INIT-01, INIT-02, INIT-03, INIT-06, INIT-07

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/ui.rs`: Complete ratatui terminal setup pattern — `setup_terminal()`, `restore_terminal()`, panic hook restoring terminal on panic, event loop using `event::poll(Duration::from_millis(250))`. The wizard should use the same pattern verbatim.
- `src/commands/ui.rs`: `enable_raw_mode` / `disable_raw_mode` + `EnterAlternateScreen` / `LeaveAlternateScreen` already imported and working.
- Existing widget imports: `Block`, `Borders`, `Paragraph`, `Layout`, `Constraint`, `Direction`, `Color`, `Style`, `Modifier`, `Line`, `Span` — all in scope.

### Established Patterns
- No existing form/text-input widget in the codebase — must implement `TextInputState` (cursor position + buffer string) manually. This is standard ratatui practice.
- Event handling: `KeyEventKind::Press` guard already used in ui.rs — reuse the same pattern.
- TUI color conventions in use: `Color::Cyan` for focused panels, `Color::Green`/`Color::Yellow`/`Color::Red` for status. Follow the same palette.

### Integration Points
- `src/commands/init.rs::run()` is the entry point — wizard will be called at the top of this function when no squad.yml exists, before `config::load_config()`.
- Wizard returns a structured value (e.g. `WizardResult { project: String, agents: Vec<AgentInput> }`) that Phase 17 will use to generate squad.yml.
- The wizard lives in a new file: `src/commands/wizard.rs` (or `src/wizard.rs`) with a `pub async fn run() -> anyhow::Result<Option<WizardResult>>` signature.

</code_context>

<specifics>
## Specific Ideas

- No specific references cited — standard ratatui wizard feel is the target
- Tool selector should feel like a toggle/radio, not a dropdown

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 16-tui-wizard*
*Context gathered: 2026-03-17*
