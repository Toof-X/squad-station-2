# Requirements: Squad Station

**Defined:** 2026-03-10
**Core Value:** Routing messages đáng tin cậy giữa Orchestrator và agents — gửi task đúng agent, nhận signal khi hoàn thành, notify Orchestrator — tất cả qua stateless CLI commands không cần daemon

## v1.4 Requirements

### Playbook

Unified orchestrator playbook replacing fragmented 3-file context approach.

- [ ] **PLAY-01**: `context` generates a single `squad-orchestrator.md` (replaces 3 fragmented files: squad-delegate, squad-monitor, squad-roster)
- [ ] **PLAY-02**: Playbook uses `withClaudeCodeTmux.vi.toml` wording as base template, adapted for markdown delivery
- [ ] **PLAY-03**: Playbook dynamically injects agent list from `squad.yml` (names, models, descriptions, roles)
- [ ] **PLAY-04**: `init` "Get Started" console output references the new `squad-orchestrator.md` path instead of deprecated `squad-delegate.md`

### LocalDB

Move DB into project directory for data locality, simplicity, and no name-collision risk.

- [ ] **LODB-01**: Default DB path changes from `~/.agentic-squad/<project>/station.db` to `<cwd>/.squad/station.db`
- [ ] **LODB-02**: `dirs` crate dependency removed from `Cargo.toml` (no longer needed for home dir resolution)
- [ ] **LODB-03**: `.gitignore` gets `.squad/` entry so the local DB isn't committed
- [ ] **LODB-04**: `SQUAD_STATION_DB` env var override continues to work (no behavior change)
- [ ] **LODB-05**: Test `test_db_path_resolution_default` updated to assert `.squad/station.db` (not `.agentic-squad`)
- [ ] **LODB-06**: `CLAUDE.md` and `README.md` updated with new DB path

## v2 Requirements

None — both gaps are fully scoped for v1.4.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Migration script for existing `~/.agentic-squad/` DBs | Users on dev builds; no production data to migrate |
| Support for multiple DB locations | Adds complexity; env var override covers edge cases |
| TOML playbook format for non-Claude providers | Markdown is provider-agnostic; TOML only relevant for Antigravity/Claude Code specifically |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| PLAY-01 | Phase 14 | Pending |
| PLAY-02 | Phase 14 | Pending |
| PLAY-03 | Phase 14 | Pending |
| PLAY-04 | Phase 14 | Pending |
| LODB-01 | Phase 15 | Pending |
| LODB-02 | Phase 15 | Pending |
| LODB-03 | Phase 15 | Pending |
| LODB-04 | Phase 15 | Pending |
| LODB-05 | Phase 15 | Pending |
| LODB-06 | Phase 15 | Pending |

**Coverage:**
- v1.4 requirements: 10 total
- Mapped to phases: 10
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-10*
*Last updated: 2026-03-10 after initial definition*
