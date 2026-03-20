# Pitfalls Research

**Project:** Squad Station (Rust CLI + embedded SQLite + tmux automation + npm distribution)
**Researched:** 2026-03-06
**Overall Confidence:** HIGH (all claims verified against official docs, issue trackers, or primary sources)

---

## SQLite Pitfalls

### CRITICAL — Pitfall 1: SQLITE_BUSY on Concurrent Hook Invocations

**What goes wrong:** Multiple agent hooks fire nearly simultaneously (e.g., two agents complete tasks within milliseconds of each other). Each hook spawns a new `squad-station signal` process. Both processes attempt to open a write transaction on the same `.db` file. The second process gets `SQLITE_BUSY` immediately — or after a very short default timeout — and exits with an error, silently dropping the signal.

**Why it happens:** SQLite allows only one writer at a time even in WAL mode. A stateless CLI binary opens a brand-new connection per invocation. Without an explicit `PRAGMA busy_timeout`, the default rusqlite timeout is 5000ms — but this value is not guaranteed to persist and may change between rusqlite versions. More dangerously, if you omit the pragma entirely in early prototypes, the default may be 0ms.

**Consequences:** Dropped signals. Orchestrator never learns an agent finished. Deadlock in multi-agent workflow. Extremely hard to reproduce since it is timing-dependent.

**Warning signs:**
- Intermittent "database is locked" messages in hook stderr
- Agent status stuck as "busy" after the agent visibly went idle
- Bugs only appear when 3+ agents finish simultaneously in tests

**Prevention:**
1. Enable WAL mode once at DB creation time: `PRAGMA journal_mode=WAL;` — this setting is sticky across connections, so it only needs to be set once per `.db` file.
2. Set `PRAGMA busy_timeout=5000;` (5 seconds) on every connection open — do NOT rely on rusqlite's default.
3. Use `BEGIN IMMEDIATE` for all write transactions. Do not start a `BEGIN DEFERRED` and then upgrade to a write — this upgrade fails immediately with `SQLITE_BUSY` if any other writer is active.
4. Keep write transactions minimal: one logical operation per transaction, no reads followed by writes in the same transaction.

**Phase:** Must be addressed in the SQLite foundation phase (Phase 1 / core infrastructure). Non-negotiable before any hook integration.

**Sources:** [SQLite WAL Docs](https://sqlite.org/wal.html) | [SQLITE_BUSY deep dive](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/) | [rusqlite Connection docs](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html)

---

### CRITICAL — Pitfall 2: PRAGMA journal_mode=WAL Cannot Live Inside a Migration Transaction

**What goes wrong:** You use `rusqlite_migration` and put `PRAGMA journal_mode=WAL;` inside a migration. The library wraps all migrations in a single transaction. SQLite silently ignores (or errors) PRAGMAs with side effects run inside a transaction — `journal_mode=WAL` is one of them.

**Why it happens:** `rusqlite_migration` issue #4 explicitly documents this: PRAGMA statements with side effects cannot be run inside a transaction block, but the library runs all migrations inside one big transaction by default.

**Consequences:** WAL mode silently never gets enabled. You don't notice until production when concurrent writes start failing under load.

**Warning signs:**
- `PRAGMA journal_mode;` returns `delete` instead of `wal` after migrations run
- No error during startup, failure only appears under concurrent load

**Prevention:**
- Run `PRAGMA journal_mode=WAL;` as a one-time setup step *before* running migrations, directly after opening the connection for the first time on a new database.
- Do not include WAL or `synchronous` in any migration SQL.

**Phase:** Phase 1 — DB initialization bootstrap sequence.

**Sources:** [rusqlite_migration issue #4](https://github.com/cljoly/rusqlite_migration/issues/4) | [WAL mode persistence](https://sqlite.org/wal.html)

---

### MODERATE — Pitfall 3: Schema Migration Without Version Guard Causes Rewrite Pain

**What goes wrong:** Early prototype hardcodes `CREATE TABLE IF NOT EXISTS` on every startup without tracking schema version. When schema needs to change (e.g., adding `last_heartbeat_at` column), you have no migration path. Every existing user's `.db` file is on old schema.

**Why it happens:** Rushing to prove the concept without wiring up schema versioning.

**Consequences:** Breaking change on binary upgrade. Users must manually delete their `~/.agentic-squad/<project>/station.db` or encounter panics / "no such column" errors.

**Warning signs:**
- Schema change needed but no `PRAGMA user_version` is set in the existing code
- `ALTER TABLE` added without migration guard

**Prevention:**
- Use `rusqlite_migration` from day one. Wire it up in Phase 1 even if there is only one migration.
- Treat every schema change as a versioned migration. Never use bare `CREATE TABLE IF NOT EXISTS` as your upgrade mechanism.
- Set `PRAGMA user_version` after each migration via rusqlite_migration's built-in mechanism.

**Phase:** Phase 1 (must be in foundation). Retrofitting migrations later causes compatibility breaks.

**Sources:** [rusqlite_migration crate](https://github.com/cljoly/rusqlite_migration) | [user_version strategy](https://levlaz.org/sqlite-db-migrations-with-pragma-user_version/)

---

### MODERATE — Pitfall 4: WAL File Growing Unbounded (Checkpoint Starvation)

**What goes wrong:** A long-running read transaction (e.g., `squad-station ui` TUI that holds a read handle open while displaying a dashboard) prevents SQLite from completing a WAL checkpoint. The WAL file grows indefinitely. Read performance degrades.

**Why it happens:** SQLite's checkpointer cannot reclaim WAL file space if any reader holds a snapshot from before the checkpoint point. A TUI holding an open connection for its refresh loop is exactly this pattern.

**Consequences:** Disk usage grows slowly. On developer machines this is tolerable, but it signals a deeper connection management flaw.

**Warning signs:**
- `~/.agentic-squad/<project>/station.db-wal` file growing beyond a few MB
- TUI refresh loop holding connections open across refresh cycles

**Prevention:**
- TUI should open a short-lived connection per refresh, not hold one open persistently.
- Alternatively, use `PRAGMA wal_autocheckpoint=100;` (default) and ensure all read transactions are properly closed.
- Do not use connection pools or long-lived connection objects across the UI refresh loop.

**Phase:** Phase N (TUI / dashboard). Early phases unaffected since they are truly stateless.

**Sources:** [SQLite WAL checkpoint starvation](https://sqlite.org/wal.html) | [Fly.io WAL internals](https://fly.io/blog/sqlite-internals-wal/)

---

## tmux Automation Pitfalls

### CRITICAL — Pitfall 5: Shell Initialization Race Condition on Session Creation

**What goes wrong:** `squad-station send` creates a new tmux session and immediately calls `tmux send-keys` to inject the agent prompt. The shell (zsh/bash with plugins like `oh-my-zsh`, `starship`, `nvm`) has not finished initializing. The injected keystrokes either get lost, appear as literal text before the prompt, or execute before the shell is ready. The agent never actually starts.

**Why it happens:** `tmux new-session` returns as soon as the pane is created, not when the shell is ready. This is a documented race condition in Claude Code's own agent system (issue #23513).

**Consequences:** Agent session is created in the registry but the agent process never actually started. Orchestrator sends tasks to a dead agent. No error is surfaced.

**Warning signs:**
- Agent session exists in tmux (`tmux list-sessions`) but no process is running inside
- `capture-pane` shows partial shell initialization output instead of a prompt
- Bug is machine-specific: affects users with heavy shell configs (oh-my-zsh, nvm, rbenv)

**Prevention:**
- Use `tmux new-session -d` to create detached, then poll for shell readiness before sending keys.
- Readiness detection: use `tmux display-message -p -t <session> '#{pane_current_command}'` in a loop until it returns the expected shell name (e.g., `zsh`), with a short sleep and timeout.
- Alternatively, use `tmux new-session -d 'command-to-run'` to pass the agent command directly as the initial process — avoids shell init entirely, but requires knowing the full invocation upfront.
- Add a configurable `send_delay_ms` option in `squad.yml` as a fallback escape hatch for users with slow shell init.

**Phase:** Phase 1 / core `send` command. This is a foundational correctness issue.

**Sources:** [Claude Code tmux race condition issue #23513](https://github.com/anthropics/claude-code/issues/23513) | [tmux send-keys async issue #1517](https://github.com/tmux/tmux/issues/1517)

---

### CRITICAL — Pitfall 6: Special Character Injection Breaking Agent Prompts

**What goes wrong:** The Orchestrator's task message injected via `tmux send-keys` contains characters that tmux interprets as commands: semicolons (`;`), escape sequences, backticks, single quotes. Tmux parses a trailing semicolon as a command separator — the rest of the string becomes a new tmux command, not part of the message. The injected text is corrupted or truncated.

**Why it happens:** `tmux send-keys` performs its own parsing pass on the string before sending it to the terminal. Semicolons, in particular, are parsed as command terminators at the tmux level even when shell-quoted.

**Consequences:** Garbled task messages. Potential for unintended tmux commands to execute. Agent receives incomplete instructions.

**Warning signs:**
- Messages with semicolons get split in the target pane
- Quoted strings lose their spaces (multiple words passed without quotes drop the delimiter)
- `\;` in messages causes surprising behavior

**Prevention:**
- Always use `tmux send-keys -l` (literal mode) when sending arbitrary text. The `-l` flag disables tmux key lookup and treats the string as raw characters, bypassing tmux's special character parsing.
- For the Enter key, send it as a separate `tmux send-keys -t <target> '' Enter` call after sending the literal message content.
- Escape the message content at the Rust level before constructing the `Command` call: replace or quote characters that tmux's argument parser may misinterpret.
- Integration test: send a message containing `; ls /tmp;`, single quotes, backticks, and newlines. Verify agent receives them verbatim.

**Phase:** Phase 1 / core `send` command. Must be caught before any agent workflow testing.

**Sources:** [tmux semicolon parsing issue #1849](https://github.com/tmux/tmux/issues/1849) | [tmux send-keys spaces issue #1425](https://github.com/tmux/tmux/issues/1425) | [tmux issue #4350](https://github.com/tmux/tmux/issues/4350)

---

### MODERATE — Pitfall 7: capture-pane Output Is Brittle for Structured Parsing

**What goes wrong:** `squad-station view` or the Orchestrator using `tmux capture-pane` to read agent output encounters ANSI escape codes, terminal control sequences, wrapped lines, and trailing spaces. If you try to parse agent output by grepping for specific strings (e.g., "Task complete"), escape codes embedded in the output break the match.

**Why it happens:** By default, `tmux capture-pane` strips ANSI codes. With `-e`, it preserves them but they appear as raw escape sequences. AI coding tools (Claude Code, Gemini CLI) use rich terminal output with colors, progress spinners, and cursor manipulation.

**Consequences:** Completion detection via output pattern matching is unreliable. Hook-based completion (which does not depend on output parsing) avoids this problem — but if anyone adds output-parsing as a fallback, it breaks.

**Warning signs:**
- Pattern matches against captured output fail intermittently
- Output looks correct when viewed in terminal but grep fails against the captured text
- Long lines appear duplicated or wrapped when captured

**Prevention:**
- Do NOT use `capture-pane` output parsing as the primary mechanism for detecting agent completion. Rely exclusively on the hook-driven signal system.
- If `capture-pane` is used for display purposes (TUI dashboard), use `capture-pane -p` (print to stdout) without `-e`, accepting that colors are stripped for display.
- For the `split view` feature, use `tmux join-pane` or `tmux link-window` to display the actual live pane rather than a captured copy.

**Phase:** Phase N (TUI/view features). Primary `signal` mechanism is immune. Note this risk in TUI phase planning.

**Sources:** [tmux ANSI issue #3401](https://github.com/tmux/tmux/issues/3401) | [tmux ANSI filtered issue #2254](https://github.com/tmux/tmux/issues/2254)

---

### MODERATE — Pitfall 8: Agent Lifecycle Detection False Negatives

**What goes wrong:** `squad-station` marks an agent as "idle" or "dead" based on tmux session/pane state, but the detection is wrong:
- `tmux has-session` returns 0 (success) even if the session exists but the agent process inside has exited — the shell is still running.
- `#{pane_dead}` only returns true when the pane's *shell itself* exits, not when the agent subprocess inside the shell exits.

**Why it happens:** tmux tracks pane liveness at the shell level, not the subprocess level. An agent that crashes but leaves a shell prompt open is "alive" to tmux.

**Consequences:** Dead agents appear as idle in the registry. Orchestrator assigns tasks to agents that cannot process them.

**Warning signs:**
- Agent shows as idle but no response to sent tasks
- `tmux display-message -p '#{pane_current_command}'` returns `zsh` or `bash` instead of the agent tool name when agent is supposedly running

**Prevention:**
- Use `tmux display-message -p -t <session> '#{pane_current_command}'` to check the *current running command* in the pane. If it returns `zsh`/`bash` (the shell), the agent process has exited.
- Define "agent alive" as: session exists AND `pane_current_command` equals the expected agent binary name.
- Implement a heartbeat timeout in the DB: if `last_signal_at` has not updated in N minutes and the pane shows the shell, mark the agent as dead.
- The hook system provides the authoritative liveness signal — if a hook fires, the agent was alive at that moment.

**Phase:** Phase 2 / agent lifecycle management. Build this before status reporting is used by Orchestrators.

**Sources:** [tmux pane_dead detection](https://tmuxai.dev/tmux-respawn-pane/) | [tmux has-session](https://davidltran.com/blog/check-tmux-session-exists-script/)

---

## Hook System Pitfalls

### CRITICAL — Pitfall 9: Orchestrator Hook Triggers Infinite Signal Loop

**What goes wrong:** The hook is installed globally (in `~/.claude/settings.json` or equivalent). When the Orchestrator itself finishes a task, its Stop/PostToolUse hook fires. The hook calls `squad-station signal`. This could trigger the Orchestrator to re-evaluate and start a new task, which finishes, fires the hook again — infinite loop.

**Why it happens:** The hook system is provider-level (it fires for ALL sessions of that provider, including the Orchestrator session). Squad Station's hook does not distinguish between Orchestrator and agent by default. This is documented as a known problem in Claude Code's own hook system (issue #3573, bug in claude-flow project issue #427).

**Consequences:** Orchestrator fork-bombs itself. Rapidly fills SQLite with spurious signal records. CPU runaway. User loses control of their terminal.

**Warning signs:**
- Orchestrator session starts rapidly looping after hook is installed
- `station.db` grows rapidly with thousands of signal records
- CPU spikes in the `claude` or `gemini` process after hook installation

**Prevention:**
- The `squad-station signal` command must check the invoking session name against the Orchestrator's session name from the registry. If the session is the Orchestrator, exit 0 silently.
- `tmux display-message -p '#S'` from within the hook gives the current session name. The binary must check this against the `orchestrator` record in SQLite.
- For Claude Code specifically, check `stop_hook_active` field in hook JSON input — if true, exit 0 immediately (this is the official guard pattern).
- Document this clearly: `squad-station init` must register the Orchestrator session name at setup time so the skip logic has something to check against.

**Phase:** Phase 1 — the skip guard must exist BEFORE the hook is documented or distributed. Shipping hooks without this guard is dangerous.

**Sources:** [Claude Code infinite loop issue #10205](https://github.com/anthropics/claude-code/issues/10205) | [claude-flow infinite recursion issue #427](https://github.com/ruvnet/claude-flow/issues/427) | [Claude Code hooks guide](https://code.claude.com/docs/en/hooks-guide) | [Steve Kinney hook control flow](https://stevekinney.com/courses/ai-development/claude-code-hook-control-flow)

---

### MODERATE — Pitfall 10: Provider-Specific Hook Event Names Diverge

**What goes wrong:** Claude Code uses `Stop` event. Gemini CLI uses `AfterAgent` event (or similar). Codex/Aider may use different hooks entirely or none at all. A hook script written for Claude Code's JSON input schema fails silently when run under Gemini CLI because the JSON keys differ.

**Why it happens:** The project is explicitly provider-agnostic, but hook systems are per-provider and non-standardized. Each provider defines its own event names, JSON payload schema, and trigger conditions.

**Consequences:** Hook works for Claude Code users, silently fails for Gemini CLI users. Agent signals are never received for Gemini agents. Hard to debug because hook "runs" (exits 0) but sends wrong data.

**Warning signs:**
- Gemini CLI agents never transition to idle in the registry
- Hook is installed and runs (no error), but `station.db` shows no records for Gemini sessions

**Prevention:**
- Design `squad-station signal` to be invoked with explicit arguments: `squad-station signal --agent <name> --event completed`. The hook script itself handles the provider-specific JSON parsing and calls the binary with normalized arguments.
- Keep provider-specific glue in thin hook scripts (bash), not in the Rust binary. The binary is provider-agnostic.
- Ship example hook scripts for each supported provider (Claude Code, Gemini CLI) in documentation or as installable templates via `squad-station install-hooks`.
- Explicitly test each provider's hook invocation in the integration test suite.

**Phase:** Phase 2 / hook integration. Architect for this in Phase 1 by keeping the CLI interface provider-agnostic.

**Sources:** PROJECT.md context | [Claude Code hooks guide](https://code.claude.com/docs/en/hooks-guide)

---

### MODERATE — Pitfall 11: Hook Output Errors Go to Wrong Stream

**What goes wrong:** The hook script writes error messages (e.g., "station.db not found") to stdout. The AI provider's hook runner captures stdout for structured data or ignores it. The user never sees the error. The hook silently fails.

**Why it happens:** Many CLI tools default to stderr for errors, but hook runners vary in how they surface each stream. Some providers use stdout exit codes; others parse stdout JSON.

**Consequences:** Hook errors are invisible. Users cannot debug why agents aren't being signaled.

**Prevention:**
- `squad-station signal` must write all diagnostic/error output to stderr.
- Exit with non-zero exit code on any error so the hook runner surfaces a failure.
- Design hook scripts to forward stderr from `squad-station` to the provider's error channel.
- In `--verbose` mode, emit structured logs to stderr for debugging.

**Phase:** Phase 1 — error handling contract must be established from the start.

---

## npm Distribution Pitfalls

### CRITICAL — Pitfall 12: Platform Package Publication Order Matters

**What goes wrong:** You publish the base `squad-station` npm package (which lists platform-specific packages as `optionalDependencies`) before publishing the platform packages themselves. npm resolves optional dependencies at install time — if `squad-station-darwin-arm64` does not exist in the registry yet, installation fails with a cryptic 404 error for every user who installs during that window.

**Why it happens:** CI pipelines often build and publish in a single job. If the base package publishes first (alphabetically, or because the platform build step is separate), users hit a broken state. This is documented as a specific failure mode by Orhun's packaging guide.

**Consequences:** New release is broken for all users for however long it takes to notice and republish.

**Prevention:**
- CI pipeline must publish ALL platform-specific packages first, wait for them to be resolvable (add a brief verification step), then publish the base wrapper.
- Use a single GitHub Actions workflow with explicit dependency between jobs: `publish-platform-packages` → `publish-base-package`.
- Test the full install flow in a clean environment as a post-release smoke test.

**Phase:** Phase N / npm distribution. But design the CI structure with this ordering constraint from the first release.

**Sources:** [Orhun's npm packaging guide](https://blog.orhun.dev/packaging-rust-for-npm/) | [Sentry binary publishing guide](https://sentry.engineering/blog/publishing-binaries-on-npm)

---

### CRITICAL — Pitfall 13: Executable Permissions Lost in CI Upload/Download

**What goes wrong:** GitHub Actions artifacts or npm tarballs strip executable bits from binary files. The published binary is not executable on the user's machine. Installation appears to succeed, but running `squad-station` produces "permission denied."

**Why it happens:** GitHub's `upload-artifact` / `download-artifact` actions do not preserve Unix file permissions. This is a well-documented issue. npm tarballs can also strip execute bits depending on how they are created.

**Consequences:** Binary installs but cannot run. Error is confusing ("permission denied" is not obviously a packaging bug).

**Warning signs:**
- `ls -la $(which squad-station)` shows mode `644` instead of `755`
- Users report "permission denied" after successful `npm install`

**Prevention:**
- In the postinstall script or npm wrapper's JS entrypoint: call `fs.chmodSync(binaryPath, 0o755)` after locating the binary.
- In CI, explicitly `chmod +x` the binary before creating the npm tarball.
- Add a smoke test in CI that installs the package and verifies the binary is executable before publishing.

**Phase:** Phase N / npm distribution. Catch this in the first packaging spike.

**Sources:** [Orhun's packaging guide — executable bits note](https://blog.orhun.dev/packaging-rust-for-npm/) | [Sentry guide](https://sentry.engineering/blog/publishing-binaries-on-npm)

---

### MODERATE — Pitfall 14: postinstall Script Disabled by Security-Conscious Environments

**What goes wrong:** Corporate environments, security-hardened CI systems, and some package managers disable `postinstall` scripts by default (`npm install --ignore-scripts`, `yarn --ignore-optional`). If your distribution strategy relies on a postinstall script to download the binary, those users get an empty install.

**Why it happens:** The npm ecosystem increasingly discourages postinstall scripts due to supply chain attack vectors. Organizations actively block them.

**Consequences:** Silent install failure. Binary is missing. Users get "command not found" with no clear explanation.

**Prevention:**
- Use the `optionalDependencies` strategy (napi-rs pattern) as the primary distribution mechanism — platform-specific packages are installed by npm's own resolver, no postinstall required.
- The postinstall script acts as a fallback only, with a clear error message explaining how to manually install if scripts are disabled.
- Document the `--ignore-scripts` limitation explicitly in the README.
- Consider the `napi-postinstall` helper package which handles legacy npm version quirks.

**Phase:** Phase N / npm distribution. Decide on primary mechanism (optionalDependencies vs postinstall) in Phase 1 design.

**Sources:** [napi-postinstall](https://www.npmjs.com/package/napi-postinstall) | [Tailwind optionalDependencies fallback PR #17929](https://github.com/tailwindlabs/tailwindcss/pull/17929) | [napi-rs/napi-rs issue #2569](https://github.com/napi-rs/napi-rs/issues/2569)

---

### MODERATE — Pitfall 15: macOS Cross-Compilation from Linux is Extremely Difficult

**What goes wrong:** CI runs on Linux (GitHub Actions default). Cross-compiling the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets from Linux requires the macOS SDK, which Apple does not freely license for use in Linux containers. The `cross` tool cannot provide Apple target images due to licensing. Ad-hoc solutions using `osxcross` are fragile and maintenance-intensive.

**Why it happens:** Apple explicitly restricts redistribution of their toolchain. Cross-compilation to Darwin targets requires licensing workarounds or native macOS runners.

**Consequences:** macOS builds break in CI as SDK versions age. Complex Dockerfile maintenance. Potential licensing violation if using redistributed SDKs.

**Prevention:**
- Use GitHub Actions `macos-latest` runners for Darwin target builds. They are free for public repos and provide the native SDK.
- Use `ubuntu-latest` runners for Linux target builds.
- Structure CI with a matrix strategy: one job per target platform using the native runner OS.
- Do not attempt Darwin cross-compilation from Linux.

**Phase:** Phase N / npm distribution and CI. Architecture decision made in CI design.

**Sources:** [Rust cross-compilation journey](https://blog.crafteo.io/2024/02/29/my-rust-cross-compilation-journey/) | [Cross-compilation post 2025](https://fpira.com/blog/2025/01/cross-compilation-in-rust)

---

### MINOR — Pitfall 16: npm Package Name Spam Filter Blocks Numeric Suffixes

**What goes wrong:** npm's spam detection system blocks package names containing numbers in certain patterns. Attempting to publish `squad-station-win32-x64` may be blocked, requiring renaming to `squad-station-windows-x64`.

**Why it happens:** npm applies heuristics to detect spam packages. Numeric patterns in scoped or unscoped package names trigger these filters.

**Prevention:**
- Use `windows` instead of `win32` in package names.
- Test package name availability before building CI pipelines around specific names.
- Have a contingency name (e.g., `squad-station-windows-amd64`) ready.

**Phase:** Phase N / npm distribution. Quick to discover, quick to fix. Just do not design CI around assumed names without testing.

**Sources:** [Orhun's npm packaging guide — naming constraints](https://blog.orhun.dev/packaging-rust-for-npm/)

---

## Multi-Process / Concurrency Pitfalls

### CRITICAL — Pitfall 17: Stale Agent Status After Crash / Restart

**What goes wrong:** An agent session crashes (or the user kills it with `tmux kill-session`). The agent's `status` column in SQLite remains `busy`. The hook never fired (agent died mid-task, no clean exit). On next startup, the Orchestrator queries `squad-station list` and sees a busy agent that is actually dead. Tasks sent to this agent are silently discarded by the tmux pane that no longer exists.

**Why it happens:** The hook-driven model is fundamentally optimistic — it only fires on clean completion. Crashes, kills, and `Ctrl-C` produce no hook event. The DB has no mechanism to detect this divergence.

**Consequences:** Workflow stalls. Orchestrator's agent pool appears full but no work is being processed.

**Warning signs:**
- tmux session does not exist (`tmux has-session` fails) but `squad-station list` shows the agent as busy
- No signals received from an agent that was marked busy for more than N minutes

**Prevention:**
- Implement a reconciliation step: every `squad-station list` (or on any invocation) cross-checks the DB status against the actual tmux session state. If the session is dead, auto-mark the agent as dead in the DB.
- Add a `last_seen_at` timestamp updated by every `signal` call. A busy agent with `last_seen_at` older than a configurable threshold (default: 10 minutes) is presumed dead.
- `squad-station gc` or `squad-station status --repair` command for manual reconciliation.

**Phase:** Phase 2 / agent lifecycle. Build reconciliation before the system is used in real workflows.

**Sources:** [Multi-agent stale state patterns](https://dev.to/uenyioha/porting-claude-codes-agent-teams-to-opencode-4hol) | PROJECT.md — "Agent lifecycle detection (idle/busy/dead)"

---

### MODERATE — Pitfall 18: Rust SIGPIPE Panic on Piped Output

**What goes wrong:** User runs `squad-station list | head -5`. When `head` closes its stdin after 5 lines, the pipe breaks. Rust's default behavior is to panic with "broken pipe" on the next `println!` call rather than exit cleanly. The user sees a panic backtrace instead of normal termination.

**Why it happens:** Rust ignores SIGPIPE at the OS level (since 2014) and instead returns a `BrokenPipe` error from IO operations. `println!` and related macros unwrap this error, causing a panic. This is a well-known, long-standing Rust issue (rust-lang/rust#46016, open since 2017).

**Consequences:** Confusing panic output in normal shell usage. Looks like a bug even though the behavior is "correct" functionally.

**Prevention:**
- Add a SIGPIPE handler at the top of `main()` that resets SIGPIPE to default behavior using `libc::signal(libc::SIGPIPE, libc::SIG_DFL)` (requires the `libc` crate).
- Or use the `nightly` `-Zon-broken-pipe=default` compiler flag once stabilized.
- Alternatively, replace all `println!` with `writeln!` to `stdout()` and explicitly handle `BrokenPipe` errors by exiting with code 0.
- Use the `pipecheck` or `calm_io` crates for a clean cross-platform solution.

**Phase:** Phase 1 / core CLI setup. One-time fix, add it before shipping any commands.

**Sources:** [Rust SIGPIPE issue #46016](https://github.com/rust-lang/rust/issues/46016) | [Rust unstable on_broken_pipe](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/on-broken-pipe.html) | [pipecheck crate](https://docs.rs/pipecheck/latest/pipecheck/)

---

### MODERATE — Pitfall 19: Race Condition Between `register` and First `send`

**What goes wrong:** Orchestrator calls `squad-station register` to add a new agent, then immediately calls `squad-station send` in the next shell command. On a heavily loaded machine, the `register` process may not have flushed its write to SQLite before `send` opens the DB. `send` looks up the agent, finds nothing, and errors out.

**Why it happens:** Stateless CLI means each command is a separate process. The OS makes no guarantee about when a previous process's writes become visible to the next process, especially if the write transaction was not explicitly committed with `SYNCHRONOUS` mode.

**Consequences:** Intermittent "agent not found" errors immediately after registration. Only appears under load or on slow storage.

**Prevention:**
- SQLite with WAL and `PRAGMA synchronous=NORMAL` provides sufficient durability — the transaction is visible to all subsequent connections after commit.
- Ensure `register` explicitly commits its transaction and does not rely on implicit commit on `Connection` drop (rusqlite `Connection::close()` drops without commit if auto-commit is not active).
- Add error message guidance: "if this error occurred immediately after `register`, wait 100ms and retry."

**Phase:** Phase 1. Simple to address with explicit transaction handling.

---

### MINOR — Pitfall 20: Multi-Project DB Path Collision on Non-Standard Home Directories

**What goes wrong:** DB path is hardcoded as `~/.agentic-squad/<project>/station.db`. On systems where `$HOME` is non-standard, symlinked, or mounted on a network filesystem, the path expands incorrectly or WAL mode fails (WAL requires shared memory, which does not work over NFS).

**Why it happens:** WAL mode requires `mmap` and shared memory between processes. Network-mounted filesystems (NFS, AFP, SMB) do not support this reliably.

**Consequences:** `SQLITE_IOERR_SHMOPEN` or similar errors on network home directories. Common in corporate/research environments with NFS home directories.

**Prevention:**
- Use `dirs::home_dir()` (from the `dirs` crate) for cross-platform home directory resolution rather than `~` expansion via shell.
- Allow `SQUAD_STATION_DATA_DIR` environment variable override for non-standard environments.
- In the error handler for WAL setup failure, provide a clear message: "WAL mode requires a local filesystem. Set SQUAD_STATION_DATA_DIR to a local path."

**Phase:** Phase 1 / DB initialization. Add env var override from the start.

---

## Prevention Summary

| Pitfall | Severity | Phase | Key Prevention |
|---------|----------|-------|----------------|
| SQLITE_BUSY concurrent hooks | CRITICAL | Phase 1 | WAL mode + `busy_timeout=5000` + `BEGIN IMMEDIATE` |
| PRAGMA WAL inside migration | CRITICAL | Phase 1 | Run WAL pragma before migrations, not inside them |
| Schema migration skipped | MODERATE | Phase 1 | Use `rusqlite_migration` from day one |
| WAL checkpoint starvation | MODERATE | TUI phase | Short-lived read connections in refresh loop |
| Shell init race on session create | CRITICAL | Phase 1 | Poll for shell readiness before `send-keys` |
| Special char injection via send-keys | CRITICAL | Phase 1 | Use `tmux send-keys -l` (literal mode) |
| capture-pane ANSI brittleness | MODERATE | TUI phase | Never parse capture-pane for completion detection |
| Agent lifecycle false negatives | MODERATE | Phase 2 | Check `pane_current_command`, not just session existence |
| Orchestrator infinite hook loop | CRITICAL | Phase 1 | Session name skip guard before any hook is shipped |
| Provider-specific hook divergence | MODERATE | Phase 2 | Provider-agnostic binary CLI, thin provider hook scripts |
| Hook errors to wrong stream | MODERATE | Phase 1 | All errors to stderr, non-zero exit on failure |
| npm publication order failure | CRITICAL | Dist phase | Publish platform packages first, base package last |
| Executable permission loss | CRITICAL | Dist phase | `chmod 0o755` in postinstall + CI pre-tarball |
| postinstall script disabled | MODERATE | Dist phase | optionalDependencies as primary, postinstall as fallback |
| macOS cross-compile from Linux | MODERATE | Dist phase | Use native macOS runners, not cross-compilation |
| npm name numeric suffix block | MINOR | Dist phase | Use `windows` not `win32` in package names |
| Stale agent status after crash | CRITICAL | Phase 2 | Reconcile DB status against live tmux state on every list |
| Rust SIGPIPE panic | MODERATE | Phase 1 | Reset SIGPIPE to SIG_DFL at top of main() |
| register/send race condition | MODERATE | Phase 1 | Explicit transaction commit, clear error messages |
| NFS WAL failure | MINOR | Phase 1 | env var override + clear error message |

### Phase-Gated Action Items

**Phase 1 — Must resolve before any integration testing:**
- WAL + busy_timeout setup on DB open
- `BEGIN IMMEDIATE` for all writes
- `rusqlite_migration` wired up
- Shell init readiness poll before `send-keys`
- `tmux send-keys -l` for all agent message injection
- Orchestrator skip guard in `signal` command
- SIGPIPE handler in `main()`
- All errors to stderr

**Phase 2 — Must resolve before multi-agent workflows:**
- Agent liveness reconciliation (DB vs tmux)
- Heartbeat timeout for dead agent detection
- Provider-agnostic hook script templates
- Stale status cleanup on `list` command

**Distribution Phase — Must resolve before any public release:**
- CI publishes platform packages before base package
- postinstall or optionalDependencies permission fix
- Native macOS runners for Darwin builds
- End-to-end install smoke test in clean environment

---

## Sources

- [SQLite WAL Documentation](https://sqlite.org/wal.html)
- [SQLite SQLITE_BUSY deep dive](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)
- [SQLite concurrent writes — SkyPilot Blog](https://blog.skypilot.co/abusing-sqlite-to-handle-concurrency/)
- [rusqlite Connection docs](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html)
- [rusqlite_migration crate](https://github.com/cljoly/rusqlite_migration)
- [rusqlite_migration PRAGMA issue #4](https://github.com/cljoly/rusqlite_migration/issues/4)
- [SQLite user_version strategy](https://levlaz.org/sqlite-db-migrations-with-pragma-user_version/)
- [WAL mode persistence — Simon Willison](https://til.simonwillison.net/sqlite/enabling-wal-mode)
- [tmux send-keys race condition — Claude Code issue #23513](https://github.com/anthropics/claude-code/issues/23513)
- [tmux send-keys async issue #1517](https://github.com/tmux/tmux/issues/1517)
- [tmux semicolon parsing issue #1849](https://github.com/tmux/tmux/issues/1849)
- [tmux spaces stripping issue #1425](https://github.com/tmux/tmux/issues/1425)
- [tmux send-keys semicolon issue #4350](https://github.com/tmux/tmux/issues/4350)
- [tmux ANSI escape in capture-pane issue #3401](https://github.com/tmux/tmux/issues/3401)
- [tmux ANSI filtered issue #2254](https://github.com/tmux/tmux/issues/2254)
- [tmux has-session session detection](https://davidltran.com/blog/check-tmux-session-exists-script/)
- [Claude Code hooks infinite loop issue #10205](https://github.com/anthropics/claude-code/issues/10205)
- [Claude Code Stop hook GitHub Actions loop issue #3573](https://github.com/anthropics/claude-code/issues/3573)
- [claude-flow infinite recursion issue #427](https://github.com/ruvnet/claude-flow/issues/427)
- [Claude Code hooks guide](https://code.claude.com/docs/en/hooks-guide)
- [Hook control flow — Steve Kinney](https://stevekinney.com/courses/ai-development/claude-code-hook-control-flow)
- [Orhun's npm Rust packaging guide](https://blog.orhun.dev/packaging-rust-for-npm/)
- [Sentry binary publishing guide](https://sentry.engineering/blog/publishing-binaries-on-npm)
- [binary-install helper](https://github.com/EverlastingBugstopper/binary-install)
- [napi-postinstall](https://www.npmjs.com/package/napi-postinstall)
- [Tailwind optionalDependencies fallback PR](https://github.com/tailwindlabs/tailwindcss/pull/17929)
- [napi-rs postinstall discussion](https://github.com/napi-rs/napi-rs/issues/2569)
- [Rust cross-compilation journey](https://blog.crafteo.io/2024/02/29/my-rust-cross-compilation-journey/)
- [Rust cross-compilation 2025](https://fpira.com/blog/2025/01/cross-compilation-in-rust)
- [Rust SIGPIPE issue #46016](https://github.com/rust-lang/rust/issues/46016)
- [Rust unstable on_broken_pipe](https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/on-broken-pipe.html)
- [pipecheck crate](https://docs.rs/pipecheck/latest/pipecheck/)
- [Multi-agent coordination — OpenCode](https://dev.to/uenyioha/porting-claude-codes-agent-teams-to-opencode-4hol)
- [devenv fork-bomb via hook re-evaluation](https://github.com/cachix/devenv/issues/2497)
