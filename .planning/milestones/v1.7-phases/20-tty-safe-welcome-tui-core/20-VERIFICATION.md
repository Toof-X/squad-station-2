---
phase: 20-tty-safe-welcome-tui-core
verified: 2026-03-17T15:00:00Z
status: human_needed
score: 8/9 must-haves verified (automated); 1 truth requires human confirmation
re_verification: false
human_verification:
  - test: "Run squad-station with no args in a TTY"
    expected: "AlternateScreen opens with pixel-font SQUAD-STATION title in red (BigText HalfHeight), version string, tagline, 11-entry commands table, and hint bar showing auto-exit countdown ticking down from 5s"
    why_human: "BigText pixel-font rendering and AlternateScreen visual output cannot be confirmed programmatically — only a real terminal can show whether HalfHeight BigText actually renders the title correctly"
  - test: "Run squad-station | cat"
    expected: "Static welcome text is printed (ASCII art + version + subcommands), no raw mode, no TUI artifacts"
    why_human: "Non-TTY path confirmed in code; visual output correctness and absence of terminal artifacts requires human eyes"
  - test: "Wait for auto-exit countdown in TUI"
    expected: "Countdown ticks from 5s to 1s in the hint bar, then TUI exits cleanly to shell prompt"
    why_human: "Real-time countdown rendering and clean exit behavior can only be confirmed in a live terminal session"
---

# Phase 20: TTY-Safe Welcome TUI Core — Verification Report

**Phase Goal:** Implement a TTY-safe welcome TUI with pixel-font title, interactive commands table, countdown auto-exit, and conditional routing to init wizard or dashboard.
**Verified:** 2026-03-17T15:00:00Z
**Status:** HUMAN_NEEDED (all automated checks pass; visual terminal behavior awaits human confirmation)
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `squad-station` with no args in a TTY opens an AlternateScreen TUI with pixel-font SQUAD-STATION title | ? HUMAN NEEDED | `run_welcome_tui()` exists with `setup_terminal()` (EnterAlternateScreen), `BigText::builder().pixel_size(PixelSize::HalfHeight)`, and `draw_welcome()` wired in event loop — visual output must be confirmed in terminal |
| 2 | The TUI shows version string, tagline, commands table, and hint bar with countdown | ✓ VERIFIED | `draw_welcome()` renders all 4 elements: `env!("CARGO_PKG_VERSION")` in chunk[1], `"Multi-agent orchestration for AI coding"` in chunk[3], `commands_list()` in chunk[5], `hint_bar_text()` in chunk[6] |
| 3 | The TUI auto-exits after 5 seconds if no key is pressed | ✓ VERIFIED | Event loop uses `deadline = Instant::now() + Duration::from_secs(5)` with `remaining.is_zero()` break condition — timeout = silent exit |
| 4 | Running `squad-station` with stdout piped prints static welcome text without entering raw mode | ✓ VERIFIED | `src/main.rs` None arm checks `std::io::stdout().is_terminal()` — false branch calls `commands::welcome::print_welcome()` with no raw mode |
| 5 | `cargo test` passes after ratatui 0.30 upgrade (no `frame.size()` errors) | ✓ VERIFIED | `cargo test` exits 0; 230 total tests across all suites pass; no `frame.size()` present in `ui.rs` or `wizard.rs` (both use `frame.area()`) |
| 6 | When no `squad.yml` exists and user presses Enter, TUI closes and init wizard launches | ✓ VERIFIED | `main.rs` match arm `Some(WelcomeAction::LaunchInit)` calls `commands::init::run(PathBuf::from("squad.yml"), false).await?` |
| 7 | When `squad.yml` exists and user presses Enter, TUI closes and dashboard launches | ✓ VERIFIED | `main.rs` match arm `Some(WelcomeAction::LaunchDashboard)` calls `commands::ui::run().await?` |
| 8 | When user presses Q or Esc, the TUI closes without launching anything | ✓ VERIFIED | `routing_action()` maps `KeyCode::Char('q') \| KeyCode::Esc` to `WelcomeAction::Quit`; event loop sets `action = None` for Quit; `main.rs` `_` arm exits silently |
| 9 | When countdown reaches zero, TUI closes without launching anything | ✓ VERIFIED | Timeout breaks the loop with `action` remaining `None`; `main.rs` `_` arm exits silently |

**Score:** 8/9 verified automatically; 1 needs human confirmation (visual BigText pixel-font rendering)

---

## Required Artifacts

### Plan 20-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | ratatui 0.30, crossterm 0.29, tui-big-text 0.8 | ✓ VERIFIED | Line 16: `ratatui = "0.30"`, line 17: `crossterm = "0.29"`, line 18: `tui-big-text = "0.8"` — all three present, no old versions |
| `src/commands/welcome.rs` | `run_welcome_tui()`, `WelcomeAction` enum, `hint_bar_text()`, `draw_welcome()` | ✓ VERIFIED | All exports present: `pub enum WelcomeAction`, `pub async fn run_welcome_tui()`, `pub fn hint_bar_text()`, `pub fn routing_action()`, `fn draw_welcome()`, `fn setup_terminal()`, `fn restore_terminal()` |
| `src/main.rs` | TTY guard in None arm, routes to TUI or static fallback | ✓ VERIFIED | `is_terminal()` check at line 27, `run_welcome_tui(has_config).await?` call, `print_welcome()` in else branch |
| `src/commands/ui.rs` | Uses `frame.area()` (not `frame.size()`) | ✓ VERIFIED | Line 176: `.split(frame.area())` — no `frame.size()` present |
| `src/commands/wizard.rs` | Uses `frame.area()` (not `frame.size()`) | ✓ VERIFIED | Line 755: `.split(frame.area())` — no `frame.size()` present |

### Plan 20-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/main.rs` | `WelcomeAction::LaunchInit` routing to `init::run()` | ✓ VERIFIED | Lines 31-32: match arm calls `commands::init::run(PathBuf::from("squad.yml"), false).await?` |
| `src/commands/welcome.rs` | `routing_action()` pure function, 5 routing unit tests | ✓ VERIFIED | `pub fn routing_action()` at line 39; tests: `test_routing_action_enter_no_config`, `test_routing_action_enter_with_config`, `test_routing_action_quit_q`, `test_routing_action_quit_esc`, `test_routing_action_ignored_key` |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs` | `src/commands/welcome.rs` | `run_welcome_tui()` call in None arm | ✓ WIRED | Line 29: `commands::welcome::run_welcome_tui(has_config).await?` |
| `src/commands/welcome.rs` | `tui-big-text` crate | `BigText::builder()` widget rendering | ✓ WIRED | Line 18: `use tui_big_text::{BigText, PixelSize}`, line 120: `BigText::builder().pixel_size(PixelSize::HalfHeight)` |
| `src/main.rs` | `src/commands/init.rs` | `WelcomeAction::LaunchInit` match arm | ✓ WIRED | Lines 31-33: `Some(commands::welcome::WelcomeAction::LaunchInit) => { commands::init::run(PathBuf::from("squad.yml"), false).await?; }` |
| `src/main.rs` | `src/commands/ui.rs` | `WelcomeAction::LaunchDashboard` match arm | ✓ WIRED | Lines 34-36: `Some(commands::welcome::WelcomeAction::LaunchDashboard) => { commands::ui::run().await?; }` |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| WELCOME-01 | 20-01 | Bare `squad-station` always shows interactive TUI | ✓ SATISFIED | `main.rs` None arm routes to `run_welcome_tui()` when `is_terminal()` |
| WELCOME-02 | 20-01 | TUI displays large SQUAD-STATION title using pixel-font big text | ✓ SATISFIED | `BigText::builder().pixel_size(PixelSize::HalfHeight)` with `Line::from("SQUAD-STATION")` in `draw_welcome()` |
| WELCOME-03 | 20-01 | TUI displays current version below title | ✓ SATISFIED | `Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))` in chunk[1] |
| WELCOME-04 | 20-01 | TUI shows hint bar at bottom with available keys and auto-exit countdown | ✓ SATISFIED | `hint_bar_text(has_config, remaining_secs)` rendered in chunk[6]; format: `"Enter: ... Q: Quit  auto-exit Ns"` |
| WELCOME-06 | 20-01 | TUI auto-exits after N seconds if no key pressed | ✓ SATISFIED | `deadline = Instant::now() + Duration::from_secs(5)`, `remaining.is_zero()` break |
| WELCOME-07 | 20-01 | Non-TTY fallback: static text when stdout is not a terminal | ✓ SATISFIED | `std::io::stdout().is_terminal()` guard; `else` branch calls `print_welcome()` |
| INIT-01 | 20-02 | Enter key in welcome TUI (no `squad.yml`) launches init wizard | ✓ SATISFIED | `WelcomeAction::LaunchInit` arm calls `commands::init::run()` |
| INIT-02 | 20-02 | Enter key (with `squad.yml`) closes welcome without re-init | ✓ SATISFIED | `WelcomeAction::LaunchDashboard` arm calls `commands::ui::run()` — init is NOT called when config exists |
| INIT-03 | 20-02 | Q / Escape closes the welcome TUI without launching anything | ✓ SATISFIED | `routing_action()` maps Q/Esc to `WelcomeAction::Quit`; loop sets `action = None`; `main.rs` `_` arm exits silently |

**Orphaned requirements in REQUIREMENTS.md for Phase 20:** None. All requirements mapped to Phase 20 (`WELCOME-01` through `WELCOME-04`, `WELCOME-06`, `WELCOME-07`, `INIT-01`, `INIT-02`, `INIT-03`) are accounted for in plans 20-01 and 20-02.

**Note:** `WELCOME-05` is mapped to Phase 21 in REQUIREMENTS.md (Pending — not a Phase 20 requirement). No orphan.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | No anti-patterns found |

Scanned `src/commands/welcome.rs` and `src/main.rs` for: TODO/FIXME/HACK/PLACEHOLDER comments, empty returns (`return null`, `return {}`, `return []`), console-log-only implementations, and routing placeholders (`let _ = action`). None found.

The `let _ = action` placeholder from Plan 20-01 was correctly removed in commit `d0fe3bc` and replaced with the full `match action` routing block.

---

## Human Verification Required

### 1. Pixel-Font Title Rendering

**Test:** Run `./target/release/squad-station` (or `squad-station` if symlinked) with no arguments in a real terminal session.
**Expected:** AlternateScreen opens showing a large red pixel-font "SQUAD-STATION" title (HalfHeight BigText), version string centered below it, tagline "Multi-agent orchestration for AI coding", the 11-entry commands table, and a dim hint bar at the bottom showing `"Enter: Set up  Q: Quit  auto-exit 5s"`.
**Why human:** BigText HalfHeight pixel rendering requires a real terminal with sufficient width. The widget is wired correctly in code but visual quality (correct font size, centering, red color) cannot be asserted programmatically.

### 2. Countdown Tick and Auto-Exit

**Test:** Open the TUI and wait without pressing any key.
**Expected:** The countdown in the hint bar decrements each second (5s → 4s → 3s → 2s → 1s), then the TUI exits cleanly to the shell prompt with no terminal artifacts.
**Why human:** Real-time rendering of countdown ticks and clean terminal state after AlternateScreen exit requires visual confirmation.

### 3. Non-TTY Static Fallback

**Test:** Run `squad-station | cat` in a terminal.
**Expected:** Plain text output appears (ASCII art title, version, subcommand list) with no raw mode escape sequences, no TUI rendering, and no terminal state corruption.
**Why human:** Pipe-detection behavior and absence of terminal artifacts require observation in a shell session.

---

## Commits Verified

| Hash | Message | Verified |
|------|---------|---------|
| `9ce2f3b` | chore(20-01): upgrade ratatui 0.30 + crossterm 0.29 + tui-big-text 0.8 | ✓ Exists in git log |
| `3fa19c3` | feat(20-01): implement welcome TUI with BigText title, countdown, and TTY guard | ✓ Exists in git log |
| `d17a527` | test(20-02): add failing routing_action tests (RED) | ✓ Exists in git log |
| `d0fe3bc` | feat(20-02): wire WelcomeAction routing in main.rs and add routing_action() | ✓ Exists in git log |

---

## Test Suite Results

```
cargo test welcome   → 13 passed, 0 failed (all hint_bar_text, routing_action, commands_list, welcome_content tests)
cargo test (full)    → 230 passed across all suites, 0 failed
```

---

## Gaps Summary

No automated gaps. All 9 observable truths pass automated verification. The single `HUMAN_NEEDED` item is the visual quality of BigText pixel-font rendering in a real terminal — the code is correctly wired but the visual output (pixel font size, color, centering) requires a human to confirm in a live session.

The phase goal is functionally complete. Routing, TTY guard, fallback path, unit tests, and dependency upgrade are all verified against the actual codebase.

---

_Verified: 2026-03-17T15:00:00Z_
_Verifier: Claude (gsd-verifier)_
