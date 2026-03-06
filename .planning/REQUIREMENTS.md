# Requirements: Squad Station

**Defined:** 2026-03-06
**Core Value:** Routing messages đáng tin cậy giữa Orchestrator và agents — gửi task đúng agent, nhận signal khi hoàn thành, notify Orchestrator

## v1 Requirements

### Session Management

- [ ] **SESS-01**: User can initialize squad from squad.yml — creates DB, registers agents, creates tmux sessions, launches AI tools
- [ ] **SESS-02**: User can register new agent at runtime without editing squad.yml
- [ ] **SESS-03**: Station tracks agent status as idle, busy, or dead based on current activity
- [ ] **SESS-04**: Station reconciles agent liveness by checking tmux session existence
- [ ] **SESS-05**: Station auto-generates orchestrator context file listing available agents and usage commands

### Messaging

- [ ] **MSG-01**: Orchestrator can send task to agent via `squad-station send` — writes to DB and injects prompt into agent tmux session
- [ ] **MSG-02**: Hook can signal agent completion via `squad-station signal` — updates DB status and notifies orchestrator via tmux send-keys
- [ ] **MSG-03**: Send and signal operations are idempotent — duplicate hook fires do not create duplicate messages or state corruption
- [ ] **MSG-04**: User can list messages with filters by agent, status, and limit
- [ ] **MSG-05**: Messages support priority levels (normal, high, urgent)
- [ ] **MSG-06**: Agent can peek for pending tasks via `squad-station peek`

### Hook System

- [ ] **HOOK-01**: Signal command skips orchestrator sessions (role=orchestrator) to prevent infinite loop
- [ ] **HOOK-02**: Hook scripts work for both Claude Code (Stop event) and Gemini CLI (AfterAgent event)
- [ ] **HOOK-03**: Hook gracefully exits when not in tmux or agent not registered (4-layer guard)

### Views

- [ ] **VIEW-01**: User can see squad overview via `squad-station status` (text output)
- [ ] **VIEW-02**: User can list agents and their status via `squad-station agents`
- [ ] **VIEW-03**: User can view interactive TUI dashboard via `squad-station ui` (ratatui)
- [ ] **VIEW-04**: User can view split tmux pane layout of all agents via `squad-station view`

### Safety

- [ ] **SAFE-01**: SQLite uses WAL mode with busy_timeout to handle concurrent writes from multiple agent signals
- [ ] **SAFE-02**: tmux send-keys uses literal mode (-l) to prevent special character injection
- [ ] **SAFE-03**: tmux send-keys waits for shell readiness before injecting prompt
- [ ] **SAFE-04**: SIGPIPE handler installed at binary startup

## v2 Requirements

### Distribution

- **DIST-01**: npm wrapper package with platform-specific binaries (optionalDependencies pattern)
- **DIST-02**: Cross-compile CI via GitHub Actions (darwin arm64/amd64, linux amd64/arm64)
- **DIST-03**: Support cargo install from source

### Methodology Integration

- **METH-01**: squad.yml supports methodology field (bmad, superpower, speckit, openspec, none)
- **METH-02**: Init auto-installs selected methodology

## Out of Scope

| Feature | Reason |
|---------|--------|
| Task management / workflow logic | Orchestrator AI's responsibility, not Station's |
| Orchestration decisions / reasoning | AI model's responsibility |
| File sync / code sharing between agents | Out of scope — agents work on same codebase via git |
| Web UI / browser dashboard | TUI sufficient for v1; complexity not justified |
| Git conflict resolution | Complex problem, orchestrator should sequence work to avoid |
| Agent-to-agent direct messaging | All communication routes through orchestrator |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SESS-01 | Phase 1 | Pending |
| SESS-02 | Phase 1 | Pending |
| SESS-03 | Phase 2 | Pending |
| SESS-04 | Phase 2 | Pending |
| SESS-05 | Phase 2 | Pending |
| MSG-01 | Phase 1 | Pending |
| MSG-02 | Phase 1 | Pending |
| MSG-03 | Phase 1 | Pending |
| MSG-04 | Phase 1 | Pending |
| MSG-05 | Phase 1 | Pending |
| MSG-06 | Phase 1 | Pending |
| HOOK-01 | Phase 2 | Pending |
| HOOK-02 | Phase 2 | Pending |
| HOOK-03 | Phase 2 | Pending |
| VIEW-01 | Phase 3 | Pending |
| VIEW-02 | Phase 3 | Pending |
| VIEW-03 | Phase 3 | Pending |
| VIEW-04 | Phase 3 | Pending |
| SAFE-01 | Phase 1 | Pending |
| SAFE-02 | Phase 1 | Pending |
| SAFE-03 | Phase 1 | Pending |
| SAFE-04 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0

---
*Requirements defined: 2026-03-06*
*Last updated: 2026-03-06 after roadmap creation — all 22 v1 requirements mapped*
