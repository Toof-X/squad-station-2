# Phase 21: Quick Guide and Install Flow - Research

**Researched:** 2026-03-18
**Domain:** Rust TUI state machine extension (ratatui 0.30) + shell/Node.js install script TTY detection
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Quick guide content**
- Mental model: 1 orchestrator AI coordinates N worker agents via squad-station. Each agent runs in its own tmux session. Orchestrator sends tasks, agents signal completion.
- Format: concept summary (1-2 lines) + 3 numbered steps in plain English (no CLI commands on this page)
  1. Set up your squad (init)
  2. Send tasks to agents
  3. Agents signal completion automatically via hooks
- Tone: minimal, sparse — plain text with breathing room. No borders or boxes.
- Footer line: "Run squad-station --help for all commands" (consistent with existing static welcome screen)

**Guide page layout**
- Full area given to guide content — no BigText title on the guide page
- Centered header line: "Quick Guide" (or similar)
- Blank line, then numbered steps, then blank line, then footer line
- Same hint bar area at the bottom (just different hint text)

**Guide navigation**
- Key to open guide from title page: Tab or Right arrow — shown in hint bar as `Tab: Guide`
- On guide page, hint bar: `Tab/←: Back  Q: Quit` — no Enter action on the guide page
- Back key: same key (Tab or Left arrow) returns to title page
- Countdown behavior: resets to 5s when entering the guide page (user is actively reading)
- Title page hint bar updated to include: `Enter: [action]  Tab: Guide  Q: Quit  auto-exit Ns`

**Auto-launch after install**
- Both install paths exec the binary in interactive terminals (REQUIREMENTS.md wins)
- Guard condition: TTY check only
  - Shell (curl installer): `[ -t 1 ]` (stdout is a terminal)
  - Node (npm run.js): `process.stdout.isTTY === true`
- No additional CI env var or root guards — TTY check alone is sufficient
- Non-interactive environments (CI, pipes, sudo) degrade silently — no exec, no extra output

**npm auto-launch placement**
- Auto-launch added at the bottom of the existing `install()` function in `npm-package/bin/run.js`
- No new postinstall.js file — install flow already lives in `npx squad-station install`
- Implementation: `spawnSync(destPath, [], { stdio: 'inherit' })` using the full `destPath` resolved during binary download
- TTY check: `if (process.stdout.isTTY) { spawnSync(destPath, [], { stdio: 'inherit' }); }`

**curl installer auto-launch**
- Auto-launch added at end of `install.sh` after the success message
- Uses full install path: `exec "${INSTALL_DIR}/squad-station"`
- Guard: `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi`
- `exec` replaces the shell process — clean handoff, no extra process in the tree

### Claude's Discretion
- Exact wording of the 3 numbered steps on the guide page
- Exact wording of the "Quick Guide" header line
- Left arrow key binding details (whether Left arrow is also accepted as a back key alongside Tab)
- Whether the guide page shows a page indicator (e.g., "1/2" or "● ○") in the hint bar

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WELCOME-05 | TUI includes quick guide page explaining Squad Station concept and basic workflow | Extend `WelcomeAction` enum + `run_welcome_tui()` with a `WelcomePage` state variable; add `draw_guide()` and `guide_hint_bar_text()` pure functions; wire Tab/Right arrow key in `routing_action()` |
| INSTALL-01 | npm postinstall checks `process.stdout.isTTY` and auto-launches `squad-station` if interactive | Add `if (process.stdout.isTTY)` guard + `spawnSync(destPath, [], { stdio: 'inherit' })` at bottom of `install()` in `npm-package/bin/run.js`; `destPath` is already resolved in `installBinary()` — must be returned and threaded through |
| INSTALL-02 | curl | sh installer checks `[ -t 1 ]` and auto-launches `squad-station` if interactive | Add `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi` at end of `install.sh` after success echo statements |
| INSTALL-03 | Both install scripts degrade silently in non-interactive environments (CI, pipes, sudo) | TTY check alone is the guard — the `if` simply doesn't fire; no special CI detection needed |
</phase_requirements>

---

## Summary

Phase 21 is a focused extension of the welcome TUI state machine and both install entry-point scripts. The Rust side requires adding a second TUI "page" (guide) to `src/commands/welcome.rs` without touching any other module. The JavaScript and shell sides each require inserting a single guarded block at the end of their respective install functions.

All building blocks are verified in place from Phase 20: ratatui 0.30, crossterm 0.29, tui-big-text 0.8, the `WelcomeAction` enum, the `routing_action()` / `hint_bar_text()` pure-function pattern, and the `Instant`-based deadline countdown. The only new Rust concepts needed are a page-state enum (`WelcomePage`) and deadline reassignment on page transition.

The install script changes are shell/Node primitives (`exec`, `[ -t 1 ]`, `spawnSync`, `isTTY`) with no new dependencies.

**Primary recommendation:** Implement in two plans — Plan 21-01 for the guide page (Rust, pure-function-first with tests), Plan 21-02 for install auto-launch (shell + JS, verified by inspection and silent CI behavior).

---

## Standard Stack

### Core (all already in Cargo.toml — no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI framework: Layout, Paragraph, Alignment, Frame | Already in use — guide page uses same widgets as title page |
| crossterm | 0.29 | Keyboard event types: KeyCode::Tab, KeyCode::Left | Already in use — Tab and Left arrow are existing KeyCode variants |
| tui-big-text | 0.8 | BigText pixel font | Already in use — guide page deliberately omits BigText |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::time::{Duration, Instant}` | stdlib | Countdown deadline reset on page change | Existing pattern — reassign deadline on Tab keypress |
| `child_process.spawnSync` | Node built-in | Sync binary launch in npm run.js | Existing pattern in `proxyToBinary()` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `exec` in install.sh | subprocess call | `exec` replaces the shell cleanly; subprocess leaves the installer waiting — exec is correct |
| `spawnSync` in run.js | `execFileSync` | `spawnSync` with `stdio: 'inherit'` is already the project's pattern in `proxyToBinary()`; `execFileSync` adds no value |
| Page state enum | boolean flag | Enum (`Title` / `Guide`) is self-documenting and extensible; boolean is fine for 2 states but enum matches project style |

**Installation:** No new packages needed. All dependencies already present.

---

## Architecture Patterns

### Recommended Project Structure (changes only)

```
src/commands/welcome.rs   ← all guide-page additions live here
npm-package/bin/run.js    ← one guarded block added at bottom of install()
install.sh                ← one guarded block added after success echo
```

### Pattern 1: WelcomePage State Enum

**What:** A simple enum held as a `mut` variable in the `run_welcome_tui()` event loop. Determines which draw function is called and which key bindings are active each tick.

**When to use:** Any time the TUI has more than one screen that must share the same terminal session and countdown.

**Example:**
```rust
// Follows existing WelcomeAction pattern in welcome.rs
#[derive(Debug, Clone, PartialEq)]
enum WelcomePage {
    Title,
    Guide,
}
```

### Pattern 2: Deadline Reset on Page Transition

**What:** When the user navigates to the guide page the countdown resets to 5 s, giving them time to read. Same `deadline` variable reassigned in-place.

**When to use:** Any page that represents deliberate user engagement (not idle waiting).

**Example:**
```rust
// Inside the event loop, on Tab/Right press from Title page:
page = WelcomePage::Guide;
deadline = Instant::now() + Duration::from_secs(5);
```

### Pattern 3: Pure Functions First

**What:** All page-specific logic (routing, hint text, content text) is extracted as pure functions before the draw function. This keeps `draw_guide()` thin and lets unit tests cover behavior without a terminal.

**When to use:** Every new TUI function — this is the established project pattern from Phase 20.

**Example — guide hint bar (pure, testable):**
```rust
pub fn guide_hint_bar_text() -> String {
    "Tab/←: Back  Q: Quit".to_string()
}
```

**Example — guide routing (pure, testable):**
```rust
pub fn guide_routing_action(key: KeyCode) -> Option<WelcomeAction> {
    match key {
        KeyCode::Tab | KeyCode::Left => Some(WelcomeAction::ShowTitle),
        KeyCode::Char('q') | KeyCode::Esc => Some(WelcomeAction::Quit),
        _ => None,
    }
}
```

### Pattern 4: Title-Page routing_action Extension

**What:** Extend the existing `routing_action()` to return `ShowGuide` on Tab/Right arrow. The function signature gains no new parameters — `has_config` is already there.

**Example (extended):**
```rust
pub fn routing_action(key: KeyCode, has_config: bool) -> Option<WelcomeAction> {
    match key {
        KeyCode::Enter => { /* existing */ }
        KeyCode::Tab | KeyCode::Right => Some(WelcomeAction::ShowGuide),
        KeyCode::Char('q') | KeyCode::Esc => Some(WelcomeAction::Quit),
        _ => None,
    }
}
```

### Pattern 5: draw_guide() Layout

**What:** Parallel to `draw_welcome()` but simpler — no BigText widget. Uses `Paragraph` throughout.

**Example:**
```rust
fn draw_guide(frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header "Quick Guide"
            Constraint::Length(1), // blank
            Constraint::Min(0),    // guide content (steps)
            Constraint::Length(1), // hint bar
        ])
        .split(frame.area());

    let header = Paragraph::new("Quick Guide")
        .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // chunks[1]: blank — no widget

    let content = Paragraph::new(guide_content());
    frame.render_widget(content, chunks[2]);

    let hint = Paragraph::new(guide_hint_bar_text())
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(hint, chunks[3]);
}
```

### Pattern 6: npm TTY-guarded exec

**What:** At the bottom of `install()`, after `scaffoldProject()` and the "Next steps" printout, add a TTY guard.

**Key insight:** `destPath` is declared inside `installBinary()` as a local `var`. To use it in `install()`, either return it from `installBinary()` or re-derive it in `install()` using the same path logic. Returning it is cleaner.

**Example:**
```javascript
function install() {
  // ... existing setup ...
  var destPath = installBinary();   // installBinary now returns destPath
  scaffoldProject(force);
  // ... existing "Next steps" output ...

  if (process.stdout.isTTY) {
    spawnSync(destPath, [], { stdio: 'inherit' });
  }
}
```

### Pattern 7: shell exec guard

**What:** At the end of `install.sh`, after the success echo, replace the shell process with the binary.

**Example:**
```sh
# At the very end of install.sh, after existing echo statements:
if [ -t 1 ]; then
  exec "${INSTALL_DIR}/squad-station"
fi
```

### Anti-Patterns to Avoid

- **Checking CI env vars in addition to TTY:** CONTEXT.md is explicit — TTY check alone is sufficient. Adding `$CI`, `$GITHUB_ACTIONS`, etc. is over-engineering.
- **Using `spawn` instead of `exec` in install.sh:** `spawn` would leave the installer process waiting for the binary to exit; `exec` replaces the process cleanly.
- **Adding guide page Draw logic to run_welcome_tui():** Keep draw logic in `draw_guide()`, keep routing logic in `guide_routing_action()`. The event loop should only dispatch.
- **Using `process.env.isTTY`:** The correct Node.js check is `process.stdout.isTTY` (boolean property on the stream, not a process env var).
- **Resetting deadline on Tab press back to title:** Only reset when entering the guide page. Returning to title page can keep whatever time is left (or also reset — either is fine, but only the guide-entry reset is locked).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TTY detection in shell | Custom /proc parsing or stty check | `[ -t 1 ]` POSIX test | Standard, portable, already approved in CONTEXT.md |
| TTY detection in Node.js | Env var inspection | `process.stdout.isTTY` | Node built-in boolean, reliable across all Node versions |
| Sync binary launch in Node | Custom promise/exec chain | `spawnSync(path, [], { stdio: 'inherit' })` | Already used in `proxyToBinary()` — same pattern, same outcome |
| Multi-page TUI framework | Custom router/stack | Simple `WelcomePage` enum + `match` in event loop | Two pages do not need a framework |

**Key insight:** This phase has zero new dependencies. Everything reuses patterns already in the codebase.

---

## Common Pitfalls

### Pitfall 1: destPath scoping in run.js

**What goes wrong:** `destPath` is declared inside `installBinary()`. Referencing it from `install()` fails with `ReferenceError` or requires duplication of path-resolution logic.

**Why it happens:** Current code assigns `destPath` as a `var` local to `installBinary()` — it is not returned.

**How to avoid:** Change `installBinary()` to return `destPath`. Use the return value in `install()`. Early-return case (already installed, correct version) must also return the path.

**Warning signs:** If `destPath` is undefined at the `spawnSync` call, the binary launches with an empty string path.

### Pitfall 2: exec in install.sh skips trap cleanup

**What goes wrong:** `install.sh` uses `trap 'rm -f "$TMPFILE"' EXIT`. `exec` replaces the shell process, which DOES fire the EXIT trap — the tmpfile is cleaned up correctly before exec. However, if the `exec` is placed before `trap` has a chance to fire in normal flow, there could be ordering issues.

**Why it happens:** Confusion about when `exec` fires `EXIT`.

**How to avoid:** Place the `if [ -t 1 ]; then exec ...; fi` AFTER all cleanup-sensitive operations. The trap fires on EXIT when `exec` is called. The tmpfile is already moved (not a tempfile at binary path) by the time exec runs — no issue in practice.

**Verified:** The `TMPFILE` is moved via `mv "$TMPFILE" "${INSTALL_DIR}/squad-station"` before the echo statements. The `exec` block comes after. The trap cleans up the now-nonexistent temp file (no-op `rm -f`). Clean.

### Pitfall 3: Double-rendering on page switch

**What goes wrong:** The event loop calls `terminal.draw()` at the top of each iteration. If the page switch and deadline reset happen inside the same iteration that calls draw, the user might see one frame of the old page before the new page appears.

**Why it happens:** Draw happens before key-event handling in the current loop structure.

**How to avoid:** The current loop structure in `run_welcome_tui()` draws first, then polls for events. Page switch updates `page` and `deadline` after the key is received. The next iteration draws the new page. This is correct — one frame latency is imperceptible.

### Pitfall 4: Tab key consumed by terminal before reaching crossterm

**What goes wrong:** In some terminal configurations, Tab may be intercepted by the shell or readline before crossterm sees it.

**Why it happens:** Raw mode should suppress this. `enable_raw_mode()` is called in `setup_terminal()`.

**How to avoid:** Raw mode is already enabled before the event loop starts. `crossterm::event::poll` + `event::read()` in raw mode receives all keypresses including Tab. No action needed.

**Confidence:** HIGH — verified by crossterm documentation and the fact that raw mode is already in place.

### Pitfall 5: hint_bar_text() signature change breaks existing tests

**What goes wrong:** The existing `hint_bar_text(has_config, remaining_secs)` is tested with specific expected strings. If the signature or format changes (e.g., adding "Tab: Guide" to the output), the 3 existing hint bar tests will fail.

**Why it happens:** The title-page hint bar text must now include `Tab: Guide`.

**How to avoid:** Update `hint_bar_text()` to include Tab hint, then update the 3 existing test assertions to match the new format. This is expected and intentional — not a regression.

---

## Code Examples

Verified patterns from the existing codebase:

### WelcomeAction Enum Extension
```rust
// Current (welcome.rs line 31-35)
#[derive(Debug, Clone, PartialEq)]
pub enum WelcomeAction {
    LaunchInit,
    LaunchDashboard,
    Quit,
}

// Extended for Phase 21:
#[derive(Debug, Clone, PartialEq)]
pub enum WelcomeAction {
    LaunchInit,
    LaunchDashboard,
    Quit,
    ShowGuide,
    ShowTitle,
}
```

### Event Loop Page-State Extension
```rust
// Extended run_welcome_tui() event loop skeleton:
pub async fn run_welcome_tui(has_config: bool) -> anyhow::Result<Option<WelcomeAction>> {
    // ... existing terminal setup and panic hook ...
    let mut deadline = Instant::now() + Duration::from_secs(5);
    let mut action: Option<WelcomeAction> = None;
    let mut page = WelcomePage::Title;   // NEW

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() { break; }
        let remaining_secs = remaining.as_secs().max(1);

        terminal.draw(|f| match page {             // NEW dispatch
            WelcomePage::Title => draw_welcome(f, remaining_secs, has_config),
            WelcomePage::Guide => draw_guide(f),
        })?;

        if event::poll(remaining.min(Duration::from_secs(1)))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let act = match page {         // NEW per-page routing
                        WelcomePage::Title => routing_action(key.code, has_config),
                        WelcomePage::Guide => guide_routing_action(key.code),
                    };
                    if let Some(a) = act {
                        match a {
                            WelcomeAction::ShowGuide => {
                                page = WelcomePage::Guide;
                                deadline = Instant::now() + Duration::from_secs(5); // reset
                            }
                            WelcomeAction::ShowTitle => {
                                page = WelcomePage::Title;
                                // deadline: keep remaining or reset — discretion
                            }
                            WelcomeAction::Quit => { action = None; break; }
                            other => { action = Some(other); break; }
                        }
                    }
                }
            }
        }
    }
    // ... existing restore_terminal ...
}
```

### Existing spawnSync Pattern (from proxyToBinary in run.js)
```javascript
// Source: npm-package/bin/run.js line 172
var result = spawnSync(binaryPath, process.argv.slice(2), { stdio: 'inherit' });
// Phase 21 uses the same pattern with no args:
spawnSync(destPath, [], { stdio: 'inherit' });
```

### Updated hint_bar_text Format
```rust
// Updated signature (title page now shows Tab hint):
pub fn hint_bar_text(has_config: bool, remaining_secs: u64) -> String {
    if has_config {
        format!("Enter: Open dashboard  Tab: Guide  Q: Quit  auto-exit {}s", remaining_secs)
    } else {
        format!("Enter: Set up  Tab: Guide  Q: Quit  auto-exit {}s", remaining_secs)
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `frame.size()` (ratatui <0.29) | `frame.area()` | ratatui 0.29+ | Phase 20 already migrated — guide page must use `frame.area()` |
| Static welcome string printed to stdout | Interactive TUI with AlternateScreen | Phase 20 | Guide page is additive to the TUI path, not the static path |

**Deprecated/outdated:**
- `frame.size()`: removed in ratatui 0.29, replaced by `frame.area()`. Already migrated in Phase 20 — do not use `frame.size()` in `draw_guide()`.

---

## Open Questions

1. **Should returning to title page reset the countdown or preserve remaining time?**
   - What we know: CONTEXT.md only mandates reset on entering guide page.
   - What's unclear: Whether users expect the countdown to reset or continue when pressing Tab back.
   - Recommendation: Claude's discretion — preserving remaining time is the safer choice (less likely to surprise a user who is navigating back quickly). If less than 2s remains when returning, optionally bump to 2s to prevent immediate exit.

2. **Should Left arrow also navigate to the guide page from the title page?**
   - What we know: CONTEXT.md says Tab or Right arrow opens guide; Tab or Left arrow goes back. Left arrow on the title page is not explicitly addressed.
   - What's unclear: Whether Left arrow on the title page should be a no-op or open the guide (wrapping).
   - Recommendation: Claude's discretion — treat Left arrow on title page as no-op (non-wrapping). Simpler and less confusing.

3. **Page indicator (1/2 or dot indicator) in guide hint bar?**
   - What we know: CONTEXT.md lists this as Claude's discretion.
   - What's unclear: Whether it adds clarity or clutter at the hint bar width.
   - Recommendation: Include a simple dot indicator appended to the hint: `● ○` on title page, `○ ●` on guide page, prepended to the hint bar text. Adds minimal characters and communicates "there are two pages" clearly.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none — inline `#[cfg(test)]` modules |
| Quick run command | `cargo test welcome` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WELCOME-05 | `routing_action()` returns `ShowGuide` on Tab keypress | unit | `cargo test welcome::tests::test_routing_action_tab_opens_guide` | Wave 0 |
| WELCOME-05 | `routing_action()` returns `ShowGuide` on Right arrow | unit | `cargo test welcome::tests::test_routing_action_right_opens_guide` | Wave 0 |
| WELCOME-05 | `guide_routing_action()` returns `ShowTitle` on Tab | unit | `cargo test welcome::tests::test_guide_routing_tab_returns_title` | Wave 0 |
| WELCOME-05 | `guide_routing_action()` returns `ShowTitle` on Left arrow | unit | `cargo test welcome::tests::test_guide_routing_left_returns_title` | Wave 0 |
| WELCOME-05 | `guide_routing_action()` returns `Quit` on Q | unit | `cargo test welcome::tests::test_guide_routing_quit` | Wave 0 |
| WELCOME-05 | `guide_hint_bar_text()` returns expected string | unit | `cargo test welcome::tests::test_guide_hint_bar_text` | Wave 0 |
| WELCOME-05 | `hint_bar_text()` (title page) includes "Tab: Guide" | unit | `cargo test welcome::tests::test_hint_bar_text_includes_tab_guide` | Update existing |
| WELCOME-05 | `guide_content()` contains expected step text | unit | `cargo test welcome::tests::test_guide_content` | Wave 0 |
| INSTALL-01 | npm `install()` does not call spawnSync when `isTTY` is false | unit | manual-only — Node.js TTY property can't be mocked in a unit test without a test framework | N/A |
| INSTALL-02 | curl installer exits 0 in non-interactive mode | smoke | `echo "" \| sh install.sh 2>/dev/null; echo $?` | N/A |
| INSTALL-03 | Silent non-interactive behavior | smoke | pipe test above — no output when piped | N/A |

**Note on INSTALL-01/02/03:** These are script behaviors with no Rust unit tests. Verification is by code inspection (the guard condition is trivially correct) plus a smoke test that the install.sh exits cleanly when piped.

### Sampling Rate

- **Per task commit:** `cargo test welcome`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/commands/welcome.rs` — new test functions for guide routing, guide hint bar text, guide content, and updated title hint bar tests (inline `#[cfg(test)]` module — file already exists)
- [ ] No new test files needed — all tests live in `welcome.rs`'s existing test module

---

## Sources

### Primary (HIGH confidence)

- Existing `src/commands/welcome.rs` — read directly; all patterns verified in current codebase
- Existing `npm-package/bin/run.js` — read directly; `spawnSync` pattern and `destPath` variable confirmed
- Existing `install.sh` — read directly; `INSTALL_DIR` variable and script structure confirmed
- `src/main.rs` — read directly; WelcomeAction routing and TTY guard confirmed
- `.planning/phases/21-quick-guide-and-install-flow/21-CONTEXT.md` — locked decisions read directly
- `.planning/phases/20-tty-safe-welcome-tui-core/20-VERIFICATION.md` — confirmed Phase 20 completion status and test count (230 total, all passing)

### Secondary (MEDIUM confidence)

- crossterm KeyCode enum: Tab (`KeyCode::Tab`) and Left arrow (`KeyCode::Left`) are standard crossterm variants — confirmed by existing use of `KeyCode::Enter`, `KeyCode::Char`, `KeyCode::Esc` in the same file; no documentation fetch needed
- POSIX `[ -t fd ]` test: standard portable shell TTY check — high confidence from general knowledge; no verification source needed given the decision is already locked in CONTEXT.md
- Node.js `process.stdout.isTTY`: standard Node.js API — confirmed by existing ecosystem knowledge; decision locked in CONTEXT.md

### Tertiary (LOW confidence)

None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies already in Cargo.toml, no new dependencies needed
- Architecture: HIGH — all patterns derived directly from existing welcome.rs code, verified by reading the file
- Pitfalls: HIGH — derived from direct code reading (destPath scope, exec/trap interaction, existing test format)
- Install script changes: HIGH — trivial guard additions to existing scripts; both files read directly

**Research date:** 2026-03-18
**Valid until:** 2026-04-18 (stable domain — ratatui, crossterm, POSIX shell, Node.js TTY APIs are not fast-moving)
