# Phase 31 Context: End-to-End Test Coverage

**Phase Goal:** The full tick loop behavior is verified by integration tests that run against a real SQLite DB, so correctness of deadlock detection, debounce, and deduplication is not only manually verifiable

**Created:** 2026-03-24
**Status:** Ready for research/planning

## Prior Decisions (from v2.0 planning + Phases 29-30)

- Telegram delegated to orchestrator MCP — reply-based model, no HTTP client in Rust binary (Phase 30)
- Reply-based alerting: orchestrator uses `chat_id` from inbound Telegram `<channel>` tag; no stored chat IDs needed (Phase 30, updated 2026-03-24)
- Two separate NudgeState instances (idle + deadlock) — from Phase 29
- DeadlockState debounce (3 cycles default), cooldown (600s default) — from Phase 29
- Watchdog deadlock messages include stuck message IDs, escalation levels — from Phase 29
- `channels: Option<Vec<String>>` on AgentConfig, `--channels plugin:telegram` for claude-code orchestrator — from Phase 30

## Existing Test Baseline

`watch.rs` already contains **15 integration tests** built alongside Phases 29-30:

| # | Test | Coverage |
|---|------|----------|
| 1 | `test_tick_deadlock_fires_after_debounce` | DETECT-01, DETECT-02 |
| 2 | `test_tick_no_false_alert_for_pending_only` | False positive guard |
| 3 | `test_tick_no_deadlock_when_agent_is_busy` | DETECT-01 boundary |
| 4 | `test_tick_activity_resets_nudges` | State reset on activity |
| 5 | `test_tick_cooldown_prevents_repeated_alerts` | ALERT-02 |
| 6 | `test_tick_dry_run_no_tmux_injection` | OPS-03 |
| 7 | `test_tick_young_messages_not_stale` | DETECT-03 |
| 8 | `test_tick_debounce_resets_when_condition_clears` | DETECT-02 edge case |
| 9 | `test_tick_max_nudges_stops_alerts` | ALERT-02 escalation cap |
| 10 | `test_tick_prolonged_busy_alert` | DETECT-04 |
| 11 | `test_tick_deadlock_escalation_telegram_instructions` | ALERT-03 (all 3 levels) |
| 12 | `test_tick_prolonged_busy_telegram_instruction` | ALERT-03 (busy path) |
| 13 | `test_tick_deadlock_alert_contains_message_ids_and_age` | ALERT-01 content |
| 14 | `test_tick_idle_nudge_does_not_contain_telegram_instruction` | Negative: idle != Telegram |
| 15 | `test_tick_antigravity_orchestrator_no_telegram_alert` | Antigravity guard |

Plus **9 unit tests** for NudgeState, DeadlockState, log_watch, and message format.

**Test infrastructure (private to watch.rs):**
- `MockTmux` — records tmux calls without executing
- `TestTime` — controllable clock for deterministic testing
- `setup_test_db()` — temp SQLite with migrations
- `seed_agents()` — creates orch + worker pair
- `insert_old_processing_msg()` — backdated processing message

## Decisions

### 1. Test Organization — Hybrid Approach

**Decision:** Keep existing 15 in-module integration tests in `watch.rs` (they test the private `tick()` function). Add new CLI-level tests in `tests/test_watchdog.rs` that exercise the public binary interface.

**Why:** The in-module tests correctly access private internals (`tick()`, `MockTmux`, `TestTime`) — moving them to `tests/` would require leaking private API as `pub`. CLI-level tests verify the real binary behavior (flag parsing, process lifecycle, output format) which is a different concern.

**CLI-level tests to add in `tests/test_watchdog.rs`:**
- `watch --status` output format when daemon is running vs not running
- `watch --dry-run` exits cleanly and produces log output
- `watch --help` shows all flags (interval, stall-threshold, cooldown, debounce, dry-run, status, daemon, stop)
- Flag validation (invalid values rejected)

### 2. Outdated Success Criterion — send_telegram() Replaced

**Decision:** Delete success criterion 3 ("A test that calls `alert::send_telegram()` with absent env vars returns false"). Replace with: "CLI-level test verifies `watch --status` prints structured output when daemon PID file exists, and prints 'not running' when absent."

**Why:** `send_telegram()` doesn't exist — Telegram is delegated to orchestrator MCP (reply-based). The Telegram relay instruction correctness is already verified by 3 existing tests (`test_tick_deadlock_escalation_telegram_instructions`, `test_tick_prolonged_busy_telegram_instruction`, `test_tick_idle_nudge_does_not_contain_telegram_instruction`). The replacement criterion tests an actually-untested public interface.

### 3. Updated Success Criteria

**Original:**
1. ~~Deadlock fires after N debounce cycles~~ → already covered
2. ~~Pending-only no false alert~~ → already covered
3. ~~`alert::send_telegram()` with absent env vars~~ → deleted (no such function)
4. `cargo test` passes with all new + existing tests

**Revised:**
1. `tests/test_watchdog.rs` exists with CLI-level tests exercising the binary
2. `watch --status` output is tested (daemon running vs not running)
3. `watch --dry-run` is tested at CLI level (exits cleanly, log file created)
4. `cargo test` passes with all new tests green alongside the existing suite
5. All v2.0 requirements have at least one test covering them (traceability verified)

### 4. Channels Config Test Coverage

**Decision:** Add test in existing `tests/test_commands.rs` or `test_watchdog.rs` verifying that `get_launch_command()` appends `--channels plugin:telegram` when config has channels, and omits it when channels is None.

**Why:** This is the ALERT-04 verification — the channels config field must wire through correctly to the launch command. Currently untested at integration level.

## Code Context

### Key files to create:
- `tests/test_watchdog.rs` — CLI-level integration tests for `watch` subcommand

### Key files to reference:
- `src/commands/watch.rs` — existing 15 in-module tests (no changes needed)
- `tests/helpers.rs` — `setup_test_db()` pattern for CLI tests
- `tests/test_commands.rs` — pattern for binary-level testing (`Command::new(...)`)
- `src/commands/init.rs` — `get_launch_command()` for channels test

### Key patterns to follow:
- `cmd_with_db(db_path)` pattern from test_commands.rs for CLI binary tests
- `write_squad_yml()` for test config setup
- `tempfile::TempDir` for isolated test directories
- Assert on stdout/stderr content and exit codes

## Deferred Ideas

None captured during discussion.

---
*Context created: 2026-03-24*
