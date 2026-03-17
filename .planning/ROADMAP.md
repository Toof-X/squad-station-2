# Roadmap: Squad Station

## Milestones

- [x] **v1.0 MVP** - Phases 1-3 (shipped 2026-03-06)
- [x] **v1.1 Design Compliance** - Phases 4-6 (shipped 2026-03-08)
- [x] **v1.2 Distribution** - Phases 7-9 (shipped 2026-03-09)
- [x] **v1.3 Antigravity & Hooks Optimization** - Phases 10-13 (shipped 2026-03-09)
- [x] **v1.4 Unified Playbook & Local DB** - Phases 14-15 (shipped 2026-03-10)
- [ ] **v1.5 Interactive Init Wizard** - Phases 16-17 (in progress)

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

### v1.5 Interactive Init Wizard (In Progress)

**Milestone Goal:** Replace the require-squad.yml-first flow with a guided TUI wizard that generates squad.yml interactively, making `squad-station init` self-contained for first-time setup.

- [x] **Phase 16: TUI Wizard** - Interactive ratatui form collecting project name, agent count, and per-agent config with validation (completed 2026-03-17)
- [x] **Phase 17: Init Flow Integration** - squad.yml generation from wizard answers and re-init handling (completed 2026-03-17)

## Phase Details

### Phase 16: TUI Wizard
**Goal**: Users can interactively configure a squad through a guided TUI form before any files are written
**Depends on**: Phase 15 (existing init infrastructure)
**Requirements**: INIT-01, INIT-02, INIT-03, INIT-06, INIT-07
**Success Criteria** (what must be TRUE):
  1. Running `squad-station init` in a directory without squad.yml opens a ratatui TUI form
  2. User can navigate field by field: project name, agent count, then per-agent role/tool/model/description
  3. Submitting an empty required field (role) or unknown tool value shows an inline error without exiting the wizard
  4. Completing the wizard with valid inputs returns control to the calling code with all collected values
**Plans:** 2/2 plans complete
Plans:
- [x] 16-01-PLAN.md — Complete wizard module: types, validation, TUI rendering, event loop
- [ ] 16-02-PLAN.md — Wire wizard into init.rs and verify interactive flow

### Phase 17: Init Flow Integration
**Goal**: Users can run `squad-station init` from scratch or re-init an existing project, with squad.yml generated or updated automatically
**Depends on**: Phase 16
**Requirements**: INIT-04, INIT-05
**Success Criteria** (what must be TRUE):
  1. After completing the wizard, a valid squad.yml is written to disk matching the entered values before agent registration begins
  2. Running `squad-station init` when squad.yml already exists prompts the user to overwrite, add agents, or abort — and each choice produces the correct outcome
  3. Choosing abort leaves the existing squad.yml unchanged and exits cleanly
**Plans:** 2/2 plans complete
Plans:
- [x] 17-01-PLAN.md — squad.yml generation from WizardResult, model validation update, worker-only wizard entry point
- [ ] 17-02-PLAN.md — Re-init prompt (overwrite/add agents/abort) and end-to-end verification

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-3. MVP | v1.0 | - | Complete | 2026-03-06 |
| 4-6. Design Compliance | v1.1 | - | Complete | 2026-03-08 |
| 7-9. Distribution | v1.2 | - | Complete | 2026-03-09 |
| 10-13. Antigravity & Hooks | v1.3 | - | Complete | 2026-03-09 |
| 14. Unified Orchestrator Playbook | v1.4 | 2/2 | Complete | 2026-03-10 |
| 15. Local DB Storage | v1.4 | 2/2 | Complete | 2026-03-10 |
| 16. TUI Wizard | 2/2 | Complete   | 2026-03-17 | - |
| 17. Init Flow Integration | 2/2 | Complete   | 2026-03-17 | - |
