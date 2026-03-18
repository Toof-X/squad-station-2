# Stack Research: v1.8 Install & Live Status

**Domain:** Rust CLI — install subcommand, folder-name defaulting, orchestrator pane-state detection
**Researched:** 2026-03-18
**Confidence:** HIGH

> **Scope:** This document covers only what is NEW or CHANGED for v1.8. The existing stack
> (ratatui 0.30, crossterm 0.29, tui-big-text 0.8, clap 4.5, tokio 1.37, sqlx 0.8,
> owo-colors 3, uuid 1.8) is validated and NOT re-researched here.

---

## Executive Summary

Zero new Rust crates are needed for v1.8. All three features are implementable with the
existing dependency set plus Rust stdlib primitives. The only integration-surface change
is in the npm layer: adding a `"scripts"` block to `package.json` so the `postinstall` hook
calls `squad-station install [--tui]` instead of running the binary directly. The curl
installer gets a matching one-line change.

---

## Recommended Stack

### Core Technologies — No Changes

| Technology | Version (locked) | Purpose | v1.8 Status |
|------------|-----------------|---------|-------------|
| clap 4 | 4.5.60 | CLI subcommand dispatch | Add `Install` variant to `Commands` enum — no version change, no new dependency |
| ratatui 0.30 | 0.30.0 | TUI rendering | Already used in `welcome.rs`, `ui.rs`, `wizard.rs` — no change |
| crossterm 0.29 | 0.29 | Terminal raw-mode, TTY detection | `std::io::IsTerminal` (stdlib) already used in `init.rs` — no change |
| tokio 1 | 1.37 | Async runtime | `tokio::time::interval` available for TUI polling loop — no change |
| sqlx 0.8 | 0.8.x | SQLite persistence | No schema change for these three features |
| std::env | stdlib | Folder-name defaulting | `current_dir()` + `Path::file_name()` — zero-cost, no crate needed |
| std::process::Command | stdlib | tmux capture-pane calls | Already used in `tmux.rs` for all tmux interaction — no change |

### Supporting Libraries — No Additions Required

| Library | Version | Purpose | v1.8 Relevance |
|---------|---------|---------|----------------|
| anyhow 1.0 | 1.0 | Error propagation | `install.rs` uses same `anyhow::Result<()>` pattern as all other commands |
| owo-colors 3 | 3.x | Colored output | Install banner reuses existing color helpers |
| uuid 1.8 | 1.8 | Temp file naming | No change — used in `inject_single`, not in install path |

---

## Feature-by-Feature Stack Analysis

### Feature 1: `squad-station install [--tui]` subcommand

**What it needs:**
- A new `Commands::Install { tui: bool }` variant in `cli.rs`
- A new `src/commands/install.rs` handler

**Implementation uses only stdlib:**
- `std::process::Command` — shell out to `curl` for binary download (same pattern as `tmux.rs`)
- `std::fs`, `std::path` — scaffold project files
- `std::io::IsTerminal` — already imported in `init.rs` for the `--tui` auto-launch guard

**Why `curl`, not `reqwest`:** The existing install flow in both `bin/run.js` and `install.sh`
already shells out to `curl`. Using `reqwest` would add ~1.5 MB to the release binary, require
async TLS setup, and complicate musl static builds — all for a single download call that
`curl` handles correctly on every target platform.

**npm `package.json` change — add a `scripts` block:**

```json
"scripts": {
  "postinstall": "node bin/run.js install --tui"
}
```

This routes the postinstall auto-launch through the Rust `install` subcommand. The `--tui`
flag tells the handler to launch the welcome TUI after install completes, replacing the
current `spawnSync` call in `proxyToBinary`. The `bin/run.js` `install()` function can stay
as a fallback for users who call `npx squad-station install` without the native binary present.

**`install.sh` change — one line:**

Replace the final auto-launch block:
```sh
# current:
exec "${INSTALL_DIR}/squad-station"

# v1.8:
exec "${INSTALL_DIR}/squad-station" install --tui
```

This makes the curl and npm install paths consistent: both call the Rust `install` subcommand,
which owns the post-install setup and TUI launch sequence.

---

### Feature 2: Folder name as default project name

**What it needs:** `std::env::current_dir()` and `Path::file_name()` — pure stdlib.

```rust
// Derive basename of cwd
let folder_name = std::env::current_dir()
    .ok()
    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    .unwrap_or_default();
```

**Integration points (all in existing files, no new files):**
- `wizard.rs` ProjectName page: pre-fill the input field with `folder_name` instead of an empty string
- `src/commands/init.rs` `generate_squad_yml`: use `folder_name` as the project field when the wizard result is empty
- Dashboard (`ui.rs`) and `status.rs`: already read `config.project` from squad.yml — no change needed after init writes the basename

**What NOT to use:** Do not add the `dirs` crate (removed in v1.4) or any path-manipulation
crate. The four lines above are the entire implementation. `to_string_lossy()` handles
non-UTF-8 path components safely.

---

### Feature 3: Orchestrator "processing" state via `capture-pane` polling

**What it needs:**
- A new `capture_pane(session_name: &str) -> anyhow::Result<String>` function in `tmux.rs`
- Pattern matching on pane content using stdlib string ops (`str::contains`, `str::lines()`)
- A polling interval in the TUI refresh cycle using `tokio::time::interval` (already available)

**Add to `tmux.rs`:**

```rust
pub fn capture_pane(session_name: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
```

This follows the same `std::process::Command` pattern used by every other tmux function in the
file. No new crate required.

**Detection heuristics use stdlib string ops:**
- Orchestrator idle: pane ends with a shell prompt (`$`, `>`, `%`) or known AI tool prompt
- Orchestrator typing/mid-input: pane last line is non-empty and does not match a completed-prompt pattern
- Orchestrator processing: known "Thinking..." or spinner indicator present in last N lines

If detection logic proves complex enough to require pattern matching, `regex` is available as a
dev dependency candidate — but start with `str::contains` and promote to `regex` only if needed.

**Polling in TUI:** The `ui.rs` event loop already has a tick interval. Extend the existing
`tokio::select!` branch to call `capture_pane` on the orchestrator session name at each tick.
No new async primitives are needed.

**What NOT to use:** Do not add `notify` (filesystem watcher) or any IPC crate. Pane polling
via `capture-pane` on a timer is the correct pattern for tmux-based state detection — the
orchestrator writes to its pane, not to a file.

---

## npm Package Changes

| File | Change | Reason |
|------|--------|--------|
| `package.json` | Add `"scripts": { "postinstall": "node bin/run.js install --tui" }` | Route postinstall through Rust `install` subcommand |
| `bin/run.js` | Remove or gate the `spawnSync` welcome-TUI auto-launch from `proxyToBinary` | Rust handler owns the install + launch sequence; JS should not duplicate it |
| `install.sh` | Change final `exec` from bare binary to `exec ... install --tui` | Consistent install path between npm and curl |

**No new npm dependencies.** The package has zero runtime Node deps by design. Node stdlib
(`child_process`, `fs`, `path`) handles everything.

---

## Cargo.toml Changes

**None.** All three features are implementable with the existing dependency set.

The `version` field bump (e.g., `0.5.3` → `0.6.0`) is a project management step, not a
stack change, and is not covered here.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `curl` via `std::process::Command` for download | `reqwest` async HTTP | ~1.5 MB binary size increase, musl TLS complications, not justified for a single one-time download |
| `std::env::current_dir()` for folder name | `dirs` crate | Removed in v1.4; stdlib is sufficient |
| `str::contains` for pane content detection | `regex` crate | Simple "is orchestrator typing?" heuristics do not require regex; add only if detection complexity grows |
| Extend `tokio::time::interval` in TUI tick | Separate background thread + `std::sync::mpsc` | TUI already has a tick loop; a channel + thread adds complexity with no benefit at this poll frequency |
| Rust `Commands::Install` variant | Keep install entirely in `bin/run.js` | JS cannot launch the Rust TUI cleanly in all environments; Rust owns the TUI, Rust should own the post-install launch sequence |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `reqwest` | Binary size (+1.5 MB), async TLS setup, musl build complications | `std::process::Command` shelling to `curl` — same pattern already in the codebase |
| `regex` (unless needed) | Overkill for initial pane content matching | `str::contains`, `str::ends_with`, `str::lines()` iteration |
| `dirs` crate | Removed in v1.4; no home-dir resolution needed | `std::env::current_dir()` for folder name |
| Any npm runtime dependency | npm wrapper has zero runtime deps by design | Keep it; Node stdlib is sufficient |
| A dedicated `install` npm sub-package | Unnecessary complexity | Single `bin/run.js` with routing already handles this |

---

## Version Compatibility

All existing crate versions are compatible with the three new features. No version bumps needed.

| Package | Locked Version | Compatibility Note |
|---------|---------------|-------------------|
| clap 4.5 | 4.5.60 | New `Install` subcommand variant is a trivial derive addition — no API change |
| ratatui 0.30 | 0.30.0 | `capture_pane` result displayed in existing TUI — same widget API |
| tokio 1.37 | 1.37 | `tokio::time::interval` stable since tokio 1.0 |
| crossterm 0.29 | 0.29 | No change; `std::io::IsTerminal` (stdlib) handles TTY guard for install subcommand |

---

## Sources

- `Cargo.lock` (local) — confirmed locked versions: clap 4.5.60, ratatui 0.30.0, crossterm 0.29, tokio 1.37
- `src/cli.rs` — confirmed `Commands` enum structure for `Install` variant placement
- `src/tmux.rs` — confirmed `std::process::Command` pattern; all tmux calls follow the same shape as the new `capture_pane` function
- `bin/run.js` — confirmed current npm install + auto-launch flow targeted for migration
- `install.sh` — confirmed curl installer final `exec` line targeted for migration
- `src/commands/init.rs` line 1 — confirmed `std::io::IsTerminal` already imported
- Rust stdlib (stable since 1.70) — `std::env::current_dir`, `Path::file_name`, `std::io::IsTerminal`

---

*Stack research for: squad-station v1.8 — install subcommand, folder-name defaulting, orchestrator pane-state detection*
*Researched: 2026-03-18*
