# Requirements: Squad Station

**Defined:** 2026-03-17
**Core Value:** Routing messages reliably between Orchestrator and agents — stateless CLI, no daemon

## v1.7 Requirements

### Welcome TUI

- [x] **WELCOME-01**: Bare `squad-station` always shows interactive TUI (replaces static welcome screen)
- [x] **WELCOME-02**: TUI displays large SQUAD-STATION title using pixel-font big text
- [x] **WELCOME-03**: TUI displays current version below title
- [x] **WELCOME-04**: TUI shows hint bar at bottom with available keys and auto-exit countdown
- [x] **WELCOME-05**: TUI includes quick guide page explaining Squad Station concept and basic workflow
- [x] **WELCOME-06**: TUI auto-exits after N seconds if no key pressed (countdown shown in hint bar)
- [x] **WELCOME-07**: Non-TTY fallback — when stdout is not a terminal, print static text instead of TUI

### First-Run Init

- [x] **INIT-01**: When no squad.yml exists, Enter key in welcome TUI launches the init wizard directly
- [x] **INIT-02**: When squad.yml exists, Enter key closes welcome (no re-init triggered)
- [x] **INIT-03**: Q / Escape closes the welcome TUI without launching anything

### Install Flow

- [x] **INSTALL-01**: npm postinstall checks `process.stdout.isTTY` and auto-launches `squad-station` if interactive
- [x] **INSTALL-02**: curl | sh installer checks `[ -t 1 ]` and auto-launches `squad-station` if interactive
- [x] **INSTALL-03**: Both install scripts degrade silently in non-interactive environments (CI, pipes, sudo)

## v2 Requirements

*(None identified — scope is focused)*

## Out of Scope

| Feature | Reason |
|---------|--------|
| Web onboarding page | TUI is sufficient; web adds infra complexity |
| Auto-update check on welcome | Network call on every bare invocation is slow and privacy-invasive |
| Analytics / telemetry on first-run | Out of scope by design — no tracking |
| Animated splash screen | tui-big-text renders static pixel font; animation adds complexity without value |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| WELCOME-01 | Phase 20 | Complete |
| WELCOME-02 | Phase 20 | Complete |
| WELCOME-03 | Phase 20 | Complete |
| WELCOME-04 | Phase 20 | Complete |
| WELCOME-05 | Phase 21 | Complete |
| WELCOME-06 | Phase 20 | Complete |
| WELCOME-07 | Phase 20 | Complete |
| INIT-01 | Phase 20 | Complete |
| INIT-02 | Phase 20 | Complete |
| INIT-03 | Phase 20 | Complete |
| INSTALL-01 | Phase 21 | Complete |
| INSTALL-02 | Phase 21 | Complete |
| INSTALL-03 | Phase 21 | Complete |

**Coverage:**
- v1.7 requirements: 13 total
- Mapped to phases: 13
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-17*
*Last updated: 2026-03-17 — traceability populated after roadmap creation*
