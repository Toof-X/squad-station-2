---
phase: 21-quick-guide-and-install-flow
verified: 2026-03-18T03:00:00Z
status: passed
score: 13/13 must-haves verified
re_verification: false
---

# Phase 21: Quick Guide and Install Flow — Verification Report

**Phase Goal:** Add a quick guide page to the welcome TUI and wire up TTY-guarded auto-launch in both install paths so users see the TUI immediately after a successful install.
**Verified:** 2026-03-18T03:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Tab key on title page navigates to guide page | VERIFIED | `routing_action()` has `KeyCode::Tab \| KeyCode::Right => Some(WelcomeAction::ShowGuide)` at line 58 |
| 2 | Right arrow on title page navigates to guide page | VERIFIED | Same arm as above covers `KeyCode::Right` |
| 3 | Tab key on guide page returns to title page | VERIFIED | `guide_routing_action()` has `KeyCode::Tab \| KeyCode::Left => Some(WelcomeAction::ShowTitle)` at line 67 |
| 4 | Left arrow on guide page returns to title page | VERIFIED | Same arm as above covers `KeyCode::Left` |
| 5 | Guide page displays concept summary and 3 numbered steps | VERIFIED | `guide_content()` builds multi-line string: orchestrator line, 3 numbered steps, footer; rendered via `draw_guide()` |
| 6 | Guide page hint bar shows Tab/arrow-back and Q: Quit | VERIFIED | `guide_hint_bar_text()` returns `"○ ●  Tab/←: Back  Q: Quit"` (Unicode open/filled circles + left arrow); rendered in `draw_guide()` chunk 3 |
| 7 | Title page hint bar includes Tab: Guide | VERIFIED | `hint_bar_text()` returns format string containing `Tab: Guide` in both has_config branches (lines 79-82) |
| 8 | Countdown resets to 5s when entering guide page | VERIFIED | `ShowGuide` arm in `run_welcome_tui()` sets `deadline = Instant::now() + Duration::from_secs(5)` (line 265) |
| 9 | Guide page has no BigText title | VERIFIED | `draw_guide()` uses only `Paragraph` widgets; no `BigText` builder call anywhere in the function |
| 10 | npm install auto-launches squad-station in interactive terminals after install | VERIFIED | `if (process.stdout.isTTY) { spawnSync(destPath, [], { stdio: 'inherit' }); }` at lines 44-46 of run.js, inside `install()` after all console.log output |
| 11 | curl installer auto-launches squad-station in interactive terminals after install | VERIFIED | `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi` at lines 70-72 of install.sh, after echo and FALLBACK check |
| 12 | npm install completes silently in non-interactive environments (no launch attempt) | VERIFIED | TTY guard `process.stdout.isTTY` is the sole condition — no launch when isTTY is false/undefined (CI, pipes, non-TTY) |
| 13 | curl installer completes silently in non-interactive environments (no launch attempt) | VERIFIED | `[ -t 1 ]` tests stdout file descriptor — does not fire in pipes, CI, or redirected output |

**Score:** 13/13 truths verified

---

## Required Artifacts

### Plan 01 Artifacts (WELCOME-05)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/welcome.rs` | WelcomePage enum, ShowGuide/ShowTitle variants, draw_guide(), guide_routing_action(), guide_hint_bar_text(), guide_content() | VERIFIED | All 6 expected items present; file compiles cleanly (`cargo check` exits 0) |

**Substantive check:** File is 582 lines with full implementations — no stubs or placeholder returns.

**Wiring check:** `welcome.rs` is the only file in plan 01. All internal functions are called from within the same file's `run_welcome_tui()` event loop. `draw_guide()` is dispatched from the `terminal.draw()` match on `WelcomePage::Guide`. Both routing functions are dispatched from the key-event match on page state.

### Plan 02 Artifacts (INSTALL-01, INSTALL-02, INSTALL-03)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `npm-package/bin/run.js` | TTY-guarded spawnSync auto-launch inside install(); installBinary() returns destPath | VERIFIED | `process.stdout.isTTY` guard at line 44; `spawnSync(destPath, [], { stdio: 'inherit' })` at line 45; `return destPath` appears at lines 87 and 117 (both exit paths); `var destPath = installBinary()` at line 31 |
| `install.sh` | TTY-guarded exec auto-launch at end of script | VERIFIED | `if [ -t 1 ]; then` at line 70; `exec "${INSTALL_DIR}/squad-station"` at line 71; block appears after final echo and FALLBACK if-block |

---

## Key Link Verification

### Plan 01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `routing_action()` | `WelcomeAction::ShowGuide` | `KeyCode::Tab \| KeyCode::Right` match arm | WIRED | Line 58: `KeyCode::Tab \| KeyCode::Right => Some(WelcomeAction::ShowGuide)` |
| `guide_routing_action()` | `WelcomeAction::ShowTitle` | `KeyCode::Tab \| KeyCode::Left` match arm | WIRED | Line 67: `KeyCode::Tab \| KeyCode::Left => Some(WelcomeAction::ShowTitle)` |
| `run_welcome_tui()` | `draw_guide(f)` | `match page { WelcomePage::Guide => ... }` | WIRED | Lines 249-252: `terminal.draw(\|f\| match page { WelcomePage::Title => draw_welcome(...), WelcomePage::Guide => draw_guide(f) })` |

### Plan 02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `npm-package/bin/run.js install()` | `spawnSync(destPath)` | `if (process.stdout.isTTY)` guard | WIRED | Line 44-46: guard present; `destPath` captured from `installBinary()` return at line 31 |
| `install.sh` | `exec "${INSTALL_DIR}/squad-station"` | `if [ -t 1 ]` guard | WIRED | Lines 70-72: guard and exec present; `INSTALL_DIR` is set earlier in script at line 37/40 |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| WELCOME-05 | 21-01-PLAN.md | TUI includes quick guide page explaining Squad Station concept and basic workflow | SATISFIED | `WelcomePage::Guide` state machine, `draw_guide()`, `guide_content()` with concept summary + 3 steps, bidirectional Tab/arrow navigation |
| INSTALL-01 | 21-02-PLAN.md | npm postinstall checks `process.stdout.isTTY` and auto-launches `squad-station` if interactive | SATISFIED | `if (process.stdout.isTTY)` at run.js line 44 with `spawnSync(destPath, ...)` |
| INSTALL-02 | 21-02-PLAN.md | curl \| sh installer checks `[ -t 1 ]` and auto-launches `squad-station` if interactive | SATISFIED | `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi` at install.sh lines 70-72 |
| INSTALL-03 | 21-02-PLAN.md | Both install scripts degrade silently in non-interactive environments | SATISFIED | No CI env var checks added — TTY check alone is the gate; both scripts exit normally when TTY check is false |

**Orphaned requirements check:** REQUIREMENTS.md maps WELCOME-05, INSTALL-01, INSTALL-02, INSTALL-03 to Phase 21. All four are claimed in plan frontmatter. No orphans.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | None found |

Scanned `src/commands/welcome.rs`, `npm-package/bin/run.js`, and `install.sh` for TODO/FIXME/PLACEHOLDER, empty return bodies (`return null`, `return {}`, `return []`), and console.log-only implementations. No issues found.

---

## Test Suite Results

| Suite | Passed | Failed | Notes |
|-------|--------|--------|-------|
| `cargo test welcome` | 24 | 0 | All 11 new tests + 13 existing tests pass |
| `cargo test` (full suite) | 241 | 0 | No regressions |
| `cargo check` | — | — | Exits 0, no errors, no warnings |

New tests verified present and passing:
- `test_routing_action_tab_opens_guide`
- `test_routing_action_right_opens_guide`
- `test_routing_action_left_noop`
- `test_guide_routing_tab_returns_title`
- `test_guide_routing_left_returns_title`
- `test_guide_routing_quit`
- `test_guide_routing_esc_quit`
- `test_guide_routing_enter_noop`
- `test_guide_hint_bar_text`
- `test_guide_content`
- `test_hint_bar_text_includes_tab_guide`

---

## Commit Verification

| Hash | Description | Status |
|------|-------------|--------|
| `a1e155d` | feat(21-01): add WelcomePage enum, guide pure functions, extend routing | EXISTS |
| `ea37a21` | feat(21-01): add draw_guide() and wire WelcomePage state into event loop | EXISTS |
| `99b3c12` | feat(21-02): add TTY-guarded auto-launch to npm and curl install paths | EXISTS |

---

## Human Verification Required

### 1. Guide page visual layout

**Test:** Run `squad-station` in an interactive terminal. Press Tab to navigate to the guide page.
**Expected:** Guide page header "Quick Guide" is centered; concept summary line appears followed by 3 numbered steps; hint bar at bottom shows `○ ●  Tab/←: Back  Q: Quit`; no BigText pixel font title; no red color accent.
**Why human:** Ratatui rendering (alignment, layout constraints, terminal dimensions) cannot be verified by grep.

### 2. Title page hint bar dot indicator

**Test:** Launch `squad-station`. Observe the hint bar before pressing any key.
**Expected:** Hint bar begins with `● ○` (filled circle, open circle) indicating page 1 of 2, followed by `Enter: Set up  Tab: Guide  Q: Quit  auto-exit 5s`.
**Why human:** Unicode rendering in actual terminal differs from string content checks.

### 3. npm auto-launch after install

**Test:** Run `npx squad-station install` in an interactive terminal with no existing binary.
**Expected:** After install completes and "Next steps" text is printed, the welcome TUI launches automatically; user sees the full TUI screen.
**Why human:** Requires actual binary download and TTY context — cannot simulate in static analysis.

### 4. curl auto-launch after install

**Test:** Run `curl -fsSL .../install.sh | sh` in an interactive terminal (where stdout is a TTY — note: pipe from curl means stdout of the script IS a terminal for the exec'd binary).
**Expected:** After install completes, `exec` replaces the shell with `squad-station`, and the welcome TUI appears.
**Why human:** Requires network access to release URL and actual TTY environment; `exec` behavior in pipe context needs real validation.

### 5. Non-interactive degradation

**Test:** Run `npx squad-station install 2>/dev/null | cat` (or in a CI environment).
**Expected:** Install completes normally; no TUI is launched; process exits with 0.
**Why human:** Requires an actual non-TTY context to confirm silent degradation.

---

## Summary

Phase 21 goal is fully achieved. The welcome TUI state machine correctly implements a two-page navigation flow (Title ↔ Guide) controlled by Tab and arrow keys, with a 5-second countdown reset on guide entry. Both install paths (npm and curl) include TTY-guarded auto-launch using the minimal correct mechanism (`process.stdout.isTTY` and `[ -t 1 ]` respectively). All four requirements (WELCOME-05, INSTALL-01, INSTALL-02, INSTALL-03) are satisfied with substantive, wired implementations backed by 24 passing unit tests and a full 241-test green suite.

---

_Verified: 2026-03-18T03:00:00Z_
_Verifier: Claude (gsd-verifier)_
