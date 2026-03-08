# Roadmap: Squad Station

## Milestones

- ✅ **v1.0 MVP** — Phases 1-3 (shipped 2026-03-06)
- 🚧 **v1.1 Design Compliance** — Phases 4-6 (in progress)

## Phases

<details>
<summary>✅ v1.0 MVP (Phases 1-3) — SHIPPED 2026-03-06</summary>

- [x] Phase 1: Core Foundation (5/5 plans) — completed 2026-03-06
- [x] Phase 2: Lifecycle and Hooks (3/3 plans) — completed 2026-03-06
- [x] Phase 3: Views and TUI (2/2 plans) — completed 2026-03-06

</details>

### 🚧 v1.1 Design Compliance (In Progress)

**Milestone Goal:** Refactor codebase to match `docs/SOLUTION-DESIGN.md` exactly — close all 10 gaps identified in `docs/GAP-ANALYSIS.md`.

- [ ] **Phase 4: Schema and Config Refactor** - Align DB schema and squad.yml config format with solution design
- [ ] **Phase 5: Feature Completion** - Add notification hooks, fix CLI syntax, enforce naming, standardize signal format
- [ ] **Phase 6: Documentation** - Update all docs and planning files to reflect the refactored codebase

## Phase Details

### Phase 4: Schema and Config Refactor
**Goal**: DB schema and config format fully match solution design — no gaps between code and design docs
**Depends on**: Phase 3 (v1.0 complete)
**Requirements**: CONF-01, CONF-02, CONF-03, CONF-04, MSGS-01, MSGS-02, MSGS-03, MSGS-04, AGNT-01, AGNT-02, AGNT-03
**Success Criteria** (what must be TRUE):
  1. User can write squad.yml with `project: myapp` string, `model`/`description` per agent, no `command` field, and `tool` instead of `provider` — and `init` succeeds
  2. Messages table tracks `from_agent`, `to_agent`, `type`, `processing` status, and `completed_at` — visible in `list` output
  3. Agents table stores `model`, `description`, `current_task` FK, and `tool` — visible in `agents` output
  4. Migration runs cleanly on a fresh DB and on an existing v1.0 DB without data loss
**Plans**: 3 plans

Plans:
- [ ] 04-01-PLAN.md — Config refactor: SquadConfig.project→String, AgentConfig tool/model/description, remove command, update squad.yml
- [ ] 04-02-PLAN.md — Messages schema migration: 0003_v11.sql, messages.rs struct/CRUD, test_db.rs message tests
- [ ] 04-03-PLAN.md — Agents schema migration + command callers: agents.rs struct/CRUD, init/register/send/signal/agents/list/status wired

### Phase 5: Feature Completion
**Goal**: All user-facing behavioral changes land — new hooks ship, CLI syntax is correct, naming is enforced, signal format is standard
**Depends on**: Phase 4
**Requirements**: HOOK-01, HOOK-02, CLI-01, CLI-02, CLI-03, SIG-01
**Success Criteria** (what must be TRUE):
  1. User can register `claude-code-notify.sh` as a Notification hook in Claude Code settings and it fires correctly on task completion
  2. User can register `gemini-cli-notify.sh` as a Notification hook in Gemini CLI settings and it fires correctly on task completion
  3. User runs `send myagent --body "task..."` (flag syntax) and the task is queued — positional body arg is rejected
  4. Running `init` with a squad.yml agent named `backend` auto-registers it as `<project>-<tool>-backend` in the DB
  5. Running `context` outputs `model` and `description` per agent alongside existing fields
  6. Signal notification sent to orchestrator uses the format `"<agent> completed <msg-id>"`
**Plans**: TBD

Plans:
- [ ] 05-01: Notification hooks (`claude-code-notify.sh` + `gemini-cli-notify.sh`)
- [ ] 05-02: CLI send flag syntax (`--body`), agent auto-prefix on init, context output update, signal format standardization

### Phase 6: Documentation
**Goal**: All docs and planning files accurately describe the refactored system — no stale references to removed fields or old CLI syntax
**Depends on**: Phase 5
**Requirements**: DOCS-01, DOCS-02
**Success Criteria** (what must be TRUE):
  1. `.planning/research/ARCHITECTURE.md` describes the actual sqlx + flat module structure (no stale references to pre-v1.0 design decisions)
  2. `docs/PLAYBOOK.md` shows correct `send --body` syntax, correct squad.yml format with `tool`/`model`/`description`, and correct agent naming convention
**Plans**: TBD

Plans:
- [ ] 06-01: Update `.planning/research/ARCHITECTURE.md` to match current codebase
- [ ] 06-02: Rewrite `docs/PLAYBOOK.md` with post-refactor CLI syntax and config format

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Core Foundation | v1.0 | 5/5 | Complete | 2026-03-06 |
| 2. Lifecycle and Hooks | v1.0 | 3/3 | Complete | 2026-03-06 |
| 3. Views and TUI | v1.0 | 2/2 | Complete | 2026-03-06 |
| 4. Schema and Config Refactor | v1.1 | 0/3 | Planned | - |
| 5. Feature Completion | v1.1 | 0/2 | Not started | - |
| 6. Documentation | v1.1 | 0/2 | Not started | - |
