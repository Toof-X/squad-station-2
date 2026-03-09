# Roadmap: Squad Station

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- ✅ **v1.1 Design Compliance** — Phases 4-6 (shipped 2026-03-08)
- ✅ **v1.2 Distribution** — Phases 7-9 (shipped 2026-03-09)
- 🚧 **v1.3 Antigravity & Hooks Optimization** — Phases 10-13 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

- [x] Phase 1: Core Foundation (5/5 plans) — completed 2026-03-06
- [x] Phase 2: Lifecycle and Hooks (3/3 plans) — completed 2026-03-06
- [x] Phase 3: Views and TUI (2/2 plans) — completed 2026-03-06

</details>

<details>
<summary>✅ v1.1 Design Compliance (Phases 4-6) — SHIPPED 2026-03-08</summary>

- [x] Phase 4: Schema and Config Refactor (3/3 plans) — completed 2026-03-08
- [x] Phase 5: Feature Completion (2/2 plans) — completed 2026-03-08
- [x] Phase 6: Documentation (2/2 plans) — completed 2026-03-08

</details>

<details>
<summary>✅ v1.2 Distribution (Phases 7-9) — SHIPPED 2026-03-09</summary>

- [x] Phase 7: CI/CD Pipeline (1/1 plans) — completed 2026-03-08
- [x] Phase 8: npm Package (2/2 plans) — completed 2026-03-08
- [x] Phase 9: Install Script and Docs (2/2 plans) — completed 2026-03-09

</details>

### 🚧 v1.3 Antigravity & Hooks Optimization (In Progress)

**Milestone Goal:** Add Antigravity IDE orchestrator support and centralize the hook system into a single CLI command with safe tmux injection.

- [x] **Phase 10: Centralized Hooks** — signal reads `$TMUX_PANE`; shell scripts marked deprecated (completed 2026-03-09)
- [ ] **Phase 11: Antigravity Provider Core** — provider enum + conditional skip-notify + skip-init behavior
- [ ] **Phase 12: IDE Context & Hook Setup** — `.agent/workflows/` generation + safe settings.json merge
- [ ] **Phase 13: Safe Injection & Documentation** — load-buffer/paste-buffer + PLAYBOOK rewrite

## Phase Details

### Phase 10: Centralized Hooks
**Goal**: Users can register a zero-argument inline hook command that works for any provider without maintaining shell scripts
**Depends on**: Phase 9
**Requirements**: HOOK-01, HOOK-02
**Success Criteria** (what must be TRUE):
  1. Running `squad-station signal` inside a provider hook (with `$TMUX_PANE` set) correctly identifies and signals the agent without any additional arguments
  2. The inline hook command `squad-station signal $TMUX_PANE` can be placed directly in `settings.json` Stop/AfterAgent hooks with no wrapper script
  3. Existing `hooks/claude-code.sh` and `hooks/gemini-cli.sh` have deprecation notices in their file headers
**Plans**: 2 plans

Plans:
- [ ] 10-01-PLAN.md — Make `signal` agent arg optional with pane-to-session resolution (HOOK-01)
- [ ] 10-02-PLAN.md — Add deprecation headers to hook shell scripts (HOOK-02)

### Phase 11: Antigravity Provider Core
**Goal**: Antigravity IDE orchestrator is a recognized provider that gets DB-only operations (no tmux interaction)
**Depends on**: Phase 10
**Requirements**: AGNT-01, AGNT-02, AGNT-03
**Success Criteria** (what must be TRUE):
  1. `config.rs` parses `provider: antigravity` from `squad.yml` without error and integration tests cover this path
  2. When orchestrator provider is `antigravity`, `squad-station signal` updates the DB record but does not call `tmux send-keys` to deliver the notification
  3. When orchestrator provider is `antigravity`, `squad-station init` skips tmux session creation for the orchestrator and prints a clear log message explaining DB-only registration
**Plans**: 2 plans

Plans:
- [ ] 11-01-PLAN.md — Add `is_db_only()` helper to `AgentConfig` and AGNT-01 config tests (AGNT-01)
- [ ] 11-02-PLAN.md — Guard `signal.rs` and `init.rs` for antigravity provider with integration tests (AGNT-02, AGNT-03)

### Phase 12: IDE Context & Hook Setup
**Goal**: IDE orchestrators can find their workflow instructions in `.agent/workflows/` and `init` sets up hooks safely in existing settings files
**Depends on**: Phase 11
**Requirements**: AGNT-04, AGNT-05, AGNT-06, HOOK-03, HOOK-04
**Success Criteria** (what must be TRUE):
  1. `squad-station context` generates `.agent/workflows/squad-delegate.md` containing delegation instructions and exact CLI commands for IDE orchestrators to assign tasks to agents
  2. `squad-station context` generates `.agent/workflows/squad-monitor.md` containing polling/monitoring guidance with behavioral anti-context-decay rules
  3. `squad-station context` generates `.agent/workflows/squad-roster.md` listing all registered agents with names, models, and descriptions
  4. `squad-station init` on a project with an existing `settings.json` merges hook entries and creates a `.bak` backup before modifying the file
  5. `squad-station init` on a project without `settings.json` prints human-readable hook setup instructions to stdout
**Plans**: TBD

Plans:
- [ ] 12-01: TBD

### Phase 13: Safe Injection & Documentation
**Goal**: Multiline task bodies are delivered safely via tmux buffer pattern; PLAYBOOK documents the complete v1.3 workflow
**Depends on**: Phase 12
**Requirements**: TMUX-01, TMUX-02, DOCS-01, DOCS-02, DOCS-03
**Success Criteria** (what must be TRUE):
  1. `squad-station send --body` with multiline content delivers the full body to the agent without shell-injection artifacts or truncation
  2. `tmux.rs` uses `load-buffer` + `paste-buffer` via a temp file for content delivery, and the temp file is cleaned up after each send
  3. `PLAYBOOK.md` documents the `squad-station signal $TMUX_PANE` inline hook command as the canonical setup method, replacing the shell script reference
  4. `PLAYBOOK.md` covers the Antigravity provider and IDE orchestrator mode with correct `squad.yml` syntax
**Plans**: TBD

Plans:
- [ ] 13-01: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Core Foundation | v1.0 | 5/5 | Complete | 2026-03-06 |
| 2. Lifecycle and Hooks | v1.0 | 3/3 | Complete | 2026-03-06 |
| 3. Views and TUI | v1.0 | 2/2 | Complete | 2026-03-06 |
| 4. Schema and Config Refactor | v1.1 | 3/3 | Complete | 2026-03-08 |
| 5. Feature Completion | v1.1 | 2/2 | Complete | 2026-03-08 |
| 6. Documentation | v1.1 | 2/2 | Complete | 2026-03-08 |
| 7. CI/CD Pipeline | v1.2 | 1/1 | Complete | 2026-03-08 |
| 8. npm Package | v1.2 | 2/2 | Complete | 2026-03-08 |
| 9. Install Script and Docs | v1.2 | 2/2 | Complete | 2026-03-09 |
| 10. Centralized Hooks | v1.3 | 2/2 | Complete | 2026-03-09 |
| 11. Antigravity Provider Core | 1/2 | In Progress|  | - |
| 12. IDE Context & Hook Setup | v1.3 | 0/? | Not started | - |
| 13. Safe Injection & Documentation | v1.3 | 0/? | Not started | - |
