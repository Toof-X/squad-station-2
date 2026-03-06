# Roadmap: Squad Station

## Overview

Squad Station is built in three phases that follow a strict dependency chain: first the core messaging and safety foundation (without which nothing is safe to test), then agent lifecycle and hook integration (which depends on reliable send/signal/status), then visual tooling (which depends on accurate agent state). Distribution is v2. Each phase delivers a coherent, independently testable capability.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Core Foundation** - Stateless CLI binary with DB schema, agent registration, send/signal messaging, and all safety primitives wired in from day one (completed 2026-03-06)
- [ ] **Phase 2: Lifecycle and Hooks** - Reliable agent liveness detection, provider-agnostic hook scripts for Claude Code and Gemini CLI, orchestrator context file generation
- [ ] **Phase 3: Views and TUI** - Text status views, interactive ratatui dashboard, and split tmux pane layout for fleet monitoring

## Phase Details

### Phase 1: Core Foundation
**Goal**: Users can register agents, send tasks, receive completion signals, and query agent status — with all safety primitives preventing data corruption, injection, and infinite loops from the first invocation
**Depends on**: Nothing (first phase)
**Requirements**: SESS-01, SESS-02, MSG-01, MSG-02, MSG-03, MSG-04, MSG-05, MSG-06, SAFE-01, SAFE-02, SAFE-03, SAFE-04
**Success Criteria** (what must be TRUE):
  1. User can run `squad-station init` with a squad.yml and get a populated DB with registered agents and tmux sessions launched
  2. User can run `squad-station send <agent> <task>` and the prompt appears in the correct agent tmux session without special character corruption
  3. User can run `squad-station signal <agent>` from a hook and the orchestrator receives a completion notification; duplicate hook fires do not corrupt state
  4. User can run `squad-station list` and see messages filtered by agent, status, and limit; messages reflect correct priority levels
  5. Concurrent hook signals from multiple agents do not produce SQLite busy errors or lost writes
**Plans:** 5/5 plans complete

Plans:
- [x] 01-01-PLAN.md — Project foundation: Cargo.toml deps, DB schema, config types, CLI skeleton, tmux module, safety primitives
- [ ] 01-02-PLAN.md — Session management: init from squad.yml and runtime agent registration
- [ ] 01-03-PLAN.md — Core messaging: send task to agent and signal completion to orchestrator
- [ ] 01-04-PLAN.md — Query commands: list messages with filters and peek for pending tasks
- [ ] 01-05-PLAN.md — Integration tests and full suite verification across all requirements

### Phase 2: Lifecycle and Hooks
**Goal**: Agent status is always accurate (reconciled against live tmux state), hook scripts handle both Claude Code and Gemini CLI, and the orchestrator never triggers an infinite loop
**Depends on**: Phase 1
**Requirements**: SESS-03, SESS-04, SESS-05, HOOK-01, HOOK-02, HOOK-03
**Success Criteria** (what must be TRUE):
  1. User can run `squad-station agents` and see each agent as idle, busy, or dead — status reconciles against live tmux session existence, not just DB cache
  2. Hook scripts work end-to-end for Claude Code (Stop event) and Gemini CLI (AfterAgent event) without manual per-provider configuration
  3. Orchestrator session running a hook does not trigger itself — signal silently exits 0 when the current session is the orchestrator
  4. Hook gracefully exits without error when invoked outside tmux or when the agent is not registered
  5. Running `squad-station context` generates a file that lists available agents and usage commands, ready to paste into an orchestrator prompt
**Plans:** 3 plans

Plans:
- [ ] 02-01-PLAN.md — DB migration for agent status columns, signal command 4-layer guard refactor
- [ ] 02-02-PLAN.md — agents command with tmux reconciliation, context command, provider hook scripts
- [ ] 02-03-PLAN.md — Integration tests for lifecycle, guards, and phase gate verification

### Phase 3: Views and TUI
**Goal**: Users can monitor the entire agent fleet at a glance via text commands, an interactive terminal dashboard, and a split tmux pane layout — without needing to query agents individually
**Depends on**: Phase 2
**Requirements**: VIEW-01, VIEW-02, VIEW-03, VIEW-04
**Success Criteria** (what must be TRUE):
  1. User can run `squad-station status` and see a text summary of the squad — all agents, their current status, and recent message activity
  2. User can run `squad-station agents` and get a list of all registered agents with their status (text output, scriptable)
  3. User can run `squad-station ui` and see a live ratatui dashboard that refreshes agent status without holding a persistent DB connection that starves WAL checkpoints
  4. User can run `squad-station view` and see all agent tmux panes arranged in a split layout within the current terminal
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Core Foundation | 5/5 | Complete   | 2026-03-06 |
| 2. Lifecycle and Hooks | 0/3 | Planning complete | - |
| 3. Views and TUI | 0/TBD | Not started | - |
