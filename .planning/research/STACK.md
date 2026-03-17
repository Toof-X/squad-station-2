# Stack Research: v1.7 First-Run Onboarding

**Domain:** Interactive ratatui welcome TUI + post-install auto-launch for an existing Rust CLI
**Researched:** 2026-03-17
**Confidence:** HIGH

> **Scope:** This document covers only the NEW additions needed for v1.7. The existing stack
> (ratatui 0.26, crossterm 0.27, clap 4.5, tokio, sqlx 0.8, owo-colors 3) is validated and
> NOT re-researched here. All recommendations below are additive or upgrade decisions.

---

## Recommended Stack

### Core Technologies

| Technology | Current | Recommended | Purpose |
|------------|---------|-------------|---------|
| ratatui | 0.26 | **0.29** | Interactive welcome TUI — stay below 0.30 to avoid workspace split API surface |
| crossterm | 0.27 | **0.28** | Required by ratatui 0.29; `crossterm::tty::IsTty` for TTY guard |
| tui-big-text | (none) | **0.7.x** | Pixel-font large title ("SQUAD STATION") in welcome TUI |

**Why ratatui 0.29, not 0.30:**
ratatui 0.30 (December 2025) split into a modular workspace (`ratatui-core`, `ratatui-widgets`,
`ratatui-crossterm`). All widgets still re-export from the root crate, but the API surface is
larger and `Frame::area()` is required (replaces deprecated `Frame::size()`). Jumping to 0.29
avoids the 0.30 workspace split overhead while picking up `frame.area()` stabilization. Stay
on 0.29 unless a 0.30-only feature is needed.

**Why tui-big-text 0.7.x:**
The current `welcome.rs` uses plain ASCII art embedded as a `&str` constant. Converting to a
ratatui TUI screen means the static art must render as a proper widget. `tui-big-text` renders
large pixel-font glyphs via `font8x8` — the `BigText::builder()` API produces a widget renderable
via `frame.render_widget()`. Version 0.7.x is compatible with ratatui 0.29. Version 0.8.x requires
ratatui 0.30+.

**Why crossterm 0.28, not 0.27:**
ratatui 0.28+ uses crossterm 0.28. Mixing crossterm 0.27 with ratatui 0.28+ causes two versions
of crossterm in the dependency tree, producing type incompatibilities (events from one version
cannot be pattern-matched against the other). The upgrade is a single line in Cargo.toml and
requires no application code changes — the APIs used by the project (`KeyCode`, `KeyEventKind`,
`enable_raw_mode`, `EnterAlternateScreen`) are unchanged in 0.28.

---

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tui-big-text | 0.7.x | Pixel-font title widget | Welcome TUI screen — renders "SQUAD STATION" as large block letters |
| std::io::IsTerminal | (stdlib, Rust 1.70+) | TTY detection in Rust binary | Guard `run_welcome_tui()` — fall back to `print_welcome()` when stdout is not a TTY |

**Stdlib TTY detection — use `std::io::IsTerminal`, not crossterm:**
`crossterm::tty::IsTty` works but `std::io::IsTerminal` is the standard library equivalent
available since Rust 1.70 (project already uses 1.86+). For the welcome TUI guard this is
sufficient. Keep crossterm for terminal backend only; do not add a new dependency for TTY checks.

---

### Distribution Changes

#### npm postinstall auto-launch

| Component | Change | How |
|-----------|--------|-----|
| `npm-package/bin/run.js` | Add post-install first-run launch | After binary download completes, spawn `squad-station` with `stdio: 'inherit'` if `process.stdout.isTTY` is true |
| `install.sh` | Add post-install launch | After `chmod 755`, run `exec squad-station` at end of script |

**npm postinstall TTY check — required:**
npm postinstall scripts run non-interactively in CI environments (GitHub Actions, Docker builds,
etc.). Unconditionally launching the TUI in postinstall breaks those environments.
Guard: `if (process.stdout.isTTY) { spawnSync(binary, [], { stdio: 'inherit' }); }`.
This is the standard Node.js idiom — `process.stdout.isTTY` is `undefined` (falsy) in pipes
and CI, `true` only in interactive terminals.

**curl installer auto-launch — use `exec`, not `&`:**
After install, use `exec "${INSTALL_DIR}/squad-station"` not a background `&`. The user is
already in a shell waiting for the install to finish. `exec` replaces the install script
process cleanly. Do not use `nohup` or background the binary — the welcome TUI is interactive
and must have a controlling terminal.

**Shell TTY guard for curl installer:**
```sh
if [ -t 1 ]; then
  exec "${INSTALL_DIR}/squad-station"
fi
```
`[ -t 1 ]` tests whether file descriptor 1 (stdout) is a terminal. POSIX sh, no bashisms,
works on every target platform. Runs the binary only when the user is in an interactive session.

---

## Installation (Cargo.toml changes)

```toml
# Upgrade existing entries:
ratatui = "0.29"          # was "0.26"
crossterm = "0.28"        # was "0.27"

# New dependency:
tui-big-text = "0.7"
```

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| ratatui 0.29 | ratatui 0.30 | 0.30 requires frame.area() (breaking rename) and workspace split adds migration risk with no benefit for this feature set |
| ratatui 0.29 | Stay on 0.26 | 0.26 uses deprecated `frame.size()` — adds compiler warnings; tui-big-text 0.7.x minimum is 0.28 |
| tui-big-text 0.7.x | Hand-rolled ASCII art `&str` | Static art already exists but cannot be styled/colored as a ratatui widget without a wrapper; tui-big-text gives proper widget lifecycle, respects frame area, handles resize |
| tui-big-text 0.7.x | figlet-rs | figlet-rs generates strings, not widgets; must wrap in Paragraph anyway; tui-big-text is purpose-built for ratatui and has better pixel density options |
| std::io::IsTerminal | atty crate | `atty` is unmaintained (last release 2021); `std::io::IsTerminal` is stdlib since Rust 1.70 and functionally equivalent |
| std::io::IsTerminal | crossterm::tty::IsTty | Both work; stdlib is zero-dependency; no reason to add a crossterm import to the welcome module for this check |
| exec in install.sh | background spawn | The TUI is interactive; backgrounding it loses the controlling TTY and crossterm raw mode will fail |
| process.stdout.isTTY guard | unconditional launch | Unconditional launch breaks npm install in CI, Docker, and non-interactive shells |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| ratatui 0.30 | Workspace split changes import paths; `Frame::size()` is removed (not just deprecated) — all existing TUI code in `ui.rs` and `wizard.rs` must be updated | Stay on 0.29 for this milestone |
| tui-big-text 0.8.x | Requires ratatui 0.30 — wrong version | tui-big-text 0.7.x |
| crossterm 0.29 | Not required by ratatui 0.29; only needed with ratatui 0.30 | crossterm 0.28 |
| `dialoguer` or `inquire` | These are form-input crates — the project already has a full ratatui wizard; mixing paradigms creates inconsistency | Continue using ratatui raw-mode event loop |
| `indicatif` (progress bars) | No progress to show during welcome TUI; install script already uses echo | Not needed |
| `console` (terminal utilities) | Already have owo-colors + crossterm; `console` adds duplicate terminal abstraction | Not needed |

---

## Version Compatibility Matrix

| ratatui | crossterm | tui-big-text | Status |
|---------|-----------|--------------|--------|
| 0.26 | 0.27 | — | Current (v1.6); no BigText support |
| 0.27 | 0.27 | — | Skip — transitional; 0.27 re-exports crossterm internally |
| 0.28 | 0.28 | 0.7.x | Valid — but no meaningful feature advantage over 0.29 |
| **0.29** | **0.28** | **0.7.x** | **Recommended — latest stable below 0.30 workspace split** |
| 0.30 | 0.29 | 0.8.x | Next cycle — requires frame.area() migration in ui.rs + wizard.rs |

**Key constraint:** ratatui 0.28+ re-exports its crossterm version under `ratatui::crossterm`.
If Cargo.toml specifies crossterm independently, it MUST match the version ratatui requires or
two incompatible crossterm versions will exist in the tree. Specifying `crossterm = "0.28"` and
`ratatui = "0.29"` pins both to the same semver-compatible version.

---

## Integration Points

### In Rust (src/commands/welcome.rs)

The existing `print_welcome()` function must gain a ratatui path:

```
run_welcome_tui()   ← new: ratatui TUI screen with BigText title, Paragraph content, Enter-to-proceed
print_welcome()     ← existing: unchanged, used as TTY fallback
```

The dispatch in `src/main.rs` `None` arm becomes:
```
if stdout is TTY → run_welcome_tui() (ratatui, blocking until key press)
else            → print_welcome()   (existing behavior, pipe/CI safe)
```

The `run_welcome_tui()` function follows the same pattern as `src/commands/ui.rs`:
`setup_terminal()` → event loop → `restore_terminal()`. No new terminal management helpers
are needed — the same `enable_raw_mode / EnterAlternateScreen / disable_raw_mode /
LeaveAlternateScreen` pattern used by the existing TUI and wizard applies.

### In Node.js (npm-package/bin/run.js)

No new npm dependencies are needed. Node.js stdlib `spawnSync` with `stdio: 'inherit'`
and `process.stdout.isTTY` guard covers the auto-launch requirement entirely.

### In Shell (install.sh)

Only the final two lines change: add the TTY check and `exec` call. No new tools required.
POSIX `[ -t 1 ]` is universally available on macOS and Linux.

---

## Confidence Assessment

| Area | Confidence | Rationale |
|------|------------|-----------|
| ratatui 0.29 upgrade path | HIGH | Official release notes + crossterm incompatibility advisory (github.com/ratatui/ratatui/issues/1298) confirm version pairing |
| tui-big-text 0.7.x for ratatui 0.29 | HIGH | docs.rs confirms tui-big-text 0.8.x requires ratatui ^0.30; 0.7.x is the 0.29-compatible release |
| crossterm 0.28 compatibility | HIGH | Official advisory explicitly states 0.27↔0.28 are semver-incompatible; ratatui 0.28+ uses 0.28 |
| frame.area() rename | HIGH | Documented in BREAKING-CHANGES.md and v0.28.0 highlights; affects ui.rs and wizard.rs |
| std::io::IsTerminal in stdlib | HIGH | Rust docs confirm stable since 1.70; project MSRV is 1.86 |
| npm isTTY guard pattern | HIGH | Standard Node.js idiom; `process.stdout.isTTY` documented in Node.js TTY module docs |
| POSIX `[ -t 1 ]` for shell TTY check | HIGH | POSIX.1-2008 standard; works on all macOS and Linux shells |
| exec vs background spawn for TUI | HIGH | Crossterm raw mode requires a controlling TTY; background processes lose it |

---

## Sources

- [ratatui v0.28.0 highlights](https://ratatui.rs/highlights/v028/) — crossterm 0.28 requirement, frame.area() rename confirmed
- [ratatui v0.29.0 highlights](https://ratatui.rs/highlights/v029/) — feature set verified
- [ratatui v0.30.0 highlights](https://ratatui.rs/highlights/v030/) — workspace split confirmed; frame.size() removed
- [Ratatui / Crossterm Version incompatibility advisory](https://github.com/ratatui/ratatui/issues/1298) — semver conflict between crossterm 0.27 and 0.28 confirmed
- [tui-big-text docs.rs](https://docs.rs/tui-big-text/latest/tui_big_text/) — version 0.8.2 requires ratatui ^0.30.0 (MEDIUM — inferred 0.7.x for 0.29 compatibility)
- [crossterm::tty docs](https://docs.rs/crossterm/latest/crossterm/tty/index.html) — IsTty trait confirmed; stdlib alternative is preferable
- [std::io::IsTerminal](https://doc.rust-lang.org/std/io/trait.IsTerminal.html) — stable since Rust 1.70
- [Node.js TTY module docs](https://nodejs.org/api/tty.html) — process.stdout.isTTY documented
- [Publishing binaries on npm — Sentry Engineering](https://sentry.engineering/blog/publishing-binaries-on-npm) — spawnSync with stdio:inherit pattern

---

*Stack research for: v1.7 First-Run Onboarding (interactive welcome TUI + post-install auto-launch)*
*Researched: 2026-03-17*
