# Phase 20: TTY-Safe Welcome TUI Core - Research

**Researched:** 2026-03-17
**Domain:** Rust TUI (ratatui + crossterm + tui-big-text), TTY detection, event-loop countdown
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Screen mode:** AlternateScreen — full-screen, clean slate, no scrollback pollution. After `LeaveAlternateScreen`, return to shell prompt with no extra output printed.

**Welcome screen content:**
- pixel-font SQUAD-STATION title via tui-big-text (centered)
- Version string below title (centered)
- Tagline: "Multi-agent orchestration for AI coding" (centered)
- Two-column commands table matching current static welcome: `cmd    description` format, all 11 subcommands (init, send, signal, peek, list, ui, view, status, agents, context, register)
- All content centered horizontally

**Countdown behavior:**
- Auto-exit after 5 seconds if no key pressed
- Any valid keypress (Enter, Q, Esc) cancels countdown and acts immediately
- On timeout: silent close, return to shell — same as Q; no wizard auto-launch on timeout

**Conditional routing UX:**
- No squad.yml: hint bar = `Enter: Set up  Q: Quit  auto-exit 5s` → Enter closes TUI and launches init wizard
- squad.yml exists: hint bar = `Enter: Open dashboard  Q: Quit  auto-exit 5s` → Enter closes TUI and launches `squad-station ui`
- No check for active tmux sessions — unconditionally offer dashboard when squad.yml exists

**Non-TTY fallback:** When `stdout` is not a terminal (`!stdout.is_terminal()`), skip TUI entirely and print static text (current `print_welcome()` pattern). No raw mode attempted in non-TTY context.

### Claude's Discretion
- Exact ratatui layout constraint values (padding, margin sizes)
- tui-big-text font variant (BigText default or specific style)
- Countdown tick implementation (crossterm event timeout polling)
- Color scheme for the TUI (red accent for title consistent with current ASCII art, or full ratatui styling)

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WELCOME-01 | Bare `squad-station` always shows interactive TUI (replaces static welcome screen) | `main.rs:25` None arm replaces `print_welcome()` call; TTY guard routes non-TTY to static fallback |
| WELCOME-02 | TUI displays large SQUAD-STATION title using pixel-font big text | tui-big-text 0.8.2 with `PixelSize::Full` or `HalfHeight`; requires ratatui 0.30 upgrade |
| WELCOME-03 | TUI displays current version below title | `env!("CARGO_PKG_VERSION")` Paragraph widget, centered |
| WELCOME-04 | TUI shows hint bar at bottom with available keys and auto-exit countdown | Constraint::Length(1) bottom row; live-updated Paragraph with remaining seconds |
| WELCOME-06 | TUI auto-exits after N seconds if no key pressed (countdown shown in hint bar) | `event::poll(Duration::from_secs(1))` loop; `Instant` elapsed tracking; 5 iterations then exit |
| WELCOME-07 | Non-TTY fallback — when stdout is not a terminal, print static text instead of TUI | `std::io::stdout().is_terminal()` guard in `main.rs` None arm before calling TUI |
| INIT-01 | When no squad.yml exists, Enter key in welcome TUI launches init wizard directly | `Path::new("squad.yml").exists()` check; on Enter → `commands::init::run(PathBuf::from("squad.yml"), false).await` |
| INIT-02 | When squad.yml exists, Enter key closes welcome (no re-init triggered) | Same check; on Enter → `commands::ui::run().await` |
| INIT-03 | Q / Escape closes the welcome TUI without launching anything | `KeyCode::Char('q') | KeyCode::Esc` arm in key handler sets `quit = true` and `action = None` |
</phase_requirements>

---

## Summary

Phase 20 replaces `print_welcome()` in `main.rs:25` with an interactive ratatui TUI. The existing `ui.rs` provides the complete AlternateScreen + raw mode pattern to replicate verbatim. The key new element is `tui-big-text` for the pixel-font title, a 5-second countdown auto-exit, and conditional routing (Enter behavior depends on `squad.yml` presence).

**Critical version finding:** The CONTEXT.md states "upgrade to ratatui 0.29 + crossterm 0.28" but this is now outdated. `tui-big-text 0.8.2` (the current latest) requires `ratatui-core 0.1` which ships with **ratatui 0.30.0** (released December 2024), which requires **crossterm 0.29**. The planner must use ratatui 0.30 + crossterm 0.29, not 0.29 + 0.28. Additionally, `frame.size()` used in `ui.rs` and `wizard.rs` is deprecated in 0.29 and removed in 0.30 — must be replaced with `frame.area()` when upgrading.

**Primary recommendation:** Upgrade to ratatui 0.30.0 + crossterm 0.29 + tui-big-text 0.8.2. Fix `frame.size()` → `frame.area()` across `ui.rs` and `wizard.rs` as part of the upgrade. Copy the AlternateScreen event-loop skeleton from `ui.rs` into a new `run_welcome_tui()` function in `welcome.rs`.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI framework: layout, widgets, rendering | Required by tui-big-text 0.8.x (ratatui-core 0.1); project already uses ratatui |
| crossterm | 0.29 | Terminal backend, raw mode, event polling | Default backend for ratatui 0.30; supports crossterm_0_28 and crossterm_0_29 features |
| tui-big-text | 0.8.2 | BigText widget for pixel-font title rendering | Only maintained library for 8x8 font pixel text in ratatui ecosystem |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::io::IsTerminal | stdlib | TTY guard for non-interactive fallback | Always — `std::io::stdout().is_terminal()` is the established pattern in this codebase |
| std::time::Instant | stdlib | Countdown elapsed tracking | Countdown tick loop |
| std::path::Path | stdlib | squad.yml existence check for conditional routing | INIT-01/INIT-02 routing logic |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tui-big-text 0.8.2 | tui-big-text 0.7.x | 0.7.x requires ratatui 0.29; but 0.29 is superseded by 0.30 which is the current stable. No reason to pin to 0.7.x. |
| ratatui 0.30 | ratatui 0.29 | 0.29 is one version behind, tui-big-text 0.8.2 requires ratatui-core 0.1 (0.30 only). Must use 0.30. |
| `event::poll(1s)` loop | tokio timer + select! | Crossterm event polling is the established pattern in this codebase (ui.rs, wizard.rs). No async complexity needed. |

**Installation:**
```bash
cargo add ratatui@0.30 crossterm@0.29 tui-big-text@0.8
```

Then remove the old `ratatui = "0.26"` and `crossterm = "0.27"` lines from `Cargo.toml`.

---

## Architecture Patterns

### Recommended Project Structure

No new files needed. All changes are contained in:
```
src/
├── main.rs                    # Line 25: replace print_welcome() call + add TTY guard
├── commands/
│   ├── welcome.rs             # Add run_welcome_tui() alongside existing print_welcome()
│   └── ui.rs                  # frame.size() → frame.area() (upgrade migration)
│   └── wizard.rs              # frame.size() → frame.area() (upgrade migration)
Cargo.toml                     # Version bumps: ratatui 0.26→0.30, crossterm 0.27→0.29, +tui-big-text 0.8
```

### Pattern 1: AlternateScreen Event Loop (from ui.rs)

**What:** Enter AlternateScreen + enable raw mode → draw loop → poll events → exit on quit.
**When to use:** All interactive TUI sessions in this codebase. Welcome TUI uses exactly this pattern.

```rust
// Source: src/commands/ui.rs (verbatim pattern)
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
```

### Pattern 2: Countdown with event::poll

**What:** Poll events with 1-second timeout to tick countdown. Any keypress within the timeout cancels/acts immediately.
**When to use:** Auto-exit countdown pattern. This is the idiomatic crossterm approach.

```rust
// Pseudocode — event loop for welcome TUI
let deadline = Instant::now() + Duration::from_secs(5);
loop {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        // Timeout: silent exit
        break;
    }

    terminal.draw(|f| draw_welcome(f, seconds_left, has_config))?;

    if event::poll(remaining.min(Duration::from_secs(1)))? {
        if let event::Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Enter => { action = Some(route); break; }
                    KeyCode::Char('q') | KeyCode::Esc => { break; }
                    _ => {}
                }
            }
        }
    }
    // Redraw with updated countdown
}
```

**Note:** Using `remaining.min(Duration::from_secs(1))` as the poll timeout ensures the countdown updates at least once per second, while still waking immediately on keypress.

### Pattern 3: TTY Guard at Entry Point

**What:** Check `stdout.is_terminal()` before entering raw mode. Fall through to static print if not a TTY.
**When to use:** Every TUI entry point in this codebase. Established in `notify.rs`, `send.rs`, `signal.rs`.

```rust
// Source: existing pattern in this codebase (notify.rs, send.rs, signal.rs)
use std::io::IsTerminal;

// In main.rs None arm:
if std::io::stdout().is_terminal() {
    commands::welcome::run_welcome_tui().await?;
} else {
    commands::welcome::print_welcome();
}
```

### Pattern 4: tui-big-text BigText Widget

**What:** Renders 8x8 pixel-font glyphs as a ratatui widget.
**When to use:** SQUAD-STATION title display.

```rust
// Source: docs.rs/tui-big-text (verified 2026-03-17)
use tui_big_text::{BigText, PixelSize};

let big_text = BigText::builder()
    .pixel_size(PixelSize::Full)   // or HalfHeight for compact display
    .style(Style::default().fg(Color::Red))
    .lines(vec![Line::from("SQUAD-STATION")])
    .centered()
    .build();
frame.render_widget(big_text, title_area);
```

**Available PixelSize variants:** Full, HalfHeight, HalfWidth, Quadrant, ThirdHeight, Sextant, QuarterHeight, Octant.

**Recommendation (Claude's discretion):** Use `PixelSize::HalfHeight` — it renders at half the row height so the title doesn't consume the entire screen and leaves room for version, tagline, commands table, and hint bar. `Full` requires 8 rows per character row which may not fit on standard 24-line terminals. `HalfHeight` requires 4 rows per character row.

### Pattern 5: Conditional Routing After TUI Exit

**What:** After AlternateScreen closes, call the appropriate command handler based on squad.yml presence.
**When to use:** After welcome TUI exits with Enter keypress only.

```rust
// Routing logic in welcome.rs or delegated back to main.rs
enum WelcomeAction {
    LaunchInit,
    LaunchDashboard,
    Quit,
}

// After restore_terminal():
match action {
    Some(WelcomeAction::LaunchInit) => commands::init::run(PathBuf::from("squad.yml"), false).await?,
    Some(WelcomeAction::LaunchDashboard) => commands::ui::run().await?,
    None => {} // Q/Esc/timeout — just return
}
```

### Layout Structure for Welcome TUI

```
┌─────────────────────────────────────┐
│        [BigText: SQUAD-STATION]     │  ~4-8 rows (PixelSize::HalfHeight)
│          v0.5.3                     │  1 row
│   Multi-agent orchestration for AI  │  1 row
│                                     │
│  Commands:                          │
│    init        Initialize squad...  │  11 rows
│    send        Send a task...       │
│    ...                              │
│                                     │
│ Enter: Set up  Q: Quit  auto-exit 5s│  1 row (Constraint::Length(1))
└─────────────────────────────────────┘
```

```rust
// Layout example
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(big_text_height), // tui-big-text title
        Constraint::Length(1),               // version
        Constraint::Length(1),               // tagline
        Constraint::Length(1),               // spacer
        Constraint::Min(0),                  // commands table
        Constraint::Length(1),               // hint bar
    ])
    .split(frame.area());  // frame.area() NOT frame.size()
```

### Anti-Patterns to Avoid

- **Calling `frame.size()`:** Deprecated since ratatui 0.29, removed in 0.30. Use `frame.area()` exclusively.
- **Leaving raw mode on panic:** Install a panic hook (same as `ui.rs:285`) that calls `disable_raw_mode()` + `execute!(stdout, LeaveAlternateScreen)` before the default hook.
- **Attempting to enter raw mode without TTY check:** Crossterm will error if stdin/stdout is not a TTY. The TTY guard must be placed before `enable_raw_mode()`.
- **Polling with `Duration::from_millis(250)` for countdown:** This would require 4 polls per second to update the display, wasting cycles. Poll with `remaining.min(Duration::from_secs(1))` instead.
- **Routing init after timeout:** The CONTEXT.md is explicit: timeout = silent exit, same as Q. Do not auto-launch wizard on timeout.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Pixel-font title | Custom ASCII art string (existing `ASCII_ART` const) | tui-big-text BigText widget | Hand-rolled art doesn't resize; BigText adapts to PixelSize; already the decided approach |
| Terminal size detection | Custom ioctl calls | ratatui `terminal.size()` or layout auto-sizing | ratatui handles terminal resize events and redraws |
| Raw mode cleanup | Manual signal handlers | Panic hook pattern (exact copy from `ui.rs:284-289`) | Handles panics; crossterm's disable_raw_mode is idempotent |
| Countdown timer | tokio Timer + select! | `Instant + event::poll(timeout)` | No async complexity; crossterm event poll is already async-friendly |

**Key insight:** The countdown + event-poll pattern is simpler than async timer channels because `event::poll(Duration)` blocks for exactly that duration or until an event arrives — it's a combined sleep+wake primitive.

---

## Common Pitfalls

### Pitfall 1: Version Mismatch — ratatui 0.29 vs 0.30

**What goes wrong:** CONTEXT.md says "upgrade to ratatui 0.29 + crossterm 0.28" but tui-big-text 0.8.2 depends on `ratatui-core 0.1`, which is only available in ratatui 0.30.0+.
**Why it happens:** The context was written before verifying current crate versions. ratatui 0.30 was released December 2024.
**How to avoid:** Use ratatui 0.30.0 + crossterm 0.29 + tui-big-text 0.8.2. This is the validated combination.
**Warning signs:** `cargo build` fails with "no matching package named ratatui-core" or "version requirement not met".

### Pitfall 2: `frame.size()` compilation failure after upgrade

**What goes wrong:** Upgrading to ratatui 0.30 causes compilation errors in `ui.rs` and `wizard.rs` because `frame.size()` was removed.
**Why it happens:** `frame.size()` was deprecated in 0.29 and removed in 0.30.
**How to avoid:** As part of the ratatui 0.30 upgrade (Plan 20-01), find and replace all `frame.size()` with `frame.area()` in `ui.rs` and `wizard.rs`.
**Warning signs:** `error[E0599]: no method named 'size' found for struct 'Frame'`.

### Pitfall 3: Raw mode not restored on panic

**What goes wrong:** If the welcome TUI panics, the terminal is left in raw mode. The user's terminal becomes unusable (no echo, no line editing).
**Why it happens:** `restore_terminal()` is only called at the normal exit path.
**How to avoid:** Install a panic hook before `setup_terminal()`, identical to the pattern in `ui.rs:284-289`.
**Warning signs:** After a crash, terminal shows no typed characters; user must run `reset` or `stty sane`.

### Pitfall 4: Trying to enter raw mode in non-TTY context (CI, pipes)

**What goes wrong:** Crossterm returns `Err` when `enable_raw_mode()` is called and stdin/stdout is not a TTY (e.g., `squad-station | grep foo`).
**Why it happens:** Raw mode requires a real terminal file descriptor.
**How to avoid:** Check `std::io::stdout().is_terminal()` before calling `run_welcome_tui()`. This is WELCOME-07 and is a locked decision.
**Warning signs:** `Error: inappropriate ioctl for device` in CI logs.

### Pitfall 5: BigText title overflows on small terminals

**What goes wrong:** `PixelSize::Full` renders each glyph at 8 rows × 8 columns. "SQUAD-STATION" (13 chars) requires 104 columns and 8 rows — may overflow on 80-column terminals.
**Why it happens:** tui-big-text clips to the available area but the text becomes unreadable.
**How to avoid:** Use `PixelSize::HalfHeight` (4 rows × 8 cols per glyph) or `PixelSize::Quadrant` (4 rows × 4 cols per glyph). Recommend `HalfHeight` for a balance of legibility and compactness. The `centered()` builder method handles horizontal centering.
**Warning signs:** Title appears as partial characters or disappears entirely on narrow terminals.

### Pitfall 6: Countdown not refreshing because poll timeout is too long

**What goes wrong:** The countdown display in the hint bar jumps from 5s to 0s without showing intermediate values.
**Why it happens:** If `event::poll(Duration::from_secs(5))` is used for the entire duration, the screen doesn't redraw until the poll returns.
**How to avoid:** Poll with `remaining.min(Duration::from_secs(1))` so the loop body (and terminal.draw()) executes at least once per second.

---

## Code Examples

### TTY Guard + Entry Point (main.rs)

```rust
// Source: established pattern from notify.rs, send.rs — verified in codebase
use std::io::IsTerminal;

// In run() match arm: None =>
if std::io::stdout().is_terminal() {
    commands::welcome::run_welcome_tui().await?;
} else {
    commands::welcome::print_welcome();
}
Ok(())
```

### AlternateScreen Setup/Teardown (exact pattern from ui.rs)

```rust
// Source: src/commands/ui.rs:145-158
use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

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
```

### squad.yml Existence Check

```rust
// Simple path check — no config loading required for routing decision
let has_config = std::path::Path::new("squad.yml").exists();
```

### Hint Bar Text

```rust
// Locked wording from CONTEXT.md
let hint = if has_config {
    format!("Enter: Open dashboard  Q: Quit  auto-exit {}s", remaining_secs)
} else {
    format!("Enter: Set up  Q: Quit  auto-exit {}s", remaining_secs)
};
```

### frame.area() (ratatui 0.30 migration)

```rust
// Was: frame.size()  (ratatui 0.26 — DEPRECATED in 0.29, REMOVED in 0.30)
// Now: frame.area()  (ratatui 0.29+)
let area = frame.area();
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([...])
    .split(area);
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ratatui 0.26 + crossterm 0.27 | ratatui 0.30 + crossterm 0.29 | ratatui 0.30 released Dec 2024 | Breaking: `frame.size()` removed; modular crate structure |
| `frame.size()` | `frame.area()` | Deprecated in ratatui 0.29, removed in 0.30 | All TUI draw functions must update |
| tui-big-text via standalone crate | tui-big-text 0.8.x via ratatui-widgets organization | tui-big-text moved to ratatui/tui-widgets monorepo | API is stable but now depends on `ratatui-core 0.1` |
| Manual `init()/restore()` terminal setup | `ratatui::run()` closure API | ratatui 0.30 | Optional — existing manual setup still works; no migration required |

**Deprecated/outdated:**
- `ratatui = "0.26"`: Replace with `"0.30"` — incompatible with tui-big-text 0.8.x
- `crossterm = "0.27"`: Replace with `"0.29"` — ratatui 0.30 default backend
- `frame.size()`: Replace with `frame.area()` — removed in ratatui 0.30

---

## Open Questions

1. **PixelSize selection for BigText title**
   - What we know: `HalfHeight` uses 4 rows per glyph row; `Full` uses 8 rows. "SQUAD-STATION" at `HalfHeight` is approximately 4 rows tall and 104 columns wide.
   - What's unclear: Whether standard 80-column terminals will clip the title with `HalfHeight`. The text may need `Quadrant` (4 rows × 4 cols) on narrow terminals, or the title could be split onto two lines ("SQUAD" / "STATION").
   - Recommendation: Implement `HalfHeight` first; if integration testing shows clipping at 80 columns, fall back to `Quadrant` or split the string.

2. **Interaction between welcome TUI exit and init wizard raw mode**
   - What we know: `init.rs` calls `wizard.rs` which uses its own crossterm event loop. The welcome TUI calls `restore_terminal()` before handing off.
   - What's unclear: Whether there is any terminal state leak between the two raw-mode sessions.
   - Recommendation: Ensure `restore_terminal()` + `show_cursor()` completes fully before calling `commands::init::run()`. The current `init.rs` and `wizard.rs` patterns handle their own raw mode setup independently — this should be clean.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none (inline `#[cfg(test)] mod tests` in each source file) |
| Quick run command | `cargo test welcome` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WELCOME-01 | bare invocation routes to TUI or static fallback | unit | `cargo test welcome` | ❌ Wave 0 — new tests in `welcome.rs` |
| WELCOME-02 | pixel-font title widget renders without panic | unit | `cargo test welcome` | ❌ Wave 0 |
| WELCOME-03 | version string appears in TUI content | unit | `cargo test welcome` | ❌ Wave 0 (replaces `test_welcome_content_has_version`) |
| WELCOME-04 | hint bar text matches expected string per routing state | unit | `cargo test welcome` | ❌ Wave 0 |
| WELCOME-06 | countdown logic: `remaining_secs` decrements correctly | unit | `cargo test welcome` | ❌ Wave 0 |
| WELCOME-07 | `print_welcome()` (static fallback) still works | unit | `cargo test welcome` | ✅ `test_welcome_content_has_ascii_art` (keep) |
| INIT-01 | Enter with no squad.yml triggers init routing | unit | `cargo test welcome` | ❌ Wave 0 |
| INIT-02 | Enter with squad.yml triggers dashboard routing | unit | `cargo test welcome` | ❌ Wave 0 |
| INIT-03 | Q/Esc produces no routing action | unit | `cargo test welcome` | ❌ Wave 0 |

**Note:** WELCOME-01, WELCOME-02 TUI rendering cannot be fully tested without a real terminal. Tests should cover: (a) the routing logic via pure functions (has_config → action), (b) hint bar string generation, (c) static fallback text content. The actual TUI draw loop is best validated by e2e observation.

### Sampling Rate
- **Per task commit:** `cargo test welcome`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Update existing tests in `welcome.rs` — `test_welcome_content_has_init_hint` references "squad-station init" hint which will no longer appear in the TUI (routing is keyboard-driven now). Decide: keep static fallback test or update assertion.
- [ ] New unit tests for `hint_bar_text(has_config, remaining_secs)` pure function
- [ ] New unit test for `routing_action(key, has_config)` pure function returning `Option<WelcomeAction>`
- [ ] `cargo test` baseline must pass after ratatui 0.30 upgrade (frame.size() → frame.area() migration may break existing ui.rs/wizard.rs tests)

---

## Sources

### Primary (HIGH confidence)
- Codebase direct read: `src/commands/ui.rs` — AlternateScreen setup/teardown + event loop pattern (verbatim)
- Codebase direct read: `src/commands/welcome.rs` — existing print_welcome(), tests to update
- Codebase direct read: `src/main.rs` — None arm entry point
- Codebase direct read: `Cargo.toml` — current versions: ratatui 0.26, crossterm 0.27
- `cargo add tui-big-text --dry-run` — confirmed latest is 0.8.2
- `cargo add ratatui@0.30 --dry-run` — confirmed ratatui 0.30.0 available with crossterm_0_29 as default feature
- docs.rs/tui-big-text — `ratatui-core 0.1` as runtime dependency; `ratatui 0.30` in dev-deps

### Secondary (MEDIUM confidence)
- [ratatui releases page](https://github.com/ratatui/ratatui/releases) — ratatui 0.30.0 released December 2024; is latest stable
- [ratatui FAQ](https://ratatui.rs/faq/) — crossterm version compatibility and feature flags documented
- [ratatui v0.30 highlights](https://ratatui.rs/highlights/v030/) — `frame.area()` required; `frame.size()` removed; `ratatui::run()` new API
- [tui-big-text lib.rs](https://lib.rs/crates/tui-big-text) — runtime deps include `ratatui-core 0.1`

### Tertiary (LOW confidence)
- WebSearch results re: ratatui 0.29 crossterm 0.29 relationship — cross-verified with cargo dry-run output

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — `cargo add --dry-run` confirmed versions; docs.rs confirmed tui-big-text 0.8.2 ratatui-core dep
- Architecture: HIGH — copy of existing ui.rs pattern; only new element is BigText widget and countdown
- Pitfalls: HIGH — frame.size() removal verified via ratatui 0.30 release notes; TTY guard pattern verified in existing codebase

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (ratatui is stable; tui-big-text unlikely to change in 30 days)
