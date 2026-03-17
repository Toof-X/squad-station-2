# Roadmap: Squad Station

## Milestones

- [x] **v1.0 MVP** - Phases 1-3 (shipped 2026-03-06)
- [x] **v1.1 Design Compliance** - Phases 4-6 (shipped 2026-03-08)
- [x] **v1.2 Distribution** - Phases 7-9 (shipped 2026-03-09)
- [x] **v1.3 Antigravity & Hooks Optimization** - Phases 10-13 (shipped 2026-03-09)
- [x] **v1.4 Unified Playbook & Local DB** - Phases 14-15 (shipped 2026-03-10)
- [x] **v1.5 Interactive Init Wizard** - Phases 16-17 (shipped 2026-03-17)
- [ ] **v1.6 UX Polish** - Phases 18-19 (in progress)

## Phases

<details>
<summary>v1.0 MVP (Phases 1-3) - SHIPPED 2026-03-06</summary>

Stateless CLI binary with SQLite WAL, priority messaging, TUI dashboard, and provider-agnostic hook scripts.

</details>

<details>
<summary>v1.1 Design Compliance (Phases 4-6) - SHIPPED 2026-03-08</summary>

Config/DB schema refactor, bidirectional messages, notification hooks, auto-prefix naming, and PLAYBOOK/ARCHITECTURE rewrite.

</details>

<details>
<summary>v1.2 Distribution (Phases 7-9) - SHIPPED 2026-03-09</summary>

GitHub Actions CI/CD cross-compilation, npm package, curl | sh installer, README.md.

</details>

<details>
<summary>v1.3 Antigravity & Hooks Optimization (Phases 10-13) - SHIPPED 2026-03-09</summary>

Inline signal via $TMUX_PANE, antigravity DB-only provider, .agent/workflows/ context files, safe load-buffer injection, PLAYBOOK v1.3 rewrite.

</details>

<details>
<summary>v1.4 Unified Playbook & Local DB (Phases 14-15) - SHIPPED 2026-03-10</summary>

Unified squad-orchestrator.md replacing 3 fragmented context files, DB moved to .squad/station.db for data locality.

</details>

<details>
<summary>✅ v1.5 Interactive Init Wizard (Phases 16-17) - SHIPPED 2026-03-17</summary>

Multi-page ratatui TUI wizard for `squad-station init`: collects project name, SDD workflow, orchestrator + worker configs; generates squad.yml; handles re-init (overwrite/add-agents/abort).

- [x] Phase 16: TUI Wizard (2/2 plans) — completed 2026-03-17
- [x] Phase 17: Init Flow Integration (2/2 plans) — completed 2026-03-17

</details>

### v1.6 UX Polish (In Progress)

**Milestone Goal:** Improve first-run and post-init UX with a branded welcome screen, an agent relationship diagram after init, and simplified claude-code model names in the wizard.

- [ ] **Phase 18: Welcome Screen & Wizard Polish** - Red ASCII title on bare invocation + simplified claude-code model names in wizard
- [ ] **Phase 19: Agent Diagram** - ASCII relationship diagram printed after init completes

## Phase Details

### Phase 18: Welcome Screen & Wizard Polish
**Goal**: Users see a branded welcome screen on bare `squad-station` invocation, and the wizard offers clean model names for claude-code
**Depends on**: Phase 17
**Requirements**: WEL-01, WEL-02, WEL-03, WEL-04, WIZ-01, WIZ-02
**Success Criteria** (what must be TRUE):
  1. Running `squad-station` with no arguments prints a large ASCII "SQUAD-STATION" title in red (owo-colors)
  2. The welcome screen shows the current binary version and a "run `squad-station init` to get started" hint
  3. The welcome screen lists all available subcommands (init, send, signal, peek, list, ui, view, status, agents, context, register)
  4. In the wizard, selecting claude-code as provider shows model options `sonnet`, `opus`, `haiku` — no version suffix strings
  5. After wizard completion, the generated squad.yml stores simplified model names (e.g., `model: sonnet`, not `model: claude-sonnet-4-6`)
**Plans**: TBD

### Phase 19: Agent Diagram
**Goal**: Users see a visual summary of their agent fleet immediately after init completes
**Depends on**: Phase 18
**Requirements**: DIAG-01, DIAG-02, DIAG-03
**Success Criteria** (what must be TRUE):
  1. After `squad-station init` completes, an ASCII diagram is printed showing each agent as a labeled box containing name, role, provider, and tmux session name
  2. The diagram shows directional arrows from the orchestrator box to each worker agent box
  3. Each agent box displays the agent's current DB status (idle / busy / dead)
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
| 18. Welcome Screen & Wizard Polish | v1.6 | 0/? | Not started | - |
| 19. Agent Diagram | v1.6 | 0/? | Not started | - |
