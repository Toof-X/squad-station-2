# Requirements: Squad Station

**Defined:** 2026-03-17
**Core Value:** Routing messages reliably between Orchestrator and agents — stateless CLI, no daemon

## v1.7 Requirements

### Welcome TUI

- [ ] **WELCOME-01**: Bare `squad-station` always shows interactive TUI (replaces static welcome screen)
- [ ] **WELCOME-02**: TUI displays large SQUAD-STATION title using pixel-font big text
- [ ] **WELCOME-03**: TUI displays current version below title
- [ ] **WELCOME-04**: TUI shows hint bar at bottom with available keys and auto-exit countdown
- [ ] **WELCOME-05**: TUI includes quick guide page explaining Squad Station concept and basic workflow
- [ ] **WELCOME-06**: TUI auto-exits after N seconds if no key pressed (countdown shown in hint bar)
- [ ] **WELCOME-07**: Non-TTY fallback — when stdout is not a terminal, print static text instead of TUI

### First-Run Init

- [ ] **INIT-01**: When no squad.yml exists, Enter key in welcome TUI launches the init wizard directly
- [ ] **INIT-02**: When squad.yml exists, Enter key closes welcome (no re-init triggered)
- [ ] **INIT-03**: Q / Escape closes the welcome TUI without launching anything

### Install Flow

- [ ] **INSTALL-01**: npm postinstall checks `process.stdout.isTTY` and auto-launches `squad-station` if interactive
- [ ] **INSTALL-02**: curl | sh installer checks `[ -t 1 ]` and auto-launches `squad-station` if interactive
- [ ] **INSTALL-03**: Both install scripts degrade silently in non-interactive environments (CI, pipes, sudo)

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

*(Populated during roadmap creation)*

| Requirement | Phase | Status |
|-------------|-------|--------|
| WELCOME-01 | — | Pending |
| WELCOME-02 | — | Pending |
| WELCOME-03 | — | Pending |
| WELCOME-04 | — | Pending |
| WELCOME-05 | — | Pending |
| WELCOME-06 | — | Pending |
| WELCOME-07 | — | Pending |
| INIT-01 | — | Pending |
| INIT-02 | — | Pending |
| INIT-03 | — | Pending |
| INSTALL-01 | — | Pending |
| INSTALL-02 | — | Pending |
| INSTALL-03 | — | Pending |

**Coverage:**
- v1.7 requirements: 13 total
- Mapped to phases: 0
- Unmapped: 13 ⚠️

---
*Requirements defined: 2026-03-17*
*Last updated: 2026-03-17 after initial definition*
