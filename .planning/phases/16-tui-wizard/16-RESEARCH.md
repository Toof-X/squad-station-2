# Phase 16: TUI Wizard - Research

**Researched:** 2026-03-17
**Domain:** ratatui TUI form / multi-page wizard in Rust
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Form layout**
- Multi-page / step-by-step: each section gets its own screen
  - Page 1: project name
  - Page 2: agent count (integer input)
  - Pages 3..N+2: per-agent config (one page per agent)
  - Final page: summary/review screen
- Progress indicator in header (e.g. "Step 2 of 5")
- Summary/review screen after all agents filled — shows project name + all agents as a list. User confirms with Enter or goes back with Esc.

**Agent entry flow**
- User types explicit agent count (integer) on page 2, then presses Enter
- Sequential per-agent pages follow: "Agent 1 of N", "Agent 2 of N", etc.
- Required fields per agent: role, tool
- Optional fields per agent: model, description (can be left blank, stored as None/empty)

**Validation UX**
- Errors appear inline below the offending field in red text (warning message)
- Offending field border turns red on error
- Cursor stays in the field — user must fix before advancing
- Validation fires on Enter (submit attempt), not on field exit
- Tool field is NOT a text input — it is an enum selector (Left/Right arrow or Tab to cycle through: claude-code → gemini-cli → antigravity)
- Role field: validate non-empty string on Enter
- Agent count field: validate positive integer (>= 1) on Enter

**Key bindings & navigation**
- Enter: submit current field / advance to next page
- Esc: go back one page (not cancel — navigates backward through wizard steps)
- Ctrl+C: cancel wizard entirely, return None/error to caller (no files written)
- Left/Right arrows (or Tab): cycle tool selector values on the tool field
- Backspace: delete last character in text fields

### Claude's Discretion
- Exact color scheme for focused vs unfocused fields
- Border width, padding, text field width
- Whether to show a "hint" line below optional fields (e.g. "optional — press Enter to skip")

### Deferred Ideas (OUT OF SCOPE)
- None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INIT-01 | User is prompted for project name during `init` when no squad.yml exists | TextInputState pattern covers text capture; wizard entry point in init.rs covers trigger condition |
| INIT-02 | User is prompted for number of agents (integer input) | Same TextInputState with integer parse validation on Enter |
| INIT-03 | For each agent, user is prompted for role, tool (claude-code/gemini-cli/antigravity), model, and description | Multi-page pattern with ToolSelector enum covers this; optional fields skip to None |
| INIT-06 | Wizard is presented as a TUI screen (ratatui) with field-by-field form navigation | Existing ui.rs terminal setup pattern is directly reusable; ratatui 0.26.3 confirmed |
| INIT-07 | Wizard validates inputs (non-empty role, known tool values) with inline error feedback | Inline error rendering with red border pattern documented below |
</phase_requirements>

---

## Summary

Phase 16 adds a multi-page ratatui TUI wizard to `src/commands/wizard.rs` (new file). It is invoked from the top of `src/commands/init.rs::run()` when no `squad.yml` is present, collects all configuration interactively, and returns a `WizardResult` struct. No files are written — that is Phase 17's job.

The project already runs ratatui 0.26.3 with crossterm 0.27.0. The full terminal setup/teardown pattern (`enable_raw_mode`, `EnterAlternateScreen`, panic hook, `event::poll`) is already proven in `src/commands/ui.rs` and must be copied verbatim. There is no existing text-input widget in the codebase; a minimal `TextInputState` struct (buffer + cursor) must be hand-rolled — this is standard ratatui practice because ratatui intentionally excludes form widgets from its core.

The wizard state machine drives page transitions. Each page renders one or more fields. Validation fires on Enter. Errors render as red `Paragraph` widgets below the offending field; the field's `Block` border color also turns red. The tool field is a cycle-selector (not a text input) backed by an enum, navigated with Left/Right/Tab.

**Primary recommendation:** Implement `WizardState` as a flat enum of pages; `TextInputState` as a two-field struct; render each page with stacked `Layout` constraints; return `Option<WizardResult>` — `None` on Ctrl+C.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.26.3 | TUI rendering (widgets, layout, frame) | Already in Cargo.toml; all existing TUI code uses it |
| crossterm | 0.27.0 | Terminal raw mode, key events, alternate screen | Already in Cargo.toml; paired with ratatui |
| anyhow | 1.0 | Error propagation from wizard run() | Already in Cargo.toml; project-wide convention |

### No New Dependencies Needed
The wizard requires zero new crate additions. Everything is already present.

**Installation:**
```bash
# No new crates — ratatui 0.26.3 and crossterm 0.27.0 already in Cargo.toml
```

---

## Architecture Patterns

### File Location
```
src/
├── commands/
│   ├── init.rs       # MODIFIED: call wizard::run() at top when no squad.yml
│   ├── wizard.rs     # NEW: entire wizard lives here
│   └── ui.rs         # REFERENCE: copy terminal setup pattern from here
```

### Data Structures

```rust
// Source: project convention (config.rs AgentConfig pattern)

pub struct WizardResult {
    pub project: String,
    pub agents: Vec<AgentInput>,
}

pub struct AgentInput {
    pub role: String,
    pub tool: Tool,          // enum, not String
    pub model: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    ClaudeCode,
    GeminiCli,
    Antigravity,
}

impl Tool {
    pub fn cycle_next(self) -> Self { ... }
    pub fn as_str(self) -> &'static str { ... }
    // Values: "claude-code", "gemini-cli", "antigravity"
    // Must match VALID_PROVIDERS in config.rs
}

// Minimal text input (no third-party crate needed)
pub struct TextInputState {
    pub value: String,
    pub error: Option<String>,
}

impl TextInputState {
    pub fn push(&mut self, c: char) { self.value.push(c); }
    pub fn pop(&mut self) { self.value.pop(); }
    pub fn clear_error(&mut self) { self.error = None; }
}
```

### Page State Machine

```rust
// Source: standard ratatui wizard pattern
enum WizardPage {
    ProjectName,
    AgentCount,
    AgentConfig { index: usize },   // index 0..agent_count-1
    Summary,
}

struct WizardState {
    page: WizardPage,
    project_input: TextInputState,
    count_input: TextInputState,
    agent_count: usize,             // parsed from count_input after validation
    agents: Vec<AgentDraft>,        // grows as user completes per-agent pages
}

struct AgentDraft {
    role: TextInputState,
    tool: Tool,
    model: TextInputState,
    description: TextInputState,
}
```

### Pattern 1: Terminal Setup (copy from ui.rs verbatim)

**What:** Acquire alternate screen, install panic hook that restores terminal.
**When to use:** At the top of `wizard::run()` before any rendering.

```rust
// Source: src/commands/ui.rs (lines 145-158, 267-272)
fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout())).map_err(Into::into)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// Panic hook (install before setup_terminal):
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    original_hook(info);
}));
```

### Pattern 2: Event Loop

**What:** Poll with 250ms timeout, guard on `KeyEventKind::Press`.
**When to use:** Inner loop of wizard — identical to ui.rs event loop.

```rust
// Source: src/commands/ui.rs (lines 304-310)
if event::poll(std::time::Duration::from_millis(250))? {
    if let event::Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            // dispatch to wizard state machine
        }
    }
}
```

### Pattern 3: Field Rendering with Inline Error

**What:** Render a text field as a `Paragraph` inside a `Block`; on error, set border color to red and render a red error paragraph below.
**When to use:** Every text input field on every page.

```rust
// Source: ui.rs color conventions + standard ratatui Paragraph/Block pattern
let border_color = if field.error.is_some() { Color::Red } else { Color::Cyan };
let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(border_color))
    .title(" Project Name ");
let input_widget = Paragraph::new(field.value.as_str()).block(block);
frame.render_widget(input_widget, input_area);

if let Some(err) = &field.error {
    let err_widget = Paragraph::new(format!("  {}", err))
        .style(Style::default().fg(Color::Red));
    frame.render_widget(err_widget, error_area);
}
```

### Pattern 4: Tool Cycle Selector

**What:** Display current tool value inside a styled block; Left/Right/Tab cycles through values.
**When to use:** The tool field on every per-agent page.

```rust
// Enum cycling — no text input involved
KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
    draft.tool = draft.tool.cycle_next();
}
// Render as non-editable Paragraph showing current selection
let label = format!("[ {} ]", draft.tool.as_str());
let selector = Paragraph::new(label)
    .block(Block::default().borders(Borders::ALL).title(" Tool "));
frame.render_widget(selector, tool_area);
```

### Pattern 5: Page Layout with Progress Header

**What:** Split terminal vertically into header (progress), content (fields), footer (key hints).
**When to use:** Every page of the wizard.

```rust
// Source: ratatui 0.26 Layout API (same as ui.rs)
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),   // header: "Step N of M — Page Title"
        Constraint::Min(10),     // content: fields
        Constraint::Length(2),   // footer: key hints
    ])
    .split(frame.size());       // frame.size() is valid in ratatui 0.26
```

### Pattern 6: Ctrl+C Handling

**What:** Detect `KeyCode::Char('c')` with `KeyModifiers::CONTROL`; restore terminal, return `Ok(None)`.
**When to use:** In the key dispatch block of every page.

```rust
// Source: crossterm key event API
use crossterm::event::{KeyCode, KeyModifiers};

if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
    restore_terminal(&mut terminal)?;
    return Ok(None);
}
```

### Pattern 7: Wizard Entry Point in init.rs

**What:** Check if `squad.yml` exists before calling `config::load_config`. If absent, run wizard; if user cancels, bail early.
**When to use:** Top of `commands::init::run()`.

```rust
// Insert before line 10 of src/commands/init.rs (before load_config call)
if !config_path.exists() {
    match commands::wizard::run().await? {
        Some(result) => {
            // Phase 17 uses result to write squad.yml and continue
            // For Phase 16: return Ok(()) after collecting — file writing is Phase 17
            return Ok(());
        }
        None => {
            println!("Init cancelled.");
            return Ok(());
        }
    }
}
```

### Anti-Patterns to Avoid

- **Re-implementing tui-input or tui-textarea:** Those crates target newer ratatui versions (0.28+); using them with 0.26.3 causes API conflicts. Hand-roll `TextInputState` instead.
- **Storing cursor position as byte offset:** Use char-aware operations or simply store the buffer as a `String` and use `pop()`/`push()` — simpler and sufficient for short wizard inputs.
- **Forgetting to restore terminal on early return:** Every `return` path (Ctrl+C, errors) must call `restore_terminal()`. Panic hook handles panics but not explicit returns.
- **Using `frame.area()` instead of `frame.size()`:** In ratatui 0.26, `frame.size()` is the correct method. `frame.area()` was introduced in 0.28+.
- **Mutating `agents: Vec<AgentDraft>` mid-wizard without bounds checking:** When user presses Esc on an agent page, do NOT pop the draft — keep it so re-entering the page restores prior input.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal setup/teardown | Custom raw mode logic | Copy `setup_terminal`/`restore_terminal` from `ui.rs` | Already tested in production |
| Key event dispatching | Custom event system | `crossterm::event::poll` + `event::read()` from `ui.rs` | Same 250ms loop pattern already proven |
| Tool validation | Custom string matching | `Tool` enum + `as_str()` method | Enums make invalid states unrepresentable |
| VALID_PROVIDERS matching | Duplicate constant | `Tool::as_str()` must match `config::VALID_PROVIDERS` values exactly | Single source of truth |

**Key insight:** The wizard is pure UI — no DB, no tmux, no file I/O. All complexity is in the state machine and rendering logic, both of which follow established ratatui patterns. No new infrastructure is needed.

---

## Common Pitfalls

### Pitfall 1: Terminal Not Restored on Ctrl+C
**What goes wrong:** User hits Ctrl+C; wizard exits but terminal stays in raw mode / alternate screen. Shell becomes unusable.
**Why it happens:** Ctrl+C sends SIGINT; the panic hook doesn't catch it; early `return Ok(None)` skips `restore_terminal`.
**How to avoid:** Handle `KeyModifiers::CONTROL + KeyCode::Char('c')` explicitly in the event loop. Call `restore_terminal` before returning. The panic hook handles panics only.
**Warning signs:** If you see "raw mode" in manual testing, a return path skipped restore.

### Pitfall 2: frame.size() Deprecated Warning Confusion
**What goes wrong:** Developer sees online examples using `frame.area()` and switches to it, causing compile errors on ratatui 0.26.
**Why it happens:** `frame.area()` was introduced in ratatui 0.28. The project uses 0.26.3.
**How to avoid:** Use `frame.size()` — it is the correct API for 0.26.x. Verified in existing `ui.rs`.

### Pitfall 3: Esc on Page 1 Panics / Exits
**What goes wrong:** User presses Esc on the first page (ProjectName). Code tries to go to the "previous page" but there is none.
**Why it happens:** Page transition logic doesn't check for the initial page.
**How to avoid:** On `WizardPage::ProjectName`, Esc is a no-op (or shows a hint: "Press Ctrl+C to cancel"). Only Ctrl+C cancels.

### Pitfall 4: Agent Count Accepted Before Parsing
**What goes wrong:** User enters "abc" for agent count; wizard advances to agent pages; later code panics parsing an invalid count.
**Why it happens:** Validation was skipped or fired on field exit instead of Enter.
**How to avoid:** On Enter on the count page: parse as `usize`, validate `>= 1`, store to `agent_count` field only on success. Show error inline on failure.

### Pitfall 5: Esc on Agent Page Loses Draft
**What goes wrong:** User fills in agent 2's role, presses Esc to go back to agent 1, then re-enters agent 2 — all fields are blank.
**Why it happens:** Code recreates `AgentDraft` when entering a page instead of restoring existing draft.
**How to avoid:** `agents: Vec<AgentDraft>` is pre-allocated with `agent_count` empty drafts after count confirmation. Page transitions index into this vec by position — never push/pop during back navigation.

### Pitfall 6: Summary Page Shows Stale Data
**What goes wrong:** User backs into agent 2, changes role, goes forward to summary — summary still shows old role.
**Why it happens:** Summary renders snapshot taken on first forward pass.
**How to avoid:** Summary page renders directly from `state.agents` vec at draw time — no snapshot needed.

---

## Code Examples

### Wizard Public API

```rust
// Source: design decision from 16-CONTEXT.md
// File: src/commands/wizard.rs

pub struct WizardResult {
    pub project: String,
    pub agents: Vec<AgentInput>,
}

pub struct AgentInput {
    pub role: String,
    pub tool: String,            // "claude-code" | "gemini-cli" | "antigravity"
    pub model: Option<String>,
    pub description: Option<String>,
}

/// Runs the interactive TUI wizard.
/// Returns Ok(Some(result)) on completion, Ok(None) if user cancelled (Ctrl+C).
pub async fn run() -> anyhow::Result<Option<WizardResult>> {
    // ... terminal setup, event loop, state machine
}
```

### Summary Page Rendering

```rust
// Source: ratatui 0.26 List widget pattern (same as ui.rs agent list)
let items: Vec<ListItem> = state.agents.iter().enumerate().map(|(i, draft)| {
    let line = format!(
        "Agent {}: role={}, tool={}, model={}, desc={}",
        i + 1,
        draft.role.value,
        draft.tool.as_str(),
        draft.model.value.as_deref().unwrap_or("-"),
        draft.description.value.as_deref().unwrap_or("-"),
    );
    ListItem::new(line)
}).collect();

let summary = List::new(items)
    .block(Block::default().borders(Borders::ALL).title(" Review "));
frame.render_widget(summary, content_area);
```

### Validation Example

```rust
// Source: pattern from config.rs validate_agent_config
fn validate_count(input: &str) -> Result<usize, String> {
    input.trim().parse::<usize>()
        .map_err(|_| "Please enter a whole number (e.g. 2)".to_string())
        .and_then(|n| {
            if n >= 1 { Ok(n) }
            else { Err("Agent count must be at least 1".to_string()) }
        })
}

fn validate_role(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Role is required".to_string())
    } else {
        Ok(())
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tui crate | ratatui (ratatui fork) | 2022 — tui-rs unmaintained | Project already on ratatui 0.26.3 |
| frame.render_widget(..., frame.size()) | Same in 0.26; frame.area() added in 0.28 | 0.28 release | Use frame.size() — matches existing ui.rs |
| Third-party tui-input 0.x | Hand-rolled TextInputState | tui-input targets ratatui 0.28+ | Simpler, no version conflict |

**Deprecated/outdated:**
- `tui` crate (not `ratatui`): archived, do not use
- `tui-input` crate: targets ratatui >= 0.27 with different API surface — incompatible with 0.26.3

---

## Open Questions

1. **init.rs integration: where exactly is the squad.yml check?**
   - What we know: `init.rs::run()` starts by calling `config::load_config(&config_path)` at line 10; `config_path` is the PathBuf from the CLI (defaults to `"squad.yml"`).
   - What's unclear: Whether to check `config_path.exists()` or catch the error from `load_config`. Checking existence is cleaner and avoids loading a file only to discard the result.
   - Recommendation: Check `!config_path.exists()` before `load_config`. If false, run wizard; if squad.yml exists, fall through to existing init logic (Phase 17 adds the re-init prompt).

2. **Cursor visibility during text input**
   - What we know: `restore_terminal` calls `terminal.show_cursor()`. During wizard, cursor should appear in text fields.
   - What's unclear: ratatui 0.26 does not control cursor position within a Paragraph widget — cursor rendering is terminal-native.
   - Recommendation: Call `terminal.show_cursor()` after setup and render a visual cursor indicator (e.g., append "|" to the buffer string) for clarity. This is the standard ratatui workaround for cursor-in-input.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (tokio async runtime via `#[tokio::test]`) |
| Config file | None — standard cargo test discovery |
| Quick run command | `cargo test wizard` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INIT-01 | `TextInputState` captures project name string | unit | `cargo test wizard::tests::test_text_input` | Wave 0 |
| INIT-02 | `validate_count` accepts positive integers, rejects zero and non-numeric | unit | `cargo test wizard::tests::test_validate_count` | Wave 0 |
| INIT-03 | `validate_role` rejects empty string; `Tool::cycle_next` cycles all 3 values | unit | `cargo test wizard::tests::test_validate_role` / `test_tool_cycle` | Wave 0 |
| INIT-06 | Wizard module compiles and exports `run()` with correct signature | compile check | `cargo check` | Wave 0 |
| INIT-07 | Inline error set on failed validation, cleared on next successful attempt | unit | `cargo test wizard::tests::test_validation_error_cleared` | Wave 0 |

Note: Full TUI rendering and key-event dispatch cannot be automatically tested without a PTY. Tests target the pure-logic layer (state machine, validation functions, TextInputState mutations) which is separable from rendering. Rendering is verified manually.

### Sampling Rate
- **Per task commit:** `cargo test wizard`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src/commands/wizard.rs` — module does not exist yet; must be created in Wave 1
- [ ] Inner `#[cfg(test)]` module inside `wizard.rs` — covers all 5 requirement mappings above
- [ ] No new test file needed in `tests/` — wizard logic tests live inside the module (unit tests)

---

## Sources

### Primary (HIGH confidence)
- `src/commands/ui.rs` (local) — terminal setup/teardown pattern, widget imports, color conventions, event loop
- `src/commands/init.rs` (local) — entry point structure, config_path parameter
- `src/config.rs` (local) — VALID_PROVIDERS list, AgentConfig struct shape, Tool string values
- `Cargo.toml` (local) — ratatui 0.26.3, crossterm 0.27.0 confirmed
- `.planning/phases/16-tui-wizard/16-CONTEXT.md` (local) — locked decisions, all UX requirements

### Secondary (MEDIUM confidence)
- ratatui 0.26 changelog / docs (training knowledge, cross-verified against actual working code in ui.rs) — `frame.size()` API confirmed

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all library versions confirmed from Cargo.toml lock
- Architecture: HIGH — all patterns verified against working code in ui.rs and init.rs
- Pitfalls: HIGH — derived from known ratatui version mismatches and wizard state machine edge cases, verified against codebase
- Validation architecture: HIGH — follows established cargo test patterns in the project

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (ratatui 0.26.x is stable; no fast-moving dependencies)
