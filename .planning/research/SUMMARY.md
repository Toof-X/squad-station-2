# Project Research Summary

**Project:** Squad Station v1.7 — First-Run Onboarding TUI + Post-Install Auto-Launch
**Domain:** Rust CLI — interactive ratatui welcome screen additive to an existing stable binary
**Researched:** 2026-03-17
**Confidence:** HIGH

## Executive Summary

Squad Station v1.7 upgrades the bare-invocation path (`squad-station` with no arguments) from a static `println!`-based welcome screen to an interactive ratatui TUI, and wires both distribution paths (npm postinstall and curl installer) to surface the binary to the user immediately after install. This is an additive change to a well-structured existing codebase: the infrastructure — ratatui terminal management, TTY guards, the multi-page wizard TUI, and both distribution scripts — already exists and is validated. The v1.7 work is primarily about connecting existing pieces correctly and guarding the TTY boundary at every entry point.

The recommended approach is narrow in scope: modify only `welcome.rs` (new `run_welcome_tui()` ratatui event loop with AlternateScreen + panic hook), `main.rs` (swap `print_welcome()` call for `run_welcome().await?`), `install.sh` (TTY-guarded hint or exec at end), and `run.js` (TTY-guarded spawnSync or hint at end). The ratatui upgrade path is ratatui 0.29 + crossterm 0.28 + tui-big-text 0.7.x — all version-pinned with HIGH-confidence rationale derived from official release notes and the crossterm incompatibility advisory. No new architectural patterns are introduced; the welcome TUI reuses the exact AlternateScreen + panic hook + restore sequence already proven in `wizard.rs` and `ui.rs`.

The primary risk is the TTY boundary: CI environments, piped invocations, and install scripts must never enter crossterm raw mode. All six critical pitfalls identified in research are Phase 1 concerns, all preventable with the TTY guard pattern already established in `init.rs`. A secondary risk is an unresolved design conflict: STACK.md and FEATURES.md recommend TTY-guarded auto-launch from install scripts, but PITFALLS.md demonstrates that `curl | sh` stdin and npm postinstall stdio are not reliably TTY contexts — the safe design is print-hint-only from install scripts, letting the user invoke `squad-station` manually. This conflict must be resolved as an explicit roadmap decision before Phase 1 implementation begins.

## Key Findings

### Recommended Stack

The existing dependency tree needs two version upgrades and one new dependency. Ratatui must move from 0.26 to 0.29 to gain `frame.area()` stabilization and enable tui-big-text 0.7.x compatibility. Crossterm must move from 0.27 to 0.28 because ratatui 0.28+ re-exports crossterm internally — mixing versions causes type incompatibilities that prevent pattern-matching on crossterm events (confirmed in the ratatui/ratatui GitHub issue #1298 advisory). tui-big-text 0.7.x is the only net-new dependency, providing a ratatui-native pixel-font title widget. All three changes are a single Cargo.toml edit; application code changes are limited to the `frame.size()` → `frame.area()` rename in `ui.rs` and `wizard.rs`. Ratatui 0.30 is explicitly out of scope: it introduces a workspace split and removes `frame.size()` outright, requiring broader migration with no v1.7 feature benefit.

**Core technologies:**
- **ratatui 0.29**: Interactive TUI framework — latest stable below the 0.30 workspace split; picks up `frame.area()` stabilization; minimum required by tui-big-text 0.7.x
- **crossterm 0.28**: Terminal backend — must match ratatui's internal crossterm version; mixing 0.27 and 0.28 causes two incompatible crossterm type trees
- **tui-big-text 0.7.x**: Pixel-font title widget — renders "SQUAD STATION" as block letters via `font8x8`; 0.8.x requires ratatui 0.30 and is excluded
- **std::io::IsTerminal** (stdlib, Rust 1.70+): TTY detection — preferred over `atty` (unmaintained) and `crossterm::tty::IsTty` (adds unnecessary coupling); project MSRV is already 1.86
- **Node.js `process.stdout.isTTY`** (stdlib): TTY detection in npm scripts — no additional npm dependency needed
- **POSIX `[ -t 1 ]`**: TTY detection in shell installers — universally available on macOS and Linux; guards install.sh binary launch

### Expected Features

The v1.7 feature set is well-defined and bounded. All P1 features are low-to-medium complexity with clear precedents in the existing codebase. The research provides an important anti-feature finding: persistent interactive TUI on every bare invocation for returning users (squad.yml exists) is wrong — returning users should see a static reference guide, not be re-onboarded. The feature dependency chain is simple: TTY guard unlocks TUI; TUI unlocks conditional routing; conditional routing unlocks CTA-to-wizard handoff.

**Must have (table stakes):**
- **Interactive ratatui welcome TUI** — replaces static `print_welcome()` when stdout is a TTY; large ASCII title + version; alternate screen; Enter/q/Esc navigation; key hint bar at bottom
- **TTY guard on welcome dispatch** — non-TTY (CI, pipes, scripts) falls back to existing `print_welcome()`; preserves all 211 existing passing tests with zero changes
- **Conditional routing on squad.yml presence** — no config: first-run CTA ("Press Enter to set up your first squad") → hands off to existing `wizard::run()`; config exists: returning-user reference guide, no wizard prompt
- **Key hint bar** — always-visible `[Enter] continue  [q] quit` at bottom of every TUI page; prevents users getting stuck
- **Post-install messaging** — print next-steps hint from both install paths (see pitfall conflict note; auto-launch is optional if design decision resolves in its favor)

**Should have (differentiators):**
- **Quick guide page** — second TUI page (state machine: Title → QuickGuide) showing 3-4 line mental model; deferred to v1.7.x pending Phase 1 validation
- **State-aware returning-user view** — subcommand reference table + "Run squad-station ui" hint when squad.yml exists; distinct from first-run CTA; can iterate without touching Phase 1 logic

**Defer (v2+):**
- Fleet status summary on welcome (requires DB read at startup; high complexity vs value for a bare invocation)
- Animated title / ratatui-splash-screen integration (purely aesthetic; defer until core experience is stable)

**Critical conflict — post-install auto-launch:**
STACK.md and FEATURES.md list TTY-guarded auto-launch (exec in install.sh, spawnSync in run.js) as P1. PITFALLS.md Pitfalls 2 and 3 override this recommendation: `curl | sh` sets bash stdin to the curl pipe making `[ -t 1 ]` unreliable in orchestrated environments; npm postinstall runs with stdio as pipe and `process.stdout.isTTY` is undefined. The safe design is print-hint-only from install scripts; auto-launch is triggered only when the user invokes `squad-station` manually. The roadmapper must resolve this conflict explicitly as a named decision before Phase 1 scope is locked.

### Architecture Approach

The architecture is minimal-change: four files modified, zero new files, zero new modules. `welcome.rs` gains a `run_welcome_tui()` private function implementing the ratatui event loop using the AlternateScreen + panic hook + restore pattern copy-verified from `ui.rs` lines 284–338. The welcome TUI must call `restore_terminal()` and exit AlternateScreen before delegating to `commands::init::run()` on Enter — nested alternate buffers corrupt terminal state on exit (Pitfall 3 in ARCHITECTURE.md). The build order is strictly: (1) ratatui skeleton with terminal lifecycle, (2) squad.yml detection and branched display, (3) Enter-to-init wiring with AlternateScreen exit first, (4) TTY guard public API, (5) main.rs call-site swap, (6-7) install scripts in parallel with Rust work.

**Major components:**
1. **`src/commands/welcome.rs` (MODIFY)** — add `run_welcome()` public async fn with TTY guard; add private `run_welcome_tui()` ratatui event loop; keep `print_welcome()` as non-TTY fallback; `welcome_content()` unchanged for tests
2. **`src/main.rs` (MODIFY)** — swap `commands::welcome::print_welcome()` for `commands::welcome::run_welcome().await?` in the `None` arm; one-line change
3. **`npm-package/bin/run.js` (MODIFY)** — append TTY-guarded next-steps hint (or spawnSync per design decision) at end of `install()`
4. **`install.sh` (MODIFY)** — append TTY-guarded next-steps echo (or exec per design decision) before final exit
5. **`src/commands/wizard.rs`, `init.rs`, `cli.rs` (NO CHANGE)** — welcome TUI delegates to `init::run()` on Enter; no logic is duplicated in welcome.rs

### Critical Pitfalls

1. **Missing TTY guard on welcome TUI** — `enable_raw_mode()` called with stdout as a pipe returns `ENOTTY` ("Inappropriate ioctl for device"); apply `std::io::stdout().is_terminal()` check as the first gate in the bare-invocation path; non-TTY falls back to `print_welcome()`; this guard is the prerequisite for all TUI code

2. **npm postinstall auto-launch hangs in CI** — npm postinstall runs with `stdio: pipe`; `process.stdout.isTTY` is `undefined`; even a TTY-guarded `spawnSync` is problematic in container CI; design decision required: postinstall should print a hint and never launch the binary

3. **curl | sh auto-launch breaks when stdin is the pipe** — bash stdin is connected to curl output, not the terminal; `[ -t 1 ]` may return false in devcontainer and Docker build contexts; install.sh should end with an echo hint, not a binary launch

4. **Terminal not restored on panic or early return** — welcome TUI must install the same panic hook (`take_hook` / `set_hook` / `disable_raw_mode` / `LeaveAlternateScreen`) as `wizard.rs` and `ui.rs`; calling `restore_terminal()` on every exit path (normal, Esc/q, `?` error propagation) is mandatory; consider extracting a shared `tui_guard` module to eliminate three diverging copies of the pattern

5. **Alternate screen swallows welcome content from scrollback** — `LeaveAlternateScreen` removes all TUI content; user sees nothing after dismissing; evaluate using main-buffer raw-mode-only pattern for the welcome screen specifically (content persists in scrollback); this architectural decision must be made before writing the event loop to avoid a rendering rewrite

6. **Cargo test suite broken by TUI in tests** — `cargo test` on macOS in iTerm2 runs with stdout connected to a TTY; if any test calls `enable_raw_mode()` the terminal enters raw mode and test output is invisible; the `welcome_content()` pattern (pure-string function, tested separately from render) is already the correct model; never call TUI render functions from tests

## Implications for Roadmap

Based on combined research, two phases are sufficient for v1.7. All P1 features and all critical pitfalls map to Phase 1. Phase 2 covers polish and deferred differentiators. The phase ordering is driven by the safety-before-features principle: TTY contract and terminal-restore contract must be established before any TUI code merges.

### Phase 1: TTY-Safe Welcome TUI Core

**Rationale:** All six critical pitfalls must be addressed before any TUI code is wired into the bare-invocation path. The TTY guard, terminal-restore panic hook, test-isolation contract, minimum terminal size fallback, and auto-exit timeout are prerequisites — not afterthoughts. The ratatui/crossterm/tui-big-text version upgrade must happen at the start of Phase 1 because all subsequent TUI code targets the new API. The post-install auto-launch design decision (hint vs. exec) must be resolved before either install script is modified. The alternate-screen-vs-main-buffer decision must be resolved before the event loop is written.

**Delivers:**
- ratatui 0.29 + crossterm 0.28 + tui-big-text 0.7.x Cargo.toml upgrade with `frame.size()` → `frame.area()` migration in `ui.rs` and `wizard.rs`
- `run_welcome_tui()` ratatui event loop: AlternateScreen (or main-buffer per design decision), panic hook, restore on all exit paths
- TTY guard in `run_welcome()` routing to static `print_welcome()` fallback
- squad.yml detection: first-run CTA vs. returning-user reference guide
- Enter key → `restore_terminal()` → `commands::init::run()` handoff (no nested AlternateScreen)
- Key hint bar at bottom of every TUI page
- Minimum terminal size check (< 10 rows or < 40 cols → fall back to `print_welcome()`)
- Auto-exit timeout in event loop (no indefinite blocking on keypress)
- main.rs call-site swap: `print_welcome()` → `run_welcome().await?`
- Post-install messaging in install.sh and run.js (hint or exec per design decision)

**Addresses:** All P1 features from FEATURES.md
**Avoids:** Pitfalls 1, 2, 3, 4, 5, 6, 7, 8, 10 from PITFALLS.md (all critical and all blocking moderate pitfalls)
**Stack used:** ratatui 0.29, crossterm 0.28, tui-big-text 0.7.x, std::io::IsTerminal, existing wizard.rs/ui.rs patterns
**Design decisions to resolve before implementation:**
- Alternate screen vs. main-buffer-raw-mode for welcome TUI (Pitfall 8)
- Auto-launch vs. print-hint-only for install scripts (Pitfalls 2 and 3)

### Phase 2: Quick Guide and UX Polish

**Rationale:** Deferred until Phase 1 is validated in real installs. The quick guide page adds a second TUI state machine frame and is fully independent of Phase 1 event loop logic. State-aware returning-user refinements can be iterated without touching the Phase 1 guard contract. Any UX issues surfaced by Phase 1 dogfooding (scrollback visibility, key navigation feel, title sizing) are addressed here.

**Delivers:**
- Quick guide page (second TUI state: Title → QuickGuide state machine; 3-4 lines of mental model; wizard CTA at end)
- Post-install "installed successfully" plain-text confirmation printed before TUI launches in both install paths
- UX refinements from Phase 1 dogfooding (layout, typography, key navigation feel)
- Returning-user reference guide refinements (richer subcommand table, hints)

**Uses:** ratatui multi-page state machine (same two-page pattern as wizard.rs)
**Implements:** P2 differentiator features from FEATURES.md

### Phase Ordering Rationale

- **Safety before features:** The TTY guard and terminal-restore contract must be committed before any TUI event loop code merges. A TUI without the guard ships a binary that hangs CI pipelines or corrupts user terminals.
- **Version upgrade at the start of Phase 1:** The ratatui 0.29 bump must precede all TUI code because tui-big-text 0.7.x requires it. Upgrading mid-phase forces rewriting already-merged code.
- **Design decisions before implementation:** The alternate-screen and auto-launch decisions are architectural — making them after the event loop is written forces a rewrite. Lock them as the first deliverable of Phase 1.
- **Install scripts last in Phase 1:** The install script changes are two-line additions that depend on stable binary behavior. They belong at the end of Phase 1, not the beginning, so they can be validated against the final binary behavior.
- **Phase 2 after validation:** The quick guide page adds complexity to the state machine. It should only be built after Phase 1 is confirmed to work correctly in real install flows.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1 (alternate screen vs. main buffer):** ratatui raw-mode-only without EnterAlternateScreen is less common in the ecosystem; a concrete code prototype is needed to verify the pattern works cleanly with ratatui 0.29 before committing to it as the architecture
- **Phase 1 (tui-big-text 0.7.x / ratatui 0.29 compatibility):** The 0.7.x compatibility with ratatui 0.29 is inferred from docs.rs (0.8.x requires ratatui ^0.30.0); validate with `cargo add tui-big-text@0.7` immediately after the ratatui upgrade before building the title widget

Phases with standard patterns (skip research):
- **Phase 1 (TTY guard):** `init.rs` line 103 is the exact pattern; copy-verify only
- **Phase 1 (AlternateScreen + panic hook):** `ui.rs` lines 284–338 are the authoritative in-codebase reference; copy-verify only
- **Phase 1 (squad.yml existence check):** `Path::new("squad.yml").exists()` — trivial; no research needed
- **Phase 1 (ratatui/crossterm version upgrade):** All breaking changes documented in official BREAKING-CHANGES.md; `frame.size()` → `frame.area()` is the only rename
- **Phase 2 (multi-page state machine):** wizard.rs multi-page pattern is the reference; no new research needed

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All version decisions verified against official ratatui release notes, docs.rs, and the crossterm incompatibility advisory; tui-big-text 0.7.x compatibility is MEDIUM (inferred, needs `cargo add` validation) |
| Features | HIGH | Primary sources: direct codebase analysis + clig.dev canonical CLI UX guidance + BMAD-METHOD live documentation; all P1 features verified against existing code patterns in the codebase |
| Architecture | HIGH | All findings derived from direct source inspection of the live codebase; no inferred patterns; build order validated against actual file dependencies; line-number references to ui.rs provided |
| Pitfalls | HIGH | All six critical pitfalls verified against crossterm docs, npm lifecycle docs, POSIX shell specification, and ratatui BREAKING-CHANGES.md; recovery strategies provided |

**Overall confidence:** HIGH

### Gaps to Address

- **Auto-launch design decision (unresolved conflict):** STACK.md/FEATURES.md recommend TTY-guarded auto-launch; PITFALLS.md recommends print-hint-only. Recommendation: default to print-hint-only for safety; document the decision explicitly in the roadmap phase scope.
- **Alternate screen vs. main buffer (unresolved):** The welcome screen UX (content visible in scrollback vs. full-screen immersive) requires an explicit decision before Phase 1 implementation begins. Build a minimal prototype of the main-buffer-raw-mode pattern to validate it works with ratatui 0.29.
- **tui-big-text 0.7.x compatibility:** Inferred, not confirmed. Validate immediately as step 0 of Phase 1.
- **frame.size() scope in ui.rs and wizard.rs:** The exact line count of the `frame.size()` → `frame.area()` rename across both files was not audited in research. Confirm scope before estimating Phase 1 effort; may affect whether wizard.rs needs review.

## Sources

### Primary (HIGH confidence)
- `src/commands/welcome.rs`, `src/commands/wizard.rs`, `src/commands/ui.rs`, `src/commands/init.rs`, `src/main.rs`, `src/cli.rs` — direct codebase inspection; all pattern references
- `npm-package/bin/run.js`, `install.sh` — direct codebase inspection; install flow verification
- [ratatui v0.28.0 highlights](https://ratatui.rs/highlights/v028/) — crossterm 0.28 requirement; frame.area() rename confirmed
- [ratatui v0.29.0 highlights](https://ratatui.rs/highlights/v029/) — feature set verification
- [ratatui v0.30.0 highlights](https://ratatui.rs/highlights/v030/) — workspace split confirmed; frame.size() removed (not just deprecated)
- [Ratatui / Crossterm Version incompatibility advisory](https://github.com/ratatui/ratatui/issues/1298) — semver conflict between crossterm 0.27 and 0.28 confirmed
- [ratatui panic hooks recipe](https://ratatui.rs/recipes/apps/panic-hooks/) — panic hook pattern verified
- [ratatui alternate screen concept](https://ratatui.rs/concepts/backends/alternate-screen/) — alternate screen architecture
- [ratatui terminal and event handler recipe](https://ratatui.rs/recipes/apps/terminal-and-event-handler/) — event loop pattern
- [std::io::IsTerminal](https://doc.rust-lang.org/std/io/trait.IsTerminal.html) — stable since Rust 1.70 confirmed
- [Node.js TTY module docs](https://nodejs.org/api/tty.html) — process.stdout.isTTY documented
- [Command Line Interface Guidelines — clig.dev](https://clig.dev/) — canonical CLI UX patterns; anti-feature rationale

### Secondary (MEDIUM confidence)
- [tui-big-text docs.rs](https://docs.rs/tui-big-text/latest/tui_big_text/) — version 0.8.2 requires ratatui ^0.30.0; 0.7.x compatibility with 0.29 inferred
- [BMAD-METHOD interactive installation — DeepWiki](https://deepwiki.com/bmadcode/BMAD-METHOD/2.1-cli-installation) — competitor first-run onboarding pattern analysis
- [Publishing binaries on npm — Sentry Engineering](https://sentry.engineering/blog/publishing-binaries-on-npm) — spawnSync with stdio:inherit pattern
- [npm postinstall non-TTY issue #16608](https://github.com/npm/npm/issues/16608) — postinstall stdio behavior confirmed as pipe
- [curl | sh pitfalls overview](https://www.arp242.net/curl-to-sh.html) — stdin-as-pipe behavior documented
- [crossterm enable_raw_mode ENOTTY](https://docs.rs/crossterm/latest/crossterm/terminal/fn.enable_raw_mode.html) — non-TTY error behavior
- [ratatui BREAKING-CHANGES.md](https://github.com/ratatui/ratatui/blob/main/BREAKING-CHANGES.md) — version-by-version API changes

### Tertiary (LOW confidence)
- [ratatui-splash-screen — orhun/ratatui-splash-screen](https://github.com/orhun/ratatui-splash-screen) — animated title option for v2+; web search verified; not evaluated for v1.7
- [UX patterns for CLI tools — lucasfcosta.com](https://lucasfcosta.com/2022/06/01/ux-patterns-cli-tools.html) — general CLI UX guidance (2022; patterns assumed stable)

---
*Research completed: 2026-03-17*
*Ready for roadmap: yes*
