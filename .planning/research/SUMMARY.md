# Project Research Summary

**Project:** squad-station v1.8 — Install Subcommand, Folder-Name Default, Orchestrator Processing State
**Domain:** Rust CLI — stateless AI agent fleet orchestration with embedded SQLite, ratatui TUI, and tmux integration
**Researched:** 2026-03-18
**Confidence:** HIGH

## Executive Summary

Squad Station v1.8 is a focused feature increment on top of a stable, well-tested v1.7 foundation. The three additions — a Rust `install` subcommand, folder-name-as-project-name defaulting, and orchestrator "processing" state detection via tmux pane polling — all share a key characteristic: zero new Cargo dependencies. The existing stack (clap 4.5, ratatui 0.30, tokio 1.37, sqlx 0.8, crossterm 0.29, and Rust stdlib) is fully sufficient. The recommended approach treats these three features as independent work streams with a clear build order dictated by shared file ownership rather than logical dependencies.

The recommended implementation sequence is: (1) lay the tmux `capture_pane` foundation and processing-state display infrastructure, (2) wire the pane polling into the TUI refresh loop with the correct DB write pattern, (3) inject folder-name defaults into the wizard and dashboard, then (4) add the `install` subcommand and update distribution files. This order minimizes rebase conflicts on `ui.rs` and `tmux.rs`, which are the most heavily touched files. All three features can be reviewed in isolation once landed.

The highest-risk area is orchestrator pane-state detection. Scraping terminal UI output for semantic state is inherently heuristic — false positives (always showing "processing") and blocked async executor threads are the two most dangerous failure modes. Both are preventable by design: isolate the classification logic in a pure testable function, poll only the orchestrator (not all agents), use a separate slower interval (10–15s) for pane polling vs. the existing 3s DB refresh, and treat the detected state as a TUI-only overlay rather than a DB status value. The `install` subcommand carries its own secondary risk: a JS/Rust version mismatch during release can break `npx squad-station install` with a clap parse error. The safest mitigation is to keep the npm auto-launch as a bare binary call and document `install --tui` as the explicit user-facing form.

---

## Key Findings

### Recommended Stack

No new Rust crates are needed for v1.8. All three features are implementable with the existing dependency set and Rust stdlib primitives. The only external-surface change is in the npm layer: adding a `"scripts"` block to `package.json` and a one-line change to `install.sh`.

**Core technologies (unchanged from v1.7):**
- **clap 4.5.60** — CLI subcommand dispatch — add `Install { tui: bool }` variant; no version change, no new dep
- **ratatui 0.30 + crossterm 0.29** — TUI rendering and TTY detection — extend existing patterns, no change
- **tokio 1.37** — async runtime — `tokio::time::interval` already available; `tokio::process::Command` recommended for non-blocking capture-pane calls in the TUI loop
- **sqlx 0.8 + SQLite WAL** — persistence — no schema migration needed; `processing` is TUI-only (not written to DB)
- **std::env::current_dir() + Path::file_name()** — zero-cost folder-name derivation — no crate needed

**Rejected alternatives:** `reqwest` (+1.5 MB binary size, musl TLS complications — use `curl` via subprocess instead), `dirs` crate (removed in v1.4 — stdlib suffices), `regex` (start with `str::contains`; promote only if classification complexity grows), `notify` filesystem watcher (pane polling via `capture-pane` is the correct tmux-native pattern).

### Expected Features

**Must have (table stakes — P1):**
- `squad-station install [--tui]` — bare = one-line silent confirmation + exit 0; `--tui` = launch welcome TUI with TTY guard; every serious CLI has a discrete install command
- Update npm postinstall and curl installer to call `install --tui` — required for the install subcommand to be the canonical path rather than dead code
- Folder name pre-filled in wizard project name field — universal scaffolding convention (`cargo init`, `npm create vite`, `create-next-app` all do this)
- Folder name fallback in `generate_squad_yml()` — prevents broken YAML with empty `project:` field, which breaks agent auto-naming (`<project>-<tool>-<role>`)
- Dashboard title shows project name from config — makes TUI feel scoped to the current project rather than a generic debug tool

**Should have (differentiators — P2):**
- Orchestrator "processing" state indicator in TUI dashboard — no competing tool (tmuxcc, NTM, Ralph TUI, TUICommander) distinguishes orchestrator-idle vs. orchestrator-mid-response; reduces unnecessary interrupts from human observers

**Defer (post-v1.8):**
- Richer orchestrator state: distinguish "thinking" vs. "waiting for approval" vs. "idle at shell" — requires provider-specific content patterns beyond basic prompt regex
- `--silent` flag as explicit synonym for bare install
- Install subcommand with version pinning (`--version x.y.z`)
- TUI project switcher (significant `ui.rs` refactor)

**Anti-features (do not build):**
- Buffer-diff typing detection — tmux capture-pane returns a static rendered screen, not a keystroke stream; diff noise creates false positives
- Block `send` when orchestrator is "processing" — violates stateless design, introduces race conditions
- Auto-detect project name from git remote URL — brittle, adds git subprocess dependency; folder name is universally available

### Architecture Approach

The v1.8 changes are additive layers on a stable layered architecture: CLI dispatch → command handlers → tmux abstraction + DB layer. All new code follows three established patterns: command-per-file (`pub async fn run(...)` in a new `install.rs`), connect-per-refresh for writable pools in the TUI (open a writable `db::connect()` only when a status write is needed; drop immediately after), and argument-builder functions in `tmux.rs` (private `_args()` for unit testability, public function calls `Command::new("tmux")`).

**Files changed across all three features:**

| File | Change | Feature |
|------|--------|---------|
| `src/cli.rs` | Add `Install { tui: bool }` variant | install |
| `src/commands/install.rs` | NEW — `run(tui: bool)` handler | install |
| `src/commands/mod.rs` | Add `pub mod install;` | install |
| `src/main.rs` | Add `Install { tui }` match arm | install |
| `src/commands/wizard.rs` | Pre-populate `project_input` with folder name | folder default |
| `src/commands/ui.rs` | Add `project: String` to `App`; title bar; `processing` color; capture-pane poll | folder default + processing |
| `src/tmux.rs` | Add `capture_pane_args()` + `capture_pane()` | processing |
| `src/commands/helpers.rs` | Add `"processing"` arm to `colorize_agent_status()` | processing |
| `npm-package/bin/run.js` | Update auto-launch call | install |
| `install.sh` | Update exec target | install |

**Key architectural decision — processing state storage:** Treat `processing` as a TUI-only overlay, not a DB status value. DB status tracks hook-driven lifecycle (`idle`/`busy`/`dead`). The TUI optionally overlays a `processing` indicator from pane polling displayed alongside (not replacing) the DB status. This avoids a migration, keeps the DB contract stable, and prevents `squad-station status` and `agents --json` from breaking.

### Critical Pitfalls

1. **npm/Rust `install` command boundary confusion** — `npx squad-station install` is intercepted by `run.js` (JS download path) before reaching the binary. The Rust `install` subcommand is the post-binary-install welcome UX path. These must stay separate. Prevention: keep `run.js` auto-launch as a bare binary call; `install --tui` is the explicitly user-invoked form. Document the JS/Rust boundary in code comments.

2. **`current_dir()` edge cases corrupt tmux session names** — `file_name()` returns `None` at filesystem root; `to_string_lossy()` introduces replacement characters for non-UTF-8 paths; directory names with spaces break tmux session creation. Prevention: apply an extended sanitizer at derivation time (`[a-z0-9A-Z_-]` allowlist), handle `None` gracefully with empty-string fallback, and extract derivation to a pure function accepting `&Path` for testability.

3. **`capture-pane` called on non-existent session floods TUI with errors** — orchestrators with `tool = "antigravity"` have no tmux session; dead sessions no longer exist after `squad-station close`. Prevention: explicit guard chain — check `agent.tool != "antigravity"`, then `tmux::session_exists()`, then return `None` on any error; never surface capture-pane failures as user-visible errors.

4. **False positive "processing" detection creates a permanent "processing" state** — idle prompt lines (`>`, `$`) look like activity; stale pane content from completed tasks triggers the heuristic. Prevention: conservative provider-specific patterns, prefer DB state when ambiguous, implement `classify_pane_output()` as a pure testable function with no I/O.

5. **Blocking `std::process::Command` for capture-pane blocks the tokio executor** — the existing tmux functions use blocking subprocess calls; adding capture-pane to the 3s TUI refresh loop creates visible UI lag and drops key events under load. Prevention: use `tokio::process::Command` for capture-pane, poll on a separate 10–15s interval (not the 3s DB interval), cache last pane content so slow calls don't freeze the render cycle.

6. **Version mismatch breaks `npx squad-station install` during partial release** — if the npm package is published with `spawnSync(destPath, ['install', '--tui'])` before the new binary is released, users on old binaries get a clap parse error. Prevention: keep auto-launch as bare `spawnSync(destPath, [])` in v1.8; or add version detection in JS before using the subcommand form.

---

## Implications for Roadmap

All three features are mutually independent and can be developed in any order. The recommended sequence below is driven by shared file ownership (minimizing merge conflicts on `ui.rs`) and by the principle of laying stable foundations before wiring up polling logic.

### Phase 1: `capture_pane` Foundation and Processing Display Infrastructure

**Rationale:** Pure additions to `tmux.rs` and `helpers.rs` with no side effects. Landing the arg-builder function and colorize arm first makes Phase 2's TUI wiring a clean, focused change. These foundational modules should be stable before the event loop changes land.

**Delivers:** `tmux::capture_pane()` (unit-testable via arg-builder pattern), `"processing"` color arm in `helpers.rs` and `ui.rs::status_color()`, `classify_pane_output(output: &str) -> bool` pure function

**Addresses features:** Orchestrator processing state (display infrastructure only)

**Avoids pitfalls:** Arg-builder pattern and pure classification function enforced from the start, before any polling code is written

**Research flag:** Standard patterns — follows established `tmux.rs` arg-builder pattern exactly; no deeper research needed

---

### Phase 2: Processing State Detection in TUI Refresh Loop

**Rationale:** Requires Phase 1 `capture_pane()` to exist. Contains the most risk (heuristic accuracy, async blocking, DB write pattern) and should be validated early while research findings are fresh.

**Delivers:** Live orchestrator status polling in the TUI dashboard with guard chain (antigravity check, dead session check, error fallback), separate 10–15s poll interval, TUI-only overlay using `classify_pane_output()`, `tokio::process::Command` for non-blocking subprocess

**Addresses features:** Orchestrator "processing" state (P2 differentiator — fully shipped)

**Avoids pitfalls:** Pitfall 3 (guard chain), Pitfall 4 (conservative heuristic, pure function), Pitfall 5 (async-safe subprocess, separate interval), Pitfall 9 (TUI-only — no DB contract change)

**Research flag:** Needs attention during implementation — the provider-specific heuristic patterns for `classify_pane_output()` (Claude Code vs. Gemini CLI) require manual testing against real sessions before the patterns are finalized. The DB-vs-TUI-only decision must be explicitly locked before code is written.

---

### Phase 3: Folder Name as Project Name Default

**Rationale:** Fully independent of Phases 1 and 2. Zero risk to existing functionality. Batching the `ui.rs` title bar change after Phase 2's larger `ui.rs` changes reduces merge noise.

**Delivers:** Pre-populated project name field in wizard (sanitized, with cursor at end), project name in dashboard title bar, folder-name fallback in `generate_squad_yml()`, pure derivation function accepting `&Path` for testability

**Addresses features:** Folder name pre-fill (P1), dashboard title (P1), squad.yml fallback (P1)

**Avoids pitfalls:** Pitfall 2 (sanitization and `None` guard built in from the start), Pitfall 7 (wizard validation reviewed alongside the default introduction), Pitfall 8 (pure function accepting `&Path` for testability — no direct `current_dir()` call inside unit-tested code)

**Research flag:** Standard patterns — `cargo init` convention is universal; sanitization approach is straightforward; no deeper research needed

---

### Phase 4: `install` Subcommand and Distribution Updates

**Rationale:** Most review surface area (new file + CLI enum + distribution files). Independent of Phases 1–3, but benefits from landing after core binary changes are stable. The JS/Rust boundary decision must be settled before writing code.

**Delivers:** `Commands::Install { tui: bool }`, `src/commands/install.rs` config-free handler, updated `npm-package/bin/run.js` and `install.sh`, documented JS/Rust ownership boundary in code comments

**Addresses features:** `install` subcommand (P1 table stakes), unified install path (P1 differentiator)

**Avoids pitfalls:** Pitfall 1 (clean JS/Rust boundary documented), Pitfall 6 (keep npm auto-launch as bare call to avoid version coupling), Pitfall 10 (install handler explicitly config-free — no `load_config()` before `squad.yml` exists)

**Research flag:** Standard patterns for CLI subcommand structure. The npm/Rust ownership boundary is a documented design decision (not a research gap) that must be written into the implementation plan before coding starts.

---

### Phase Ordering Rationale

- **Phases 1 and 2 are sequenced together** because they share `tmux.rs` and `ui.rs`. Landing the pure foundation (Phase 1) before wiring the polling loop (Phase 2) avoids a single large diff that combines display infrastructure with event loop logic.
- **Phase 3 is independent** of all other phases and could be Phase 1. It is placed third only to avoid a second `ui.rs` PR conflicting with Phase 2's larger changes.
- **Phase 4 is last** because it has the most distribution touchpoints and the longest review cycle. The core binary features (Phases 1–3) should be stable before the CLI surface and distribution files change.
- **All three features can be parallelized** by separate developers. The only coordination point is `ui.rs`: Phase 2 (processing color + poll) and Phase 3 (title bar + project field) both modify `ui.rs`. Assign these to the same developer or coordinate the merge order explicitly.

### Research Flags

Phases needing attention during planning or implementation:
- **Phase 2 (pane polling):** Provider-specific heuristic patterns for `classify_pane_output()` require manual validation against real Claude Code and Gemini CLI sessions. The DB-vs-TUI-only overlay decision is an architectural choice that must be documented in the implementation plan before any code is written.

Phases with standard, well-documented patterns:
- **Phase 1 (capture_pane foundation):** Follows `tmux.rs` arg-builder pattern exactly. No research needed.
- **Phase 3 (folder name default):** `cargo init` convention is universal; sanitization is straightforward. No research needed.
- **Phase 4 (install subcommand):** clap subcommand pattern is well-established. JS/Rust boundary is a design decision, not a research gap.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All findings derived from direct `Cargo.lock` inspection and codebase review. Zero new dependencies confirmed across all three features. |
| Features | HIGH | Features are well-defined; ecosystem patterns verified (clig.dev, cargo init, tmuxcc, NTM, TUICommander). Anti-features explicitly identified with justification. |
| Architecture | HIGH | All findings from direct source inspection of v1.7 codebase. File change matrix is complete and verified against actual module structure. |
| Pitfalls | HIGH | Based on direct codebase inspection plus patterns from v1.5–v1.7 development history. 10 specific pitfalls with phase-level prevention guidance and recovery costs. |

**Overall confidence:** HIGH

### Gaps to Address

- **Pane content heuristic patterns:** The specific string patterns for detecting "processing" vs. "idle" in Claude Code and Gemini CLI panes cannot be finalized without manual testing against real sessions. The Phase 2 implementation plan should include a validation step (run the heuristic against known-idle and known-active sessions) before the code is merged.

- **npm auto-launch backward compatibility:** The safest approach (keep bare invocation in `run.js`) is documented but not the only option. If the team wants to use `spawnSync(destPath, ['install', '--tui'])` in the auto-launch path, a version-detection guard must be added. This decision should be made explicitly during Phase 4 planning, not left to the implementor.

- **`processing` DB vs. TUI-only decision:** Research recommends TUI-only overlay. If any future feature (e.g., `squad-station status` reporting processing state, external monitoring tools parsing `agents --json`) requires the DB value, a schema migration and complete consumer audit will be needed. Lock this decision at the start of Phase 2.

---

## Sources

### Primary (HIGH confidence)
- Squad Station codebase v1.7 (direct inspection): `src/cli.rs`, `src/main.rs`, `src/commands/ui.rs`, `src/commands/wizard.rs`, `src/commands/init.rs`, `src/commands/helpers.rs`, `src/tmux.rs`, `src/db/agents.rs`, `src/db/migrations/`, `npm-package/bin/run.js`, `install.sh`
- `Cargo.lock` (local) — confirmed locked versions: clap 4.5.60, ratatui 0.30.0, crossterm 0.29, tokio 1.37
- Rust stdlib (stable since 1.70): `std::env::current_dir`, `Path::file_name`, `std::io::IsTerminal`, `OsStr::to_str` / `to_string_lossy`
- [cargo init — The Cargo Book](https://doc.rust-lang.org/cargo/commands/cargo-init.html) — folder-name-as-default convention
- [Command Line Interface Guidelines (clig.dev)](https://clig.dev/) — interactive/silent modes, flag conventions, TTY guard rationale

### Secondary (MEDIUM confidence)
- [tmuxcc GitHub](https://github.com/nyanko3141592/tmuxcc) — pane content pattern detection for AI agent state
- [TUICommander](https://tuicommander.com/) — agent status detection patterns, provider-specific regex approach
- [Ralph TUI](https://ralph-tui.com/) — task execution state visualization in TUI dashboards
- [tmux man page](https://man7.org/linux/man-pages/man1/tmux.1.html) — `capture-pane`, `display-message`, `pane_current_command`, session name restrictions
- clap docs — unrecognized subcommand error behavior, `Commands` derive macro
- tokio `process::Command` docs — non-blocking subprocess calls in async runtimes

### Tertiary (LOW confidence — validate during implementation)
- Provider-specific pane content patterns for Claude Code and Gemini CLI — documented from community observation; must be validated against real sessions before finalizing `classify_pane_output()`

---

*Research completed: 2026-03-18*
*Ready for roadmap: yes*
