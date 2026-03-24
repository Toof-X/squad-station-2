# Roadmap: Squad Station

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- ✅ **v1.1 Design Compliance** — Phases 4-6 (shipped 2026-03-08)
- ✅ **v1.2 Distribution** — Phases 7-9 (shipped 2026-03-09)
- ✅ **v1.3 Antigravity & Hooks Optimization** — Phases 10-13 (shipped 2026-03-09)
- ✅ **v1.4 Unified Playbook & Local DB** — Phases 14-15 (shipped 2026-03-10)
- ✅ **v1.5 Interactive Init Wizard** — Phases 16-17 (shipped 2026-03-17)
- ✅ **v1.6 UX Polish** — Phases 18-19 (shipped 2026-03-17)
- ✅ **v1.7 First-Run Onboarding** — Phases 20-21 (shipped 2026-03-18)
- ✅ **v1.8 Smart Agent Management** — Phases 22-24 (shipped 2026-03-19)
- ✅ **v1.9 Browser Visualization** — Phases 25-28 (shipped 2026-03-22)
- 🚧 **v2.0 Workflow Watchdog** — Phases 29-31 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

Stateless CLI binary with SQLite WAL, priority messaging, TUI dashboard, and provider-agnostic hook scripts.

</details>

<details>
<summary>✅ v1.1 Design Compliance (Phases 4-6) — SHIPPED 2026-03-08</summary>

Config/DB schema refactor, bidirectional messages, notification hooks, auto-prefix naming, and PLAYBOOK/ARCHITECTURE rewrite.

</details>

<details>
<summary>✅ v1.2 Distribution (Phases 7-9) — SHIPPED 2026-03-09</summary>

GitHub Actions CI/CD cross-compilation, npm package, README.md.

</details>

<details>
<summary>✅ v1.3 Antigravity & Hooks Optimization (Phases 10-13) — SHIPPED 2026-03-09</summary>

Inline signal via $TMUX_PANE, antigravity DB-only provider, .agent/workflows/ context files, safe load-buffer injection, PLAYBOOK v1.3 rewrite.

</details>

<details>
<summary>✅ v1.4 Unified Playbook & Local DB (Phases 14-15) — SHIPPED 2026-03-10</summary>

Unified squad-orchestrator.md replacing 3 fragmented context files, DB moved to .squad/station.db for data locality.

</details>

<details>
<summary>✅ v1.5 Interactive Init Wizard (Phases 16-17) — SHIPPED 2026-03-17</summary>

Multi-page ratatui TUI wizard for `squad-station init`: collects project name, SDD workflow, orchestrator + worker configs; generates squad.yml; handles re-init (overwrite/add-agents/abort).

</details>

<details>
<summary>✅ v1.6 UX Polish (Phases 18-19) — SHIPPED 2026-03-17</summary>

Branded welcome screen on bare invocation, ASCII agent fleet diagram after init, and simplified claude-code model names in wizard.

</details>

<details>
<summary>✅ v1.7 First-Run Onboarding (Phases 20-21) — SHIPPED 2026-03-18</summary>

Interactive ratatui welcome TUI (BigText title, countdown, Tab-navigable Quick Guide) replacing static ASCII screen; TTY-guarded auto-launch from npm install path.

</details>

<details>
<summary>✅ v1.8 Smart Agent Management (Phases 22-24) — SHIPPED 2026-03-19</summary>

Fleet Status metrics in orchestrator context, dynamic agent cloning command, 11 role templates in init wizard with split-pane TUI selector and Routing Matrix in context output.

</details>

<details>
<summary>✅ v1.9 Browser Visualization (Phases 25-28) — SHIPPED 2026-03-22</summary>

Embedded axum web server with React + React Flow SPA served from binary, live node-graph visualization with event-driven WebSocket streaming, animated in-flight edges, and dark/light theme.

- [x] Phase 25: Architecture Research (2/2 plans) — completed 2026-03-22
- [x] Phase 26: Axum Server & CLI Command (2/2 plans) — completed 2026-03-22
- [x] Phase 27: Event-Driven WebSocket Streaming (2/2 plans) — completed 2026-03-22
- [x] Phase 28: React Flow Node Graph (2/2 plans) — completed 2026-03-22

</details>

### v2.0 Workflow Watchdog (In Progress)

**Milestone Goal:** Detect stalled workflows where no agent is busy but pending/processing messages exist, and alert both the orchestrator (tmux injection) and the user (Telegram).

- [x] **Phase 29: Watchdog Core Correctness** - Deadlock detection, debounce, deduplication, prolonged-busy injection, configurable operations flags, and --status subcommand (completed 2026-03-24)
- [x] **Phase 30: Telegram Integration** - Delegation-based Telegram alerting via orchestrator MCP plugin: updated watchdog messages with relay instructions, --channels config, and orchestrator context section (completed 2026-03-24)
- [ ] **Phase 31: End-to-End Test Coverage** - CLI-level integration tests for watch subcommand: --status output, --dry-run lifecycle, --help flag completeness, channels config parsing, and v2.0 requirement traceability

## Phase Details

### Phase 29: Watchdog Core Correctness
**Goal**: Users can run `squad-station watch` and have the daemon reliably detect real stalls — deadlocks and prolonged-busy agents — without false positives, while safely coexisting with all other CLI commands
**Depends on**: Phase 28 (v1.9 shipped baseline)
**Requirements**: DETECT-01, DETECT-02, DETECT-03, DETECT-04, ALERT-01, ALERT-02, OPS-01, OPS-02, OPS-03
**Success Criteria** (what must be TRUE):
  1. Running `watch --status` after `watch --daemon` prints daemon PID, alive status, and uptime
  2. A deadlock state (processing messages present, zero busy agents) triggers a tmux injection into the orchestrator pane with agent count, pending message count, and stall duration — but only after N consecutive poll cycles confirm it (debounce)
  3. A stall alert fires exactly once per stall event; subsequent polls during the same stall do not re-inject until the configurable cooldown expires
  4. Messages younger than the configurable age threshold do not trigger stall alerts
  5. Running `watch --dry-run` logs stall detections to watch.log without injecting into any tmux pane
**Plans**: 3 plans
Plans:
- [x] 29-01-PLAN.md — CLI flags, DB query, main.rs dispatch (foundation)
- [x] 29-02-PLAN.md — Deadlock detection, debounce, message age, dry-run, prolonged-busy injection
- [x] 29-03-PLAN.md — Status file writing and --status subcommand

### Phase 30: Telegram Integration
**Goal**: When a stall is detected, the user receives a Telegram message on their phone — delegated through the orchestrator's MCP plugin, with no HTTP client in the Rust binary
**Depends on**: Phase 29
**Requirements**: ALERT-03, ALERT-04
**Success Criteria** (what must be TRUE):
  1. Watchdog deadlock and prolonged-busy alert messages contain explicit "IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN" instruction for the orchestrator
  2. The orchestrator context markdown includes a "Watchdog Alert Relay" section explaining how to handle Telegram relay requests
  3. The orchestrator's Claude Code session is launched with `--channels plugin:telegram` when the channels config field is present in squad.yml
**Plans**: 2 plans
Plans:
- [x] 30-01-PLAN.md — Update watchdog alert messages and orchestrator context with Telegram relay instructions
- [x] 30-02-PLAN.md — Add channels config field, wire through launch command and YAML generation, rewrite requirements

### Phase 31: End-to-End Test Coverage
**Goal**: The full tick loop behavior is verified by integration tests that run against a real SQLite DB, so correctness of deadlock detection, debounce, and deduplication is not only manually verifiable
**Depends on**: Phase 30
**Requirements**: (test coverage for all v2.0 requirements — no new feature requirements)
**Success Criteria** (what must be TRUE):
  1. `tests/test_watchdog.rs` exists with CLI-level tests exercising the binary
  2. `watch --status` output is tested (daemon running vs not running)
  3. `watch --dry-run` is tested at CLI level (exits cleanly, log file created)
  4. `cargo test` passes with all new tests green alongside the existing suite
  5. All v2.0 requirements have at least one test covering them (traceability verified)
**Plans**: 1 plan
Plans:
- [ ] 31-01-PLAN.md — CLI-level watchdog tests: --status, --dry-run, --help, flag validation, channels config

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-3. MVP | v1.0 | - | Complete | 2026-03-06 |
| 4-6. Design Compliance | v1.1 | - | Complete | 2026-03-08 |
| 7-9. Distribution | v1.2 | - | Complete | 2026-03-09 |
| 10-13. Antigravity & Hooks | v1.3 | - | Complete | 2026-03-09 |
| 14. Unified Orchestrator Playbook | v1.4 | 2/2 | Complete | 2026-03-10 |
| 15. Local DB Storage | v1.4 | 2/2 | Complete | 2026-03-10 |
| 16. TUI Wizard | v1.5 | 2/2 | Complete | 2026-03-17 |
| 17. Init Flow Integration | v1.5 | 2/2 | Complete | 2026-03-17 |
| 18. Welcome Screen & Wizard Polish | v1.6 | 2/2 | Complete | 2026-03-17 |
| 19. Agent Diagram | v1.6 | 1/1 | Complete | 2026-03-17 |
| 20. TTY-Safe Welcome TUI Core | v1.7 | 2/2 | Complete | 2026-03-17 |
| 21. Quick Guide and Install Flow | v1.7 | 2/2 | Complete | 2026-03-18 |
| 22. Orchestrator Intelligence Data | v1.8 | 2/2 | Complete | 2026-03-19 |
| 23. Dynamic Agent Cloning | v1.8 | 2/2 | Complete | 2026-03-19 |
| 24. Agent Role Templates in Wizard | v1.8 | 3/3 | Complete | 2026-03-19 |
| 25. Architecture Research | v1.9 | 2/2 | Complete | 2026-03-22 |
| 26. Axum Server & CLI Command | v1.9 | 2/2 | Complete | 2026-03-22 |
| 27. Event-Driven WebSocket Streaming | v1.9 | 2/2 | Complete | 2026-03-22 |
| 28. React Flow Node Graph | v1.9 | 2/2 | Complete | 2026-03-22 |
| 29. Watchdog Core Correctness | v2.0 | 3/3 | Complete | 2026-03-24 |
| 30. Telegram Integration | v2.0 | 2/2 | Complete | 2026-03-24 |
| 31. End-to-End Test Coverage | v2.0 | 0/1 | Not started | - |
