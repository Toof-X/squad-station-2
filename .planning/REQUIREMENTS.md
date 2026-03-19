# Requirements: Squad Station

**Defined:** 2026-03-19
**Core Value:** Reliable message routing between Orchestrator and agents — stateless CLI, no daemon

## v1.8 Requirements

Requirements for Smart Agent Management milestone. Each maps to roadmap phases.

### Orchestrator Intelligence

- [ ] **INTEL-01**: Orchestrator context file includes pending message count per agent (SQL aggregate from messages table)
- [ ] **INTEL-02**: Orchestrator context file includes busy-time duration for each agent (derived from busy_since or status_updated_at)
- [ ] **INTEL-03**: Orchestrator context file includes task-role alignment hints (keyword overlap between recent task bodies and agent role/description)
- [ ] **INTEL-04**: Orchestrator context embeds CLI commands for live re-query instead of stale pre-computed values
- [ ] **INTEL-05**: `build_orchestrator_md()` remains a pure function — metrics fetched externally and passed as parameter

### Dynamic Agent Cloning

- [ ] **CLONE-01**: User can run `squad-station clone <agent-name>` to create a duplicate agent with same role/model/description
- [ ] **CLONE-02**: Cloned agent receives auto-incremented name following `<project>-<tool>-<role>-N` convention (checks both DB and tmux for uniqueness)
- [ ] **CLONE-03**: Clone command registers agent in DB before launching tmux session; rolls back DB record if tmux launch fails
- [ ] **CLONE-04**: Clone command rejects cloning the orchestrator agent with a clear error message (prevents signal routing breakage)
- [ ] **CLONE-05**: Clone command auto-regenerates `squad-orchestrator.md` after successful clone so orchestrator learns about new agent
- [ ] **CLONE-06**: Cloned agent appears in TUI dashboard on next poll cycle with no additional TUI code changes

### Agent Role Templates

- [ ] **TMPL-01**: Init wizard presents a predefined role menu with 8-12 role templates (e.g., frontend-engineer, backend-engineer, qa-engineer, architect, devops-engineer, code-reviewer)
- [ ] **TMPL-02**: Each template includes role name, description text, default model suggestion, and routing hints
- [ ] **TMPL-03**: User can select "Custom" to skip templates and enter free-text role/description (existing behavior preserved)
- [ ] **TMPL-04**: Selecting a template auto-fills the model selector with the template's suggested model (user can override)
- [ ] **TMPL-05**: Template routing hints are embedded in `squad-orchestrator.md` via the context command
- [ ] **TMPL-06**: Template list ordering adapts based on detected SDD workflow (bmad/gsd/superpower) from wizard page 1

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Cloning Enhancements

- **CLONE-07**: `clone --n <count> <agent>` batch clone shorthand for creating multiple clones in one command
- **CLONE-08**: Clone count limit guardrail — warn when more than N agents with same role exist

### Intelligence Enhancements

- **INTEL-06**: Task-role alignment scoring with ML embeddings for richer misrouting detection
- **INTEL-07**: Metrics history (snapshots over time) via agent_events table for trend analysis

### Template Enhancements

- **TMPL-07**: User-defined local template registry for custom team-specific role packages

## Out of Scope

| Feature | Reason |
|---------|--------|
| Auto-scaling (daemon-based clone triggering) | Stateless CLI constraint — no daemon; orchestrator AI makes scaling decisions |
| Agent-to-agent direct messaging | All communication routes through orchestrator (PROJECT.md constraint) |
| Role-based access control on send | Moves task-semantic decisions from AI to CLI; AI is better positioned to enforce routing |
| Template marketplace / community registry | Network dependency in a zero-runtime-dependency CLI; 8-12 compiled templates cover 90% of cases |
| Cloning with modified configuration | Divergent clones break orchestrator's mental model; use `register` for distinct agents |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| INTEL-01 | — | Pending |
| INTEL-02 | — | Pending |
| INTEL-03 | — | Pending |
| INTEL-04 | — | Pending |
| INTEL-05 | — | Pending |
| CLONE-01 | — | Pending |
| CLONE-02 | — | Pending |
| CLONE-03 | — | Pending |
| CLONE-04 | — | Pending |
| CLONE-05 | — | Pending |
| CLONE-06 | — | Pending |
| TMPL-01 | — | Pending |
| TMPL-02 | — | Pending |
| TMPL-03 | — | Pending |
| TMPL-04 | — | Pending |
| TMPL-05 | — | Pending |
| TMPL-06 | — | Pending |

**Coverage:**
- v1.8 requirements: 17 total
- Mapped to phases: 0
- Unmapped: 17

---
*Requirements defined: 2026-03-19*
*Last updated: 2026-03-19 after initial definition*
