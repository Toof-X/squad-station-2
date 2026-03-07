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

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Tests | Key Change |
|-----------|--------|-------|-------|------------|
| v1.0 | 3 | 10 | 58 | Initial process — strict phase dependencies, Nyquist validation |

### Cumulative Quality

| Milestone | Tests | Failures | Tech Debt Items |
|-----------|-------|----------|-----------------|
| v1.0 | 58 | 0 | 6 (all non-critical) |

### Top Lessons (Verified Across Milestones)

1. Safety-first architecture: wire all safety primitives in the foundation phase
2. Stateless CLI + SQLite WAL = simple, testable, concurrent-safe
