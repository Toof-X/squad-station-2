# Pitfalls Research

**Domain:** First-Run Onboarding TUI + Post-Install Auto-Launch for Rust CLI (v1.7 milestone)
**Researched:** 2026-03-17
**Confidence:** HIGH — verified against crossterm/ratatui docs, npm lifecycle docs, Rust stdlib docs, and codebase inspection

---

## Scope of This Document

This file covers pitfalls **specific to v1.7**: adding a ratatui first-run TUI welcome screen and
post-install auto-launch to an existing, stable Rust CLI. The existing codebase already ships:

- `wizard.rs` — ratatui TUI wizard with `setup_terminal()` / `restore_terminal()` + panic hook
- `init.rs` — TTY guard via `std::io::stdin().is_terminal()` (prevents wizard launch in CI)
- `welcome.rs` — plain-text welcome screen, content extracted to testable `welcome_content()` fn
- `npm-package/bin/run.js` — Node.js wrapper that proxies all non-`install` subcommands to binary
- 211 passing tests (no ratatui render in test suite — all TUI code is untested at unit level)

The key constraint: the new TUI welcome screen must run when `squad-station` is invoked bare
(no subcommand), AND post-install scripts in both npm and curl contexts must either auto-launch
it or explicitly skip it. Both paths cross a TTY boundary that can silently break.

---

## Critical Pitfalls

### Pitfall 1: wizard.rs TTY Guard Pattern Not Applied to Welcome TUI

**What goes wrong:**
The existing `wizard.rs::run()` and `run_worker_only()` functions call `setup_terminal()` directly
without checking `std::io::stdout().is_terminal()` first. The callers in `init.rs` provide the
guard (`std::io::stdin().is_terminal()` at line 103), but `welcome.rs::print_welcome()` (the
bare-invocation path in `main.rs`) currently prints plain text and does NOT call any TUI code.

When v1.7 upgrades `print_welcome()` to launch a ratatui TUI, if the developer forgets to add
the same TTY guard, `enable_raw_mode()` will be called with stdout connected to a pipe (during
`npx squad-station` or `squad-station | cat`). Crossterm returns an `ENOTTY` `io::Error` from
`enable_raw_mode()` in non-TTY contexts — but only if the error is propagated. If `setup_terminal()`
is called via `?` inside an `async fn run()` that returns `anyhow::Result<()>`, the error will
propagate to `main.rs` and print "Error: Inappropriate ioctl for device" — a confusing message
that gives the user no actionable guidance.

**Why it happens:**
The wizard is always called from `init.rs` which already holds the TTY guard. The new welcome
TUI has no such parent guard. Developers porting the TUI launch pattern from `wizard.rs` copy
the terminal setup code without copying the guard that belongs one level up.

**How to avoid:**
Check TTY before any `enable_raw_mode()` call in the welcome path:

```rust
// In welcome.rs or main.rs bare-invocation arm:
if std::io::stdout().is_terminal() {
    run_welcome_tui().await?;
} else {
    print_welcome();  // fall back to existing plain-text output
}
```

The non-TTY fallback must be the existing `print_welcome()` so that `squad-station | grep init`
and similar pipelines continue to work correctly.

**Warning signs:**
- Running `squad-station | head` prints `Error: Inappropriate ioctl for device` instead of help text
- `npx squad-station` (before binary is installed) triggers the error in the npm postinstall context
- CI jobs that call `squad-station` for scripted setup fail with the ENOTTY error

**Phase to address:** Phase 1 of v1.7 — the TTY guard must exist before the TUI is wired into
the bare-invocation arm. The guard is the prerequisite, not the TUI itself.

---

### Pitfall 2: npm postinstall Auto-Launch Hangs in Non-Interactive Contexts

**What goes wrong:**
The v1.7 goal is "post-install auto-launch" of the welcome TUI. If the npm `postinstall` script
(or `bin/run.js`'s install command) calls `squad-station` bare (no subcommand) expecting the user
to see the welcome TUI, it will **block** in CI environments. npm postinstall runs with stdin
connected to `/dev/null` (or a closed pipe) and stdout/stderr connected to the npm install log
stream — not a TTY. If the binary enters an `event::poll()` loop waiting for keyboard input, the
process never receives any input events and hangs indefinitely.

The current `run.js` install command does NOT call `squad-station` at the end. If v1.7 adds a
postinstall call, the hang risk is introduced.

**Why it happens:**
npm spawns lifecycle scripts in a non-interactive subprocess. `process.stdout.isTTY` is `undefined`
in postinstall. The npm docs explicitly state: "postinstall scripts run with stdio configured as
`pipe`." Any attempt to interact with the terminal from a postinstall script will hang or error.

**How to avoid:**
Do NOT auto-launch `squad-station` from `postinstall`. The correct pattern is:

1. `postinstall` installs the binary (already done in `run.js`)
2. `postinstall` prints: "Run `squad-station` to get started"
3. User runs `squad-station` manually — the TTY check in Pitfall 1 then determines whether to
   show the TUI or plain text

If auto-launch from install is required, gate it on `process.stdout.isTTY` in `run.js`:

```javascript
// In run.js install() — only launch TUI if stdout is a TTY
if (process.stdout.isTTY) {
    spawnSync(binaryPath, [], { stdio: 'inherit' });
}
// Otherwise: just print the next-steps hint
```

**Warning signs:**
- `npm install squad-station` hangs indefinitely in CI
- GitHub Actions or other CI systems time out on the install step
- Users report `npm install` taking minutes instead of seconds

**Phase to address:** Phase 1 of v1.7 — before any postinstall modification. The rule is explicit:
postinstall must never block. Enforce this with a CI test that runs `npm install` with stdin closed.

---

### Pitfall 3: curl | sh Auto-Launch Breaks When stdin Is the Pipe

**What goes wrong:**
When a user runs `curl -fsSL https://... | sh`, bash's stdin is connected to the output of curl —
not to the terminal. Any command in the install script that reads from stdin (including spawning
a process that calls `crossterm::event::read()` to wait for keypresses) will immediately receive
EOF, hang waiting for pipe data that never arrives, or error with "read: Input/output error."

`install.sh` currently ends with `echo "Run: squad-station --version"` and exits cleanly. If
v1.7 adds `squad-station` at the end of `install.sh` to auto-launch the welcome TUI, the same
hang risk from Pitfall 2 applies. Additionally, bash scripts running via curl pipe cannot use
`/dev/tty` reliably on all systems because the terminal may not have a controlling TTY in some
orchestrated install environments (Docker build, devcontainer setup).

**Why it happens:**
`curl | sh` is a pipe — bash's stdin is the curl output stream, not the terminal. The install
script runs to completion reading from curl's output. Any child process that tries to read
keyboard input blocks forever or gets EOF immediately. The `[ -t 0 ]` test (stdin is TTY) returns
false in this context.

**How to avoid:**
Do NOT call `squad-station` from `install.sh`. The install script's job is installation only.
End `install.sh` with a print message, not with a binary launch. The TTY guard in the binary
itself (`is_terminal()`) provides defense-in-depth, but the correct fix is at the install script
level: never launch interactive programs from a piped shell script.

**Warning signs:**
- `curl -fsSL https://... | sh` hangs at the last step
- The install step in a devcontainer or Dockerfile never completes
- Users running install in a tmux pane that lacks a controlling terminal report hangs

**Phase to address:** Phase 1 of v1.7 — specifically the "post-install auto-launch" design
decision. The decision must be: "auto-launch is only user-initiated, not script-initiated."

---

### Pitfall 4: Terminal Not Restored After Early Return / Error in Welcome TUI

**What goes wrong:**
The welcome TUI enters alternate screen mode and raw mode. If an error occurs mid-render
(e.g., terminal resize event triggers a panicking unwrap, or a `?` in the event loop returns
early), `restore_terminal()` is never called. The user's terminal is left in raw mode with the
alternate screen visible. They see a blank screen. `Ctrl+C` does not work normally. They must
type `reset` blindly to recover.

The existing `wizard.rs::run()` installs a panic hook that calls `disable_raw_mode()` and
`execute!(LeaveAlternateScreen)`. The welcome TUI needs the same pattern. If it is written as
a new function without copying the panic hook, the panic recovery is missing.

Additionally: `ui.rs::run()` installs a panic hook via `std::panic::take_hook()` and
`std::panic::set_hook()`, but restores `let _ = std::panic::take_hook()` at the end (which drops
the hook). If the welcome TUI is added as another TUI entry point without coordination, two
take_hook/set_hook pairs can interfere: one TUI's cleanup hook is accidentally dropped when a
different TUI sets a new hook.

**Why it happens:**
Multiple TUI modules each manage their own panic hook lifecycle independently. When multiple TUI
entrypoints exist in the same binary (welcome TUI, wizard TUI, ui.rs dashboard), the
take_hook/set_hook pattern becomes fragile. Each module saves and restores the hook, but if
they are ever called in sequence in the same process (unlikely now, possible in tests or future
features), the hook chain gets corrupted.

**How to avoid:**
- Extract `setup_terminal()` / `restore_terminal()` / panic-hook installation into a shared
  module (`src/tui.rs` or `src/commands/tui_guard.rs`) used by wizard.rs, ui.rs, and the new
  welcome TUI. This guarantees consistent behavior.
- Alternatively, adopt `ratatui::init()` / `ratatui::restore()` introduced in ratatui 0.28.1,
  which handles panic hook setup automatically. The current codebase uses ratatui 0.26 (per
  PROJECT.md), so this requires a version bump — evaluate the breaking-change surface before
  adopting.
- At minimum: every new TUI entrypoint must install the panic hook and call `restore_terminal()`
  on every code path that exits the loop (normal exit, Ctrl+C, error propagation via `?`).

**Warning signs:**
- After dismissing the welcome TUI with Ctrl+C, subsequent terminal output appears garbled
- Running `squad-station` twice in sequence leaves the terminal in raw mode after the second run
- Tests that call TUI functions leave the test runner in raw mode, breaking subsequent test output

**Phase to address:** Phase 1 of v1.7 — terminal cleanup contract must be established before
any TUI code is added to the welcome path.

---

### Pitfall 5: Cargo Test Suite Breaks When TUI Code Touches Real Terminal

**What goes wrong:**
`cargo test` runs tests in parallel. Tests share the same process's stdout/stderr. If any test
calls a function that calls `enable_raw_mode()` or `EnterAlternateScreen` on real stdout, it
corrupts the terminal state for all other tests running concurrently. The effect is:
- Test output becomes garbled
- Some tests produce no output at all (alternate screen hides it)
- The terminal is left in raw mode after the test suite exits

The current test suite avoids this by the `welcome_content()` pattern: the testable pure-string
function is separate from `print_welcome()` which touches the terminal. The wizard tests only
test pure-logic functions (`generate_squad_yml`, `append_workers_to_yaml`, etc.) — they never
call `wizard::run()` which enters the TUI.

If v1.7 adds a `welcome_tui::run()` function and tests inadvertently call it (e.g., an
integration test that calls `commands::welcome::run_tui()` without a TTY guard), the entire
test suite output becomes unreadable.

**Why it happens:**
Developers write an integration test that exercises the full code path through `main.rs` with
`None` command. The test passes because the TTY check returns false (test runner has no TTY),
but if the check is missing or the test somehow provides a TTY (e.g., via `pty` or when run
interactively), the TUI launches and corrupts output.

Additionally: `cargo test` on macOS in iTerm2 or Terminal.app runs with stdout connected to a
TTY. If a developer runs `cargo test` interactively and any test calls `enable_raw_mode()`, the
terminal enters raw mode and the test runner output is invisible.

**How to avoid:**
- Never call `enable_raw_mode()` or terminal-manipulating functions from test code
- Design TUI entry points to accept a TTY availability parameter or check it internally:
  `fn run_tui_if_interactive() -> bool` that returns false and does nothing when not a TTY
- The pattern already in `welcome_content()` is correct: extract pure content, test content,
  never test render
- Add a `#[cfg(not(test))]` guard or `is_terminal()` check inside `run_welcome_tui()` so it
  is a no-op during `cargo test`
- Document in `CLAUDE.md`: "Never call TUI run() functions from unit or integration tests"

**Warning signs:**
- `cargo test` leaves terminal in raw mode after completion
- Test output shows garbled characters or blank lines where test names should be
- Tests pass individually but fail or produce no output when run in parallel

**Phase to address:** Phase 1 of v1.7 — test isolation contract must be established as the
first thing, before any TUI code is merged. Every TUI PR should be reviewed for test isolation.

---

### Pitfall 6: Welcome TUI Blocks When Terminal Is Too Small to Render

**What goes wrong:**
ratatui renders into the actual terminal dimensions. If the terminal is very small (e.g., 20x5
characters — common in CI terminals, embedded terminals, or split tmux panes), the layout
constraints may produce zero-height widgets, causing ratatui to panic in debug builds or silently
render nothing in release builds. The welcome TUI then appears blank or crashes.

Additionally: `frame.size()` returns the terminal dimensions at render time. If the terminal is
resized to zero dimensions (possible in some CI virtual terminals), `Layout::split()` with
`Constraint::Length(3)` on a 0-height area produces a Rect with height 0, and subsequent widget
rendering into that Rect may overflow.

The existing wizard uses `Constraint::Length(3)` for the header area. A terminal with fewer than
3 rows will cause the layout to saturate and produce overlapping zero-size areas.

**Why it happens:**
Developers test the TUI in their normal development terminal (80x24 or larger). They never test
in a small terminal or a non-human-interactive context where the terminal is reported as having
minimal dimensions (e.g., 0x0 or 1x1 in some pty-less contexts).

**How to avoid:**
- Add a minimum terminal size check before entering the TUI event loop:
  ```rust
  let size = terminal.size()?;
  if size.height < 10 || size.width < 40 {
      restore_terminal(&mut terminal)?;
      print_welcome();  // fall back to plain text
      return Ok(());
  }
  ```
- This minimum size check should match the actual layout requirements of the welcome TUI design
- The existing `welcome_content()` plain-text fallback is the correct target for the small-terminal path

**Warning signs:**
- Welcome TUI appears blank in tmux split panes
- `squad-station` crashes with a layout panic when terminal window is tiny
- Users in tmux with many splits (small pane width) cannot use the welcome TUI

**Phase to address:** Phase 1 of v1.7 — minimum size guard should be part of the initial TUI
implementation, not added later as a fix.

---

## Moderate Pitfalls

### Pitfall 7: "Press Enter to Continue" Pattern Blocks Orchestrators and Scripts

**What goes wrong:**
The v1.7 design includes: "no squad.yml → Press Enter to set up → wizard." If the welcome TUI
enters a blocking wait for keypress (`event::read()` with no timeout) on the welcome screen,
any script or orchestrator that calls `squad-station` expecting it to exit immediately will block.

This is especially problematic for:
- The Gemini CLI `@squad-station` or MCP-style invocations that call the binary in a subprocess
- The npm `bin/run.js` proxy that calls the binary with `spawnSync` — this blocks the Node process
- CI scripts that call `squad-station` to check version or status before setup

The existing plain-text `print_welcome()` exits immediately. The TUI version must not hang
indefinitely.

**How to avoid:**
- Use `event::poll(timeout)` with a defined timeout (e.g., 30 seconds) rather than blocking `event::read()`
- Or: display the welcome TUI with a visible countdown: "Press any key or wait 10 seconds..."
- Or: add an `--no-wait` flag that skips the interactive welcome and falls through to plain text
- The best approach for orchestrator safety: the welcome TUI exits automatically after a short
  timeout and falls back to printing the plain-text welcome to stdout

**Warning signs:**
- `spawnSync('squad-station', [], { timeout: 5000 })` times out in `run.js`
- Orchestrator AI tools that call `squad-station` as a tool call appear to hang
- Shell scripts with `squad-station && next-command` never reach `next-command`

**Phase to address:** Phase 1 of v1.7 — the event loop design must include auto-exit timeout.

---

### Pitfall 8: Alternate Screen Swallows Welcome Output in Scripts

**What goes wrong:**
`EnterAlternateScreen` switches to a secondary buffer. When the TUI exits and calls
`LeaveAlternateScreen`, everything rendered in the TUI disappears. The user's terminal shows
their previous content with no trace of the welcome TUI.

For an informational welcome screen, this is the WRONG UX: the user sees a flash of content
and it vanishes. They cannot scroll back to see the version number or command hints.

The existing `print_welcome()` writes to the main screen buffer (stdout), where it persists in
the scrollback. The new TUI should evaluate whether alternate screen is appropriate for a
one-time welcome message that the user needs to remember.

**How to avoid:**
- For the welcome screen specifically: consider NOT using alternate screen. Render in the main
  buffer with raw mode only for keypress detection, then restore normal mode after the keypress.
  The content remains visible in scrollback.
- If alternate screen IS used, print a brief "Run `squad-station init` to set up" message to
  regular stdout AFTER `LeaveAlternateScreen`, so there is persistent text in the scrollback.
- The wizard uses alternate screen correctly (full-screen form), but the welcome screen is more
  like a splash screen — different UX requirements.

**Warning signs:**
- User dismisses welcome screen and the terminal is blank with no memory of what was shown
- Users ask "how do I see the version again?" — the answer should not be "run it again"

**Phase to address:** Phase 2 of v1.7 (UX polish), but the architectural decision (alt screen vs
main buffer) should be made in Phase 1 to avoid a rendering rewrite.

---

### Pitfall 9: Stale Binary After npm install with Version Mismatch

**What goes wrong:**
`run.js` checks if the installed binary matches the npm package version:

```javascript
var result = spawnSync(destPath, ['--version'], { encoding: 'utf8' });
if (result.stdout && result.stdout.includes(VERSION)) {
    console.log('  ✓ squad-station v' + VERSION + ' already installed');
    return;
}
```

If the binary at `destPath` is from a different install path (e.g., installed via curl to
`~/.local/bin` but npm checks `/usr/local/bin`), and the old binary is on PATH before the new
location, `squad-station` resolves to the old binary. The version check passes on the wrong file.

For v1.7: if the old binary does not have the welcome TUI (v1.6 binary) but the npm package is
v1.7, the user runs `squad-station` and gets the old plain-text welcome. They see no indication
that their install is stale.

**How to avoid:**
This is pre-existing behavior, but v1.7 should not make it worse. Ensure the version string
reported by `--version` matches `CARGO_PKG_VERSION` and is unique per release. The version check
in `run.js` is sufficient for npm-installed paths; document that mixing install methods can lead
to stale binaries.

**Warning signs:**
- User upgrades npm package but `squad-station` behavior looks unchanged
- `squad-station --version` reports a different version than `npm list squad-station`

**Phase to address:** Not introduced by v1.7, but review during integration testing.

---

### Pitfall 10: ratatui 0.26 API Differences From Current Docs

**What goes wrong:**
The project uses ratatui 0.26 (per PROJECT.md Cargo context). The current ratatui documentation
at ratatui.rs and docs.rs describes ratatui 0.29+ (latest stable). Key differences:

- `ratatui::init()` / `ratatui::restore()` were introduced in 0.28.1. They are NOT available in 0.26.
- `frame.size()` was deprecated in favor of `frame.area()` in newer versions. Code written against
  new docs will not compile against 0.26.
- The `Constraint` API, `Layout::default()` pattern, and widget constructors are stable across
  0.26-0.29, but subtle deprecations cause compiler warnings that can be mistaken for bugs.

If a developer follows current ratatui tutorials or docs.rs snippets to build the welcome TUI,
they may write code that does not compile against the pinned 0.26 version.

**How to avoid:**
- Check `Cargo.toml` for the pinned ratatui version before writing any TUI code
- Use `docs.rs/ratatui/0.26.x/ratatui/` (with version pin in URL) for API reference
- Or: bump ratatui to latest (0.29+) as part of v1.7 and update the single API usage change
  (`frame.size()` → `frame.area()` if applicable)
- Review BREAKING-CHANGES.md in the ratatui repository before bumping

**Warning signs:**
- `cargo build` fails with "method `init` not found in type `ratatui`"
- Clippy warns about deprecated `frame.size()` usage
- Documentation examples don't match actual compiler behavior

**Phase to address:** Phase 1 of v1.7 — check version compatibility before writing TUI code.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Copy-paste `setup_terminal()` into welcome.rs | Quick to implement | Three copies of the same pattern; inconsistent panic hook handling | Never — extract to shared module |
| Skip alternate-screen decision, default to EnterAlternateScreen | Matches wizard pattern | Welcome content disappears from scrollback; confusing UX | Never for a one-time welcome message |
| Inline TTY check in welcome.rs only | Minimal change | TTY check not enforced in future TUI entry points | Acceptable MVP if documented as pattern |
| No minimum terminal size check | Simpler code | Crashes or blank screen in small terminals | Never — add minimum check |
| Auto-exit timeout omitted (block on keypress forever) | Simpler event loop | Hangs any script or orchestrator that calls bare `squad-station` | Never — always include timeout |
| Skip non-TTY fallback (just return Ok if not terminal) | Simpler code | Breaks `squad-station --help` in piped contexts (no help text) | Never — fallback to print_welcome() |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| npm postinstall | Add `spawnSync(binaryPath, [])` at end of `install()` in `run.js` | Never call the binary from postinstall; print next-steps hint only |
| curl \| sh install.sh | Append `$INSTALL_DIR/squad-station` at end of script | Never call the binary from install.sh; `stdin` is the curl pipe |
| npm proxy in `run.js` | `proxyToBinary()` calls binary with `stdio: 'inherit'` — TUI works when user runs `npx squad-station` | This is correct; `npx squad-station` in a real terminal has TTY — TTY check passes |
| cargo test parallel | Call `run_welcome_tui()` in integration tests | Only test `welcome_content()` (pure string); never call TUI render functions from tests |
| GitHub Actions CI job that calls `squad-station` | Binary auto-launches TUI, CI hangs | TTY guard must be the first thing in the bare-invocation path; CI has no TTY |
| tmux-based usage (existing squad users) | Welcome TUI renders inside a tmux pane — this works fine | tmux panes ARE TTYs; `is_terminal()` returns true; TUI renders correctly |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Polling too aggressively in welcome TUI event loop | High CPU on idle welcome screen | Use `event::poll(250ms)` timeout, same as wizard.rs and ui.rs | Immediately on launch |
| Loading DB or config in welcome TUI before squad.yml exists | "squad.yml not found" error on bare invocation | Welcome TUI must not read DB or config; it is pre-setup | First time user runs `squad-station` |
| Spawning external processes from welcome TUI (e.g., checking for updates) | Slow TUI startup | Welcome TUI is display-only; no network calls, no subprocess spawns | Every invocation |

---

## "Looks Done But Isn't" Checklist

- [ ] **TTY guard on welcome TUI:** Verify `squad-station | cat` still prints plain-text welcome, not an error
- [ ] **npm postinstall safety:** Verify `npm install` completes in under 30 seconds with stdin closed: `npm install squad-station < /dev/null`
- [ ] **curl install safety:** Verify `curl -fsSL <url> | sh` completes without hanging
- [ ] **Terminal restored on Ctrl+C:** After pressing Ctrl+C to dismiss welcome TUI, run `echo test` — it should appear normally
- [ ] **Terminal restored on error:** Trigger an error inside the TUI event loop (resize to 0x0), verify terminal is restored
- [ ] **Existing 211 tests still pass:** Run `cargo test` after adding TUI welcome — no test should enter raw mode
- [ ] **Minimum size fallback:** Resize terminal to 20 columns wide, run `squad-station` — should show plain text, not crash
- [ ] **Auto-exit timeout:** Run `squad-station` and do not press any key — verify it exits after the configured timeout
- [ ] **Non-interactive CI:** Run `squad-station` in a GitHub Actions step — must exit 0 without hanging

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Terminal stuck in raw mode after bad TUI exit | LOW | User types `reset` blindly; terminal recovers immediately |
| npm install hangs in CI due to auto-launch | MEDIUM | Kill the job, remove the auto-launch call, republish npm package |
| curl install.sh hangs | MEDIUM | User Ctrl+C out of it; no data loss; remove auto-launch, re-release |
| Test suite corrupted by raw mode | LOW | `cargo test -- --test-threads=1` to serialize; find which test enters TUI; add TTY guard |
| Welcome TUI crashes on small terminal | LOW | Add minimum size check in next patch; fall back to print_welcome() |
| Stale binary after npm upgrade | LOW | User runs `npx squad-station install --force` to re-download |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| No TTY guard on welcome TUI | Phase 1: Implement TTY guard before wiring TUI to bare invocation | `squad-station \| cat` outputs plain text; `cargo test` passes |
| npm postinstall auto-launch | Phase 1: Design decision — never auto-launch from postinstall | `npm install < /dev/null` completes in <30 seconds |
| curl install.sh auto-launch | Phase 1: Design decision — install.sh ends with echo, not binary call | `curl url \| sh` completes without hang |
| Terminal not restored on error/panic | Phase 1: Extract shared TUI guard module, add panic hook to welcome TUI | Ctrl+C recovery works; error path restores terminal |
| Test suite broken by TUI in tests | Phase 1: `is_terminal()` check inside `run_welcome_tui()` as no-op in tests | `cargo test` all 211+ tests pass without terminal corruption |
| Welcome TUI blocks scripts with no timeout | Phase 1: Implement event loop with auto-exit timeout | `spawnSync('squad-station', [], {timeout: 5000})` completes |
| Alternate screen swallows welcome content | Phase 1 (design) / Phase 2 (polish): Choose main buffer over alt screen | After TUI exits, scrollback shows version and hints |
| Terminal too small to render | Phase 1: Add minimum size check with print_welcome() fallback | 20x5 terminal shows plain text, not crash |
| ratatui 0.26 API mismatch | Phase 1: Check Cargo.toml version before writing TUI code | `cargo build` succeeds without version-related errors |

---

## Sources

- [crossterm enable_raw_mode docs — ENOTTY error](https://docs.rs/crossterm/latest/crossterm/terminal/fn.enable_raw_mode.html)
- [crossterm issue #912 — enable_raw_mode error in WSL](https://github.com/crossterm-rs/crossterm/issues/912)
- [ratatui panic hooks recipe](https://ratatui.rs/recipes/apps/panic-hooks/)
- [ratatui alternate screen concept](https://ratatui.rs/concepts/backends/alternate-screen/)
- [ratatui terminal and event handler recipe](https://ratatui.rs/recipes/apps/terminal-and-event-handler/)
- [ratatui BREAKING-CHANGES.md](https://github.com/ratatui/ratatui/blob/main/BREAKING-CHANGES.md)
- [ratatui v0.30 highlights](https://ratatui.rs/highlights/v030/)
- [npm postinstall non-TTY issue #16608](https://github.com/npm/npm/issues/16608)
- [npm scripts at scale — CI reliability](https://www.mindfulchase.com/explore/troubleshooting-tips/build-bundling/advanced-troubleshooting-npm-scripts-at-scale—-deterministic-builds,-workspaces,-and-ci-reliability.html)
- [curl | sh stdin behavior — linuxvox.com](https://linuxvox.com/blog/execute-bash-script-remotely-via-curl/)
- [curl | sh pitfalls overview](https://www.arp242.net/curl-to-sh.html)
- [Integration testing TUI in Rust — quantonganh.com](https://quantonganh.com/2024/01/21/integration-testing-tui-app-in-rust.md)
- [ratatui_testlib crate](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/)
- [Rust stdin/stdout testing patterns](https://jeffkreeftmeijer.com/rust-stdin-stdout-testing/)
- Squad Station codebase: `src/commands/wizard.rs`, `src/commands/init.rs`, `src/commands/welcome.rs`, `npm-package/bin/run.js`, `install.sh`

---
*Pitfalls research for: Squad Station v1.7 First-Run Onboarding TUI + Post-Install Auto-Launch*
*Researched: 2026-03-17*
