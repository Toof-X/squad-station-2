# Phase 20: TTY-Safe Welcome TUI Core - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the static `print_welcome()` call with an interactive ratatui TUI on bare `squad-station` invocation. Covers: pixel-font title, version, tagline, commands table, hint bar with auto-exit countdown, TTY guard for non-interactive fallback, and conditional Enter routing (no squad.yml → wizard; squad.yml → dashboard). Quick guide page and post-install auto-launch are Phase 21.

</domain>

<decisions>
## Implementation Decisions

### Screen mode
- Use AlternateScreen (same pattern as `ui.rs`) — full-screen, clean slate, no scrollback pollution
- Clean exit: after `LeaveAlternateScreen`, return to shell prompt with no extra output printed — nothing in scrollback

### Welcome screen content
- pixel-font SQUAD-STATION title via tui-big-text (centered)
- Version string below title (centered)
- Tagline: "Multi-agent orchestration for AI coding" (centered)
- Two-column commands table matching current static welcome: `cmd    description` format, all 11 subcommands (init, send, signal, peek, list, ui, view, status, agents, context, register)
- All content centered horizontally

### Countdown behavior
- Auto-exit after **5 seconds** if no key pressed
- Any valid keypress (Enter, Q, Esc) cancels countdown and acts immediately — countdown only matters if user is AFK
- On timeout: silent close, return to shell — same as Q; no wizard auto-launch on timeout

### Conditional routing UX
- **No squad.yml:** hint bar = `Enter: Set up  Q: Quit  auto-exit 5s` → Enter closes TUI and launches init wizard
- **squad.yml exists:** hint bar = `Enter: Open dashboard  Q: Quit  auto-exit 5s` → Enter closes TUI and launches `squad-station ui`
- No check for active tmux sessions — unconditionally offer dashboard when squad.yml exists; `ui.rs` handles empty state gracefully

### Non-TTY fallback
- When `stdout` is not a terminal (`!stdout.is_terminal()`), skip TUI entirely and print static text (current `print_welcome()` pattern)
- No raw mode attempted in non-TTY context

### Claude's Discretion
- Exact ratatui layout constraint values (padding, margin sizes)
- tui-big-text font variant (BigText default or specific style)
- Countdown tick implementation (crossterm event timeout polling)
- Color scheme for the TUI (red accent for title consistent with current ASCII art, or full ratatui styling)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing TUI implementation
- `src/commands/ui.rs` — Full ratatui TUI using AlternateScreen + raw mode; exact setup/teardown pattern to replicate in welcome TUI
- `src/commands/welcome.rs` — Current static welcome screen being replaced; `welcome_content()` defines the command list to port to TUI; tests to update/replace

### Entry point
- `src/main.rs` — `None` arm (line 24-25) calls `print_welcome()` — this becomes the welcome TUI entry point; TTY guard goes here or inside welcome module

### Requirements
- `.planning/REQUIREMENTS.md` — WELCOME-01 through WELCOME-07, INIT-01 through INIT-03 define acceptance criteria for this phase

### Dependency versions
- `Cargo.toml` — current ratatui 0.26 + crossterm 0.27; upgrade to ratatui 0.29 + crossterm 0.28 required for tui-big-text 0.7.x compatibility

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/ui.rs` `run()` function: AlternateScreen setup/teardown + raw mode pattern — copy structure directly into welcome TUI event loop
- `src/commands/welcome.rs` `welcome_content()`: command list strings to port into TUI Paragraph widget
- `std::io::stdout().is_terminal()` pattern: already used in `notify.rs`, `send.rs`, `signal.rs` — consistent TTY guard idiom

### Established Patterns
- AlternateScreen entry: `execute!(stdout, EnterAlternateScreen)` + `enable_raw_mode()` + `CrosstermBackend` → `Terminal::new()` — verbatim from `ui.rs:147`
- AlternateScreen exit: `disable_raw_mode()` + `execute!(terminal.backend_mut(), LeaveAlternateScreen)` — verbatim from `ui.rs:155`
- Event polling: `event::poll(Duration)` + `event::read()` + `KeyEventKind::Press` guard — verbatim from `ui.rs`
- `Option<Commands>` in clap Cli struct: `None` arm already exists in `main.rs:24` — welcome TUI replaces `print_welcome()` call there, no CLI changes needed

### Integration Points
- `main.rs:25`: `commands::welcome::print_welcome()` → replace with `commands::welcome::run_welcome_tui()` (or equivalent) + TTY guard
- `commands/mod.rs:19`: `pub mod welcome` already declared — new welcome TUI lives in same file, no mod changes needed
- After TUI exits with Enter + no squad.yml: call `commands::init::run()` (same as `Commands::Init` arm)
- After TUI exits with Enter + squad.yml exists: call `commands::ui::run()` (same as `Commands::Ui` arm)

</code_context>

<specifics>
## Specific Ideas

- Hint bar wording exactly: `Enter: Set up  Q: Quit  auto-exit 5s` (no squad.yml) vs `Enter: Open dashboard  Q: Quit  auto-exit 5s` (squad.yml exists)
- countdown ticks down live in the hint bar: `auto-exit 4s`, `auto-exit 3s`... down to `auto-exit 1s` then close

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 20-tty-safe-welcome-tui-core*
*Context gathered: 2026-03-17*
