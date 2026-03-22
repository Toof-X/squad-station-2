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
- 🚧 **v1.9 Browser Visualization** — Phases 25-28 (in progress)

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

GitHub Actions CI/CD cross-compilation, npm package, curl | sh installer, README.md.

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

- [x] Phase 16: TUI Wizard (2/2 plans) — completed 2026-03-17
- [x] Phase 17: Init Flow Integration (2/2 plans) — completed 2026-03-17

</details>

<details>
<summary>✅ v1.6 UX Polish (Phases 18-19) — SHIPPED 2026-03-17</summary>

Branded welcome screen on bare invocation, ASCII agent fleet diagram after init, and simplified claude-code model names in wizard.

- [x] Phase 18: Welcome Screen & Wizard Polish (2/2 plans) — completed 2026-03-17
- [x] Phase 19: Agent Diagram (1/1 plans) — completed 2026-03-17

</details>

<details>
<summary>✅ v1.7 First-Run Onboarding (Phases 20-21) — SHIPPED 2026-03-18</summary>

Interactive ratatui welcome TUI (BigText title, countdown, Tab-navigable Quick Guide) replacing static ASCII screen; TTY-guarded auto-launch from both npm and curl install paths.

- [x] Phase 20: TTY-Safe Welcome TUI Core (2/2 plans) — completed 2026-03-17
- [x] Phase 21: Quick Guide and Install Flow (2/2 plans) — completed 2026-03-18

</details>

<details>
<summary>✅ v1.8 Smart Agent Management (Phases 22-24) — SHIPPED 2026-03-19</summary>

Fleet Status metrics in orchestrator context, dynamic agent cloning command, 11 role templates in init wizard with split-pane TUI selector and Routing Matrix in context output.

- [x] Phase 22: Orchestrator Intelligence Data (2/2 plans) — completed 2026-03-19
- [x] Phase 23: Dynamic Agent Cloning (2/2 plans) — completed 2026-03-19
- [x] Phase 24: Agent Role Templates in Wizard (3/3 plans) — completed 2026-03-19

</details>

### 🚧 v1.9 Browser Visualization (In Progress)

**Milestone Goal:** Add `squad-station browser` command that serves a React + React Flow SPA from the binary via axum, delivering live node-graph visualization of agent topology with event-driven WebSocket streaming.

- [ ] **Phase 25: Architecture Research** - Spike all integration points before writing production code
- [ ] **Phase 26: Axum Server & CLI Command** - Embedded web server with SPA assets, `browser` command with port selection and browser launch
- [ ] **Phase 27: Event-Driven WebSocket Streaming** - tmux pane watcher + DB state change detector pushing real-time events to browser clients
- [ ] **Phase 28: React Flow Node Graph** - React + React Flow SPA with hierarchical auto-layout, live status nodes, animated in-flight edges, and UI polish

## Phase Details

### Phase 25: Architecture Research
**Goal**: All integration boundaries are proven and design decisions are locked before any production code is written
**Depends on**: Phase 24 (v1.8 complete)
**Requirements**: None (pre-implementation research phase)
**Success Criteria** (what must be TRUE):
  1. rust-embed integration pattern is validated: a test binary embeds a static asset and serves it via axum without runtime file dependency
  2. axum WebSocket upgrade path is proven: a minimal WS handler compiles and echoes a message to a connected client
  3. Event-detection strategy is decided: tmux pane polling interval, DB change-detection mechanism (timestamp comparison or SQLite hooks), and debounce approach are documented
  4. React + React Flow build pipeline is proven: Vite build produces a dist/ folder that rust-embed can include at compile time
  5. Architecture decisions are recorded in PROJECT.md Key Decisions table and a research spike document exists
**Plans:** 1/2 plans executed
Plans:
- [ ] 25-01-PLAN.md — Scaffold workspace, frontend, and spike server (rust-embed + axum + WS echo + build.rs)
- [ ] 25-02-PLAN.md — Verify spike end-to-end and record architecture decisions in PROJECT.md

### Phase 26: Axum Server & CLI Command
**Goal**: Users can run `squad-station browser` and see the SPA open in their browser, served entirely from the binary
**Depends on**: Phase 25
**Requirements**: SRV-01, SRV-02, SRV-03, SRV-04, UI-01
**Success Criteria** (what must be TRUE):
  1. Running `squad-station browser` starts an axum server and immediately opens the default system browser to the correct URL
  2. The SPA HTML, JS, and CSS assets are served directly from the binary — no external files required on disk
  3. Running `squad-station browser --port 9000` starts the server on port 9000; omitting `--port` selects an available port automatically
  4. Pressing Ctrl+C shuts down the server cleanly with no orphaned processes or lingering port bindings
  5. The binary size increase from embedded SPA assets is within acceptable bounds (SPA can be built and embedded)
**Plans**: TBD

### Phase 27: Event-Driven WebSocket Streaming
**Goal**: Browser clients receive real-time state-change events pushed from the server without polling
**Depends on**: Phase 26
**Requirements**: RT-01, RT-02, RT-03, RT-04
**Success Criteria** (what must be TRUE):
  1. When a WebSocket client connects, it immediately receives a full snapshot of current topology and message state as the first frame
  2. When an agent's status changes (idle/busy/dead), all connected browser clients receive a push event within the detection interval — no browser refresh needed
  3. When a new message is created or a message completes in the DB, connected clients receive a push event reflecting the change
  4. If the WebSocket connection drops, the browser client automatically reconnects and receives a fresh full-state snapshot
  5. The event-detection loop is driven by state-change observation (tmux pane watching + DB timestamp comparison), not by fixed-interval polling that ignores unchanged state
**Plans**: TBD

### Phase 28: React Flow Node Graph
**Goal**: Users see a live, visually accurate node graph of their agent fleet in the browser with real-time status and in-flight message animation
**Depends on**: Phase 27
**Requirements**: VIZ-01, VIZ-02, VIZ-03, VIZ-04, UI-02, UI-03
**Success Criteria** (what must be TRUE):
  1. Each agent is rendered as a distinct React Flow node showing the agent's name, role, model, and current status (idle/busy/dead) with color coding that updates live as status changes
  2. The graph layout is hierarchical: the orchestrator node appears at the top and worker nodes appear below, derived automatically from squad.yml topology — no manual positioning required
  3. Edges between orchestrator and agent nodes show continuous animation while a message is in-flight (processing status); animation stops when the message completes
  4. Edge labels or tooltips display the message task text, priority level, and timestamp for in-flight messages
  5. A connection status indicator is visible in the UI showing the current WebSocket state (connected / reconnecting / disconnected), and a dark/light theme toggle is accessible and persists preference
**Plans**: TBD

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
| 25. Architecture Research | 1/2 | In Progress|  | - |
| 26. Axum Server & CLI Command | v1.9 | 0/? | Not started | - |
| 27. Event-Driven WebSocket Streaming | v1.9 | 0/? | Not started | - |
| 28. React Flow Node Graph | v1.9 | 0/? | Not started | - |
