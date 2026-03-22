# Requirements: Squad Station

**Defined:** 2026-03-22
**Core Value:** Routing messages reliably between Orchestrator and agents — stateless CLI, no daemon.

## v1.9 Requirements

Requirements for v1.9: Browser Visualization. Each maps to roadmap phases.

### Web Server & CLI

- [x] **SRV-01**: `squad-station browser` starts an embedded axum HTTP server that serves the React SPA from rust-embed bundled static assets
- [x] **SRV-02**: `squad-station browser` auto-opens the default system browser to the server URL after startup
- [x] **SRV-03**: Server shuts down gracefully on Ctrl+C or SIGTERM with no orphaned processes
- [x] **SRV-04**: `--port` flag allows custom port selection (default: auto-select available port)

### Real-Time Streaming

- [ ] **RT-01**: axum WebSocket endpoint pushes state-change events to all connected browser clients
- [ ] **RT-02**: Event-driven detection watches tmux panes and DB for state changes (agent status transitions, new/completed messages)
- [ ] **RT-03**: On WebSocket connect, server sends full topology + message state snapshot as initial frame
- [ ] **RT-04**: Browser auto-reconnects on WebSocket drop and re-syncs full state

### Node-Graph Visualization

- [ ] **VIZ-01**: Each agent rendered as a React Flow node showing name, role, model, and live status (idle/busy/dead) with color coding
- [ ] **VIZ-02**: Hierarchical auto-layout — orchestrator at top, workers below — derived from squad.yml topology
- [ ] **VIZ-03**: Continuous animated arrows on edges while a message is in-flight (processing status)
- [ ] **VIZ-04**: Edge labels or tooltips showing message task, priority, and timestamp

### UI Polish

- [x] **UI-01**: SPA assets bundled via rust-embed and served directly from the binary (no external files)
- [ ] **UI-02**: Dark and light theme support with toggle
- [ ] **UI-03**: Connection status indicator in the UI showing WebSocket state (connected/reconnecting/disconnected)

## Constraints

- **Additive only**: No modifications to existing shipped core logic — new modules, new command, new files only
- **Research first**: Architecture must be researched before implementation begins

## v1.8 Requirements (Shipped)

### Fleet Intelligence

- [x] **INTEL-01**: Orchestrator context file includes count of pending messages per agent
- [x] **INTEL-02**: Orchestrator context file includes duration since agent became busy
- [x] **INTEL-03**: Orchestrator context file includes task-role alignment hints
- [x] **INTEL-04**: Fleet Status section placed after Completion Notification in orchestrator context
- [x] **INTEL-05**: `build_orchestrator_md()` accepts `&[AgentMetrics]` as pure function

### Clone Command

- [x] **CLONE-01**: `squad-station clone <agent>` creates a new agent with auto-incremented name
- [x] **CLONE-02**: Clone uses DB-first pattern with tmux rollback on failure
- [x] **CLONE-03**: Orchestrator cannot clone itself (rejection guard)
- [x] **CLONE-04**: Clone auto-regenerates orchestrator context file
- [x] **CLONE-05**: Cloned agents appear in TUI dashboard on next poll cycle
- [x] **CLONE-06**: Clone inherits model, description, and tool from source agent

### Role Templates

- [x] **TMPL-01**: Init wizard offers 11 pre-built role templates (8 worker + 3 orchestrator)
- [x] **TMPL-02**: Template selector uses split-pane TUI layout (role list + description preview)
- [x] **TMPL-03**: Selecting a template auto-fills model and description fields
- [x] **TMPL-04**: "Custom" template option allows freeform role/model/description entry
- [x] **TMPL-05**: Routing hints from templates embedded in Routing Matrix section of orchestrator context

## Future Requirements

### Verification / Integrity

- **VER-01**: Install script verifies checksum of downloaded binary (SHA256)
- **VER-02**: GitHub Release includes `checksums.txt` with SHA256 for all assets

### Extended Distribution

- **DIST-01**: Homebrew formula for `brew install squad-station`
- **DIST-02**: AUR package for Arch Linux users

### Browser Interactivity (Post v1.9)

- **BINT-01**: Click agent node to inspect message history and details
- **BINT-02**: Filter/search messages in browser UI
- **BINT-03**: Manual layout adjustment — drag and reposition nodes

## Out of Scope

| Feature | Reason |
|---------|--------|
| Task management / workflow logic | Orchestrator AI responsibility |
| Full web dashboard with CRUD / management | Browser view is read-only visualization only |
| Agent-to-agent direct messaging | All communication routes through orchestrator |
| Git conflict resolution | Orchestrator sequences work to avoid |
| Windows support | tmux not available on Windows; out of scope by architecture |
| Checksum verification | Deferred to future milestone |
| Homebrew tap | Additional maintenance burden — deferred |
| Interactive browser features (click/filter/drag) | Deferred to post-v1.9 milestone |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SRV-01 | Phase 26 | Complete |
| SRV-02 | Phase 26 | Complete |
| SRV-03 | Phase 26 | Complete |
| SRV-04 | Phase 26 | Complete |
| RT-01 | Phase 27 | Pending |
| RT-02 | Phase 27 | Pending |
| RT-03 | Phase 27 | Pending |
| RT-04 | Phase 27 | Pending |
| VIZ-01 | Phase 28 | Pending |
| VIZ-02 | Phase 28 | Pending |
| VIZ-03 | Phase 28 | Pending |
| VIZ-04 | Phase 28 | Pending |
| UI-01 | Phase 26 | Complete |
| UI-02 | Phase 28 | Pending |
| UI-03 | Phase 28 | Pending |

**Coverage:**
- v1.9 requirements: 15 total
- Mapped to phases: 15 (Phase 26: 5, Phase 27: 4, Phase 28: 6)
- Unmapped: 0

---
*Requirements defined: 2026-03-22*
*Last updated: 2026-03-22 — traceability mapped after roadmap creation (Phases 25-28)*
