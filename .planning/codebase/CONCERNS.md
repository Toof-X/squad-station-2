# Codebase Concerns

**Analysis Date:** 2026-03-08

## Tech Debt

**Reconciliation loop duplicated across four commands:**
- Issue: The same 10-line tmux reconciliation pattern (check session_exists, update status to "dead" or "idle") is copy-pasted verbatim into four command files.
- Files: `src/commands/agents.rs` (lines 20-30), `src/commands/status.rs` (lines 36-43), `src/commands/context.rs` (lines 13-21), `src/commands/view.rs` (implicit via list_live_session_names)
- Impact: Any change to reconciliation logic (e.g. adding a new status, changing the "dead" detection heuristic) must be applied in multiple places. Risk of divergence.
- Fix approach: Extract a shared `reconcile_agent_statuses(pool: &Pool, agents: &[Agent]) -> Result<()>` function into `src/db/agents.rs` or a new `src/reconcile.rs` module; call it from each command.

**`pad_colored` utility duplicated across two command files:**
- Issue: Identical `pad_colored(raw, colored, width)` function exists in both `src/commands/list.rs` and `src/commands/agents.rs` (and mirrors the logic in `src/commands/status.rs`).
- Files: `src/commands/list.rs` (lines 88-92), `src/commands/agents.rs` (lines 91-95), `src/commands/status.rs` (lines 124-128)
- Impact: Any ANSI padding fix must be made three times.
- Fix approach: Move to `src/commands/mod.rs` as a shared utility, or into a new `src/format.rs` module.

**`format_status_with_duration` and `colorize_agent_status` duplicated:**
- Issue: These two display helpers are copy-pasted between `src/commands/agents.rs` and `src/commands/status.rs`.
- Files: `src/commands/agents.rs` (lines 63-88), `src/commands/status.rs` (lines 99-128)
- Impact: Duration format changes must be applied in two places; unit tests also duplicated.
- Fix approach: Consolidate into a shared display module (e.g. `src/format.rs`).

**`squad.yml` config hard-coded to CWD in every command:**
- Issue: Every command except `init` hard-codes `std::path::Path::new("squad.yml")` as a relative path in the current directory. There is no global `--config` flag for subcommands other than `init`.
- Files: `src/commands/send.rs` (line 9), `src/commands/signal.rs` (line 16), `src/commands/agents.rs` (line 7), `src/commands/status.rs` (line 23), `src/commands/list.rs` (line 12), `src/commands/peek.rs` (line 5), `src/commands/context.rs` (line 5), `src/commands/view.rs` (line 5), `src/commands/ui.rs` (line 261)
- Impact: Cannot run any command from outside the project directory. Makes shell scripting and CI usage awkward.
- Fix approach: Promote `--config` as a global flag in `src/cli.rs`, resolve and pass the path through `run()` in `src/main.rs`.

**Schema diverges from original design (documented in GAP-ANALYSIS.md):**
- Issue: The `messages` table uses `agent_name` (unidirectional) instead of the designed `from_agent`/`to_agent` bidirectional model. The `agents` table is missing `model`, `description`, and `current_task` FK columns. These gaps are acknowledged in `docs/GAP-ANALYSIS.md` but not yet resolved.
- Files: `src/db/migrations/0001_initial.sql`, `src/db/migrations/0002_agent_status.sql`, `src/db/agents.rs`, `src/db/messages.rs`
- Impact: Future features (bidirectional messaging, model-aware routing, task tracking on the agent row) will require another migration. Adding `current_task` FK retroactively is non-trivial in SQLite (no ADD CONSTRAINT).
- Fix approach: Resolve GAP-02 and GAP-03 from `docs/GAP-ANALYSIS.md` with migration 0003 before the schema grows larger.

**`serde-saphyr` is version `0.0.17` (pre-1.0, unstable crate):**
- Issue: The YAML parsing library `serde-saphyr` is pre-1.0 and has no stability guarantee. It is a thin wrapper around the `saphyr` parser which itself is a community fork of `yaml-rust`.
- Files: `Cargo.toml` (line 14)
- Impact: Breaking API changes can land in any patch release. No major ecosystem adoption for validation.
- Fix approach: Evaluate switching to `serde_yaml` (most common) or `serde_yml` once the config schema is stabilized.

## Known Bugs

**`signal` command outputs to stdout when called in hook context with no terminal:**
- Symptoms: When `rows == 0` (duplicate signal), the command prints "Signal acknowledged (no pending task for X)" to stdout even in non-terminal contexts, which leaks into the provider's stdin stream or logs.
- Files: `src/commands/signal.rs` (lines 124-133)
- Trigger: A second hook invocation on an already-completed task when stdout is not a terminal (e.g., piped or redirected).
- Workaround: Hook scripts pipe to `grep -i "warning|error" >&2` which suppresses this, but the output still passes through the binary.

**`create_view_window` does not check split-window exit status:**
- Symptoms: If adding panes 2+ to the view window fails (e.g., terminal too small), the error is silently swallowed.
- Files: `src/tmux.rs` (lines 139-145)
- Trigger: Running `squad-station view` with a terminal smaller than the required minimum for tiling.
- Workaround: None; partial pane layout is created silently.

**`status` command counts pending messages with limit 9999:**
- Symptoms: Counting pending messages per agent uses `list_messages(..., 9999)` and then `.len()` instead of a `COUNT(*)` SQL query. For agents with many messages this loads all rows into memory.
- Files: `src/commands/status.rs` (lines 51-53)
- Trigger: Any agent with more than a few hundred pending messages.
- Workaround: None currently; degrades gracefully but wastes memory and round-trips.

## Security Considerations

**tmux session name used directly as agent identity — no validation:**
- Risk: Agent names are taken from the tmux session name (detected via `TMUX_PANE`) in hook scripts and passed to `squad-station signal <name>`. An agent name containing shell metacharacters (spaces, semicolons) could cause unexpected behavior in the hook scripts.
- Files: `hooks/claude-code.sh` (line 17), `hooks/gemini-cli.sh` (line 17), `src/commands/signal.rs` (line 6), `src/cli.rs` (line 37)
- Current mitigation: The Rust binary receives the name as a clap positional arg (no shell interpolation). The hook uses `"$SQUAD_BIN" signal "$AGENT_NAME"` with proper quoting, so shell injection into the binary call is prevented. However, no validation of allowed characters is performed on `AGENT_NAME` in the shell scripts.
- Recommendations: Add a character allowlist check in the hook scripts before calling the binary (e.g., `[[ "$AGENT_NAME" =~ ^[a-zA-Z0-9_-]+$ ]]`).

**`SQUAD_STATION_DB` environment variable allows DB path override with no access control:**
- Risk: Any process in the same environment can set `SQUAD_STATION_DB` and redirect the `register` command to an arbitrary SQLite file path.
- Files: `src/commands/register.rs` (lines 13-21)
- Current mitigation: Only the `register` command respects this env var; other commands always require `squad.yml`.
- Recommendations: Document the intended use case clearly; consider restricting to an explicit opt-in flag instead of a persistent env var.

**Hook scripts pipe all binary output through `grep` to stderr:**
- Risk: `"$SQUAD_BIN" signal "$AGENT_NAME" 2>&1 | (grep -i "warning|error" >&2 || true)` merges stderr into stdout before filtering. Any non-warning/error output on the happy path is silently dropped, which can hide unexpected output.
- Files: `hooks/claude-code.sh` (line 29), `hooks/gemini-cli.sh` (line 28)
- Current mitigation: The `|| true` prevents pipeline failure.
- Recommendations: Redirect binary stderr directly to `/dev/null` or a log file; do not merge streams.

## Performance Bottlenecks

**TUI creates a new SQLite connection pool on every 3-second refresh:**
- Problem: `fetch_snapshot` in `src/commands/ui.rs` opens a new pool, runs two queries, then explicitly drops it every 3 seconds. This is intentional (WAL checkpoint starvation prevention) but adds connection overhead and creates/destroys a pool object on every refresh cycle.
- Files: `src/commands/ui.rs` (lines 117-138)
- Cause: SQLite WAL readers held across refreshes would block writers (CHECKPOINT). The connect-per-refresh pattern avoids this but has overhead.
- Improvement path: Use a single read-only connection with explicit WAL checkpoint calls (`PRAGMA wal_checkpoint(PASSIVE)`) after each query batch, rather than a full pool teardown. This is an optimization, not a correctness issue.

**tmux reconciliation calls `tmux has-session` once per agent per command invocation:**
- Problem: Commands that reconcile status (`agents`, `status`, `context`) call `tmux::session_exists` in a sequential loop, spawning one `tmux` subprocess per registered agent.
- Files: `src/commands/agents.rs` (line 21), `src/commands/status.rs` (line 37), `src/commands/context.rs` (line 14)
- Cause: Individual `has-session` checks instead of a single `list-sessions` call.
- Improvement path: Call `tmux::list_live_session_names()` once and use a `HashSet` lookup for each agent; this reduces to one subprocess invocation regardless of agent count.

## Fragile Areas

**TUI terminal restore on panic:**
- Files: `src/commands/ui.rs` (lines 265-270)
- Why fragile: The panic hook calls `disable_raw_mode()` and `execute!(stdout, LeaveAlternateScreen)` but cannot call `terminal.show_cursor()` because it does not have access to the `Terminal` struct. If the panic occurs after raw mode is enabled but before cursor hide, the cursor is left hidden after the panic message prints.
- Safe modification: Always call `restore_terminal` from a `Drop` impl on a guard struct rather than in the panic hook.
- Test coverage: No test for panic-in-TUI terminal state restoration.

**`update_status` selects "most recent pending" by `created_at DESC` — clock dependency:**
- Files: `src/db/messages.rs` (lines 42-55)
- Why fragile: The subquery ordering relies on `created_at` strings in RFC3339 format for correct ordering. If two messages are inserted within the same second (string comparison), ordering is undefined. In practice this is unlikely for the current use case but could fail under rapid automated task dispatch.
- Safe modification: Use a monotonic integer sequence column or include `id` as a tiebreaker in the ORDER BY.
- Test coverage: Not tested with same-second timestamp collision.

**Agent name used as FK reference in `messages.agent_name` — rename not possible:**
- Files: `src/db/migrations/0001_initial.sql` (line 18), `src/db/agents.rs`, `src/db/messages.rs`
- Why fragile: `messages.agent_name` references `agents.name` as a text FK. SQLite does not enforce FK cascades by default. If an agent is ever renamed (not currently supported) or deleted, orphaned message rows will remain. The `INSERT OR IGNORE` idempotent registration means re-registering under a different name creates a new agent record.
- Safe modification: Enable `PRAGMA foreign_keys = ON` at connection time; add explicit `ON DELETE RESTRICT` or `ON DELETE CASCADE` to the FK.
- Test coverage: No test for FK violation on agent deletion.

**`frame.size()` deprecated in recent ratatui versions:**
- Files: `src/commands/ui.rs` (line 174)
- Why fragile: `ratatui` 0.26 deprecates `Frame::size()` in favor of `Frame::area()` in 0.27+. Upgrading ratatui will require updating this call.
- Safe modification: Replace `frame.size()` with `frame.area()` when upgrading ratatui.
- Test coverage: Rendering is not unit-tested; no test will catch a ratatui API break.

## Scaling Limits

**Single-writer SQLite pool with `max_connections=1`:**
- Current capacity: One concurrent write operation; all others queue behind a 5-second busy timeout.
- Limit: Under high-frequency orchestrator dispatch (many `send` calls per second), writes will timeout after 5 seconds.
- Files: `src/db/mod.rs` (lines 19-22)
- Scaling path: SQLite WAL supports concurrent readers; the single-writer limit is deliberate for correctness. For higher throughput, switch to a proper RDBMS (PostgreSQL) or use an async queue.

**No message pruning or archival:**
- Current capacity: All messages accumulate indefinitely in `messages` table.
- Limit: For long-running squads with many completed messages, table size grows unbounded. The `list` command default limit of 20 hides this, but `status` uses `limit=9999` which will degrade.
- Files: `src/db/messages.rs`, `src/commands/status.rs` (line 51)
- Scaling path: Add a `cleanup` subcommand or a TTL-based message archival policy.

## Dependencies at Risk

**`serde-saphyr 0.0.17` — pre-1.0 YAML library:**
- Risk: Pre-1.0 SemVer means breaking changes can land in minor or patch versions. Crate has low adoption compared to `serde_yaml`.
- Files: `Cargo.toml` (line 14)
- Impact: Config parsing breaks on upgrade.
- Migration plan: Migrate to `serde_yaml 0.9` or `serde_yml` once config schema is finalized.

**`ratatui 0.26` — actively releasing minor versions with API changes:**
- Risk: `frame.size()` is already deprecated. The library releases frequently and deprecates APIs in minor versions.
- Files: `Cargo.toml` (line 17), `src/commands/ui.rs` (line 174)
- Impact: Compiler warnings grow into errors on future editions; rendering may break on upgrade without code changes.
- Migration plan: Pin to `ratatui = "0.26"` (already implicit via Cargo.lock) and update `frame.size()` → `frame.area()` before any upgrade.

## Missing Critical Features

**No hook installation/validation command:**
- Problem: Hook scripts in `hooks/` must be manually registered in `.claude/settings.json` and `.gemini/settings.json` by the user. There is no `squad-station install-hooks` command or any validation that hooks are active.
- Blocks: Operators who forget hook registration will see agents complete tasks with no signal sent; messages remain `pending` indefinitely with no error.

**No message deletion or cancellation:**
- Problem: Once a message is inserted with status `pending`, there is no CLI command to cancel or delete it. A stuck or incorrect task queued to an agent cannot be retracted.
- Blocks: Operational recovery when a wrong task is dispatched.

**No `failed` message status path:**
- Problem: The schema defines only `pending` and `completed` message statuses in practice. The `colorize_status` function in `src/commands/list.rs` (line 104) handles `"failed"` visually, but no code path ever sets a message to `failed`.
- Files: `src/commands/list.rs` (line 104), `src/db/messages.rs`
- Blocks: Distinguishing agent errors from normal completion; retry logic.

## Test Coverage Gaps

**TUI rendering not tested:**
- What's not tested: The `draw_ui` function in `src/commands/ui.rs` has zero unit or integration tests. The event loop, terminal setup, and rendering paths are entirely untested.
- Files: `src/commands/ui.rs` (lines 170-254, 260-318)
- Risk: Ratatui API changes, layout regressions, or rendering panics go undetected.
- Priority: Medium — UI is a secondary surface; core DB logic is well-tested.

**tmux integration not tested end-to-end:**
- What's not tested: `tmux::send_keys_literal`, `tmux::launch_agent`, `tmux::create_view_window` are never called in the test suite. Only the argument-builder helper functions (`send_keys_args`, `launch_args`, etc.) are unit-tested.
- Files: `src/tmux.rs`, `tests/test_views.rs` (tests mock the app state but do not call tmux)
- Risk: tmux subprocess failures, session naming conflicts, or behavior changes in tmux versions go undetected.
- Priority: Low — tmux integration tests require a live tmux server; acceptable to leave as manual/e2e only.

**Hook script behavior not covered by unit tests:**
- What's not tested: `hooks/claude-code.sh` and `hooks/gemini-cli.sh` exit behavior, AGENT_NAME detection, and binary invocation are not exercised in `tests/e2e_cli.sh` (the script exists but tests basic CLI, not hook invocation).
- Files: `hooks/claude-code.sh`, `hooks/gemini-cli.sh`, `tests/e2e_cli.sh`
- Risk: Hook registration format changes in Claude Code or Gemini CLI break the signal chain silently.
- Priority: Medium — hooks are the critical production path for signal delivery.

**No test for concurrent write contention on the single-writer pool:**
- What's not tested: Two simultaneous `send` or `signal` invocations hitting the same DB. The busy_timeout of 5s is not exercised.
- Files: `src/db/mod.rs`, `tests/`
- Risk: Race conditions or timeout failures under load remain undetected.
- Priority: Low — SQLite WAL handles this correctly at the DB level; test would require subprocess concurrency.

---

*Concerns audit: 2026-03-08*
