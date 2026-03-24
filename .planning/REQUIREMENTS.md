# Requirements: Squad Station v2.0 Workflow Watchdog

**Defined:** 2026-03-24
**Core Value:** Reliable message routing between Orchestrator and agents — stateless CLI, no daemon

## v2.0 Requirements

Requirements for workflow watchdog milestone. Each maps to roadmap phases.

### Stall Detection

- [x] **DETECT-01**: Watchdog detects deadlock state — processing/pending messages exist but zero agents are busy
- [x] **DETECT-02**: Watchdog debounces stall detection across N consecutive poll cycles before triggering alert (prevents false positives from transient windows)
- [x] **DETECT-03**: Watchdog respects configurable message age threshold — only flags messages older than threshold as stalled
- [x] **DETECT-04**: Watchdog detects prolonged-busy single-agent stall and injects notification into orchestrator pane (completes existing Pass 3 gap)

### Alerting

- [x] **ALERT-01**: Watchdog injects stall notification into orchestrator's tmux pane with actionable message (agent count, pending message count, stall duration)
- [x] **ALERT-02**: Watchdog deduplicates alerts with configurable cooldown — same stall condition does not repeat until cooldown expires
- [ ] **ALERT-03**: Watchdog sends stall alert to user via Telegram Bot API (bot token + chat ID configuration)
- [ ] **ALERT-04**: Telegram dispatch is non-blocking and error-isolated — network timeouts, 429 rate limits, and MCP unavailability do not crash or stall the watchdog loop

### Operations

- [x] **OPS-01**: `squad-station watch --status` reports whether daemon is alive, PID, and uptime
- [x] **OPS-02**: Watchdog supports configurable poll interval, stall threshold, and alert cooldown via CLI flags
- [x] **OPS-03**: Watchdog supports `--dry-run` mode that logs stall detections without sending alerts

## Future Requirements

Deferred to future release. Tracked but not in current roadmap.

### Alerting Extensions

- **ALERT-05**: Standalone `squad-station alert --message "..."` subcommand for orchestrator-initiated Telegram alerts
- **ALERT-06**: Alert history persisted to DB for audit trail

### Advanced Detection

- **DETECT-05**: Watchdog detects circular dependency stalls (agents waiting on each other)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Auto-recovery / auto-restart agents | Creates restart loops — orchestrator decides recovery strategy |
| Slack / Discord integration | Premature multi-channel — Telegram sufficient for v2.0 |
| Auto-scaling on stall detection | Stalls are not capacity problems — wrong diagnosis |
| reqwest as feature-gated dependency | Decision deferred to implementation — curl shell-out or reqwest both viable |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DETECT-01 | Phase 29 | Complete |
| DETECT-02 | Phase 29 | Complete |
| DETECT-03 | Phase 29 | Complete |
| DETECT-04 | Phase 29 | Complete |
| ALERT-01 | Phase 29 | Complete |
| ALERT-02 | Phase 29 | Complete |
| ALERT-03 | Phase 30 | Pending |
| ALERT-04 | Phase 30 | Pending |
| OPS-01 | Phase 29 | Complete |
| OPS-02 | Phase 29 | Complete |
| OPS-03 | Phase 29 | Complete |

**Coverage:**
- v2.0 requirements: 11 total
- Mapped to phases: 11
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-24*
*Last updated: 2026-03-24 — traceability updated after roadmap creation*
