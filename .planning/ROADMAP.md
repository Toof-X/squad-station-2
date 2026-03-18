# Roadmap: Squad Station

## Milestones

- [x] **v1.0 MVP** - Phases 1-3 (shipped 2026-03-06)
- [x] **v1.1 Design Compliance** - Phases 4-6 (shipped 2026-03-08)
- [x] **v1.2 Distribution** - Phases 7-9 (shipped 2026-03-09)
- [x] **v1.3 Antigravity & Hooks Optimization** - Phases 10-13 (shipped 2026-03-09)
- [x] **v1.4 Unified Playbook & Local DB** - Phases 14-15 (shipped 2026-03-10)
- [x] **v1.5 Interactive Init Wizard** - Phases 16-17 (shipped 2026-03-17)
- [x] **v1.6 UX Polish** - Phases 18-19 (shipped 2026-03-17)
- 🚧 **v1.7 First-Run Onboarding** - Phases 20-21 (in progress)

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

### 🚧 v1.7 First-Run Onboarding (In Progress)

**Milestone Goal:** Replace static welcome screen with an interactive ratatui TUI that guides new users through setup automatically after install.

## Phase Details

### Phase 20: TTY-Safe Welcome TUI Core
**Goal**: Users see an interactive ratatui welcome TUI on bare `squad-station` invocation — with big pixel-font title, version, hint bar, auto-exit countdown, and conditional routing to the init wizard or exit based on squad.yml presence
**Depends on**: Phase 19 (v1.6 complete)
**Requirements**: WELCOME-01, WELCOME-02, WELCOME-03, WELCOME-04, WELCOME-06, WELCOME-07, INIT-01, INIT-02, INIT-03
**Success Criteria** (what must be TRUE):
  1. Running `squad-station` with no arguments in a real terminal opens a ratatui TUI with a large pixel-font SQUAD-STATION title and the current version string below it
  2. The TUI displays a hint bar at the bottom showing available keys (Enter, Q/Esc) and an auto-exit countdown; the screen closes automatically when the countdown reaches zero
  3. When no squad.yml exists and the user presses Enter, the TUI closes cleanly and the init wizard launches immediately
  4. When squad.yml exists and the user presses Enter, the TUI closes without triggering any re-init
  5. Running `squad-station` with stdout piped (non-TTY) prints static welcome text without attempting to enter raw mode
**Plans**: 2 plans

Plans:
- [ ] 20-01-PLAN.md — Upgrade ratatui 0.30 + crossterm 0.29 + tui-big-text 0.8 and implement welcome TUI with BigText title, countdown, hint bar, TTY guard
- [ ] 20-02-PLAN.md — Wire conditional routing (squad.yml detection, Enter-to-wizard/dashboard handoff, Q/Esc close) and human verification

### Phase 21: Quick Guide and Install Flow
**Goal**: Users see a quick guide page in the welcome TUI explaining the Squad Station concept, and both install paths surface the binary to new users immediately after a successful install in interactive environments
**Depends on**: Phase 20
**Requirements**: WELCOME-05, INSTALL-01, INSTALL-02, INSTALL-03
**Success Criteria** (what must be TRUE):
  1. The welcome TUI has a second page (quick guide) that explains the Squad Station concept and basic workflow in 3-4 lines, reachable by pressing a navigation key from the title page
  2. After `npm install -g squad-station` in an interactive terminal, the install script launches `squad-station` automatically so the user sees the welcome TUI without a separate invocation
  3. After running the curl installer in an interactive terminal, `squad-station` launches automatically at the end of the install script
  4. Running either installer in a non-interactive environment (CI pipeline, pipe, sudo) completes silently without attempting to launch the binary or enter raw mode
**Plans**: 2 plans

Plans:
- [ ] 21-01-PLAN.md — Quick guide TUI page (WelcomePage enum, draw_guide, guide routing/content pure functions, updated tests)
- [ ] 21-02-PLAN.md — TTY-guarded auto-launch in npm install and curl installer

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
| 20. TTY-Safe Welcome TUI Core | 2/2 | Complete   | 2026-03-17 | - |
| 21. Quick Guide and Install Flow | 2/2 | Complete    | 2026-03-18 | - |
