# Roadmap: Squad Station

## Milestones

- [x] **v1.0 MVP** - Phases 1-3 (shipped 2026-03-06)
- [x] **v1.1 Design Compliance** - Phases 4-6 (shipped 2026-03-08)
- [x] **v1.2 Distribution** - Phases 7-9 (shipped 2026-03-09)
- [x] **v1.3 Antigravity & Hooks Optimization** - Phases 10-13 (shipped 2026-03-09)
- [ ] **v1.4 Unified Playbook & Local DB** - Phases 14-15 (in progress)

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

### v1.4 Unified Playbook & Local DB (In Progress)

**Milestone Goal:** Replace fragmented context files with a single cohesive orchestrator playbook, and move DB into the project directory for data locality.

- [ ] **Phase 14: Unified Orchestrator Playbook** - `context` generates a single `squad-orchestrator.md` replacing the 3 fragmented workflow files
- [ ] **Phase 15: Local DB Storage** - DB path moves to `<cwd>/.squad/station.db`, `dirs` crate removed, docs and tests updated

## Phase Details

### Phase 14: Unified Orchestrator Playbook
**Goal**: The `context` command produces a single `squad-orchestrator.md` file that gives an IDE orchestrator everything it needs in one load — delegation workflow, monitoring, and agent roster — drawn from the `withClaudeCodeTmux.vi.toml` base template and dynamically populated from `squad.yml`
**Depends on**: Nothing (self-contained change to `context` command and `init` console output)
**Requirements**: PLAY-01, PLAY-02, PLAY-03, PLAY-04
**Success Criteria** (what must be TRUE):
  1. Running `squad-station context` produces `.agent/workflows/squad-orchestrator.md` and no longer produces `squad-delegate.md`, `squad-monitor.md`, or `squad-roster.md`
  2. The generated `squad-orchestrator.md` contains wording and structure derived from `withClaudeCodeTmux.vi.toml` (delegation, monitoring, roster sections in one file)
  3. The agent list section inside `squad-orchestrator.md` reflects the actual agents in `squad.yml` — correct names, models, descriptions, and roles
  4. Running `squad-station init` prints a "Get Started" message referencing `.agent/workflows/squad-orchestrator.md`, not the old `squad-delegate.md` path
**Plans**: TBD

Plans:
- [ ] 14-01: Rewrite `context` command to generate single `squad-orchestrator.md` from base template with dynamic agent injection
- [ ] 14-02: Update `init` "Get Started" console output to reference `squad-orchestrator.md`

### Phase 15: Local DB Storage
**Goal**: The DB lives at `.squad/station.db` inside the working project directory — no home-dir path resolution, no `dirs` crate, no name-collision risk — with env var override intact and all docs/tests updated to reflect the new location
**Depends on**: Phase 14
**Requirements**: LODB-01, LODB-02, LODB-03, LODB-04, LODB-05, LODB-06
**Success Criteria** (what must be TRUE):
  1. Running `squad-station init` in a project directory creates `.squad/station.db` in that directory (not under `~/.agentic-squad/`)
  2. All commands (`send`, `signal`, `list`, `peek`, `agents`, `status`, `ui`, `view`, `context`) resolve the DB from `.squad/station.db` by default without any extra flags
  3. Setting `SQUAD_STATION_DB=/custom/path/db` overrides the default and all commands use that path instead
  4. The project `.gitignore` contains a `.squad/` entry so the local DB is not accidentally committed
  5. `CLAUDE.md` and `README.md` document `.squad/station.db` as the DB location (no references to `~/.agentic-squad/`)
**Plans**: TBD

Plans:
- [ ] 15-01: Change `resolve_db_path` default to `<cwd>/.squad/station.db`, remove `dirs` from `Cargo.toml`
- [ ] 15-02: Update `.gitignore`, `test_db_path_resolution_default` assertion, `CLAUDE.md`, and `README.md`

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-3. MVP | v1.0 | - | Complete | 2026-03-06 |
| 4-6. Design Compliance | v1.1 | - | Complete | 2026-03-08 |
| 7-9. Distribution | v1.2 | - | Complete | 2026-03-09 |
| 10-13. Antigravity & Hooks | v1.3 | - | Complete | 2026-03-09 |
| 14. Unified Orchestrator Playbook | v1.4 | 0/2 | Not started | - |
| 15. Local DB Storage | v1.4 | 0/2 | Not started | - |
