---
phase: 31-e2e-test-coverage
plan: 01
subsystem: testing
tags: [e2e, watchdog, cli-tests, integration]
dependency_graph:
  requires: []
  provides: [tests/test_watchdog.rs]
  affects: [src/commands/watch.rs, src/config.rs]
tech_stack:
  added: []
  patterns: [binary-invocation integration tests, tokio::test async tests, spawn-then-kill pattern]
key_files:
  created:
    - tests/test_watchdog.rs
  modified: []
decisions:
  - "spawn-then-kill used for dry-run test because watch is an infinite loop; no dedicated exit hook needed"
  - "channels config test uses load_config() directly rather than binary invocation — config parsing is a Rust-level concern, not CLI-level"
  - "interval=0 edge-case tested by checking no panic (exit code 101), not by asserting a specific error message (clap accepts 0 as valid u64)"
metrics:
  duration: 1m51s
  completed: "2026-03-24T12:20:59Z"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 0
---

# Phase 31 Plan 01: CLI-Level Watchdog Integration Tests Summary

CLI-level watchdog integration tests covering --status output, --dry-run log creation, --help flag completeness, and channels config parsing across 7 test functions in `tests/test_watchdog.rs`.

## What Was Built

A new integration test file `tests/test_watchdog.rs` with 7 test functions that exercise the `squad-station watch` binary from the outside (binary invocation pattern), verifying the public CLI interface rather than internal tick() logic.

### Test Functions

| Test | Requirement | What It Verifies |
|------|-------------|------------------|
| `test_watch_status_no_daemon` | OPS-01 | `watch --status` prints "No watchdog daemon running" when no PID file exists, exits 0 |
| `test_watch_status_stale_pid` | OPS-01 | `watch --status` prints "stale PID"/"not running" with dead PID, cleans up PID file |
| `test_watch_help_lists_all_flags` | OPS-02 | `watch --help` stdout contains all 8 flags: interval, stall-threshold, cooldown, debounce, dry-run, status, daemon, stop |
| `test_watch_help_exit_code` | OPS-02 | `watch --help` exits with code 0 |
| `test_watch_dry_run_exits_cleanly` | OPS-03 | `watch --dry-run` starts, creates `.squad/log/watch.log`, survives 2s without crash |
| `test_watch_invalid_interval_zero` | edge-case | `watch --interval 0 --dry-run` does not panic (exit code != 101) |
| `test_watch_channels_in_squad_yml` | ALERT-04 | `load_config()` parses `channels: ["plugin:telegram"]` as `Some(Vec<String>)` |

## Requirement Traceability

All 11 v2.0 requirements now have at least one test covering them:

| Requirement | Coverage |
|-------------|----------|
| DETECT-01 | watch.rs in-module tests (existing) |
| DETECT-02 | watch.rs in-module tests (existing) |
| DETECT-03 | watch.rs in-module tests (existing) |
| DETECT-04 | watch.rs in-module tests (existing) |
| ALERT-01 | watch.rs in-module tests (existing) |
| ALERT-02 | watch.rs in-module tests (existing) |
| ALERT-03 | test_tick_deadlock_escalation_telegram_instructions (existing) |
| ALERT-04 | init.rs unit tests (existing) + new `test_watch_channels_in_squad_yml` |
| OPS-01 | new `test_watch_status_no_daemon` + `test_watch_status_stale_pid` |
| OPS-02 | new `test_watch_help_lists_all_flags` + `test_watch_help_exit_code` |
| OPS-03 | test_tick_dry_run_no_tmux_injection (existing) + new `test_watch_dry_run_exits_cleanly` |

## Verification Results

```
running 7 tests
test test_watch_channels_in_squad_yml ... ok
test test_watch_invalid_interval_zero ... ok
test test_watch_help_exit_code ... ok
test test_watch_help_lists_all_flags ... ok
test test_watch_status_no_daemon ... ok
test test_watch_status_stale_pid ... ok
test test_watch_dry_run_exits_cleanly ... ok

test result: ok. 7 passed; 0 failed; 0 ignored
```

Full suite: 0 regressions across all existing tests.

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| 51cddc6 | feat(31-01): add CLI-level watchdog tests — status, help flags, exit codes |
| 25f37f2 | feat(31-01): add dry-run lifecycle, interval-zero, and channels config tests |

## Self-Check: PASSED

- `tests/test_watchdog.rs` exists with 7 test functions (186 + 97 = 283 lines > 100 min)
- Both commits exist in git log
- `cargo test --test test_watchdog` passes all 7 tests
- `cargo test` full suite passes with 0 failures
