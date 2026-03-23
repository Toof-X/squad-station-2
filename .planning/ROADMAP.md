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
