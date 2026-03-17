# Milestones

## v1.6 UX Polish (Shipped: 2026-03-17)

**Phases completed:** 2 phases, 3 plans, 0 tasks

**Key accomplishments:**
- (none recorded)

---

## v1.5 Interactive Init Wizard (Shipped: 2026-03-17)

**Phases:** 16-17 | **Plans:** 4 | **Files changed:** 12 (+2,754 / -367) | **Timeline:** 1 day (2026-03-17)
**Git range:** feat(16-01) wizard data types → feat(17-02) re-init prompt

**Key accomplishments:**
- Multi-page ratatui wizard (1362 lines): 5 pages collecting project name, SDD workflow, orchestrator + N worker configs with cursor-aware text inputs and radio selectors
- `squad-station init` is now fully self-contained — no pre-existing squad.yml required; wizard generates one from user input
- Squad.yml generation from `WizardResult` with full model ID validation (`claude-sonnet-4-6`, `gemini-2.5-pro`, etc.) — 201 tests pass
- Re-init prompt (overwrite/add-agents/abort) via crossterm raw-mode keypress with non-interactive TTY guard for backward-compatible CI/tests
- Worker-only wizard entry point (`run_worker_only`) enables "add agents" flow without re-collecting project/orchestrator config
- SDD workflow selection (Bmad/GetShitDone/Superpower) embedded in wizard Project page

**Archives:** [v1.5-ROADMAP.md](milestones/v1.5-ROADMAP.md) | [v1.5-REQUIREMENTS.md](milestones/v1.5-REQUIREMENTS.md)

---

## v1.4 Unified Playbook & Local DB (Shipped: 2026-03-10)

**Phases:** 14-15 | **Plans:** 4 | **Files changed:** 23 (+1,505 / -213) | **Timeline:** 1 day (2026-03-10)
**Git range:** docs(14) create phase plan → docs(phase-15) complete phase execution

**Key accomplishments:**
- `context` generates single unified `squad-orchestrator.md` replacing 3 fragmented files (squad-delegate, squad-monitor, squad-roster)
- `init` Get Started message references new `squad-orchestrator.md` path
- DB path moved from `~/.agentic-squad/<project>/station.db` to `<cwd>/.squad/station.db` for data locality
- `dirs` crate removed from dependencies (no longer needed for home dir resolution)
- `.gitignore`, `CLAUDE.md`, and `README.md` updated for new DB location; all `~/.agentic-squad/` references removed

**Archives:** [v1.4-ROADMAP.md](milestones/v1.4-ROADMAP.md) | [v1.4-REQUIREMENTS.md](milestones/v1.4-REQUIREMENTS.md)

---

## v1.3 Antigravity & Hooks Optimization (Shipped: 2026-03-09)

**Phases:** 10-13 | **Plans:** 8 | **Timeline:** 1 day (2026-03-09)
**Git range:** feat(10-01) signal pane detection → docs(phase-13): complete phase execution

**Key accomplishments:**
- `signal` accepts `$TMUX_PANE` env var — zero-arg inline hook command, shell scripts deprecated
- `antigravity` provider: DB-only orchestrator skips all tmux interaction (no sessions, no send-keys)
- `context` generates `.agent/workflows/` with 3 files: squad-delegate.md, squad-monitor.md, squad-roster.md
- `init` safely merges hooks into existing `settings.json` with `.bak` backup; prints instructions when absent
- `inject_body` via `load-buffer`/`paste-buffer` + temp file — safe multiline task body delivery
- PLAYBOOK.md rewritten as authoritative v1.3 guide covering inline hooks, Antigravity mode, Notification hooks

**Archives:** [v1.3-ROADMAP.md](milestones/v1.3-ROADMAP.md) | [v1.3-REQUIREMENTS.md](milestones/v1.3-REQUIREMENTS.md) | [v1.3-MILESTONE-AUDIT.md](milestones/v1.3-MILESTONE-AUDIT.md)

---

## v1.2 Distribution (Shipped: 2026-03-09)

**Phases:** 7-9 | **Plans:** 5 | **Files changed:** 24 (+2,955 lines) | **Timeline:** 1 day (2026-03-09)

**Key accomplishments:**
- GitHub Actions matrix CI/CD: 4-target cross-compilation (darwin-arm64, darwin-x86_64, linux-x86_64, linux-arm64) with musl static Linux binaries
- npm package `squad-station` with zero-dependency postinstall binary downloader (platform/arch detection, redirect following)
- POSIX sh curl-pipe-sh installer: OS/arch detection via `uname`, GitHub Releases download, `/usr/local/bin` install with `~/.local/bin` fallback
- GitHub landing page README: npm/curl/source install methods, 5-step quickstart, architecture overview, PLAYBOOK link

---

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

