# Requirements: Squad Station

**Defined:** 2026-03-08
**Core Value:** Routing messages reliably between Orchestrator and agents — send task to right agent, receive completion signal, notify Orchestrator — all via stateless CLI commands, no daemon

## v1.1 Requirements

Requirements for v1.1 Design Compliance milestone. Closes all 10 gaps vs `docs/SOLUTION-DESIGN.md`.

### Config (CONF)

- [x] **CONF-01**: User can configure project using `project: myapp` string format in squad.yml
- [x] **CONF-02**: User can specify `model` and `description` for each agent and orchestrator in squad.yml
- [x] **CONF-03**: squad.yml no longer requires `command` field (tool infers launch command)
- [x] **CONF-04**: squad.yml and DB use `tool` field instead of `provider`

### Messages Schema (MSGS)

- [x] **MSGS-01**: System tracks message direction with `from_agent` and `to_agent` fields
- [x] **MSGS-02**: System records message type (task_request | task_completed | notify)
- [x] **MSGS-03**: System supports `processing` status alongside completed/failed
- [x] **MSGS-04**: System records `completed_at` timestamp when message finishes

### Agents Schema (AGNT)

- [ ] **AGNT-01**: System stores `model` and `description` for each registered agent
- [ ] **AGNT-02**: System tracks `current_task` FK linking agent to active message
- [ ] **AGNT-03**: Agent records use `tool` field instead of `provider`

### Hooks (HOOK)

- [ ] **HOOK-01**: User can register Notification hook for Claude Code
- [ ] **HOOK-02**: User can register Notification hook for Gemini CLI

### CLI (CLI)

- [ ] **CLI-01**: User sends task via `send <agent> --body "task..."` flag syntax
- [ ] **CLI-02**: `init` auto-prefixes agent names as `<project>-<tool>-<role>`
- [ ] **CLI-03**: `context` output includes `model` and `description` per agent

### Signal (SIG)

- [ ] **SIG-01**: Signal notifications use format `"<agent> completed <msg-id>"`

### Docs (DOCS)

- [ ] **DOCS-01**: `.planning/research/ARCHITECTURE.md` reflects current sqlx + flat module structure
- [ ] **DOCS-02**: `docs/PLAYBOOK.md` reflects correct CLI syntax and config format post-refactor

## v2 Requirements

Deferred to future milestone.

- npm wrapper distribution
- Cross-compile CI via GitHub Actions (darwin arm64/amd64, linux amd64/arm64)
- Support `cargo install` from source

## Out of Scope

| Feature | Reason |
|---------|--------|
| Task management / workflow logic | Orchestrator AI responsibility |
| Web UI / browser dashboard | TUI sufficient |
| Agent-to-agent direct messaging | All routing via orchestrator |
| Git conflict resolution | Orchestrator sequences work |
| Backward compatibility with v1.0 DB | Clean migration, no legacy support needed |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CONF-01 | Phase 4 | Complete |
| CONF-02 | Phase 4 | Complete |
| CONF-03 | Phase 4 | Complete |
| CONF-04 | Phase 4 | Complete |
| MSGS-01 | Phase 4 | Complete |
| MSGS-02 | Phase 4 | Complete |
| MSGS-03 | Phase 4 | Complete |
| MSGS-04 | Phase 4 | Complete |
| AGNT-01 | Phase 4 | Pending |
| AGNT-02 | Phase 4 | Pending |
| AGNT-03 | Phase 4 | Pending |
| HOOK-01 | Phase 5 | Pending |
| HOOK-02 | Phase 5 | Pending |
| CLI-01 | Phase 5 | Pending |
| CLI-02 | Phase 5 | Pending |
| CLI-03 | Phase 5 | Pending |
| SIG-01 | Phase 5 | Pending |
| DOCS-01 | Phase 6 | Pending |
| DOCS-02 | Phase 6 | Pending |

**Coverage:**
- v1.1 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-08*
*Last updated: 2026-03-08 after roadmap creation*
