# Feature Research

**Domain:** CLI first-run onboarding TUI — interactive welcome screen + post-install auto-launch
**Researched:** 2026-03-17
**Confidence:** HIGH (primary patterns from existing codebase analysis + BMAD-METHOD installation flow + clig.dev canonical guidance + ratatui ecosystem)

---

## Context: What Already Exists (Do Not Re-Build)

Before mapping new features, the following are confirmed shipped and must not be duplicated:

| Component | Status | Location |
|-----------|--------|----------|
| Static welcome screen (`squad-station` bare) | DONE v1.6 | `src/commands/welcome.rs` |
| ASCII title art + version + subcommand list | DONE v1.6 | `welcome.rs::print_welcome()` |
| `squad-station init` hint line | DONE v1.6 | `welcome.rs` |
| Multi-page ratatui wizard (init flow) | DONE v1.5 | `src/commands/wizard.rs` |
| Post-init ASCII agent diagram | DONE v1.6 | `src/commands/diagram.rs` |
| Alternate screen setup/teardown pattern | DONE v1.5 | `wizard.rs` + `ui.rs` |
| TTY guard (`is_terminal()`) for non-interactive contexts | DONE v1.5 | `init.rs` |
| npm binary distribution | DONE v1.2 | npm postinstall |
| curl installer (`install.sh`) | DONE v1.2 | `install.sh` |

The v1.7 milestone replaces `print_welcome()` with an interactive ratatui TUI and adds post-install auto-launch to both distribution paths.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features a CLI first-run onboarding experience must have. Missing any of these makes the tool feel incomplete or untrustworthy compared to tools like BMAD-METHOD, OpenCode, and gh.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Large branded ASCII title in alternate screen | Establishes tool identity; BMAD, gh, fly all show branding upfront; alternatescreen = professional TUI tool feel | LOW | ASCII art already exists; wrap in `ratatui::widgets::Paragraph` inside alternate screen; same `setup_terminal()` pattern from `wizard.rs` |
| Version number displayed prominently | Users must immediately confirm they have the right version; especially important after `npm install -g` | LOW | `env!("CARGO_PKG_VERSION")` macro — already done in `welcome_content()`; move to TUI Paragraph |
| One-liner product description | First-time users need "what is this?" answered before they hit a subcommand wall | LOW | Currently absent from welcome screen; add 1-2 lines below title: "AI agent fleet dispatcher via tmux — route tasks, track completion, monitor agents." |
| Clear CTA: what to do next | Users must see the single next action without reading docs; BMAD ends every onboarding page with "now run X" | LOW | "Press [Enter] to set up your squad" (no config) or "Your squad is configured — run squad-station ui" (config exists) |
| Keyboard navigation: Enter / q / Esc | Ratatui TUI convention; every TUI tool in the ecosystem requires key-driven navigation | LOW | `KeyCode::Enter` advances; `KeyCode::Char('q')` and `KeyCode::Esc` exit — same pattern as `wizard.rs` and `ui.rs` |
| Conditional routing: no `squad.yml` vs `squad.yml` exists | First-run should route to setup; returning users should see reference guide, not setup prompt — prevents re-onboarding frustration | MEDIUM | Check `Path::new("squad.yml").exists()` at startup; two distinct render modes |
| TTY guard — fall back to static output in non-interactive contexts | CI pipelines, pipes, and scripts must not hang on TUI input; npm install in CI must not fail due to binary launching TUI | LOW | `std::io::stdout().is_terminal()` — already in `init.rs`; apply same guard to `print_welcome()` dispatch |
| Graceful TUI exit with terminal restore | Exiting with `q` must restore terminal state; raw mode left active = broken shell for user | LOW | Same `restore_terminal()` / panic hook pattern from `wizard.rs` and `ui.rs` |
| Post-install auto-launch from curl installer | After `curl | sh`, user should immediately see the tool — eliminates "now what?" moment | LOW | Add TTY-guarded `squad-station` call at end of `install.sh`; `[ -t 1 ] && squad-station` |
| Post-install auto-launch from npm postinstall | Same expectation for `npm install -g squad-station` path; reduces friction to first wow moment | MEDIUM | Add TTY check in JS postinstall script: `if (process.stdout.isTTY) { execSync('squad-station') }`; must be guarded to avoid CI failures |
| Key hint bar at bottom of TUI | Users must know how to exit / advance without guessing; clig.dev and ratatui conventions both emphasize discoverability | LOW | Fixed-height bottom row: `[Enter] continue  [q] quit`; ratatui `Layout::Constraint::Length(1)` row |

### Differentiators (Competitive Advantage)

Features that set the first-run experience apart from comparable tools. Not strictly required, but high-value additions.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Quick guide page (multi-page welcome TUI) | New users see the mental model — "orchestrator sends tasks, workers complete them, hooks signal done" — in under 30 seconds before being routed to wizard; BMAD does 8 prompts, OpenCode just launches TUI | MEDIUM | Second TUI "page" after title screen; `[Enter]` or `[→]` to advance; 3-4 lines of plain-text explanation; state machine with two frames: `Title` and `QuickGuide` |
| State-aware welcome: reference guide for returning users | If `squad.yml` exists, show key commands instead of setup CTA; power users invoke bare `squad-station` to see command list, not to be re-onboarded | LOW | Second render path: title + subcommand reference table + "Run squad-station ui to monitor" hint; no Enter-to-wizard CTA |
| Skip animation on any keypress | If a logo animation is added, any key skips it immediately — respects power user time | LOW | Condition draw loop: `if frame_count < MAX_FRAMES && !any_key_pressed { advance_frame }` |
| Post-install "you're ready" message before TUI | After curl or npm install, print one plain-text line before launching TUI so the user knows install succeeded | LOW | Insert `println!("squad-station installed successfully!")` or equivalent in install scripts, then launch TUI |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Mandatory animated splash with minimum display time | Looks impressive, establishes brand | Punishes power users who run `squad-station` to check a command; adds latency to every bare invocation | If animation is added, make it instant-skip on any keypress; cap at 0.5s max |
| Auto-launch wizard without keypress confirmation | Zero friction | Violates clig.dev principle: actions with side effects must require explicit confirmation; a user who ran `squad-station` expecting info would instead trigger squad.yml creation | Always gate wizard entry behind explicit `[Enter]` press; show intent ("Press Enter to set up") before acting |
| Block postinstall until TUI interaction completes | Ensures onboarding | Breaks `npm install` in CI and scripted environments; users with `--ignore-scripts` or `npm ci` would get different behavior | TTY-gate all interactive behavior; non-interactive install always completes silently |
| Browser auto-open for documentation | Reduces friction to docs | Squad Station is local-only; no web service, no auth; browser launch is alarming in a security-conscious context | Print URL as plain text: "Docs: https://github.com/..." |
| Telemetry / "how did you hear about us?" prompt | Growth data | Breaks trust in a developer security tool; no telemetry infrastructure exists | Skip entirely |
| Persistent interactive TUI on every bare invocation for returning users | Consistent branding | Annoying for power users; `squad-station | grep send` type usage breaks with TUI in alternate screen | Interactive TUI only for first-run (no squad.yml); static text output for returning users |
| Full setup wizard within the welcome TUI | Single flow | The wizard already exists in `wizard.rs`; duplicating it in the welcome TUI creates two code paths to maintain | Welcome TUI hands off to existing wizard via CTA; no wizard logic in welcome |

---

## Feature Dependencies

```
[Post-install auto-launch (curl)]
    └──requires──> [TTY guard in install.sh: [ -t 1 ] check]
    └──requires──> [Binary installed to PATH]

[Post-install auto-launch (npm postinstall)]
    └──requires──> [TTY guard in JS: process.stdout.isTTY check]
    └──requires──> [Binary downloaded and executable]

[Interactive ratatui welcome TUI]
    └──requires──> [TTY guard (is_terminal()) — already in codebase]
    └──requires──> [Alternate screen setup/teardown — already in wizard.rs]
    └──fallback──> [Static print_welcome() for non-TTY]

[Conditional routing (first-run vs returning)]
    └──requires──> [squad.yml existence check — trivial Path::exists()]
    └──requires──> [Interactive ratatui welcome TUI]

[Quick guide page]
    └──requires──> [Interactive ratatui welcome TUI]
    └──enhances──> [Conditional routing — shown only on first-run path]
    └──hands-off-to──> [Existing wizard.rs (no duplication)]

[State-aware welcome for returning users]
    └──requires──> [squad.yml existence check]
    └──conflicts──> [Auto-wizard launch without keypress]
```

### Dependency Notes

- **Auto-launch requires TTY detection in install scripts:** npm postinstall runs in non-TTY context during `npm install` in CI. The TTY check in the JS postinstall must use `process.stdout.isTTY` (Node.js built-in); no additional npm dependency needed. Failure to guard this blocks CI pipelines.
- **Welcome TUI reuses existing infrastructure:** `setup_terminal()`, `restore_terminal()`, and the crossterm raw-mode panic hook are already implemented in `wizard.rs` and `ui.rs`. The welcome TUI is a new event loop using the same building blocks — not new infrastructure.
- **Conditional routing is a 3-line check:** `Path::new("squad.yml").exists()` decides which render path. No DB read required. No async. Runs at the top of the welcome handler.
- **Quick guide page does NOT replace wizard:** The quick guide ends with a CTA that calls `commands::wizard::run()` — the same call already in `commands::init::run()`. No wizard logic lives in the welcome module.
- **Static fallback preserves existing behavior for CI/scripts:** All 211 existing tests pass without change because the TTY guard routes non-TTY contexts to `print_welcome()` — the same behavior as today.

---

## MVP Definition

### Launch With (v1.7)

- [ ] **Interactive ratatui welcome TUI** — replaces static `print_welcome()` when stdout is a TTY; large ASCII title, version, one-liner product description; rendered in alternate screen; exits on `q`/`Esc`, advances to wizard entry on `Enter` (no squad.yml) or exits on `Enter` (squad.yml exists)
- [ ] **TTY guard on welcome dispatch** — if stdout is not a TTY (CI, pipe, redirect), fall back to existing static `print_welcome()` text; zero behavior change for automated contexts; all existing tests continue passing
- [ ] **Conditional routing based on squad.yml presence** — no config: show "Press [Enter] to set up your squad" CTA that calls existing `wizard::run()`; config exists: show subcommand quick reference, no wizard prompt
- [ ] **Key hint bar at bottom of TUI** — always-visible `[Enter] continue  [q] quit` or similar; prevents users getting stuck on welcome screen
- [ ] **Post-install auto-launch from curl installer** — append `[ -t 1 ] && squad-station` to `install.sh` so interactive installs drop directly into the welcome TUI
- [ ] **Post-install auto-launch from npm postinstall** — add `if (process.stdout.isTTY) { execSync('squad-station') }` (or equivalent) to npm postinstall JS

### Add After Validation (v1.7.x)

- [ ] **Quick guide page** — second TUI page showing 3-4 lines of "how Squad Station works" explanation, advancing to wizard CTA; trigger: user research or issue reports showing confusion after install
- [ ] **Post-install "installed successfully" message** — print plain-text confirmation before launching TUI in both install paths; helps users differentiate install output from TUI

### Future Consideration (v2+)

- [ ] **Fleet status on welcome for returning users** — show agent count + status summary from DB when `squad.yml` exists and DB is accessible; requires DB read at startup; high complexity vs value for a bare invocation
- [ ] **Animated title (skippable)** — ratatui-splash-screen integration for a brief logo animation on first-run; purely aesthetic; defer until core experience is stable and differentiated value is clear

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Interactive ratatui welcome TUI | HIGH — visual identity + first impression | LOW — same ratatui patterns as wizard.rs | P1 |
| TTY guard on welcome dispatch | HIGH — CI safety; preserves existing test suite | LOW — one `is_terminal()` call, already in codebase | P1 |
| Conditional routing (no config vs config exists) | HIGH — prevents re-onboarding returning users | LOW — `Path::new("squad.yml").exists()` check | P1 |
| Key hint bar at bottom of TUI | MEDIUM — discoverability; prevents stuck users | LOW — ratatui `Paragraph` in fixed bottom row | P1 |
| Post-install auto-launch (curl) | MEDIUM — eliminates "now what?" after install | LOW — two lines in install.sh | P1 |
| Post-install auto-launch (npm) | MEDIUM — same value for npm install path | MEDIUM — JS TTY check + execSync in postinstall | P1 |
| Quick guide page (multi-page TUI) | MEDIUM — reduces "what is this?" confusion | MEDIUM — new TUI state + render path | P2 |
| Post-install "installed successfully" text | LOW-MEDIUM — clarity | LOW — println in install scripts | P2 |
| Fleet status summary on welcome (returning users) | MEDIUM — power user convenience | HIGH — DB read at startup | P3 |
| Animated title (skippable) | LOW — aesthetics | LOW-MEDIUM — ratatui-splash-screen | P3 |

---

## Competitor / Reference Tool Analysis

| Feature | BMAD-METHOD (`npx install`) | OpenCode (bare invocation) | gh auth login | Squad Station v1.7 Target |
|---------|--------------------------|---------------------------|--------------|--------------------------|
| First-run detection | Detects `_bmad/` directory; offers quick-update vs full reinstall | No state detection; always launches TUI | Detects existing auth token | `squad.yml` existence check; two render paths |
| Welcome branding | Logo + greeting from YAML file at top of prompt sequence | Not documented | Minimal | Large ASCII art in ratatui alternate screen |
| Post-install auto-launch | `npx bmad-method install` is the install command; no separate auto-launch | Binary always launches TUI on bare call | N/A (auth separate from install) | TTY-guarded call at end of `install.sh` + npm postinstall |
| Returning user handling | 3-choice menu: quick-update / compile-agents / update | Always launches same TUI | No distinction | Show reference guide (not setup CTA) when squad.yml exists |
| Keyboard navigation | Sequential prompts via @clack/prompts; Enter to advance | Standard TUI navigation | Interactive prompt; Enter to select | Enter to advance/confirm; q/Esc to exit |
| Cancel/exit without setup | Cancel option at any @clack/prompts step | q to quit TUI | Ctrl+C | q/Esc from welcome TUI; no side effects |
| TTY guard | @clack/prompts detects non-interactive context automatically | Not documented | Falls back to non-interactive flags | Explicit `is_terminal()` guard routing to static `print_welcome()` |
| Key hint visibility | Hints embedded in @clack/prompts widgets | Visible in TUI status bar | Not applicable | Fixed bottom bar on every TUI page |

---

## Sources

- [BMAD-METHOD interactive installation — DeepWiki](https://deepwiki.com/bmadcode/BMAD-METHOD/2.1-cli-installation) — HIGH confidence; live documentation scraped
- [Command Line Interface Guidelines — clig.dev](https://clig.dev/) — HIGH confidence; canonical reference for CLI UX patterns
- [ratatui alternate screen concepts](https://ratatui.rs/concepts/backends/alternate-screen/) — HIGH confidence; official ratatui documentation
- [ratatui-splash-screen — orhun/ratatui-splash-screen](https://github.com/orhun/ratatui-splash-screen) — MEDIUM confidence; web search verified; available as crate
- [is-interactive npm package documentation](https://www.npmjs.com/package/is-interactive) — MEDIUM confidence; TTY detection pattern reference
- [UX patterns for CLI tools — lucasfcosta.com](https://lucasfcosta.com/2022/06/01/ux-patterns-cli-tools.html) — MEDIUM confidence (2022; patterns stable)
- Existing codebase analysis (`welcome.rs`, `wizard.rs`, `ui.rs`, `init.rs`, `install.sh`, `main.rs`) — HIGH confidence; direct code review

---

*Feature research for: Squad Station v1.7 First-Run Onboarding TUI + Post-Install Auto-Launch*
*Researched: 2026-03-17*
