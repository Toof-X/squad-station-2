# Roadmap: Squad Station

## Milestones

- [x] **v1.0 MVP** - Phases 1-3 (shipped 2026-03-06)
- [x] **v1.1 Design Compliance** - Phases 4-6 (shipped 2026-03-08)
- [x] **v1.2 Distribution** - Phases 7-9 (shipped 2026-03-09)
- [x] **v1.3 Antigravity & Hooks Optimization** - Phases 10-13 (shipped 2026-03-09)
- [x] **v1.4 Unified Playbook & Local DB** - Phases 14-15 (shipped 2026-03-10)
- [x] **v1.5 Interactive Init Wizard** - Phases 16-17 (shipped 2026-03-17)
- [x] **v1.6 UX Polish** - Phases 18-19 (shipped 2026-03-17)
- [x] **v1.7 First-Run Onboarding** - Phases 20-21 (shipped 2026-03-18)
- 🚧 **v1.8 Smart Agent Management** - Phases 22-24 (in progress)

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

<details>
<summary>✅ v1.6 UX Polish (Phases 18-19) - SHIPPED 2026-03-17</summary>

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

### 🚧 v1.8 Smart Agent Management (In Progress)

**Milestone Goal:** Upgrade agent management with orchestrator intelligence metrics, dynamic cloning, and role templates — giving the orchestrator the data and tools to scale the agent fleet at runtime.

## Phase Details

### Phase 22: Orchestrator Intelligence Data
**Goal**: The orchestrator context file surfaces live fleet metrics so the orchestrator can detect overload and misrouting without guessing
**Depends on**: Phase 21
**Requirements**: INTEL-01, INTEL-02, INTEL-03, INTEL-04, INTEL-05
**Success Criteria** (what must be TRUE):
  1. Running `squad-station context` produces a `squad-orchestrator.md` that includes pending message count per agent
  2. The context file includes how long each agent has been in its current busy state
  3. The context file includes task-role alignment hints derived from keyword overlap between recent task bodies and each agent's role/description
  4. The context file embeds CLI commands for live re-query rather than pre-computed static tables so values never go stale
  5. `build_orchestrator_md()` remains a pure function — calling it with an empty metrics slice produces valid output with no DB calls inside the function
**Plans**: 2 plans
Plans:
- [x] 22-01-PLAN.md — Define AgentMetrics types, alignment logic, and Fleet Status rendering in build_orchestrator_md()
- [x] 22-02-PLAN.md — Wire DB metrics queries in run() and add integration tests

### Phase 23: Dynamic Agent Cloning
**Goal**: The orchestrator can expand the agent fleet at runtime by cloning an existing agent without touching squad.yml or reinitializing
**Depends on**: Phase 22
**Requirements**: CLONE-01, CLONE-02, CLONE-03, CLONE-04, CLONE-05, CLONE-06
**Success Criteria** (what must be TRUE):
  1. User can run `squad-station clone <agent-name>` and a new agent appears in DB and tmux with an auto-incremented name following the `<project>-<tool>-<role>-N` convention
  2. Cloning the orchestrator agent fails immediately with a clear error message before any DB writes occur
  3. If the tmux session launch fails after the DB record is written, the DB record is removed and the command exits with a non-zero code (no orphaned records)
  4. After a successful clone, `squad-orchestrator.md` is regenerated automatically so the orchestrator immediately knows about the new agent
  5. The cloned agent appears in the TUI dashboard on the next poll cycle with no manual refresh required
**Plans**: 2 plans
Plans:
- [ ] 23-01-PLAN.md — Implement clone command with DB helper and CLI wiring
- [ ] 23-02-PLAN.md — Add unit and integration tests for clone logic

### Phase 24: Agent Role Templates in Wizard
**Goal**: The init wizard presents pre-built role packages so users configure agents with correct descriptions and routing hints in seconds rather than typing from scratch
**Depends on**: Phase 22
**Requirements**: TMPL-01, TMPL-02, TMPL-03, TMPL-04, TMPL-05, TMPL-06
**Success Criteria** (what must be TRUE):
  1. The wizard presents a role selector with 8-12 predefined templates (frontend-engineer, backend-engineer, qa-engineer, architect, devops-engineer, code-reviewer, and others) before the free-text description field
  2. Selecting a template auto-fills the model selector with the template's suggested model and the description field with the template's description text
  3. User can select "Custom" to skip templates entirely and enter free-text role and description as before
  4. Routing hints from the selected template appear in the generated `squad-orchestrator.md` so the orchestrator knows each agent's specialization
  5. The template list reorders based on the SDD workflow selected on wizard page 1 (bmad/gsd/superpower) — the most relevant roles for that workflow appear at the top
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
| 23. Dynamic Agent Cloning | v1.8 | 0/2 | Not started | - |
| 24. Agent Role Templates in Wizard | v1.8 | 0/TBD | Not started | - |
