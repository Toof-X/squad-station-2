# Requirements: Squad Station

**Defined:** 2026-03-17
**Core Value:** Routing messages đáng tin cậy giữa Orchestrator và agents — stateless CLI commands không cần daemon

## v1.5 Requirements

### Interactive Init Wizard

- [ ] **INIT-01**: User is prompted for project name during `init` when no squad.yml exists
- [ ] **INIT-02**: User is prompted for number of agents (integer input)
- [ ] **INIT-03**: For each agent, user is prompted for role, tool (claude-code/gemini-cli/antigravity), model, and description
- [ ] **INIT-04**: `init` generates squad.yml from wizard answers before proceeding with agent registration
- [ ] **INIT-05**: When squad.yml already exists, user is prompted to choose: overwrite, add agents, or abort

### TUI Wizard UX

- [ ] **INIT-06**: Wizard is presented as a TUI screen (ratatui) with field-by-field form navigation
- [ ] **INIT-07**: Wizard validates inputs (non-empty role, known tool values) before accepting and shows inline error feedback

## Future Requirements

### Enhanced Wizard

- **INIT-F01**: Preset squad templates (solo, frontend+backend, full squad of 3) as starting points
- **INIT-F02**: Edit existing agents inline from wizard (not just overwrite/add)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web-based setup UI | TUI sufficient, browser adds complexity |
| Config validation against live tmux | Wizard is config-authoring only; validation on run |
| Multi-project wizard | One project per `init` invocation |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INIT-01 | Phase 16 | Pending |
| INIT-02 | Phase 16 | Pending |
| INIT-03 | Phase 16 | Pending |
| INIT-04 | Phase 16 | Pending |
| INIT-05 | Phase 16 | Pending |
| INIT-06 | Phase 16 | Pending |
| INIT-07 | Phase 16 | Pending |

**Coverage:**
- v1.5 requirements: 7 total
- Mapped to phases: TBD (roadmapper will assign)
- Unmapped: 7 ⚠️

---
*Requirements defined: 2026-03-17*
*Last updated: 2026-03-17 after initial definition*
