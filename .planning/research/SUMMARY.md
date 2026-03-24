# Project Research Summary

**Project:** Squad Station v2.0 — Workflow Watchdog
**Domain:** Rust CLI — stateless binary, embedded SQLite WAL, tmux integration, long-lived background watchdog process
**Researched:** 2026-03-24
**Confidence:** HIGH

## Executive Summary

Squad Station v2.0 adds a long-lived background watchdog command to an existing, well-validated Rust CLI foundation (v1.9). The core of the watchdog (`watch.rs`) is already substantially implemented: it has a three-pass tick loop (reconcile, global stall detection, prolonged-busy detection), a daemon mode with PID file management, SIGTERM/SIGINT handling, structured logging, orchestrator tmux injection with escalating nudge sequences, and an activity-based reset. The v2.0 work is narrower than a greenfield implementation — it closes three specific gaps: adding deadlock detection (tasks stuck in `processing` with no busy agents), wiring orchestrator tmux injection for prolonged-busy agents (currently log-only), and adding Telegram alerting as a secondary alert channel.

The recommended implementation approach is minimal and non-disruptive to the existing architecture. The Telegram integration should use `tokio::process::Command` to shell out to `curl` (matching the established tmux shell-out pattern throughout the codebase), or optionally `reqwest 0.12` with `default-features = false, features = ["json", "rustls-tls"]` if native Rust HTTP is preferred. No new DB schema migration is required. The delta is two files: a new `src/commands/alert.rs` module (Telegram dispatch, ~30 lines) and modifications to `src/commands/watch.rs` (deadlock detection branch, second NudgeState instance, Telegram call sites). File count delta is 1 new, 1 modified.

The primary risks are all in connection lifecycle and stall detection correctness. A watchdog that holds a write pool across poll ticks will starve all other CLI commands — this is the highest-severity pitfall and must be prevented from the first line of code. Stall detection has multiple false-positive vectors (transient signal windows, stale DB agent status, intentional idle periods) that require debounce, message age thresholds, and a strict condition that only `processing` (not `pending`) messages trigger alerts. Alert deduplication state and a timeout wrapper around the Telegram API call are required for the v2.0 launch to be production-quality.

---

## Key Findings

### Recommended Stack

The existing v1.9 stack (tokio 1.37, sqlx 0.8, ratatui, clap 4.5, serde/serde_json, chrono, anyhow, axum 0.7, libc 0.2) requires no changes to the core runtime for v2.0. Two new crates are candidates, though one can be avoided entirely.

For Telegram alerting, there are two valid options: `reqwest 0.12` (async HTTP, ~300KB binary increase, uses `http 1.0` + `hyper 1` shared with axum 0.7, no `http` crate version duplication) or `tokio::process::Command` shelling out to `curl` (zero binary size increase, matches the existing tmux shell-out pattern, requires `curl` on PATH which is present on all target platforms). The architecture research recommends the `curl` shell-out approach as more consistent with the existing codebase convention. STACK.md recommends `reqwest 0.12` as more idiomatic Rust. Both are correct — the decision should be made explicit in Phase 2 planning.

`tokio-util 0.7` with `CancellationToken` is an optional clean alternative for graceful shutdown, but the watchdog already uses `AtomicBool` + `libc::signal()` which works correctly for daemon mode. No change to the shutdown mechanism is required.

**Core technologies for v2.0 additions:**
- `tokio::process::Command` (curl shell-out): Telegram dispatch — zero new dependency, matches established shell-out convention for all external tools
- OR `reqwest 0.12` (no default features, json + rustls-tls): Telegram dispatch — idiomatic async Rust, ~300KB overhead, shares `http 1.0` with axum 0.7, no version duplication
- `tokio::time::interval` + `AtomicBool`: polling loop and shutdown — already in `watch.rs`, no change needed
- `serde_json::json!`: Telegram request body construction — already used throughout codebase

### Expected Features

**Must have (P1, required for v2.0 launch):**
- Deadlock detection (processing messages + zero busy agents) — the failure mode most likely to leave a fleet silently broken; currently missing from `watch.rs` Pass 2
- Orchestrator tmux injection for prolonged-busy agents — Pass 3 currently only logs; wiring the injection is 4–6 lines
- Telegram alerting on stall and deadlock — the distinguishing v2.0 feature; mobile push when the human operator is not watching the terminal
- `watch --status` subcommand — verifying daemon liveness is basic operational hygiene; reads `.squad/watch.pid`, checks PID liveness, prints uptime (~20 lines)
- End-to-end test coverage for `tick()` logic — unit tests exist for NudgeState but not the full tick flow with a real DB

**Should have (P2, add after core is working):**
- Configurable nudge cooldown and max-nudges via CLI flags — currently hardcoded at 10-minute cooldown, 3 max nudges
- Stall context in Telegram alert — include stuck message count, agent states, oldest message age for actionable notifications

**Defer (post-v2.0 / v3+):**
- `--alert-webhook` generic webhook flag (covers Slack, Discord via single implementation)
- Log rotation for `.squad/log/watch.log`
- `watch --tail` flag for real-time log tailing without entering TUI
- Per-agent stall thresholds
- Time-series stall history in SQLite

**Anti-features to avoid:**
- Auto-recovery (auto-relaunching dead agents) — creates infinite restart loops; the orchestrator AI must decide
- Prometheus/OTEL metrics export — violates single-binary zero-runtime-dependency design principle
- Multiple alert channels at launch — validate Telegram first before adding N more providers

### Architecture Approach

The v2.0 architecture is additive with minimal surface area change. One new module (`alert.rs`) is introduced as an isolated Telegram dispatch function. `watch.rs` is modified to add a second stall condition branch (deadlock) and wire Telegram calls alongside the existing tmux injection paths. No new DB functions are needed — all required queries already exist (`list_agents`, `get_orchestrator`, `count_processing_all`, `total_count`, `last_activity_timestamp`). The daemon lifecycle pattern (PID file, `--daemon` fork, `--stop`) is already fully implemented.

**Major components and their v2.0 responsibilities:**
1. `src/commands/watch.rs` (MODIFY) — three-pass tick loop; add deadlock detection (Pass 2b) with a separate NudgeState; call `alert::send_telegram()` from both stall nudge paths; wire tmux injection for prolonged-busy in Pass 3
2. `src/commands/alert.rs` (NEW) — `pub async fn send_telegram(message: &str) -> bool`; reads `SQUAD_TELEGRAM_TOKEN` + `SQUAD_TELEGRAM_CHAT_ID` env vars; shells out to `curl` (or uses reqwest); returns `false` silently when unconfigured or on failure
3. `src/commands/mod.rs` (MODIFY) — add `pub mod alert;`
4. All DB, tmux, and reconcile modules: NO CHANGE

**Key architectural patterns to follow:**
- Shell-out pattern (`tokio::process::Command`) for external tools — consistent with entire tmux integration layer
- Two separate `NudgeState` instances for idle-inactivity vs. deadlock — merging them suppresses deadlock alerts after inactivity nudges fire
- "If available" graceful degradation — `let _ = alert::send_telegram(msg).await;` — Telegram failures log to `watch.log` but never stop the poll loop
- Alert dispatch priority: tmux injection is primary (always attempted with `session_exists()` check first), Telegram is secondary (parallel or fallback)

### Critical Pitfalls

1. **Long-held write pool starves all CLI commands** — `db::connect()` at watchdog startup and never closed means every `send`/`signal`/`register` call blocks for 5 seconds then errors. Use connect-per-refresh for write operations; use `connect_readonly()` per tick for reads, and ensure it does not hold open read transactions between ticks. This is the single highest-severity pitfall and cannot be patched after the fact.

2. **False positive stall alerts from transient signal windows** — when an agent completes a task, there is a brief window where the message is still `processing` but the agent is already `idle` in DB. A watchdog polling at that instant fires a false alert. Prevention: debounce with N consecutive poll cycles before alerting; only flag `processing` messages older than a configurable minimum age (recommended: 5–10 minutes).

3. **Alert deduplication absent — flood on persistent stall** — once a stall is detected, every subsequent poll cycle fires another alert. The orchestrator pane and Telegram chat receive identical messages every N seconds. Prevention: track `stall_alerted_at: Option<Instant>` in watchdog state; fire once on detection, then only re-alert after a configurable cooldown.

4. **Telegram API failure hangs or crashes the watchdog** — a network timeout, 429 rate-limit, or MCP unavailability must not stop the monitoring loop. Prevention: wrap every Telegram call in `tokio::time::timeout(Duration::from_secs(10), ...)`. Treat all Telegram failures as non-fatal. Read `retry_after` on 429 responses; never retry immediately.

5. **WAL checkpoint starvation from long-lived read pool** — a watchdog holding `connect_readonly()` continuously prevents WAL checkpointing. WAL grows unbounded over hours; DB reads degrade quadratically. Prevention: drop read connection between ticks; minimum 5-second poll interval to allow reader gaps.

---

## Implications for Roadmap

Based on combined research, three phases are recommended for v2.0, ordered by dependency and risk.

### Phase 1: Watchdog Core Correctness

**Rationale:** The daemon infrastructure and basic loop exist, but the foundation needs hardening before adding new features. Alert deduplication, debounce, connection lifecycle, and graceful shutdown are structural concerns that cannot be patched after the fact. Five of nine critical pitfalls from PITFALLS.md are Phase 1 concerns — getting connection lifecycle and stall detection correctness wrong at this stage means all subsequent work is built on a broken foundation.

**Delivers:** A production-quality watchdog daemon that runs cleanly alongside active fleet operations, detects real stalls without false positives, shuts down gracefully, and reports its daemon status.

**Addresses (from FEATURES.md P1):** Deadlock detection gap (Pass 2b in tick), prolonged-busy tmux injection gap (Pass 3 wiring), `watch --status` subcommand, configurable nudge cooldown and max-nudges

**Avoids (from PITFALLS.md):** Long-held write pool (Pitfall 1), false positives from transient windows (Pitfall 2), stale DB agent status (Pitfall 3), alert deduplication (Pitfall 4), WAL starvation (Pitfall 7), no graceful shutdown (Pitfall 8), stall-on-idle-pending (Pitfall 9)

**Build order within phase (per ARCHITECTURE.md):**
- Step 1: `src/commands/alert.rs` — new file, isolated, no dependencies on other v2.0 changes; write and test first
- Step 2: Deadlock detection in `watch.rs` tick() — second NudgeState instance, Pass 2b branch
- Step 3: Wire Telegram calls at both alert sites — depends on Steps 1 and 2
- Step 4 (optional): Update `squad-orchestrator.md` context with watchdog instructions in `context.rs`

### Phase 2: Telegram Integration and Error Handling

**Rationale:** Telegram alerting is the named differentiating feature of v2.0 but depends on debounce, deduplication, and timeout infrastructure from Phase 1. Implementing Telegram before the core is hardened means the mobile alert channel fires false positives and spams the user. Sequencing it second allows Phase 1 to validate detection correctness before adding an external channel.

**Delivers:** Mobile push notifications to a Telegram chat when a genuine stall or deadlock is detected; graceful degradation when Telegram is not configured; 429 rate-limit handling; 10-second timeout guard.

**Implements (from ARCHITECTURE.md):** `send_telegram()` in `alert.rs`; env var config (`SQUAD_TELEGRAM_TOKEN`, `SQUAD_TELEGRAM_CHAT_ID`); `tokio::time::timeout` wrapper; `retry_after` state for 429 responses; logging of Telegram send outcomes to `watch.log`

**Avoids (from PITFALLS.md):** Telegram failure crashes watchdog (Pitfall 5), tmux injection to dead orchestrator (Pitfall 6), 429 retry storm

**Key decision to make at phase kickoff:** `curl` shell-out (zero dependency, matches tmux pattern) vs. `reqwest 0.12` (idiomatic Rust, configurable timeout, slightly larger binary). Both are valid; choose and document.

### Phase 3: End-to-End Test Coverage

**Rationale:** The two previous phases produce working code, but `watch.rs` tick() logic requires integration-level tests to verify debounce, deduplication, and condition evaluation work correctly together under realistic DB state. The existing `setup_test_db()` helper is the scaffold; the tests verify acceptance criteria that are otherwise only manually verifiable.

**Delivers:** Test coverage for the full tick loop: deadlock condition triggers correctly, idle-pending condition does not trigger, debounce holds for N cycles before first alert, alert fires only once per stall event, Telegram alert function returns false cleanly when env vars are absent.

**Implements:** Integration tests in `tests/` using `setup_test_db()` for all stall detection paths; unit test for `alert::send_telegram()` with absent env vars (no subprocess spawned).

### Phase Ordering Rationale

- Phase 1 before Phase 2: All Telegram-specific pitfalls (Pitfall 5, 6) require the alert deduplication and timeout infrastructure from Phase 1. Adding Telegram before fixing deduplication guarantees a spam flood on first stall.
- Phase 2 before Phase 3: End-to-end tests must validate the full stack including the Telegram mock path; testing an incomplete implementation produces tests that need to be rewritten.
- The minimal file delta (1 new file, 1 modified file) means the entire v2.0 feature can ship as two or three focused PRs with no DB migration and no CLI surface change beyond `--status`.

### Research Flags

Phases with standard patterns (skip `/gsd:research-phase`):

- **Phase 1:** `watch.rs` already exists and is inspectable. All DB functions, tmux functions, and daemon patterns are confirmed in codebase source. NudgeState cooldown logic is implemented and readable. No new research needed — this is completion work, not discovery work.

- **Phase 2:** Telegram Bot API `sendMessage` endpoint is stable, well-documented, and HIGH confidence from multiple sources. The `curl` shell-out implementation is ~10 lines. The `reqwest 0.12` implementation is ~15 lines. No research needed beyond the decision between these two options.

- **Phase 3:** `setup_test_db()` helper and async test pattern are confirmed in existing `tests/helpers.rs`. No new research needed.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Findings grounded in direct `Cargo.toml` and `src/` source inspection. reqwest 0.12 / curl shell-out decision is validated against axum 0.7 compatibility constraints. One implementation choice remains open (reqwest vs. curl) — both options are fully researched with no unknowns. |
| Features | HIGH | Feature completeness verified directly against `watch.rs` and `cli.rs` source code. MVP gap list (deadlock detection, prolonged-busy injection, `--status`, Telegram) is confirmed missing from current code by direct inspection, not inference. |
| Architecture | HIGH | All findings from direct source inspection of `watch.rs`, `browser.rs`, `monitor.rs`, `tmux.rs`, `db/mod.rs`. No external sources required — this is integration work, not ecosystem discovery. Component boundaries and data flow are confirmed against actual function signatures. |
| Pitfalls | HIGH | All 9 critical pitfalls grounded in specific codebase code paths (`db/mod.rs` pool semantics, `signal.rs` two-write sequence, `monitor.rs` connect-per-refresh pattern). External sources verify WAL checkpoint starvation mechanics and Telegram 429 handling specifics. |

**Overall confidence:** HIGH

### Gaps to Address

- **reqwest vs. curl implementation decision:** STACK.md and ARCHITECTURE.md give different recommendations, both valid. This is an explicit implementation choice for Phase 2 kickoff. The `curl` shell-out approach is lower-risk for musl cross-compilation targets; `reqwest 0.12` is more idiomatic Rust and enables configurable timeout without subprocess complexity. Neither option has unknowns — just pick one and document the rationale.

- **Stall detection condition precision for `count_processing_all()`:** The PITFALLS.md requires that stall detection flag only `processing` messages (not `pending`). Verify that `db::messages::count_processing_all()` counts `status = 'processing'` only before writing the deadlock detection branch. If it counts `pending` as well, a separate query or `WHERE` clause is needed.

- **Configurable nudge values in Phase 1 vs. Phase 2:** Current `watch.rs` hardcodes 10-minute cooldown and 3 max nudges. FEATURES.md rates configurable nudge cooldown/max-nudges as P2. The roadmap should specify whether Phase 1 uses hardcoded values (simpler) or wires the CLI flags to `NudgeState::new(cooldown, max)` (complete). Adding `--nudge-cooldown` and `--max-nudges` at Phase 1 is low-effort and avoids a later breaking change to `NudgeState`.

---

## Sources

### Primary (HIGH confidence)

- `src/commands/watch.rs` (local) — confirmed three-pass tick loop, NudgeState with cooldown/max-nudges, daemon mode, AtomicBool SHUTDOWN signal handling
- `src/commands/browser.rs` (local) — confirmed connect-per-refresh write pattern, tokio signal graceful shutdown, read pool held for polling
- `src/commands/monitor.rs` (local) — confirmed connect-per-refresh as the established TUI polling pattern (new pool each 3s tick, pool dropped after use)
- `src/db/mod.rs` (local) — confirmed `max_connections(1)` single-writer constraint, `connect_readonly()` pattern, WAL recovery path, timeout layering
- `src/commands/signal.rs` (local) — confirmed two-sequential-write (not single-transaction) status update: message status update + agent status update create the false-positive window
- `src/tmux.rs` (local) — confirmed `send_keys_literal()` injection-safe via `-l` flag; `session_exists()` is public; `list_live_session_names()` for bulk session detection
- `src/cli.rs` (local) — confirmed `Watch` variant with `--interval`, `--stall-threshold`, `--daemon`, `--stop` flags already defined
- [Telegram Bot API 9.5](https://core.telegram.org/bots/api) — `sendMessage` endpoint URL format, parameters, rate limit behavior confirmed
- [SQLite WAL documentation](https://sqlite.org/wal.html) — checkpoint starvation mechanics: WAL cannot reset while any reader holds an open read transaction
- [Tokio graceful shutdown docs](https://tokio.rs/tokio/topics/shutdown) — CancellationToken pattern confirmed idiomatic; `tokio::signal` pattern from browser.rs confirmed correct

### Secondary (MEDIUM confidence)

- [reqwest GitHub CHANGELOG](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md) — reqwest 0.12 vs 0.13 TLS backend change confirmed; 0.12 still maintained
- [reqwest + axum 0.7 compatibility](https://users.rust-lang.org/t/a-proxy-with-axum-0-7-and-reqwest-0-12-based-on-http-1/112489) — shared `http 1.0` crate between reqwest 0.12 and axum 0.7 confirmed
- [watchdogd — Advanced system monitor for Linux](https://github.com/troglobit/watchdogd) — multi-pass monitoring, configurable thresholds, structured logging patterns
- [Overstory: tiered watchdog for AI agent fleets](https://github.com/jayminwest/overstory) — tiered nudge escalation (Tier 0/1/2 pattern); Tier 0 mechanical check maps to squad-station Pass 1–3 structure
- [GramIO: Telegram Rate Limits](https://gramio.dev/rate-limits) — `retry_after` field in 429 response; token-bucket algorithm; per-chat limits confirmed

### Tertiary (LOW confidence)

- [tmux send-keys race condition issue](https://github.com/anthropics/claude-code/issues/23513) — real-world evidence of tmux injection timing issues in AI agent workflows; supports debounce recommendation for transient window (Pitfall 2)
- [Prometheus Alertmanager Issue #2429](https://github.com/prometheus/alertmanager/issues/2429) — alert deduplication and suppress-repeat patterns; conceptual support for `stall_alerted_at` design

---

*Research completed: 2026-03-24*
*Ready for roadmap: yes*
