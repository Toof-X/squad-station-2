# Architecture Research

**Domain:** Rust CLI — ratatui welcome TUI + post-install auto-launch (v1.7)
**Researched:** 2026-03-17
**Confidence:** HIGH — all findings derived from direct source inspection of the live codebase.

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                       Entry Points                               │
│  ┌──────────────────┐  ┌──────────────────┐                      │
│  │  npm postinstall  │  │   install.sh      │                     │
│  │  (run.js install) │  │ (curl | sh)       │                     │
│  └────────┬─────────┘  └────────┬──────────┘                     │
│           │  downloads binary    │  installs binary               │
│           └──────────┬───────────┘                               │
│                      │  exec squad-station (no args)             │
└──────────────────────┼───────────────────────────────────────────┘
                       ↓
┌──────────────────────────────────────────────────────────────────┐
│                   main.rs / run()                                │
│                                                                  │
│   cli::Cli::parse()                                              │
│       ↓ cli.command                                              │
│   None ──────────────────────────────────────────────────────►  │
│                               commands::welcome::print_welcome() │
│                               [TARGET: replace with ratatui TUI] │
│   Some(Init) ──────────────────────────────────────────────────► │
│                    commands::init::run()                         │
│                      └─ no squad.yml? → wizard::run()            │
│                      └─ squad.yml exists + TTY? → prompt_reinit()│
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Current State |
|-----------|----------------|---------------|
| `src/commands/welcome.rs` | No-arg invocation output | Static print_welcome() — println! only, no ratatui |
| `src/commands/wizard.rs` | Interactive ratatui TUI form | 1362 lines, full ratatui event loop, AlternateScreen |
| `src/commands/ui.rs` | TUI dashboard (fleet monitor) | ratatui + crossterm, connect-per-refresh pattern |
| `src/commands/init.rs` | Init flow + TTY guard for reinit | Uses `std::io::stdin().is_terminal()` guard |
| `src/main.rs` | SIGPIPE + command dispatch | `Option<Commands>` — None arm routes to welcome |
| `npm-package/bin/run.js` | npm install subcommand | Downloads binary, scaffolds .squad/, no auto-launch |
| `install.sh` | curl-based installer | Downloads binary to /usr/local/bin, no auto-launch |

---

## Recommended Project Structure

```
src/commands/
├── welcome.rs          # MODIFY: add run_welcome() ratatui TUI entry point
│                       #         keep print_welcome() as non-TTY fallback
│                       #         keep welcome_content() for tests (unchanged)
├── wizard.rs           # NO CHANGE — reused via commands::init::run() delegation
├── ui.rs               # NO CHANGE — existing fleet dashboard
├── init.rs             # NO CHANGE — guard clause and reinit logic unchanged
└── mod.rs              # NO CHANGE — welcome module already declared

npm-package/bin/run.js  # MODIFY: TTY-guarded spawnSync(destPath) at end of install()
install.sh              # MODIFY: TTY-guarded exec squad-station at end
```

### Structure Rationale

- **welcome.rs only:** One Rust file changes. The welcome TUI is a new entry point in the existing module — no new file, no new module, no new dependency.
- **wizard.rs / ui.rs as reference:** Both are live ratatui implementations in this codebase. The AlternateScreen setup, panic hook, and restore pattern are copy-verified from `ui.rs` lines 284–338.
- **init delegation:** The welcome TUI must not own any init logic. It calls `commands::init::run()` on Enter. This ensures hook installation, context generation, and the post-init diagram all fire correctly.
- **run.js / install.sh:** Both need one guard-wrapped exec appended. Use absolute `destPath` (already computed during install) rather than relying on PATH resolution.

---

## Architectural Patterns

### Pattern 1: TTY Guard Before Raw Mode

**What:** Call `std::io::stdin().is_terminal()` (from `std::io::IsTerminal`) before entering crossterm raw mode. Non-TTY falls through to the existing `print_welcome()` plain-text path.

**When to use:** Required on every ratatui surface in this codebase. `init.rs` already gates `prompt_reinit()` with this check. The welcome TUI must follow the same gate.

**Trade-offs:**
- Prevents crossterm raw mode crashes in CI, piped stdin, and automated test contexts.
- Preserves the `--json` machine-readable contract completely.
- Zero cost for the common case.

**Existing precedent (init.rs line 103):**
```rust
} else if std::io::stdin().is_terminal() {
    // Re-init: squad.yml exists and we have an interactive terminal
    match prompt_reinit()? {
```

**Apply to welcome.rs:**
```rust
pub async fn run_welcome() -> anyhow::Result<()> {
    if std::io::stdin().is_terminal() {
        run_welcome_tui().await
    } else {
        print_welcome();
        Ok(())
    }
}
```

### Pattern 2: AlternateScreen + Panic Hook Restore

**What:** Enter alternate screen before the ratatui loop. Install a panic hook that calls `disable_raw_mode` + `LeaveAlternateScreen` before propagating. Restore terminal on clean exit.

**When to use:** All ratatui TUIs in this codebase. `ui.rs` lines 284–338 implement this pattern verbatim. `wizard.rs` mirrors it. The welcome TUI must replicate it.

**Trade-offs:**
- Terminal is never left in raw mode after a crash.
- ~15 lines of boilerplate per TUI surface — acceptable given the small count.

**Pattern (from ui.rs):**
```rust
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    original_hook(info);
}));
let mut terminal = setup_terminal()?;
// ... event loop ...
restore_terminal(&mut terminal)?;
let _ = std::panic::take_hook(); // restore default
```

### Pattern 3: First-Run Detection via squad.yml Existence

**What:** Check `Path::new("squad.yml").exists()` at the CWD to distinguish first-run from returning-user context. No new env var, no lockfile, no DB needed.

**When to use:** Welcome TUI entry — determines CTA ("Press Enter to set up") vs guide-only mode.

**Trade-offs:**
- Zero new state — squad.yml is already the canonical project marker used by every command.
- Consistent with init.rs guard clause (same check, same semantics).
- CWD-dependent — users must be in their project directory, which is the existing UX contract for all squad-station commands.

**Data flow:**
```
welcome::run_welcome_tui()
    ↓
Path::new("squad.yml").exists()?
    ├── NO  → show title + "Press Enter to set up your first squad"
    │         Enter → restore_terminal() → commands::init::run("squad.yml", false)
    │         Esc/q → restore_terminal() → return Ok(())
    │
    └── YES → show title + version + quick commands guide
              Any key → restore_terminal() → return Ok(())
```

**Key detail:** Welcome TUI exits AlternateScreen *before* calling init. Init's wizard then enters its own AlternateScreen fresh. This avoids nested alternate buffer state.

### Pattern 4: Post-Install Auto-Launch via Exec

**What:** After binary install completes, launch `squad-station` (no args) using the absolute install path — not via PATH lookup — so the welcome TUI appears in the same terminal session.

**When to use:** npm `install()` function in run.js and the end of install.sh, both guarded by TTY detection.

**TTY detection in install.sh:**
```sh
if [ -t 1 ]; then
  exec "$INSTALL_DIR/squad-station"
fi
```
(`exec` replaces the shell process — no orphan processes, no double prompt.)

**TTY detection in run.js:**
```javascript
if (process.stdout.isTTY) {
  spawnSync(destPath, [], { stdio: 'inherit' });
}
```
(`destPath` is the absolute path computed during `installBinary()`, not a PATH-resolved name.)

**Trade-offs:**
- Zero new infrastructure — binary is already on disk, just invoke it.
- Absolute path avoids the PATH-not-updated problem when installing to `~/.local/bin`.
- `exec` in sh eliminates orphan processes.
- TTY guard in both the installer and welcome.rs provides defense-in-depth.

---

## Data Flow

### First-Run Flow (post-install)

```
curl | sh  OR  npx squad-station install
    ↓
Binary downloaded to /usr/local/bin/squad-station (or ~/.local/bin)
.squad/ sdd/ and examples/ scaffolded (npm path only)
    ↓
[TTY check: process.stdout.isTTY OR [ -t 1 ]]
    ↓
exec /usr/local/bin/squad-station   (no args, absolute path)
    ↓
main.rs: cli.command = None
    ↓
commands::welcome::run_welcome().await
    ↓
std::io::stdin().is_terminal() = true
    ↓
Enter AlternateScreen — ratatui welcome TUI
    │
    ├── squad.yml NOT found at CWD
    │       Display: ASCII title + version
    │                "Press Enter to set up your first squad"
    │                "Esc to exit"
    │       On Enter → restore_terminal() → commands::init::run("squad.yml", false).await
    │       On Esc/q → restore_terminal() → return Ok(())
    │
    └── squad.yml found at CWD
            Display: ASCII title + version + quick commands list
            On any key → restore_terminal() → return Ok(())
```

### Returning User Flow

```
squad-station  (no args, in existing project dir)
    ↓
main.rs: cli.command = None
    ↓
commands::welcome::run_welcome()
    ↓
is_terminal() = true
squad.yml exists → show informational TUI → user presses q/Esc → exit 0
```

### Non-TTY / CI Flow

```
squad-station  (stdout not a TTY)
    ↓
commands::welcome::run_welcome()
    ↓
is_terminal() = false → print_welcome() [existing behavior] → exit 0
```

---

## Integration Points

### New vs Modified: Explicit Boundary

| Component | Change Type | What Changes |
|-----------|-------------|--------------|
| `src/commands/welcome.rs` | MODIFY | Add `run_welcome()` public async fn; add private `run_welcome_tui()` ratatui loop; keep `print_welcome()` as non-TTY path; `welcome_content()` unchanged |
| `src/main.rs` | MODIFY | `None => commands::welcome::print_welcome()` → `None => commands::welcome::run_welcome().await?` |
| `npm-package/bin/run.js` | MODIFY | Append TTY-guarded `spawnSync(destPath, [], { stdio: 'inherit' })` at end of `install()` |
| `install.sh` | MODIFY | Append `[ -t 1 ] && exec "${INSTALL_DIR}/squad-station"` before final exit |
| `src/commands/wizard.rs` | NO CHANGE | Reused as-is — welcome TUI calls init which calls wizard |
| `src/commands/init.rs` | NO CHANGE | Guard clause and reinit logic unchanged |
| `src/cli.rs` | NO CHANGE | `Option<Commands>` pattern stays; None arm behavior changes only in main.rs |

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `welcome.rs` → `init::run()` | Direct async fn call | Welcome exits AlternateScreen first, then init runs in normal terminal mode, then init's wizard opens its own AlternateScreen |
| `welcome.rs` → `print_welcome()` | Internal call | Non-TTY fallback path — no coupling change |
| `main.rs` → `welcome::run_welcome()` | Async fn call via `?` | main.rs `run()` is already `async fn` — no runtime change needed |
| `run.js install()` → binary | `spawnSync(destPath)` | Uses absolute install path computed during the same install() call |
| `install.sh` → binary | `exec "$INSTALL_DIR/squad-station"` | Absolute path, exec replaces shell process |

---

## Anti-Patterns

### Anti-Pattern 1: Duplicating Init Logic in Welcome TUI

**What people do:** Embed wizard pages directly in `welcome.rs` to avoid a function call across the module boundary.
**Why it's wrong:** `wizard.rs` is 1362 lines of validated, tested code. Hook installation (`auto_install_hooks`), context generation (`context::run()`), and the post-init diagram all live in `init.rs`. Duplicating any of this creates two sources of truth and silently breaks onboarding when either copy diverges.
**Do this instead:** Welcome TUI restores the terminal, then calls `commands::init::run(PathBuf::from("squad.yml"), false).await`. All init behavior fires correctly with no duplication.

### Anti-Pattern 2: Unconditional Exec in Installer (Missing TTY Guard)

**What people do:** Append `exec squad-station` or `spawnSync(...)` at the end of install scripts without a TTY check.
**Why it's wrong:** CI pipelines run `npm install` and `curl | sh` in non-interactive contexts. crossterm will fail to enter raw mode when there is no TTY. Even with the `is_terminal()` guard in `welcome.rs`, the spawn attempt itself is wrong in CI.
**Do this instead:** Gate the binary launch with `process.stdout.isTTY` in Node.js and `[ -t 1 ]` in sh. The binary's own TTY guard is defense-in-depth, not the primary check.

### Anti-Pattern 3: Nested AlternateScreen (Welcome Into Init Without Restoring First)

**What people do:** Welcome TUI presses Enter, stays in AlternateScreen, and calls init — which opens wizard's own AlternateScreen — while the welcome buffer is still active.
**Why it's wrong:** crossterm's `LeaveAlternateScreen` is reference-counted per nesting level on some terminals. Wizard's `LeaveAlternateScreen` may reveal the blank welcome buffer instead of the normal terminal. The user sees garbled output on exit.
**Do this instead:** Call `restore_terminal(&mut terminal)?` in welcome.rs before delegating to `init::run()`. The wizard then enters its own AlternateScreen fresh from a clean terminal state.

### Anti-Pattern 4: PATH-Based Binary Lookup for Post-Install Launch

**What people do:** After installing to `~/.local/bin`, call `spawnSync('squad-station', ...)` — relying on PATH resolution.
**Why it's wrong:** `~/.local/bin` is typically not in PATH until the user opens a new shell. The launch will fail with "command not found" immediately after the download completes.
**Do this instead:** Use `destPath` (the absolute path already computed in `installBinary()`) for the post-install spawn. Print the PATH reminder message separately but do not depend on PATH for the launch.

---

## Suggested Build Order

Dependencies flow strictly from Rust → installer scripts; no reverse dependency.

| Step | Component | Work | Depends On |
|------|-----------|------|------------|
| 1 | `welcome.rs` | Add `run_welcome_tui()` ratatui skeleton: AlternateScreen + panic hook + event loop + `restore_terminal()` on exit. No conditional logic yet — always shows title and returns. | Nothing new; ratatui/crossterm already in Cargo.toml |
| 2 | `welcome.rs` | Add `squad.yml` detection: branch display content between first-run CTA and returning-user guide. | Step 1 |
| 3 | `welcome.rs` | Wire Enter key in first-run mode: `restore_terminal()`, then `commands::init::run(PathBuf::from("squad.yml"), false).await`. | Step 2; `init::run()` is stable |
| 4 | `welcome.rs` | Expose `run_welcome()` as the public async entry point with TTY guard. | Step 3 |
| 5 | `main.rs` | Swap `print_welcome()` call for `run_welcome().await?`. | Step 4 |
| 6 | `install.sh` | Append TTY-guarded `exec "$INSTALL_DIR/squad-station"`. | Step 5 in spirit; can run in parallel with steps 1-5 |
| 7 | `run.js` | Append TTY-guarded `spawnSync(destPath, [], { stdio: 'inherit' })` at end of `install()`. | Step 5 in spirit; can run in parallel with steps 1-5 |

Steps 6 and 7 are independent of the Rust changes and can be developed and tested in parallel. The binary itself must exist at the target path before the auto-launch behavior can be validated end-to-end.

---

## Scaling Considerations

Not applicable — this is a local CLI tool used per-developer per-project. The welcome TUI is a one-time surface per install session.

| Concern | Approach |
|---------|----------|
| Multiple install methods (npm + curl) | Both trigger welcome TUI independently; no coordination needed; idempotent since binary is stateless |
| Large terminal sizes | ratatui `Constraint::Percentage` and `Constraint::Min` handle arbitrary sizes; existing wizard.rs proves this |
| Non-standard TERM values / dumb terminals | `is_terminal()` guard prevents raw mode entry; `print_welcome()` fallback covers dumb terminals |

---

## Sources

- Direct source inspection: `src/commands/welcome.rs`, `src/commands/wizard.rs`, `src/commands/ui.rs`, `src/commands/init.rs`, `src/main.rs`, `src/cli.rs`, `src/config.rs`
- Direct source inspection: `npm-package/bin/run.js`, `install.sh`
- Existing pattern references:
  - `init.rs` line 103: `is_terminal()` gate before raw mode
  - `ui.rs` lines 284–338: panic hook + AlternateScreen setup/restore pattern
  - `wizard.rs` lines 1–17: crossterm + ratatui imports (confirms no new deps needed)
- Project context: `.planning/PROJECT.md` (v1.7 milestone target features)

---
*Architecture research for: Squad Station v1.7 — First-Run Onboarding TUI + Post-Install Auto-Launch*
*Researched: 2026-03-17*
