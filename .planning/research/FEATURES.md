# Feature Research

**Domain:** Rust CLI — AI agent fleet orchestration (squad-station v1.8)
**Researched:** 2026-03-18
**Confidence:** HIGH (features are well-defined, ecosystem patterns verified, existing code reviewed directly)

---

## Context: What v1.8 Adds

Three features layered on top of v1.7 (which already ships: init wizard, TUI dashboard, welcome TUI, npm + curl install, welcome auto-launch on TTY detection).

The three v1.8 features:

1. `squad-station install [--tui]` — new Rust subcommand; bare = silent confirmation; `--tui` = launch welcome TUI. npm postinstall and curl installer call this instead of launching the binary directly.
2. Folder name as default project name — pre-fill in wizard page 1, dashboard title bar, squad.yml generation fallback.
3. Orchestrator "processing" state — TUI polls orchestrator tmux pane via `tmux capture-pane -p`, detects mid-input activity, renders a status indicator in the dashboard.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete or broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `install` subcommand with silent mode | Every serious CLI has a discrete install command; bare invocation = quiet/scriptable is assumed (git, npm, cargo init all follow this). Users running install in CI expect zero interactivity unless they explicitly opt in. | LOW | Bare `squad-station install` must produce one-line confirmation and exit 0. The JS layer in run.js already has an `install` handler — this moves the post-binary-install UX ownership to Rust. No scaffold logic moves; that stays in JS. |
| `--tui` flag for opt-in interactive path | Standard flag pattern for opt-in interactivity. clig.dev: "Only use prompts or interactive elements if stdin is a TTY. Never require a prompt — always provide a way of passing input with flags." | LOW | `--tui` must be guarded with the same `is_terminal()` check already used in welcome.rs. Non-TTY with `--tui` falls back silently — does not error. |
| Folder name pre-filled in wizard | `cargo init`, `npm create vite`, `create-next-app`, and virtually all scaffolding tools use `basename $PWD` as the default project name. Users expect to press Enter and get a sensible default. | LOW | `std::env::current_dir()` + `.file_name()` + `to_string_lossy()` for UTF-8 safety. This is a UI default only — user can still override in the input field. |
| Dashboard title reflects project name | TUI dashboards (Ralph TUI, TUICommander, tmuxcc) all show a project/session label. A dashboard without a title reads as a debug tool, not a product. | LOW | Read `project` field from squad.yml via existing `config::load()`. Folder name fallback when config is absent or field is empty. |
| squad.yml generation uses folder name fallback | When the user blanks the project name field in the wizard, the generated YAML must be valid. An empty `project:` field breaks agent auto-naming (`<project>-<tool>-<role>` convention). | LOW | Safety net for the programmatic path. The wizard already enforces non-empty input with inline error feedback (v1.5); the fallback covers edge cases in `generate_squad_yml()`. |
| npm + curl installers call `install --tui` | These are the actual install entry points for end users. If they still bypass the Rust `install` subcommand after v1.8, the install subcommand is a dead letter. | LOW | One-line change each: run.js replaces `spawnSync(destPath, [])` with `spawnSync(destPath, ['install', '--tui'])`; install.sh replaces `exec "${INSTALL_DIR}/squad-station"` with `exec "${INSTALL_DIR}/squad-station" install --tui`. |

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Orchestrator "processing" state in TUI | No competing tool (tmuxcc, NTM, Ralph TUI, TUICommander) distinguishes between "orchestrator idle at shell prompt" and "orchestrator mid-response composing a task." This lets agents and human observers see whether the orchestrator is actively working — reduces unnecessary interrupts. | MEDIUM | `tmux capture-pane -p -t <orchestrator_session>`, parse last non-empty line, match against AI prompt/spinner patterns. Standard approach across ecosystem (tmuxcc, NTM use identical pattern-matching strategy). |
| Install subcommand centralizes post-install UX in Rust | Currently npm postinstall (JS) and curl installer (sh) each independently contain TUI launch logic. Moving to `install --tui` means one canonical path — easier to test, no JS/sh divergence, version-stable behavior. | LOW | All terminal interaction already lives in Rust. The install subcommand becomes the single authoritative welcome entry point. |
| Context-aware install confirmation | `squad-station install` (bare) prints a single confirmation line to stdout and exits. Clean for CI, piped environments, and postinstall log capture. Consistent with `--json` global flag already in the CLI. | LOW | Distinct from current npm `install` which prints a multi-line banner. Silent mode = one line. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| "User is typing" detection via pane buffer diff | Seems logical: diff two capture-pane snapshots between polls and conclude typing happened | tmux capture-pane returns the static rendered screen, not a keystroke stream. AI tool animations (spinners, streaming response output) produce diff noise that creates false positives. Buffer content resets between visible screen refreshes — diffs are not stable indicators. | Heuristic pattern-match on last-line content: shell prompt regex = idle; AI tool response marker = processing. Combine with `pane_current_command` check to confirm the AI process is actually running. |
| Block `send` when orchestrator is "processing" | Sounds safe — don't interrupt a busy orchestrator | Stateless CLI model means `send` has no live visibility into orchestrator state. Adding a blocking check couples a write path to a read-only TUI heuristic. Race conditions guaranteed. Violates the "stateless, event-driven" design principle. | Surface processing state only in TUI dashboard as a visual indicator. Let `send` proceed normally. The orchestrator AI handles interruptions. |
| Auto-detect project name from git remote URL | Seems more authoritative than folder name | git remote URL parsing is brittle (SSH vs HTTPS, monorepos, forks, detached HEAD). Adds `git` subprocess dependency. Folder name is sufficient and universally available. | Use folder name as default; let user override in wizard. |
| Persist install state to `.squad/` | Track installed version in `.squad/station.db` or a lockfile | `squad-station install` is a one-shot binary operation, not a project-scoped command. Writing to `.squad/` implies the user is in a project directory, which is not guaranteed during binary install. | Binary install writes to system PATH only. Project-scoped `.squad/` writes belong to `init`. |
| Show orchestrator processing state in `send` output | Would give feedback to orchestrator when dispatching a task | `send` is called by the orchestrator AI itself, which already knows it is processing. Adding tmux pane polling to `send` adds latency to every task dispatch. | Processing state is a monitoring feature for the human observer in the TUI dashboard, not a feedback mechanism for the orchestrator. |

---

## Feature Dependencies

```
[install subcommand (Rust)]
    └──replaces──> [npm postinstall direct TUI launch (JS)]
    └──replaces──> [curl installer direct TUI launch (sh)]
    └──calls (--tui path)──> [welcome.rs run_welcome_tui()] (already exists)
    └──depends on──> [is_terminal() TTY guard] (already in welcome.rs)
    └──requires──> [new Commands::Install variant in cli.rs]
    └──requires──> [new src/commands/install.rs handler]

[folder name as default]
    └──feeds──> [wizard.rs ProjectPage: pre-fill project name input]
    └──feeds──> [ui.rs App: title bar display]
    └──feeds──> [init.rs generate_squad_yml(): fallback when project is empty]
    └──depends on──> [std::env::current_dir()] (stdlib, no new deps)

[orchestrator "processing" state]
    └──depends on──> [tmux capture-pane] (already called in tmux.rs)
    └──depends on──> [config::load() to resolve orchestrator session name]
    └──feeds──> [ui.rs App render: orchestrator row status indicator]
    └──depends on──> [TUI polling loop in ui.rs] (already polls agents + messages)
    └──requires no new DB schema changes]

[install subcommand] ──independent of──> [folder name default]
[install subcommand] ──independent of──> [orchestrator processing state]
[folder name default] ──independent of──> [orchestrator processing state]
```

### Dependency Notes

- **install subcommand requires a new cli.rs Commands variant:** Currently `Install` does not exist in the `Commands` enum. The JS run.js handles `install` before it reaches the binary. Adding the Rust subcommand means adding `Commands::Install { tui: bool }` to cli.rs and a matching `src/commands/install.rs`. The JS and sh files then delegate to `squad-station install [--tui]` instead of launching the binary bare.
- **install subcommand does not duplicate scaffold logic:** The JS install function in run.js still handles binary download and `.squad/` scaffold. The new Rust `install` subcommand handles only the post-install confirmation/welcome UX. No Rust code downloads or copies files.
- **folder name default requires no new crate dependencies:** `std::env::current_dir()` is stdlib. The three injection points (wizard pre-fill, dashboard title, squad.yml fallback) are already in the correct modules — minimal diff in each.
- **orchestrator processing state requires adding a capture-pane call to ui.rs polling:** ui.rs currently polls DB only. This adds one `tmux capture-pane -p -t <orchestrator_session>` call per refresh cycle for the orchestrator pane. The orchestrator session name comes from config (already loaded). This is the only architectural addition — no new crate, no schema change, no new tmux abstraction layer needed.
- **all three features are independent of each other:** They can be developed in parallel or sequentially in any order. No feature blocks another.

---

## MVP Definition

### This Milestone (v1.8)

Minimum set to ship v1.8 as a coherent release.

- [ ] `Commands::Install { tui: bool }` added to cli.rs — new clap variant with `--tui` flag
- [ ] `src/commands/install.rs` — bare path: print one-line "squad-station installed" confirmation + exit 0; `--tui` path: call `run_welcome_tui()` from welcome.rs with TTY guard
- [ ] Update run.js postinstall: replace `spawnSync(destPath, [])` with `spawnSync(destPath, ['install', '--tui'])` when `process.stdout.isTTY`
- [ ] Update install.sh: replace `exec "${INSTALL_DIR}/squad-station"` with `exec "${INSTALL_DIR}/squad-station" install --tui`
- [ ] Folder name default in wizard.rs ProjectPage: `current_dir()` basename set as initial value of project name text field
- [ ] Folder name fallback in init.rs `generate_squad_yml()`: use folder name when project name is empty or whitespace-only
- [ ] Dashboard title in ui.rs: display `project` from loaded config; fall back to folder name when config absent or project field empty
- [ ] Orchestrator pane capture in ui.rs polling loop: call `tmux capture-pane -p -t <orchestrator_session>`, pattern-match last non-empty line, derive `OrchestratorState::Idle | Processing`
- [ ] Render orchestrator processing state in TUI agent list — visual indicator (e.g., status tag or row highlight for the orchestrator entry)

### Add After Validation (post-v1.8)

- [ ] Richer orchestrator state: distinguish "thinking" (streaming response) vs "waiting for approval" vs "idle at shell" — requires provider-specific content keyword patterns beyond the basic prompt regex
- [ ] `squad-station install --silent` as explicit synonym for bare, for scripts that want clarity without relying on default behavior

### Future Consideration (v2+)

- [ ] Install subcommand with version pinning (`squad-station install --version x.y.z`) — relevant only when users need to pin versions across projects
- [ ] TUI dashboard project switcher — navigate between multiple `squad.yml` projects — requires significant ui.rs refactor

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| `install` subcommand (Rust, bare + `--tui`) | HIGH — unifies install UX, removes JS/sh divergence, single canonical path | LOW — new clap variant + thin dispatch to existing welcome.rs | P1 |
| Update npm + curl installers to call `install --tui` | HIGH — required for install subcommand to replace current auto-launch logic | LOW — one-line change each in run.js and install.sh | P1 |
| Folder name default in wizard pre-fill | HIGH — reduces friction for the common case (project in its own directory) | LOW — `current_dir().file_name()` + one-line pre-fill in wizard | P1 |
| Folder name fallback in squad.yml generation | HIGH — prevents broken YAML with empty `project:` field, which breaks agent naming | LOW — guard clause in existing `generate_squad_yml()` | P1 |
| Dashboard title shows project name | MEDIUM — cosmetic but makes TUI feel complete and scoped | LOW — read existing config field, folder name fallback | P1 |
| Orchestrator "processing" state in TUI | MEDIUM — useful signal for human observers; differentiator vs competing tools | MEDIUM — new capture-pane call in poll loop, heuristic regex, new UI state in render | P2 |

---

## Ecosystem Patterns Observed

### `install` Subcommand Conventions (MEDIUM confidence — clig.dev + codebase)

The existing codebase already has a JS-layer `install` subcommand in run.js. The v1.8 feature moves post-install welcome UX to a Rust subcommand. Patterns verified:

- clig.dev: "Only use prompts or interactive elements if stdin is a TTY. Never require a prompt — always provide a way of passing input with flags." — supports bare-silent + `--tui`-opt-in design.
- Oracle/Git for Windows: silent mode via flag is the established convention for scripted installs.
- `--tui` flag with TTY guard is consistent with OpenCode CLI (bare invocation = TUI) and with existing squad-station welcome TUI pattern.
- The split between "download + scaffold" (JS responsibility) and "welcome UX" (Rust responsibility) is clean: no ownership confusion.

### Folder Name as Default (HIGH confidence — cargo init official docs)

`cargo init` uses the directory name as the package name by default (overridable with `--name`). `npm create vite@latest` supports `.` for current directory scaffolding. `create-next-app` prompts with the directory basename pre-filled. This is the universal scaffolding convention — users expect it and are surprised when it is absent.

### Pane Content State Detection (MEDIUM confidence — tmuxcc, NTM, TUICommander)

tmuxcc (Rust) uses agent-specific parsers that scan the last N lines of captured pane content for status markers (`@` = processing, `*` = idle, `!` = awaiting approval). NTM strips ANSI sequences and checks lines against patterns with recency weighting. TUICommander uses provider-specific regex for rate-limit and idle detection. The standard approach across all tools: `capture-pane -p`, strip ANSI, regex-match last non-empty line. No tmux API exists for "user is typing." `pane_current_command` via `tmux display-message` confirms the process is running without parsing content — useful as a secondary check.

For Claude Code and Gemini CLI specifically: the shell prompt regex `\$\s*$` or `%\s*$` signals idle (shell returned control). AI tool activity is signaled by response stream content (`>`, `◆`, `ℹ`, or any non-prompt line on the last visible row). This heuristic is reliable enough for a visual dashboard indicator but should not gate write operations.

---

## Sources

- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) — interactive/silent modes, flag conventions
- [cargo init — The Cargo Book](https://doc.rust-lang.org/cargo/commands/cargo-init.html) — folder-name-as-default convention (HIGH confidence)
- [tmuxcc GitHub](https://github.com/nyanko3141592/tmuxcc) — pane content pattern detection for AI agent state (MEDIUM confidence)
- [TUICommander](https://tuicommander.com/) — agent status detection patterns (MEDIUM confidence)
- [Ralph TUI](https://ralph-tui.com/) — task execution state visualization (MEDIUM confidence)
- [tmux man page](https://man7.org/linux/man-pages/man1/tmux.1.html) — `capture-pane`, `display-message`, `pane_current_command` format variables
- Codebase (verified directly): `src/commands/welcome.rs`, `src/commands/ui.rs`, `src/commands/wizard.rs`, `src/commands/init.rs`, `src/cli.rs`, `src/tmux.rs`, `npm-package/bin/run.js`, `install.sh`

---

*Feature research for: squad-station v1.8 — Install subcommand, folder name defaults, orchestrator processing state*
*Researched: 2026-03-18*
