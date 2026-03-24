# Feature Research

**Domain:** Rust CLI — AI agent fleet watchdog / stall detection (squad-station v2.0)
**Researched:** 2026-03-24
**Confidence:** HIGH (milestone features defined in PROJECT.md; watch.rs and cli.rs already partially implemented; watchdog domain patterns verified against watchdogd, systemd watchdog, Overstory agent supervisor, tmux-notify, and Batty tmux agent supervisor)

---

## Context: What v2.0 Workflow Watchdog Adds

This milestone adds a long-lived background watchdog command on top of the existing v1.9 foundation (which already ships: agent lifecycle detection, message queue, TUI dashboard, browser visualization with WebSocket, fleet metrics in orchestrator context, clone/templates).

The v2.0 features are already partially implemented:

- `watch.rs` — core watchdog loop exists with: PID file, daemon fork, SIGTERM/SIGINT handler, 3-pass tick (reconcile, global stall detection, prolonged-busy detection), nudge state with cooldown + max-nudges, structured logging to `.squad/log/watch.log`
- `cli.rs` — `Watch` subcommand wired with `--interval`, `--stall-threshold`, `--daemon`, `--stop` flags

**What remains for v2.0:** Multi-channel alerting (Telegram MCP plugin), stall detection refinement (deadlock vs. prolonged-busy distinction), and verifying the `watch` command is complete and tested end-to-end.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users expect from any background monitoring/watchdog tool. Missing these makes the watchdog feel broken or untrusted.

| Feature | Why Expected | Complexity | Dependencies on Existing |
|---------|--------------|------------|--------------------------|
| Configurable poll interval | Every watchdog tool (watchdog Linux daemon, systemd, watchdogd) exposes an interval flag. Users running small teams need faster detection; large teams with slower agents need longer intervals to avoid false positives. Without it, users cannot tune the watchdog for their workflow. | LOW | Already implemented: `--interval` flag in `Watch` CLI variant; passed to `watch::run()`. Zero work needed. |
| Configurable stall threshold | Users expect to set "how long before I'm alerted." Default 5 minutes may be too short for large tasks (e.g., full codebase refactor). Without a configurable threshold, the watchdog generates noise for normal long-running work. | LOW | Already implemented: `--stall-threshold` flag in CLI. Default is 5 minutes. Zero work needed. |
| Daemon mode (fork to background) | Running `watch` in foreground blocks the terminal. Users expect to be able to detach it and continue working. Industry standard: watchdogd, supervisord all support daemon/fork modes. | MEDIUM | Already implemented: `--daemon` flag forks process, writes PID to `.squad/watch.pid`. Verified in watch.rs. |
| Single-instance enforcement | Starting two watchdogs simultaneously creates duplicate alerts, log corruption, and confusing behavior. Users expect "already running" error with clear message. | LOW | Already implemented: PID file check + `libc::kill(pid, 0)` liveness check before starting. Zero work needed. |
| Stop daemon command | Users need to stop the background daemon cleanly (no orphaned processes). `--stop` with PID file is the POSIX standard approach. | LOW | Already implemented: `--stop` reads `.squad/watch.pid`, sends SIGTERM. Cleanup on graceful exit. Zero work needed. |
| Orchestrator notification on stall | When a workflow stalls (all agents idle, messages stuck), inject an alert message into the orchestrator's tmux pane. This is the primary recovery path — the orchestrator AI reads the alert and dispatches work. | MEDIUM | Already implemented: `tmux::send_keys_literal` called on orchestrator pane with `[SQUAD WATCHDOG]` prefixed message. Nudge escalation (3 nudges, 10-minute cooldown) already implemented. |
| Structured log file | Operators need post-mortem audit trail. Every production watchdog (watchdogd, systemd watchdog) writes structured logs. `.squad/log/watch.log` is the obvious location, consistent with the `.squad/` directory convention. | LOW | Already implemented: `log_watch()` in watch.rs writes `TIMESTAMP LEVEL MESSAGE` to `.squad/log/watch.log`. Zero work needed. |
| Prolonged-busy detection | An agent stuck in "busy" for 30+ minutes likely has a hung process. Different from a global stall (where all agents are idle). Needs its own alert path so the orchestrator can investigate specific agents. | MEDIUM | Already implemented: Pass 3 in tick() checks `status_updated_at` for agents busy > 30 minutes, logs `WARN` level. Tmux pane notification NOT yet wired for prolonged-busy — only logs. Gap: needs orchestrator tmux injection for individual prolonged-busy agent. |
| Stall vs. prolonged-busy distinction | These are different failure modes: global stall = deadlock (all idle, messages stuck); prolonged busy = hung agent (one agent stuck, others may continue). Users must be able to distinguish them from alerts to know the correct recovery action. | LOW | Partially implemented: log messages differ ("NUDGE" vs "WARN" level). Alert messages injected into orchestrator need distinct wording for each case. |
| Agent reconciliation on each tick | The watchdog should continuously reconcile stuck-busy agents (DB says busy, tmux is idle) as part of each poll cycle, not just when a global stall occurs. This is routine maintenance that prevents queue backlog. | LOW | Already implemented: Pass 1 in tick() calls `reconcile::reconcile_agents(pool, false)` on every cycle. Reconcile actions logged at "RECONCILE" level. Zero work needed. |

### Differentiators (Competitive Advantage)

Features that distinguish squad-station watchdog from generic process monitors (watchdogd, systemd watchdog) or AI observability tools (LangSmith, AgentOps).

| Feature | Value Proposition | Complexity | Dependencies on Existing |
|---------|-------------------|------------|--------------------------|
| Telegram multi-channel alerting | Most developer tools (Grafana, Netdata, Sematext) support Telegram as an alert channel. For solo developers running AI agent fleets overnight, a mobile push notification is the only reliable way to know a workflow stalled. tmux injection reaches the orchestrator AI, but the human operator may not be watching. Telegram MCP plugin (already in the squad-station ecosystem) provides the channel. | HIGH | Requires: configuration in `squad.yml` or `.squad/telegram.toml` (bot token + chat ID); new alert dispatch function in watch.rs; `teloxide` crate or direct HTTPS POST to Telegram Bot API. Depends on no existing DB feature — pure HTTP call. |
| Escalating nudge sequence (warn → escalate → final) | Generic watchdogs send a single alert and stop. Squad-station escalates: first nudge is informational, second is urgent, third is final with instruction to do manual review. This mirrors the tiered watchdog approach in Overstory (Tier 0 mechanical, Tier 1 AI-assisted). The escalation sequence reduces alert fatigue while ensuring critical stalls are communicated. | LOW | Already implemented: NudgeState in watch.rs produces distinct messages for nudge count 0, 1, 2+. Cooldown (10 min) and max-nudges (3) already hardcoded. Gap: values should be configurable. |
| Activity-based nudge reset | Most watchdogs reset only on explicit configuration reload. Squad-station resets the nudge counter when new message activity is detected (`total_count` changes). This means a stall that self-resolves (orchestrator dispatched work) automatically clears the alert state without manual intervention. | LOW | Already implemented: `last_msg_count` tracking + `nudge_state.reset()` on count change. Zero work needed. |
| Antigravity-aware alerting | When the orchestrator uses `antigravity` tool (IDE-only, no tmux session), tmux injection is skipped. Without this guard, the watchdog would attempt to inject into a non-existent session on every tick. Squad-station already has this pattern from the `signal` command. | LOW | Already implemented: watch.rs checks `orch.tool != "antigravity"` before calling `send_keys_literal`. Zero work needed. |
| No-daemon, stateless fallback | Overstory and Batty both require long-lived daemon processes. Squad-station supports foreground mode (`watch` without `--daemon`) where the user controls lifetime via terminal. This is valuable during debugging — developer sees log output directly in the terminal while testing a workflow. | LOW | Already implemented: foreground mode is the default; `--daemon` is opt-in. Log written to file in both modes. |
| Fleet reconciliation integrated into watchdog | Other tools (systemd watchdog, watchdogd) are process-level monitors unaware of task state. Squad-station's watchdog simultaneously: detects global stalls, detects prolonged-busy individual agents, AND reconciles stuck agents on every tick. This is 3-in-1 fleet health management in a single background command. | LOW | Already implemented: all three passes in tick(). The value is that users get reconciliation "for free" when running the watchdog — no separate `reconcile` cron job needed. |
| PID-file based daemon management without systemd | Cross-platform (darwin + linux) daemon management without requiring systemd, launchd, or any service manager. Users can start/stop the watchdog with simple CLI flags regardless of OS or init system. Appropriate for developer tools used on macOS where systemd is unavailable. | LOW | Already implemented: `watch.pid` approach. Note: daemon fork uses `Command::spawn()` which leaves stdout/stderr as null. Log file is the only output for daemon mode. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-recovery: automatically relaunch dead agents | Sounds powerful — zero-downtime agent fleet. | Squad-station's design is explicit: the orchestrator AI decides when to relaunch agents. Auto-relaunch can create infinite restart loops if an agent crashes due to a code bug or config problem. The watchdog should observe and alert, not act autonomously on the fleet. | Log `WARN` entries for dead agents; alert orchestrator; let the orchestrator call `squad-station clone` or re-init. |
| Slack/Discord/email alerting | More channels = more coverage. Users with existing Slack teams expect Slack integration. | Multiple channel integrations require separate secrets management, different HTTP APIs, and ongoing maintenance for each provider's breaking changes. Binary size grows with HTTP client dependencies for each provider. Telegram MCP plugin is already in the squad-station ecosystem — adding N more channels before Telegram is validated is premature. | Implement Telegram first. Post-v2.0, a generic webhook URL configuration could cover Slack (via Slack Incoming Webhooks) and Discord with a single implementation. |
| Configurable alert messages | Users want to customize the text of watchdog notifications. | Alert message templates add configuration surface area with little value — the messages are short, functional, and self-explanatory. Customization creates support burden ("my custom template broke on special characters") with no functional benefit. | Hardcode functional messages; escalation sequence provides variation. |
| Auto-scaling on stall detection | If the watchdog detects a stall, automatically clone agents to increase capacity. | Stalls are not caused by capacity problems — they are caused by deadlocked workflow logic, crashed agents, or the orchestrator failing to dispatch. Adding agents to a deadlocked system wastes resources and does not resolve the deadlock. | Nudge orchestrator with stall context; let the orchestrator AI diagnose and decide whether to clone. |
| Prometheus/OpenTelemetry metrics export | Power users want to integrate squad-station metrics into existing observability stacks. | Single-binary, zero-runtime-dependency design principle. OTEL SDK adds ~10MB to binary size and requires a running collector. Appropriate for enterprise fleet management, not developer-local AI agent coordination. | Structured log file (`.squad/log/watch.log`) provides machine-parseable audit trail that can be ingested by external tools without coupling the binary to a telemetry framework. |
| Daemon auto-start on system boot (launchd/systemd unit file) | Users want the watchdog to persist across reboots. | Squad-station is project-scoped (DB per CWD). A systemd unit file with hardcoded CWD is fragile. Users with multiple projects would need multiple unit files. Not appropriate for the current single-developer use case. | Document manual invocation with `--daemon` in the project README. If systemd integration is needed, it can be an externally-maintained shell script. |
| Watchdog monitoring the watchdog (nested supervision) | Advanced reliability: supervisor-of-supervisor pattern. | Squad-station's watchdog is a developer tool, not a production-critical service. Nested supervision adds complexity (who supervises the supervisor?) for negligible reliability benefit in the target use case. | If the watchdog dies, the user starts it again. PID file cleanup on exit prevents stale state. |

---

## Feature Dependencies

```
[squad-station watch --daemon]
    └──requires──> [PID file at .squad/watch.pid]
        └──depends on──> [.squad/ directory] (already exists from DB path)
    └──requires──> [SIGTERM/SIGINT handler via libc]
        └──depends on──> [unix target cfg] (already in watch.rs)

[Stall detection (Pass 2)]
    └──requires──> [db::agents::list_agents()] (already exists)
    └──requires──> [db::messages::count_processing_all()] (already exists)
    └──requires──> [db::messages::last_activity_timestamp()] (already exists)
    └──requires──> [db::agents::get_orchestrator()] (already exists)
    └──requires──> [tmux::send_keys_literal()] (already exists)

[Activity-based nudge reset]
    └──requires──> [db::messages::total_count()] (already exists)
    └──feeds──> [NudgeState::reset()] (already in watch.rs)

[Prolonged-busy detection (Pass 3)]
    └──requires──> [agents.status_updated_at field] (already in DB schema)
    └──currently──> writes WARN log only
    └──MISSING──> orchestrator tmux injection for individual agent prolonged-busy alert

[Agent reconciliation (Pass 1)]
    └──requires──> [reconcile::reconcile_agents()] (already in reconcile.rs)

[Telegram alerting]
    └──requires──> [bot token + chat ID from config]
        └──OPTION A: squad.yml telegram section (requires config.rs extension)
        └──OPTION B: .squad/telegram.toml sidecar file (no squad.yml change)
    └──requires──> [HTTP POST to api.telegram.org/bot{token}/sendMessage]
        └──OPTION A: reqwest crate (async HTTP client — adds ~500KB to binary)
        └──OPTION B: std::process::Command curl (zero binary size increase, requires curl on PATH)
    └──feeds into──> [watch.rs tick() — called alongside tmux injection after stall detected]
    └──does NOT require DB schema changes

[watch --daemon] ──enables--> [background monitoring without blocking terminal]
[Stall detection] ──triggers--> [Orchestrator tmux injection] (already wired)
[Stall detection] ──should also trigger--> [Telegram alert] (MISSING — v2.0 goal)
[Prolonged-busy detection] ──should trigger--> [Orchestrator tmux injection for specific agent] (MISSING)
```

### Dependency Notes

- **Telegram alerting has no DB dependency:** It is purely a side-channel notification dispatched from watch.rs on stall detection. The only new dependency is an HTTP mechanism (reqwest or curl subprocess).
- **Prolonged-busy orchestrator injection is missing:** Pass 3 currently only logs. Wiring `tmux::send_keys_literal` for the prolonged-busy case requires adding a `get_orchestrator()` call in Pass 3 (already done in Pass 2, can be shared by passing `orch` into tick).
- **Config for Telegram:** The cleanest approach per existing patterns is a `[telegram]` section in `squad.yml` (token + chat_id). Sidecar `.squad/telegram.toml` avoids modifying squad.yml schema but splits config across two files.
- **All three passes are independent:** Reconcile, stall detection, and prolonged-busy can each fail independently without failing the others. Current implementation reflects this with per-pass error logging.

---

## MVP Definition

### This Milestone (v2.0 Workflow Watchdog)

The `watch` command core loop is already functional. v2.0 MVP requires closing the gaps and adding multi-channel alerting.

- [ ] Orchestrator tmux injection for prolonged-busy agents (individual agent alert, not just global stall) — currently only logged, not injected. Requires 4 lines in Pass 3 of tick().
- [ ] Telegram alerting on stall detection — bot token + chat ID config, HTTP dispatch alongside tmux injection. The distinguishing feature of v2.0.
- [ ] `squad-station watch --status` command — report whether watchdog daemon is running (PID, uptime, last alert time). Users need a way to verify the watchdog is alive without reading the PID file manually.
- [ ] Configurable nudge cooldown and max-nudges via CLI flags or squad.yml — currently hardcoded at 10-minute cooldown, 3 max nudges. Power users need to tune these.
- [ ] End-to-end test coverage for watch.rs tick() logic — existing unit tests cover NudgeState but not the full tick with a real DB.

### Add After Validation (post-v2.0)

- [ ] `--alert-webhook` flag as generic alert channel — single URL, POST JSON payload, covers Slack Incoming Webhooks and Discord without per-channel implementation. Trigger: user requests non-Telegram alerting.
- [ ] Stall context in Telegram alert — include which agents are idle, how many messages are stuck, last activity timestamp. Richer context enables the user to diagnose without opening a terminal. Trigger: user feedback that bare "system stalled" messages are not actionable.
- [ ] Log rotation for `.squad/log/watch.log` — unbounded log growth is a problem for long-running watchdog sessions. Trigger: user reports disk usage issue.
- [ ] `watch --tail` flag — print log lines to stdout in real time (like `tail -f`) without entering TUI. Useful for debugging watchdog behavior. Trigger: user struggles to debug watchdog without real-time log output.

### Future Consideration (v3+)

- [ ] Per-agent stall thresholds — some agents work on longer tasks by nature (e.g., architect vs. QA). A single global threshold generates false positives for slow-but-correct agents. Only needed when teams report threshold-tuning friction.
- [ ] Webhook for external orchestration (n8n, Zapier) — structured JSON payload on stall events enables integrating squad-station into larger automation pipelines. Deferred until enterprise/team users emerge.
- [ ] Time-series stall history in SQLite — persist stall events with timestamps for trend analysis. Only relevant if orchestrators need to understand recurring failure patterns over sessions.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Prolonged-busy orchestrator tmux injection | HIGH — closes the "agent stuck" alert gap; operators have no visibility otherwise | LOW — 4–6 lines added to Pass 3; `get_orchestrator()` already called in Pass 2 | P1 |
| Telegram alerting on stall | HIGH — mobile push when human is not watching terminal; differentiator vs. generic monitors | HIGH — new config section, HTTP client choice, error handling, tests | P1 |
| `watch --status` subcommand | MEDIUM — verifying the daemon is alive is basic operational hygiene; PID file read + process liveness check | LOW — 20 lines, reads `.squad/watch.pid`, checks PID liveness, prints uptime | P1 |
| Configurable nudge cooldown/max-nudges | MEDIUM — teams running long multi-hour tasks will hit false positives with 5-minute default threshold | LOW — add `--nudge-cooldown` and `--max-nudges` flags to Watch CLI variant; pass to NudgeState::new() | P2 |
| End-to-end tick() test coverage | MEDIUM — watch.rs has unit tests for NudgeState but not integration-level tick flow | MEDIUM — requires test DB setup with message/agent state; existing `setup_test_db()` helper usable | P2 |
| Stall context in Telegram alert | MEDIUM — richer alert reduces time to diagnosis | LOW — query DB at alert time; format agent list + pending count into message | P3 |
| Log rotation | LOW — current use case (developer local, sessions measured in hours) rarely hits disk limits | MEDIUM — manual rotation or tracing-appender crate | P3 |

**Priority key:**
- P1: Must have for v2.0 launch
- P2: Should have, add when core is working
- P3: Nice to have, future consideration

---

## Ecosystem Patterns Observed

### Stall Detection Heuristics (HIGH confidence — watch.rs code + watchdog domain patterns)

Two distinct stall patterns require different alert messages and recovery actions:

1. **Global deadlock stall:** All non-dead agents are idle AND processing message count > 0. This means messages are stuck in the queue with no agent processing them. Root cause: agent crashed without signaling, or orchestrator sent a task to a dead agent. Recovery: orchestrator investigates and re-dispatches.

2. **Prolonged-busy stall:** One or more agents have been in "busy" status for > threshold minutes (current default: 30m). The agent may be working correctly on a complex task, or its process may be hung. Recovery: orchestrator checks the specific agent's pane output and decides whether to cancel/retry.

The current watch.rs implementation correctly distinguishes these as Pass 2 and Pass 3 respectively. The gap is that Pass 3 only logs — it does not inject into the orchestrator pane. This is a low-effort completion item.

### Watchdog Daemon Patterns (HIGH confidence — Linux watchdog, systemd watchdog, watchdogd)

Standard daemon conventions already implemented in watch.rs:
- PID file at predictable location (`.squad/watch.pid`)
- `kill -0 pid` for process liveness check (not `-0` sending signal, just checking existence)
- SIGTERM handler for graceful shutdown
- PID file cleanup on exit
- Stale PID file removal on startup

The one omission: stdout/stderr are sent to `/dev/null` in daemon mode (correct), but the startup confirmation message ("Watchdog daemon started (PID X)") prints to the parent process before forking. This is correct UX.

### Telegram Alerting Pattern (MEDIUM confidence — WebSearch; Netdata, Sematext, Grafana community patterns)

Standard Telegram bot alert setup:
1. Create bot via BotFather, obtain token
2. Get chat ID (user or group chat)
3. POST `https://api.telegram.org/bot{TOKEN}/sendMessage` with JSON `{"chat_id": "...", "text": "...", "parse_mode": "Markdown"}`

For squad-station, two implementation choices:

**Option A — reqwest crate:** Async HTTP client, idiomatic Rust, adds ~500KB to binary. Enables retry logic and timeout configuration. Best approach if Telegram is the only HTTP call needed for v2.0.

**Option B — curl subprocess:** `std::process::Command::new("curl")` with args. Zero binary size increase. Requires `curl` on PATH (available on macOS and most Linux by default). Simpler to implement, harder to test. Acceptable for a secondary alert channel.

**Recommendation:** Use `reqwest` (tokio feature). It is the correct Rust async HTTP client for a tokio runtime. Binary size increase is acceptable. The existing browser feature already added `axum` (which transitively uses hyper/tokio) — the tokio runtime overhead is already paid.

### Nudge Escalation (MEDIUM confidence — Overstory agent supervisor patterns)

The Overstory project implements a tiered watchdog: Tier 0 mechanical check, Tier 1 AI-assisted triage, Tier 2 monitor agent patrol. Squad-station's equivalent is the 3-nudge escalation sequence (informational → urgent → final/manual). The pattern is: each nudge is more forceful than the last, and after the final nudge, the watchdog stops nudging (to avoid infinite noise) but continues logging the stall.

The current cooldown hardcoded at 10 minutes is appropriate for the default 5-minute stall threshold. If the stall threshold is increased (e.g., to 30 minutes for long-running workflows), the cooldown should scale proportionally. Making both configurable via CLI flags resolves this.

---

## Sources

- Codebase (verified directly): `src/commands/watch.rs`, `src/cli.rs`, `src/commands/reconcile.rs`, `src/db/messages.rs`, `src/db/agents.rs`, `src/tmux.rs` — all dependency and implementation claims (HIGH confidence)
- [Linux watchdog daemon man page](https://linux.die.net/man/8/watchdog) — PID file, daemon patterns, signal handling conventions (HIGH confidence)
- [watchdogd — Advanced system monitor for Linux](https://github.com/troglobit/watchdogd) — multi-pass monitoring, configurable thresholds, structured logging (HIGH confidence)
- [Overstory: tiered watchdog system](https://github.com/jayminwest/overstory) — Tier 0/1/2 watchdog for AI agent fleets, tmux liveness checks (MEDIUM confidence)
- [Batty: Rust tmux agent supervisor](https://dev.to/battyterm/building-a-tmux-native-agent-supervisor-in-rust-5hek) — send-keys injection for agent alerts, dead pane detection (MEDIUM confidence)
- [Telegram Bot API documentation](https://core.telegram.org/bots) — sendMessage endpoint, bot token setup (HIGH confidence)
- [Netdata Telegram notifications](https://learn.netdata.cloud/docs/alerts-&-notifications/notifications/agent-dispatched-notifications/telegram) — HTTP POST pattern for Telegram alerting from CLI tools (MEDIUM confidence)
- [Sematext Telegram alerts integration](https://sematext.com/docs/integration/alerts-telegram-integration/) — bot token + chat ID configuration pattern (MEDIUM confidence)
- [Feature request: Escalating stall recovery for sub-agents](https://github.com/openclaw/openclaw/issues/39305) — nudge → kill escalation pattern in AI agent frameworks (MEDIUM confidence)

---

*Feature research for: squad-station v2.0 Workflow Watchdog — stall detection, background daemon, multi-channel alerting*
*Researched: 2026-03-24*
