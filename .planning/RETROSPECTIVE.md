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

## Milestone: v1.2 — Distribution

**Shipped:** 2026-03-09
**Phases:** 3 (7-9) | **Plans:** 5 | **Files changed:** 24 (+2,955 lines)

### What Was Built
- GitHub Actions matrix workflow: 4-target cross-compilation (darwin-arm64, darwin-x86_64, linux-x86_64, linux-arm64) with musl static Linux binaries
- npm package `squad-station` with zero-dependency postinstall binary downloader (platform/arch detection, redirect following)
- npm end-to-end verified on darwin-arm64: `npm install -g` from packed tarball, binary in PATH, all 11 commands accessible
- POSIX sh curl-pipe-sh installer: OS/arch detection via `uname`, GitHub Releases download, `/usr/local/bin` install with `~/.local/bin` fallback
- GitHub landing page README: npm/curl/source install methods, 5-step quickstart, architecture overview, PLAYBOOK link

### What Worked
- Binary naming convention `squad-station-{os}-{arch}` established in Phase 7 consumed by Phase 8 and 9 — zero coordination overhead
- musl static binaries chosen upfront for Linux: no glibc compatibility issues at install time
- `softprops/action-gh-release@v2` idempotent asset upload: 4 parallel matrix jobs don't race on release creation
- `fail-fast: false` in matrix: all 4 targets attempted even on partial failure — useful for diagnosing platform-specific issues
- POSIX sh for install script (not bash): runs on Alpine, macOS, and minimal Linux distros without issues
- Phase dependency structure (CI/CD → npm + install) was clean: npm and install script could be developed in parallel after binaries existed

### What Was Inefficient
- No milestone audit before completion — skipped v1.2-MILESTONE-AUDIT.md (pattern from v1.1 repeated)
- SUMMARY `one_liner` frontmatter field returned `None` — milestone tools can't extract accomplishments automatically; must be filled manually
- Phase 8 plan 2 (npm E2E verify) was largely a human checkpoint — could have been scoped as a task within plan 1

### Patterns Established
- GitHub Actions release: push v* tag → 4 parallel matrix jobs → single GitHub Release with binary assets
- Musl Linux builds: `musl-tools` apt install for x86_64; `cross` (Docker) for aarch64
- npm postinstall binary download: `https.get` with redirect following, platform map, `chmodSync`
- JS bin shim: `spawn` with `stdio: 'inherit'` + `process.exit(code)` for transparent passthrough
- curl install script: detect → download → verify executable → move to PATH
- `SQLX_OFFLINE=true`: required for CI builds where no DATABASE_URL is available

### Key Lessons
1. Establish binary naming convention in the CI/CD phase — downstream phases (npm, install script) consume it directly with no flexibility to change later
2. musl for Linux CI/CD is the right default — glibc-linked binaries will silently fail on Alpine or older distros
3. Fill SUMMARY `one_liner` field during plan execution — for the third milestone in a row, this was empty
4. npm E2E verification phase is valuable but a single `npm pack && npm install -g` from a packed tarball is sufficient human verification; a full dedicated plan adds overhead
5. `softprops/action-gh-release@v2` with `fail-fast: false` is the correct pattern for multi-platform release pipelines

### Cost Observations
- Model mix: ~80% sonnet, ~20% haiku
- Sessions: ~4 execution sessions
- Notable: Distribution layer (CI/CD + npm + install + docs) shipped in 1 day

---

## Milestone: v1.3 — Antigravity & Hooks Optimization

**Shipped:** 2026-03-09
**Phases:** 4 (10-13) | **Plans:** 8 | **Requirements:** 15/15

### What Was Built
- `signal $TMUX_PANE` inline hook — no shell scripts, any provider, zero-arg hook registration
- `antigravity` provider: DB-only orchestrator skips all tmux operations (no session, no notify)
- `context` generates `.agent/workflows/` with 3 files: squad-delegate.md, squad-monitor.md, squad-roster.md
- `init` safely merges hooks into existing `settings.json` with `.bak` backup + fallback instructions
- `inject_body` via `load-buffer`/`paste-buffer` + uuid temp file for safe multiline body injection
- PLAYBOOK.md v1.3 fully rewritten — canonical guide for inline hooks, Antigravity mode, Notification hooks

### What Worked
- Milestone audit run BEFORE completion (v1.3 is first milestone with formal audit) — all gaps surfaced before archiving
- Strict phase dependency chain (10→11→12→13) prevented integration issues; each phase cleanly scoped
- TDD pattern established in v1.0 and v1.1 held — new commands (context rewrite, init merge) fully integration-tested
- `is_db_only()` helper centralized provider check — downstream guards use one function, not string comparisons
- Phase 13 merged two concerns (safe injection + docs) efficiently — single phase, clean boundary

### What Was Inefficient
- SUMMARY frontmatter `one_liner` still not populated — milestone tools returned empty accomplishments (4th milestone in a row)
- ROADMAP.md progress table rows for phases 11-13 had broken column alignment — minor but accumulated across milestone
- Phase 12 removed tmux reconciliation from `context` — good decision, but could have been scoped in phase 11 when antigravity guard was added

### Patterns Established
- Pane ID detection: `starts_with('%')` — unambiguous tmux pane vs session name dispatch
- DB-only provider guard: `is_db_only()` on `AgentConfig` — single canonical check, not string comparison at call site
- Read-only command pattern: `context` writes files without touching DB or tmux
- JSON mode guard: suppress human-readable stdout when `--json` active — composable CLI output
- `inject_body` pattern: uuid temp file → `load-buffer` → `paste-buffer` → delete temp — safe concurrent multi-agent sends
- Hook merge: `merge_hook_entry` with command-field dedup, graceful fallback on malformed JSON, `.json.bak` backup

### Key Lessons
1. Run milestone audit before completion — v1.3 first to do this; `passed` with only tech debt items, no blocking gaps
2. Fill SUMMARY `one_liner` during plan execution — 4 milestones in a row with empty field; tooling can't help otherwise
3. `inject_body` uuid temp file is the right pattern for concurrent agent sends — prevents buffer clobbering
4. DB-only provider is a clean abstraction — no tmux dependency, testable in isolation, no live tmux needed for tests
5. PLAYBOOK rewrite as a dedicated plan (13-02) was high value — documentation debt from v1.0-v1.2 fully cleared

### Cost Observations
- Model mix: ~85% sonnet, ~15% haiku
- Sessions: ~5 execution sessions
- Notable: 15 requirements, 4 phases, 8 plans shipped in 1 day with formal milestone audit

---

## Milestone: v1.4 — Unified Playbook & Local DB

**Shipped:** 2026-03-10
**Phases:** 2 (14-15) | **Plans:** 4 | **Files changed:** 23 (+1,505 / -213)

### What Was Built
- `context` generates single unified `squad-orchestrator.md` replacing 3 fragmented workflow files
- `init` Get Started message updated to reference unified playbook
- DB path moved from `~/.agentic-squad/<project>/station.db` to `<cwd>/.squad/station.db`
- `dirs` crate removed from Cargo.toml dependencies
- `.gitignore`, `CLAUDE.md`, `README.md` updated for new DB location

### What Worked
- Smallest milestone yet (2 phases, 4 plans) — focused scope led to fast execution
- `resolve_db_path` as single injection point made DB path change surgical — one function change affected all commands
- Clean break from old DB path (no migration, no warnings) — correct call for dev builds
- Phase 14→15 dependency chain was straightforward — playbook change independent of DB change

### What Was Inefficient
- SUMMARY `one_liner` field still not populated — 5th milestone in a row with empty field
- No milestone audit — requirements were simple enough that skipping was acceptable but pattern continues
- discuss-phase interrupted mid-flow — only captured one decision before user moved to planning

### Patterns Established
- `build_orchestrator_md` as pub fn — integration tests verify playbook content directly
- CWD-relative DB path: `std::env::current_dir()` replaces `dirs::home_dir()` + project name
- Single unified context file over multiple fragments — reduces orchestrator context load

### Key Lessons
1. Small milestones (2 phases) are efficient — scope clarity reduces overhead
2. Single injection point pattern (resolve_db_path) pays off for cross-cutting changes
3. No migration needed for dev-only tools — clean break saves complexity
4. Still need to fill SUMMARY one_liner — tooling gap persists across 5 milestones

### Cost Observations
- Model mix: ~90% sonnet, ~10% opus (planner + checker)
- Sessions: ~3 execution sessions
- Notable: Entire milestone planned and executed in a single day

---

## Milestone: v1.5 — Interactive Init Wizard

**Shipped:** 2026-03-17
**Phases:** 2 (16-17) | **Plans:** 4 | **Files changed:** 12 (+2,754 / -367) | **Tests:** 201

### What Was Built
- Multi-page ratatui wizard (1362 lines): 5 pages collecting project name, SDD workflow, orchestrator + N worker configs with cursor-aware text inputs and radio selectors
- `squad-station init` now fully self-contained — no pre-existing squad.yml required
- Squad.yml generation from `WizardResult` with full model ID validation (short names + full IDs)
- Re-init prompt (overwrite/add-agents/abort) via crossterm raw-mode keypress
- Non-interactive TTY guard (`is_terminal()`) preserving backward compat with integration tests
- Worker-only wizard entry point (`run_worker_only`) enabling add-agents path without re-collecting project/orchestrator config

### What Worked
- TDD from plan 1: data types + validation tests written before TUI rendering — caught type mismatch early (Tool→Provider rename)
- Guard clause approach in init.rs: minimal diff, existing path unchanged, wizard plugs in cleanly before load_config
- String builder for YAML generation: deterministic field ordering, no new dependency (serde_yaml avoided)
- `is_terminal()` guard discovered during Task 1 verification (4 tests broke immediately) — self-corrected within same commit
- Phase 16 → Phase 17 dependency was clean: wizard API stable before integration work began
- 20+ unit tests added in Phase 16 exceeded plan (11 planned) — extra coverage for cursor movement and ModelSelector

### What Was Inefficient
- Human-verify checkpoint in Phase 16 Plan 02 caused an artificial pause — summary was written before verification completed; verification gate could have been scoped as a separate task post-build
- No milestone audit run — skipped again despite being recommended
- SUMMARY `one_liner` frontmatter still inconsistently populated

### Patterns Established
- TUI wizard page state machine: `WizardPage` enum drives both render fn and handle_key dispatch
- `render_radio_list` shared helper: reusable for any enum-selector field (SDD, Provider, Model)
- `PageTransition` enum from `handle_agent_key`: decouples field logic from page routing, no call-site changes needed
- `worker_only: bool` on `WizardState`: cleaner than threading flag parameter through handle_key callees
- Crossterm raw-mode keypress for single-choice prompts: enables non-wizard interactive prompts in CLI context
- `append_workers_to_yaml`: pure string manipulation for YAML append — consistent with generate_squad_yml approach

### Key Lessons
1. TDD pays off even for TUI code — type-level tests (Provider cycling, ModelSelector) caught design issues before rendering
2. Guard clause pattern for new features in existing commands: minimal diff, no risk to existing paths
3. String building for YAML generation is preferable to serde_yaml when structure is simple — deterministic + no new dep
4. Non-interactive guard (`is_terminal()`) should be added upfront whenever crossterm raw mode is used — not as a bugfix
5. Human-verify checkpoints mid-plan create documentation state inconsistency — better scoped as "build then separately verify"

### Cost Observations
- Model mix: ~90% sonnet, ~10% haiku
- Sessions: ~3 execution sessions
- Notable: 1362-line wizard module with 20+ tests built in 3 minutes (Phase 16 Plan 01 — fast execution)

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Tests | Key Change |
|-----------|--------|-------|-------|------------|
| v1.0 | 3 | 10 | 58 | Initial process — strict phase dependencies, Nyquist validation |
| v1.1 | 3 | 7 | 58+ | Schema-first migration, TDD for shell scripts, gap-analysis-driven scope |
| v1.2 | 3 | 5 | 58+ | Distribution layer — CI/CD, npm, curl installer, no new Rust code |
| v1.3 | 4 | 8 | 58+ | Provider abstraction, safe injection, first formal milestone audit |
| v1.4 | 2 | 4 | 164 | Unified playbook, local DB, smallest milestone — focused scope |
| v1.5 | 2 | 4 | 201 | TUI wizard for init, squad.yml generation, re-init prompt, crossterm UX |

### Cumulative Quality

| Milestone | Tests | Failures | Tech Debt Items |
|-----------|-------|----------|-----------------|
| v1.0 | 58 | 0 | 6 (all non-critical) |
| v1.1 | 58+ | 0 | 0 (clean close) |
| v1.2 | 58+ | 0 | 0 (audit skipped) |
| v1.3 | 58+ | 0 | 3 (cosmetic: stale comments, 1 edge case, 1 stale doc section) |
| v1.4 | 164 | 0 | 0 (clean close, audit skipped) |
| v1.5 | 201 | 0 | 0 (clean close, audit skipped) |

### Top Lessons (Verified Across Milestones)

1. Safety-first architecture: wire all safety primitives in the foundation phase
2. Stateless CLI + SQLite WAL = simple, testable, concurrent-safe
3. Atomic schema migrations with clear before/after states — clean upgrade path, no data loss
4. Fill SUMMARY one_liner during execution — milestone tooling depends on it (6 milestones in a row with empty field)
5. Run milestone audit before completion — v1.3 first and last to do this; should be default
6. Small focused milestones (2 phases) execute efficiently — minimal overhead, clear scope
7. TDD for TUI code works — type-level tests catch design issues before rendering implementation begins
8. Non-interactive guard (`is_terminal()`) must be added upfront for crossterm raw-mode code — not as a bugfix after tests break
