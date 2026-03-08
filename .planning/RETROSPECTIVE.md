# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-06
**Phases:** 3 | **Plans:** 10 | **Tests:** 58

### What Was Built
- Stateless CLI binary with 8 subcommands for multi-agent orchestration via tmux
- SQLite WAL storage with concurrent-safe writes and idempotent send/signal
- Provider-agnostic hook scripts for Claude Code and Gemini CLI
- Agent liveness reconciliation (idle/busy/dead) via tmux session detection
- Ratatui TUI dashboard with connect-per-refresh DB strategy
- Split tmux pane layout for fleet monitoring

### What Worked
- Strict phase dependency chain (foundation → lifecycle → views) prevented integration issues
- Safety primitives wired from Phase 1 (WAL, literal mode, SIGPIPE) — no safety bugs in later phases
- TDD and integration test infrastructure established early — 58 tests all green throughout
- Stateless architecture kept each phase cleanly scoped — no daemon state to manage
- Reconciliation loop pattern (check tmux, update DB) reused across agents/status/context commands

### What Was Inefficient
- SUMMARY frontmatter `requirements_completed` not filled for Phase 3 plans — documentation tracking gap discovered at audit time
- Phase 1 plan checkboxes in ROADMAP.md not fully checked (01-02 through 01-05 still unchecked despite completion)
- Hook script registration left as "user setup required" — could have been automated or at least warned

### Patterns Established
- Single-writer SQLite pool: `max_connections(1)` prevents async deadlock
- tmux arg builder helpers: private fns for unit testability without live tmux
- INSERT OR IGNORE for idempotent registration
- UPDATE WHERE status='pending' for idempotent signal completion
- lib.rs + main.rs split for integration test access
- connect-per-refresh in TUI to prevent WAL checkpoint starvation
- Reconciliation loop duplication (per-command) over shared abstraction
- Subprocess binary invocation for end-to-end guard testing
- File-based SQLite (not in-memory) for integration tests with subprocess

### Key Lessons
1. Wire safety primitives in Phase 1 — retrofitting WAL mode or SIGPIPE handling is harder than building it in
2. Stateless CLI architecture simplifies testing enormously — each command is a pure function of (config, DB, tmux state)
3. Provider-agnostic hooks via shell scripts work well — TMUX_PANE detection is universal across providers
4. connect-per-refresh is the right SQLite pattern for long-running TUI — prevents WAL bloat without complexity
5. Reconciliation loop duplication (~10 lines) is preferable to coupling independent command files

### Cost Observations
- Model mix: ~70% sonnet, ~25% haiku, ~5% opus
- Sessions: ~8 planning + execution sessions
- Notable: Entire MVP shipped in 2 days with AI-assisted development

---

## Milestone: v1.1 — Design Compliance

**Shipped:** 2026-03-08
**Phases:** 3 (4-6) | **Plans:** 7 | **Files changed:** 47

### What Was Built
- Refactored `squad.yml` config: `project` string, `model`/`description` per agent, removed `command`, `provider`→`tool`
- Bidirectional messages schema: `from_agent`/`to_agent`, `type`, `processing` status, `completed_at`
- Agents schema: `model`, `description`, `current_task` FK, `tool` field
- Notification hooks for Claude Code and Gemini CLI (permission prompt forwarding)
- `send --body` named flag, auto-prefix agent naming `<project>-<tool>-<role>`, standardized signal format
- ARCHITECTURE.md and PLAYBOOK.md fully rewritten to document post-v1.1 codebase accurately

### What Worked
- Phase 4 landing all schema changes in a single atomic migration (0003_v11.sql) — clean upgrade path from v1.0 DB
- CONF-04 and AGNT-03 (provider→tool) landed in the same phase — kept DB + config in sync
- TDD for shell scripts (test-notify-hooks.sh) — RED/GREEN pattern works even for bash
- Strict sequence (schema → features → docs) prevented docs being out of date before code was stable
- 19/19 requirements fully checked off — no gaps or tech debt from this milestone

### What Was Inefficient
- Phase 6 plan checkboxes in ROADMAP.md showed as `[ ]` despite completion — tracking state got out of sync
- SUMMARY frontmatter `one_liner` field not populated — milestone complete tool got empty accomplishments and had to be filled manually
- No milestone audit (v1.1-MILESTONE-AUDIT.md) was created before completion — skipped the audit step

### Patterns Established
- `agent_name = to_agent` backward compat bridge on INSERT — avoids breaking subqueries while migrating column semantics
- `#[sqlx(rename)]` for reserved SQL keywords and field aliases during migration transition
- `SQUAD_STATION_DB` env var in `resolve_db_path` — single injection point for test DB isolation
- Notification hook pattern: read-stdin → TMUX_PANE check → AGENT_NAME → SQUAD_BIN guard → JSON parse → orchestrator lookup → tmux send-keys
- Documentation accuracy: src/ as single source of truth — docs updated from direct code reads only

### Key Lessons
1. Fill SUMMARY `one_liner` during plan execution — milestone tooling depends on it for accomplishments
2. Create milestone audit before completion — even a quick audit surfaces hidden gaps
3. Keep ROADMAP.md plan checkboxes in sync as plans complete — stale `[ ]` causes confusion at milestone close
4. Atomic schema migrations with clear before/after states work well — no data loss, clean upgrade path
5. TDD for shell scripts is viable — exit code tests + content checks give meaningful RED/GREEN signal

### Cost Observations
- Model mix: ~80% sonnet, ~20% haiku
- Sessions: ~5 execution sessions
- Notable: All 19 requirements shipped in 1 day — 7 plans, fast execution due to clear gap analysis upfront

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Tests | Key Change |
|-----------|--------|-------|-------|------------|
| v1.0 | 3 | 10 | 58 | Initial process — strict phase dependencies, Nyquist validation |
| v1.1 | 3 | 7 | 58+ | Schema-first migration, TDD for shell scripts, gap-analysis-driven scope |

### Cumulative Quality

| Milestone | Tests | Failures | Tech Debt Items |
|-----------|-------|----------|-----------------|
| v1.0 | 58 | 0 | 6 (all non-critical) |
| v1.1 | 58+ | 0 | 0 (clean close) |

### Top Lessons (Verified Across Milestones)

1. Safety-first architecture: wire all safety primitives in the foundation phase
2. Stateless CLI + SQLite WAL = simple, testable, concurrent-safe
3. Atomic schema migrations with clear before/after states — clean upgrade path, no data loss
4. Fill SUMMARY one_liner during execution — milestone tooling depends on it
