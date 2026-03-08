# Milestones

## v1.1 Design Compliance (Shipped: 2026-03-08)

**Phases:** 4-6 | **Plans:** 7 | **Files changed:** 47
**Lines of code:** 4,367 Rust (+6,302 / -828) | **Timeline:** 1 day (2026-03-08)
**Git range:** feat(04-01) config refactor → feat(06-02) PLAYBOOK rewrite

**Key accomplishments:**
- Refactored `squad.yml` config: `project` as string, `model`/`description` per agent, removed `command`, renamed `provider`→`tool`
- Bidirectional messages schema: `from_agent`/`to_agent`, `type` column, `processing` status, `completed_at` timestamp
- Agents schema extended: `model`, `description`, `current_task` FK linking agent to active message
- Notification hooks for Claude Code and Gemini CLI forwarding permission prompts to orchestrator
- `send` CLI changed to `--body` flag; `init` auto-prefixes agent names as `<project>-<tool>-<role>`; signal format standardized to `"<agent> completed <msg-id>"`
- ARCHITECTURE.md and PLAYBOOK.md rewritten to accurately document post-v1.1 codebase (19/19 requirements complete)

**Archives:** [v1.1-ROADMAP.md](milestones/v1.1-ROADMAP.md) | [v1.1-REQUIREMENTS.md](milestones/v1.1-REQUIREMENTS.md)

---

## v1.0 MVP (Shipped: 2026-03-06)

**Phases:** 3 | **Plans:** 10 | **Tests:** 58
**Lines of code:** 2,994 Rust | **Commits:** 64
**Timeline:** 2 days (2026-03-05 → 2026-03-06)
**Git range:** Initial commit → docs(phase-03): update validation strategy

**Key accomplishments:**
- Stateless CLI binary with 8 subcommands (init, send, signal, list, peek, register, agents, status, ui, view, context)
- SQLite WAL mode with concurrent-safe writes, idempotent send/signal messaging, priority-ordered queue
- Agent liveness reconciliation (idle/busy/dead) with live tmux session detection
- Provider-agnostic hook scripts for Claude Code (Stop event) and Gemini CLI (AfterAgent event)
- Ratatui TUI dashboard with connect-per-refresh DB strategy preventing WAL checkpoint starvation
- Split tmux pane layout for fleet-wide agent monitoring
- 58 tests, 0 failures, full Nyquist compliance across all 3 phases

**Known tech debt (6 non-critical items):**
- Phase 3 SUMMARY frontmatter missing `requirements_completed` for VIEW-01–04
- 5 human verification items pending (TUI render, tmux view, etc.)
- Stale test assertion count in `test_sigpipe_binary_starts`
- Orphaned `db::Pool` type alias
- `ui.rs` bypasses `db::connect()` with own read-only pool (intentional)
- Hook scripts require manual user registration in provider settings

**Archives:** [v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md) | [v1.0-REQUIREMENTS.md](milestones/v1.0-REQUIREMENTS.md) | [v1.0-MILESTONE-AUDIT.md](milestones/v1.0-MILESTONE-AUDIT.md)

---

