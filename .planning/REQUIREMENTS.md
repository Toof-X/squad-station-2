# Requirements: Squad Station

**Defined:** 2026-03-17
**Core Value:** Routing messages đáng tin cậy giữa Orchestrator và agents — stateless CLI commands không cần daemon

## v1.6 Requirements

### Welcome Screen

- [x] **WEL-01**: Running `squad-station` with no subcommand displays a welcome screen with a large ASCII "SQUAD-STATION" title rendered in red
- [x] **WEL-02**: Welcome screen shows the current binary version
- [x] **WEL-03**: Welcome screen shows a "next step" message directing the user to run `squad-station init`
- [x] **WEL-04**: Welcome screen lists available subcommands (init, send, signal, peek, list, ui, view, status, agents, context, register)

### Agent Diagram

- [x] **DIAG-01**: After `squad-station init` completes (wizard + agent registration), an ASCII diagram is printed showing all agents as labeled boxes with name, role, provider, and tmux session name
- [x] **DIAG-02**: Diagram shows arrows from orchestrator to each worker agent
- [x] **DIAG-03**: Diagram shows current DB status (idle/busy/dead) for each agent

### Wizard UX

- [x] **WIZ-01**: When `claude-code` is selected as provider in the wizard, model options show `sonnet`, `opus`, `haiku` (without version suffixes)
- [x] **WIZ-02**: `claude-code` model selection stores the simplified name in squad.yml (e.g., `model: sonnet`)

## Future Requirements

### Enhanced Welcome

- **WEL-F01**: Welcome screen shows link to online documentation / PLAYBOOK
- **WEL-F02**: Welcome screen detects existing squad.yml and shows squad status summary instead of init prompt

### Enhanced Diagram

- **DIAG-F01**: Diagram available as standalone `squad-station diagram` subcommand (re-runnable any time)
- **DIAG-F02**: Diagram shows message queue depth per agent

## Out of Scope

| Feature | Reason |
|---------|--------|
| Interactive diagram (navigable TUI) | Static ASCII sufficient for post-init overview |
| Web-based dashboard | TUI + CLI sufficient |
| Animated/updating diagram | Post-init is a one-time print, not a live view |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WEL-01 | Phase 18 | Complete |
| WEL-02 | Phase 18 | Complete |
| WEL-03 | Phase 18 | Complete |
| WEL-04 | Phase 18 | Complete |
| WIZ-01 | Phase 18 | Complete |
| WIZ-02 | Phase 18 | Complete |
| DIAG-01 | Phase 19 | Complete |
| DIAG-02 | Phase 19 | Complete |
| DIAG-03 | Phase 19 | Complete |

**Coverage:**
- v1.6 requirements: 9 total
- Mapped to phases: 9/9
- Unmapped: 0

---
*Requirements defined: 2026-03-17*
