# Pitfalls Research

**Domain:** Rust CLI feature additions — `squad-station install` subcommand, folder-name-as-default, orchestrator pane polling (v1.8 milestone)
**Researched:** 2026-03-18
**Confidence:** HIGH — based on direct codebase inspection (`cli.rs`, `main.rs`, `init.rs`, `config.rs`, `tmux.rs`, `ui.rs`, `npm-package/bin/run.js`, `install.sh`) plus patterns established in v1.5–v1.7 development

---

## Scope of This Document

This file covers pitfalls **specific to v1.8**: adding three features to the existing, stable v1.7 codebase.

**Feature 1 — `squad-station install [--tui]` subcommand:**
The npm postinstall (run.js) and curl installer (install.sh) currently call the bare binary directly (`spawnSync(destPath, [])` and `exec "$INSTALL_DIR/squad-station"`). v1.8 replaces these bare calls with `squad-station install --tui` so the install path becomes explicit and testable.

**Feature 2 — Folder name as project name default:**
`std::env::current_dir()` basename is used as a fallback value for the project name field in the wizard's first page, the TUI dashboard title, and `squad.yml` generation. Currently the project name field is empty-string by default.

**Feature 3 — Orchestrator "processing" state via pane polling:**
The TUI dashboard (`ui.rs`) polls `tmux capture-pane -p` on each refresh interval to inspect the content of the orchestrator's tmux pane. If the captured content indicates active work (heuristic pattern match), the orchestrator's status is surfaced as "processing" in the UI — supplementing the existing `idle`/`busy`/`dead` states tracked in the DB.

The v1.7 codebase already has: a working welcome TUI with TTY guards, a multi-page ratatui wizard, npm/curl install auto-launch with `isTTY` / `[ -t 1 ]` checks, and 241 passing tests. The pitfalls below are additive — they do NOT repeat v1.7 pitfalls already documented.

---

## Critical Pitfalls

### Pitfall 1: `install` Subcommand Name Conflicts With npm `install` Command

**What goes wrong:**
Adding `Install` to the `Commands` enum in `cli.rs` means `squad-station install` now parses as a Rust subcommand. However, `npm-package/bin/run.js` currently intercepts `install` at the Node level before any binary call: `if (subcommand === 'install') { install(); }`. This means `npx squad-station install` routes to the JavaScript install function — it never reaches the Rust binary.

After v1.8, the intent is for `npx squad-station install` to eventually call the Rust `install` subcommand (or a variant of it). If `run.js` is updated to pass `install` through to the binary while also keeping the JS install logic, the two code paths conflict. The JS `install()` function downloads the binary; the Rust `install --tui` subcommand launches the welcome TUI. Calling the Rust binary before it exists (pre-download) causes a crash.

More immediately: changing `run.js` to pass `install` arguments through to the binary changes the contract for any user who currently runs `npx squad-station install`. This is a breaking change to a shipped, documented command.

**Why it happens:**
The `install` word is overloaded: it means "download binary + scaffold files" in npm context and "finish setup + show TUI" in Rust context. The two meanings are at the same subcommand level without a clear boundary.

**How to avoid:**
Keep the split clean. The Rust subcommand should be named `install` but its purpose is narrowly defined as "first-run setup assistant that can be called post-binary-install." The JS `install()` in run.js remains the binary download step and explicitly calls `squad-station install --tui` (the Rust subcommand) as its last step, gated on `isTTY`. This means `npx squad-station install` follows the JS path (download + scaffold + optionally call binary), while `squad-station install` (binary already present) runs the Rust setup flow. These are two different invocation contexts.

Document this split explicitly in code comments so future maintainers do not merge the two paths. The Rust subcommand should fail gracefully if called before `squad.yml` context is available (do not assume a project directory).

**Warning signs:**
- `npx squad-station install` silently skips the TUI because run.js intercepts it and the binary call never happens
- Users who already have the binary and run `squad-station install` hit a "command not found" error because `install` was not in the enum prior to v1.8
- `cargo test` must include a test for `Commands::Install` parsing to prevent regression

**Phase to address:** Phase 1 of v1.8 — the subcommand boundary between JS and Rust must be decided before any code is written.

---

### Pitfall 2: `std::env::current_dir()` Returns an Unusable Basename in Edge Cases

**What goes wrong:**
`std::env::current_dir()` can return a path whose `file_name()` component is:

1. **Empty or `"."`** — when the process is launched from the filesystem root (`/`) or a path that ends in `/`. `Path::file_name()` returns `None` for paths ending in `/` or for the root `/`.
2. **Non-UTF-8 bytes** — Linux allows directory names with arbitrary byte sequences. `OsStr::to_str()` returns `None` for non-UTF-8 directory names. `to_string_lossy()` replaces invalid bytes with `U+FFFD` (replacement character), producing a name like `my-proj???backend` which is accepted by the wizard but generates an ugly `squad.yml` and broken tmux session names.
3. **tmux-unsafe characters** — directory names can contain spaces, parentheses, brackets, slashes, and other characters that `sanitize_session_name()` does not currently handle. The existing sanitizer only replaces `.`, `:`, and `"` with `-`. A directory named `my project (v2)` would become `my project (v2)-orchestrator` as a tmux session name, which tmux rejects because spaces are not valid session name characters.
4. **Very long names** — tmux has a session name length limit (typically 127 characters on most systems, but the actual limit varies). A deeply nested directory with a long basename can exceed this limit, causing `tmux new-session` to fail silently or with a confusing error.
5. **Unicode names** — directory names like `我的项目` are valid UTF-8 and `to_str()` succeeds, but `sanitize_session_name()` passes them through unchanged. tmux on macOS handles Unicode session names in many cases, but the behavior is not guaranteed across versions and platforms.

**Why it happens:**
The wizard's project name field is user-editable, so developers test the happy path (user types a clean ASCII name). The auto-populated default is never tested with adversarial directory names because test environments use clean temp directories.

**How to avoid:**
Apply a two-stage defense:

1. **Derive the default safely:** Use `std::env::current_dir()?.file_name()?.to_string_lossy().into_owned()` but immediately sanitize the result with an extended sanitizer that also strips or replaces spaces, parentheses, brackets, and any character not in `[a-z0-9A-Z_-]`. This sanitized value is the default, not the raw basename.

2. **Let the user see and edit it:** The wizard pre-populates the field with the sanitized default but requires the user to confirm (or the field is already editable, which it is). A bad default does not corrupt `squad.yml` because the wizard is interactive.

3. **Handle the `None` case explicitly:** If `current_dir()` fails or `file_name()` is `None`, fall back to an empty string (existing behavior) or the string `"my-project"`. Do not panic or propagate the error — a missing default is a cosmetic issue, not a fatal one.

**Warning signs:**
- `squad-station init` crashes with "panicked at 'called `Option::unwrap()` on a `None` value'" when run from `/`
- tmux session names with spaces cause `tmux new-session` to fail with "invalid session name"
- Generated `squad.yml` contains `project: my project (v2)` which is valid YAML but breaks agent naming

**Phase to address:** Phase 1 of v1.8 (folder name default) — the sanitization and None handling must be part of the initial implementation. Do not add the default without the guards.

---

### Pitfall 3: `capture-pane` Polling Breaks When Orchestrator Is Not in a tmux Session

**What goes wrong:**
`tmux capture-pane -p -t <session-name>` requires that a tmux session with that name exists in the current tmux server. Three conditions cause it to fail:

1. **Orchestrator has `antigravity` provider:** The DB-only orchestrator never has a tmux session. `session_exists()` returns false, but the TUI polling logic needs to know NOT to attempt capture-pane for this agent. If the polling code looks up the orchestrator by role from the DB and issues capture-pane without checking `is_db_only()`, every poll cycle produces a tmux error.

2. **Orchestrator session was killed (status = `dead`):** The squad was `close`d or crashed. The orchestrator agent row still exists in the DB with status `dead`. If polling is based on the DB row's name (which it must be, since that's how the TUI gets agent data), the capture-pane call targets a non-existent session and fails. The TUI must handle `tmux capture-pane` failures gracefully without surfacing an error to the user.

3. **TUI is launched outside of tmux:** `squad-station ui` can be run directly in a terminal, not inside a tmux session. In this case, calling `Command::new("tmux").args(["capture-pane", ...])` spawns a tmux client that connects to the default server, which may or may not have the orchestrator's session. This works if tmux is running; it fails (with exit code 1) if tmux is not available at all, which is a recoverable situation the TUI already handles for session listing.

**Why it happens:**
The existing TUI data path (`fetch_snapshot`) only reads from SQLite. Adding a `tmux capture-pane` call introduces a new external dependency on tmux availability inside the data refresh loop. Developers who test with a running tmux server never see the failure case.

**How to avoid:**
Wrap capture-pane in an explicit guard chain before any call:

```rust
// 1. Skip if orchestrator is DB-only (antigravity)
if agent.tool == "antigravity" { return None; }

// 2. Skip if session does not exist
if !tmux::session_exists(&agent.name) { return None; }

// 3. Run capture-pane; treat any error as "unknown" (not "processing")
let output = Command::new("tmux")
    .args(["capture-pane", "-p", "-t", &agent.name])
    .output()
    .ok()?;
if !output.status.success() { return None; }
```

The return value of `None` means "could not determine processing state" — the TUI shows the existing DB-derived status (`idle`/`busy`/`dead`), not an error state. Never surface capture-pane failures as user-visible errors.

**Warning signs:**
- TUI shows error messages for every refresh cycle when using antigravity provider
- TUI shows spurious error messages after `squad-station close` kills sessions
- `squad-station ui` panics when tmux is not running at all

**Phase to address:** Phase 2 of v1.8 (pane polling) — guard chain must be written first, before pattern matching on captured content.

---

### Pitfall 4: False Positive "Processing" Detection From Pane Content

**What goes wrong:**
`tmux capture-pane -p` returns the visible text content of the pane — up to the terminal scroll buffer limit. Determining whether the orchestrator is "actively processing" from this raw text is inherently heuristic. Common false positive sources:

1. **Idle prompt lines that look like activity:** Claude Code's idle prompt shows `>` and a cursor. Gemini CLI's idle prompt shows `$` or a spinner that pauses. A naive heuristic like "content is non-empty" or "last line is not empty" always returns true because both tools show persistent prompt lines.

2. **Captured content includes old output:** `capture-pane` by default returns the last N lines of the pane. If the orchestrator finished a task 10 minutes ago and the pane shows the completed task output, a naive "contains active-looking text" heuristic returns true for a session that is actually idle.

3. **Tool-version-dependent patterns:** Claude Code's active state indicator (spinner, "Thinking...", etc.) changes between tool versions. A regex written against one version silently fails against another, causing the "processing" state to never appear for users on a different version.

4. **Multiple tool simultaneous output patterns:** Gemini CLI, Claude Code, and potentially future providers each have different UI text patterns for "thinking" vs. "idle." A single heuristic that works for one provider misclassifies the other.

**Why it happens:**
Scraping terminal UI output for semantic state is fundamentally unreliable. The tmux pane content is designed for human consumption, not machine parsing. Developers prototype the heuristic with their own active session and it "looks good" in testing, but real-world patterns are more varied.

**How to avoid:**
Use a conservative, provider-aware approach:

1. **Prefer the DB state over pane content.** If the DB says `idle` and pane content is ambiguous, show `idle`. Only override to `processing` if a high-confidence signal is present.

2. **Use provider-specific patterns, not a universal heuristic.** For `claude-code`, look for the "Thinking" or spinner ANSI sequences specific to that version. For `gemini-cli`, look for the running indicators. For unknown providers, do not attempt classification — show the DB state.

3. **Add a "last changed" timestamp check.** If `status_updated_at` changed in the last N seconds (e.g., 30 seconds), the DB state is fresh and trustworthy. Pane polling adds value mainly when the DB state has been `idle` for a long time but visual activity suggests the agent is running something not yet reflected in the DB.

4. **Expose the heuristic as a labeled annotation, not a status replacement.** Instead of changing the agent's `status` field to `processing`, add a separate TUI annotation: `[idle + active in pane]`. This prevents the display from overriding the authoritative DB state.

**Warning signs:**
- Orchestrator always shows "processing" even when idle (false positive loop)
- Orchestrator never shows "processing" even during active task execution (heuristic mismatch)
- Different behavior for Claude Code vs. Gemini CLI users (provider-specific pattern failure)
- After `squad-station close`, "processing" state persists in the TUI because the old pane content is cached

**Phase to address:** Phase 2 of v1.8 (pane polling) — the heuristic approach must be decided and documented before implementation. The decision whether to replace or annotate the DB status is an architectural choice, not an implementation detail.

---

### Pitfall 5: Pane Polling Adds tmux Subprocess Overhead to Every TUI Refresh Cycle

**What goes wrong:**
The existing TUI refresh loop in `ui.rs` has a 3-second interval. Each refresh calls `fetch_snapshot()` which opens a SQLite connection, reads two tables, and drops the connection. This is a purely local I/O operation with predictable latency (typically <10ms).

Adding `tmux capture-pane -p -t <session>` per refresh cycle spawns a child process for every agent that needs polling. For a squad with 5 agents, this is 5 `Command::new("tmux")` spawns every 3 seconds. Each spawn:

- Forks a process
- Connects to the tmux server socket
- Captures pane content (which can be several kilobytes)
- Exits

On macOS with tmux, this is approximately 20-40ms per call. Five calls = 100-200ms overhead per refresh cycle. At a 3-second interval, this is tolerable but visible as UI lag. If the polling target is only the orchestrator (not all agents), the overhead is one subprocess per refresh — acceptable.

A more serious issue: if tmux is slow to respond (under load, server busy), `capture-pane` can block for several hundred milliseconds. The TUI event loop is async, but `Command::output()` is a blocking call on the thread. In the current architecture (`fetch_snapshot` is `async fn` but uses `Command::new("tmux")` blocking calls), this blocks the async executor thread for the duration of the subprocess call.

**Why it happens:**
The existing `tmux::session_exists()` and related functions in `tmux.rs` all use `Command::new("tmux").output()` — blocking subprocess calls. The pattern is established and consistent. Adding capture-pane follows the same pattern without thinking about cumulative latency.

**How to avoid:**
1. **Poll only the orchestrator, not all agents.** The "processing" state is specifically for the orchestrator's pane. Workers' completion is detected via the hook-driven `signal` command, which is already authoritative. No worker pane polling is needed.

2. **Use a longer polling interval for pane content than for DB state.** DB state refreshes every 3 seconds (already implemented). Pane content can refresh every 10-15 seconds — the processing state is not time-critical at sub-10-second granularity. Separate the two refresh timers.

3. **Use `tokio::process::Command` instead of `std::process::Command` for the capture-pane call.** The codebase currently uses blocking `std::process::Command` for all tmux calls. This is acceptable for fast operations but risky for operations that might block. Using `tokio::process::Command::output().await` keeps the async executor unblocked.

4. **Cache the last captured pane content.** If the capture fails or is slow, use the previous captured content. Do not block the render cycle waiting for fresh pane content — stale data is better than a frozen TUI.

**Warning signs:**
- TUI refresh visibly lags (>500ms) after adding pane polling
- `squad-station ui` CPU usage spikes when tmux is under load
- TUI event loop drops key presses during refresh (key events handled after long blocking call)

**Phase to address:** Phase 2 of v1.8 (pane polling) — performance constraints must be specified before implementation. The two-timer approach (DB at 3s, pane at 10-15s) is a design decision that affects the App state struct and event loop structure.

---

### Pitfall 6: Backward Compatibility Break When npm/curl Call `install --tui` on Old Binary

**What goes wrong:**
After v1.8, `install.sh` and `run.js` will call `squad-station install --tui` instead of `squad-station` (bare). Users who have an older binary (pre-v1.8) installed at the same path will get a clap parse error:

```
error: unrecognized subcommand 'install'
```

This breaks the install flow for users upgrading from v1.7 via `npx squad-station install` because:
1. `run.js` downloads the new v1.8 binary to `destPath`
2. `run.js` calls `spawnSync(destPath, ['install', '--tui'], ...)` — this calls the NEW binary, which does support `install`

So actually the forward direction is fine — the new binary is downloaded first, then called. The backward direction — running old `install.sh` or old `run.js` with a new binary — is also fine.

The problematic case is: **the auto-launch call in the EXISTING run.js (v1.7)** runs `spawnSync(destPath, [])` (bare, no subcommand). If a user has already downloaded the new v1.8 binary (e.g., via curl) but still has the old npm package, the old run.js calls the new binary bare, which still works (bare invocation routes to the welcome TUI). No issue.

The actual risk is in the OTHER direction: if run.js is updated to call `squad-station install --tui` but is published BEFORE the binary binary is published (npm and GitHub Releases get out of sync), users who `npm install squad-station` get the new run.js, download the old binary, then run.js tries `squad-station install --tui` on the old binary — and gets a clap parse error.

**Why it happens:**
npm package versions and GitHub Release binary versions can diverge during the release process. The npm package is published via `npm publish` (manual or CI), and the binary is published via `softprops/action-gh-release` on tag push. If only one of the two is published (partial release), users hit version mismatches.

**How to avoid:**
In run.js, make the `install --tui` call conditional on the binary version supporting it:

```javascript
// Check if the binary supports the install subcommand
var versionResult = spawnSync(destPath, ['--version'], { encoding: 'utf8' });
var supportsInstall = /* parse major version >= 1.8 */;
var launchArgs = supportsInstall ? ['install', '--tui'] : [];
if (process.stdout.isTTY) {
    spawnSync(destPath, launchArgs, { stdio: 'inherit' });
}
```

Or: keep the bare call (`spawnSync(destPath, [], ...)`) in run.js and let the Rust binary route bare invocation to the correct flow (which it already does in v1.7+). The `install --tui` subcommand is then only called from documented manual usage, not from the auto-launch path. This eliminates the version coupling risk entirely.

**Warning signs:**
- `npx squad-station install` fails with "error: unrecognized subcommand 'install'" after an npm-only release
- CI reports a mismatch between npm package version and GitHub Release binary version
- Users on partial upgrades see clap errors instead of the welcome TUI

**Phase to address:** Phase 1 of v1.8 — release coordination must be planned before changing the auto-launch call in run.js. The simplest mitigation (keep bare invocation) should be the default unless there is a clear reason to use the subcommand form.

---

## Moderate Pitfalls

### Pitfall 7: Wizard Project Name Field With Pre-Populated Default Bypasses Validation

**What goes wrong:**
Currently, the project name field in the wizard starts empty and the user must type something. Validation runs when the user attempts to proceed: empty string is rejected.

With v1.8, the field is pre-populated with the sanitized folder basename. A user can simply press Enter on the first page without reviewing the pre-populated value. If the sanitized default produces a project name that is technically valid (non-empty) but semantically bad — e.g., a temp directory like `tmp` or `a` — the user proceeds without noticing.

More critically: if the pre-populated default has NOT been properly sanitized (due to a sanitizer gap — see Pitfall 2), the wizard accepts it at the UI level but it later breaks tmux session creation.

**How to avoid:**
- Show the pre-populated value highlighted (distinct color or `[default: <value>]` label) so users notice and consciously accept it
- Run the tmux session name validation (not just "non-empty" check) at wizard submit time, before writing `squad.yml`. The `sanitize_session_name()` function exists in `config.rs`; use it to pre-validate the default.
- Consider adding a minimum length check (project name ≥ 3 characters) to prevent single-character project names from passing.

**Warning signs:**
- Users create squads named `tmp`, `a`, or `1` because the default was from a temp directory
- tmux session creation fails after wizard completes successfully because the project name has unsafe characters

**Phase to address:** Phase 1 of v1.8 (folder name default) — validation must be reviewed when the default is introduced.

---

### Pitfall 8: `current_dir()` Returns Different Values in Test vs. Production Context

**What goes wrong:**
In `cargo test`, `std::env::current_dir()` returns the crate root (the directory containing `Cargo.toml`). Tests that exercise the folder-name-as-default logic will pick up `squad-station` (the repo name) as the default project name, not a test-specific name. This can produce:

1. Tests that appear to pass but are testing the wrong default
2. Tests that fail when run from a different directory (e.g., CI checks out to a different path)
3. Tests that write files to the real project directory instead of a temp directory, polluting the workspace

The existing wizard tests in `init.rs` use hardcoded `WizardResult` values (`make_wizard_result()`), so they are isolated. But any new test that exercises the "derive default from current_dir" code path must explicitly set the current directory or mock the call.

**How to avoid:**
- Extract the "derive project name default" logic to a pure function that accepts a `&Path` argument instead of calling `current_dir()` internally. Tests pass a controlled path; production code passes `std::env::current_dir()`.
- Never call `std::env::current_dir()` from inside a function that is tested in the unit test suite. Keep the current_dir call at the call site in the wizard's page initialization, and pass the result as a parameter.

**Warning signs:**
- Tests for folder-name-default pass locally but fail in CI (different working directory)
- `generate_squad_yml` tests produce different output depending on where `cargo test` is run
- Files are created in the repo root during tests (test wrote to `cwd` instead of a temp dir)

**Phase to address:** Phase 1 of v1.8 (folder name default) — the pure-function extraction must be done before tests are written, not as a refactor afterward.

---

### Pitfall 9: Pane Polling Introduces "Processing" State Into DB Status Contract

**What goes wrong:**
The current DB-level agent statuses are `idle`, `busy`, and `dead`. These are authoritative and written by the `signal` command (via `update_agent_status`). The `reconcile_agent_statuses` helper also updates them by checking tmux session liveness.

If `processing` becomes a fourth status string written to the DB (instead of just a TUI display label), it must be:
- Validated by `status_color()` in ui.rs (currently only handles `idle`, `busy`, and the default `dead`/unknown case)
- Handled by `colorize_agent_status` in helpers.rs
- Handled by the `status` command output
- Handled by the `agents` command JSON output (which external consumers might parse)
- Covered by migration if the DB schema adds a constraint on status values

If `processing` is only a TUI display annotation and is never written to the DB, none of the above applies — but this must be an explicit design decision, not an accident.

**How to avoid:**
Decide explicitly: is `processing` a DB status or a TUI-only overlay?

The recommended approach: **TUI-only overlay**. The DB status tracks hook-driven lifecycle (`idle`/`busy`/`dead`). The TUI optionally overlays a `processing` indicator from pane polling, displayed alongside but not replacing the DB status. This keeps the DB contract clean and avoids migration.

If `processing` must be in the DB (e.g., for the `status` command to report it), add it as a distinct DB status with a migration, update all status-handling code paths, and update the JSON output schema. Do this in a single change, not incrementally.

**Warning signs:**
- `squad-station status` shows a blank or `[unknown]` for agents in `processing` state because `status_color()` does not handle it
- External tools parsing `squad-station agents --json` fail to handle the new status string
- The TUI shows `processing` after the task completes because the DB was written with this value and `reconcile_agent_statuses` does not know how to transition out of it

**Phase to address:** Phase 2 of v1.8 (pane polling) — the DB status contract decision must be made before writing any pane polling code.

---

### Pitfall 10: `install` Subcommand Without a `squad.yml` Context May Hit DB Errors

**What goes wrong:**
Most existing subcommands (e.g., `ui`, `agents`, `status`) call `config::load_config()` to find `squad.yml` and then `config::resolve_db_path()` to get the SQLite path. If no `squad.yml` exists, these fail with "squad.yml not found."

The new `install` subcommand is designed to run in a project directory that does NOT yet have `squad.yml` — it is a pre-init step. If the implementation accidentally calls any code path that requires the config (e.g., if `install --tui` routes to the welcome TUI which then checks `std::path::Path::new("squad.yml").exists()`), it must handle the `None`/missing-config case gracefully, not panic or propagate an error.

The current welcome TUI flow in `main.rs` already handles this: `let has_config = std::path::Path::new("squad.yml").exists();` — it does not call `load_config()`. Any code in the `install` subcommand handler must follow the same pattern.

**Why it happens:**
Developers copy the `init` subcommand's handler as a starting point for `install`. `init` calls `config::load_config()` (after wizard generates the file). If the copy-paste is incomplete, `install` inherits the config-loading call before the file exists.

**How to avoid:**
The `install` handler must explicitly NOT call `config::load_config()` or any function that transitively calls it. The only DB operations allowed in the install flow are those that come AFTER the wizard generates `squad.yml` — which is the same flow as `init`. If `install` delegates to `init` internally (e.g., `commands::init::run()` is called from `commands::install::run()`), this is safe. If `install` tries to do something before calling init, it must stay config-free.

**Warning signs:**
- `squad-station install` in a fresh directory fails with "squad.yml not found"
- `squad-station install --tui` shows an error before the TUI appears
- Integration test for install fails when run in a temp directory with no squad.yml

**Phase to address:** Phase 1 of v1.8 (install subcommand) — the install handler must be designed config-free from the start.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use raw `current_dir().file_name()` without sanitization for the default | Simpler code | Spaces and tmux-unsafe chars break session creation silently | Never — sanitize at derivation time |
| Write `processing` to the DB without updating all status-handling code | Quick TUI label | Breaks status command, agents command JSON, and any external consumer | Never without a complete audit of all status consumers |
| Poll all agent panes, not just the orchestrator | Feature parity | 5x subprocess overhead per refresh; blocks async executor | Never — scope to orchestrator only |
| Use blocking `std::process::Command` for capture-pane in async TUI loop | Consistent with existing tmux.rs patterns | Blocks tokio executor thread during slow tmux calls | Acceptable only with a separate, longer poll interval (>10s) |
| Pre-populate project name from current_dir AND keep "non-empty" as the only validation | Quick feature | Users proceed with bad defaults like `tmp`, `1`, or names with unsafe chars | Never — add tmux-safe validation alongside the default |
| Call bare `squad-station` from run.js/install.sh instead of `squad-station install --tui` | Backward compatible auto-launch | Loose coupling — install path does not benefit from future install subcommand improvements | Acceptable as a conservative v1.8 approach |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| npm run.js install + new `install` subcommand | Change `spawnSync(destPath, [])` to `spawnSync(destPath, ['install', '--tui'])` before verifying binary version support | Gate the subcommand call on version detection OR keep bare invocation |
| curl install.sh + new `install` subcommand | Append `exec "$INSTALL_DIR/squad-station" install --tui` at end of install.sh | If binary may be pre-v1.8 (cached download), keep bare exec; only use subcommand form if version check passes |
| `tmux capture-pane` in async TUI loop | Call `std::process::Command::output()` (blocking) inside an async fn | Use `tokio::process::Command` or ensure the call is on a separate timer with longer interval |
| Antigravity orchestrator + pane polling | Attempt capture-pane on orchestrator that has no tmux session | Check `agent.tool == "antigravity"` before any capture-pane call |
| `current_dir()` in unit tests | Test code calls the default-derivation function directly | Extract to pure fn accepting `&Path`; tests pass a controlled temp path |
| `processing` status in DB | Add status value without updating `status_color()`, `colorize_agent_status()`, JSON output | Treat `processing` as TUI-only OR update all status consumers atomically |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Polling all agents via capture-pane every 3 seconds | TUI refresh lag >200ms; CPU spike on refresh | Scope polling to orchestrator only; use 10–15s interval for pane content | Immediately with >3 agents |
| Blocking `std::process::Command` for capture-pane in tokio async fn | Key press events dropped during long tmux calls | Use tokio::process::Command or a dedicated OS thread via tokio::task::spawn_blocking | When tmux is under load (>100ms response) |
| Calling `session_exists()` before capture-pane when session list is stale | Extra subprocess overhead; no actual safety benefit if list was cached | Session existence check is cheap; keep it as first guard | Not a performance trap — keep the check |
| Caching pane content in App state without expiry | "Processing" state persists after session is killed | Include a capture timestamp; invalidate cache on session death | After `squad-station close` |

---

## "Looks Done But Isn't" Checklist

- [ ] **`install` subcommand in cli.rs:** Verify `squad-station install --help` works; verify `squad-station install` in a directory with no squad.yml does not panic or error on config load
- [ ] **Folder name default — None safety:** Verify `squad-station init` when run from `/` (root) does not panic; `file_name()` returns `None` for root path
- [ ] **Folder name default — tmux safety:** Verify project name derived from a directory with spaces (e.g., `/tmp/my project`) produces a valid tmux session name (no spaces in session name after sanitization)
- [ ] **Folder name default — UTF-8 edge case:** Verify non-UTF-8 directory name falls back gracefully without replacement-character artifacts in squad.yml
- [ ] **Pane polling — antigravity guard:** Verify TUI does not attempt capture-pane when orchestrator has `tool = "antigravity"`; no error logged during refresh
- [ ] **Pane polling — dead session guard:** Verify TUI does not attempt capture-pane when orchestrator has `status = "dead"`; no error logged during refresh
- [ ] **Pane polling — `processing` scope:** Verify `processing` state does not appear in `squad-station agents --json` output unless explicitly designed to do so
- [ ] **npm install backward compat:** Run `npx squad-station install` with the updated run.js against a v1.7 binary — must not fail with clap parse error
- [ ] **241 existing tests still pass:** Run `cargo test` after all v1.8 changes; no new test failures; test output is not garbled

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Install subcommand breaks npx auto-launch (clap parse error) | MEDIUM | Revert run.js to bare `spawnSync(destPath, [])` call; republish npm package; binary stays unchanged |
| Wizard populates bad folder-name default, breaks tmux session | LOW | User edits project name in wizard before proceeding; sanitizer fix in next patch |
| Pane polling causes TUI lag | LOW | Increase poll interval from 3s to 15s for capture-pane; no DB or API changes needed |
| `processing` written to DB without updating status consumers | HIGH | DB migration to remove or rename the status; update all consumers; re-release |
| False positive "processing" confuses users | LOW | Remove or restrict the heuristic patterns; add provider check; no schema changes |
| current_dir() panic from None file_name | LOW | Add `?` / `.unwrap_or_default()` at derivation; patch release |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| `install` subcommand name conflict with npm `install` | Phase 1: subcommand design — decide JS vs Rust boundary before writing code | `npx squad-station install` and `squad-station install` both work without conflict |
| `current_dir()` edge cases (None, non-UTF-8, unsafe chars) | Phase 1: folder-name default — sanitizer and None guard at derivation | Tests with `/`, space-in-path, non-UTF-8 path all pass without panic or corrupted output |
| capture-pane called when no tmux session exists | Phase 2: pane polling — guard chain as first code written | `squad-station ui` with antigravity provider shows no errors; `squad-station ui` after `close` shows no errors |
| False positive "processing" from pane content | Phase 2: pane polling — heuristic design document before implementation | Manual test: idle orchestrator shows `idle`, not `processing`; active orchestrator shows `processing` within 15s |
| capture-pane blocking async TUI executor | Phase 2: pane polling — separate interval timer for pane content | TUI refresh lag <100ms during capture-pane call; key press events not dropped |
| Backward compat break: old binary + new run.js | Phase 1: install subcommand — version check or keep bare invocation | `npm upgrade squad-station` does not produce clap errors; tested with both v1.7 and v1.8 binaries |
| Wizard project name default bypasses validation | Phase 1: folder-name default — validation review when default is introduced | Wizard rejects names with tmux-unsafe characters even when pre-populated |
| `processing` status breaks DB contract | Phase 2: pane polling — DB vs TUI-only decision before any code written | `squad-station agents --json` output schema unchanged unless DB approach is chosen |
| `install` handler calls config::load_config() before squad.yml exists | Phase 1: install subcommand — design handler as config-free | `squad-station install` in empty temp directory exits cleanly or launches TUI |

---

## Sources

- Squad Station codebase — direct inspection: `src/cli.rs`, `src/main.rs`, `src/commands/init.rs`, `src/commands/ui.rs`, `src/commands/wizard.rs`, `src/config.rs`, `src/tmux.rs`, `src/db/agents.rs`, `npm-package/bin/run.js`, `install.sh`
- Rust `std::path::Path::file_name()` docs — returns `None` for root paths and paths ending in `..`
- Rust `std::ffi::OsStr::to_str()` docs — returns `None` for non-UTF-8 byte sequences
- tmux man page — session name restrictions: no spaces, limited character set, max length implementation-defined (typically 127 chars)
- clap docs — unrecognized subcommand error behavior
- tokio `process::Command` vs `std::process::Command` — blocking behavior in async runtimes
- Existing v1.7 PITFALLS.md — TTY guard, npm postinstall, and curl stdin pitfalls (do not re-address here)

---
*Pitfalls research for: Squad Station v1.8 — install subcommand, folder-name default, orchestrator pane polling*
*Researched: 2026-03-18*
