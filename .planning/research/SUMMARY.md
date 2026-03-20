# Research Summary

**Project:** Squad Station
**Domain:** Rust CLI binary — stateless tmux message router with embedded SQLite for AI agent orchestration
**Researched:** 2026-03-06
**Confidence:** HIGH

## Executive Summary

Squad Station is a stateless Rust CLI binary that routes messages between AI coding agents running in tmux sessions, using embedded SQLite as its sole persistence layer. The research across all four areas converges on a clear, low-complexity architecture: each invocation opens the DB, executes one action, writes state, and exits. There is no daemon, no async runtime, and no background process. This design is validated by comparable tools (NTM, Agent Deck, Overstory) and is the correct choice for a developer-focused CLI that must be debuggable, CI-friendly, and installable without runtime dependencies.

The recommended stack is well-established and high-confidence: clap 4.5.x (derive macros for CLI), rusqlite 0.38.0 with bundled SQLite (synchronous, zero system dependency), ratatui 0.30 + crossterm (TUI dashboard), serde + serde-saphyr (YAML config — serde_yaml is deprecated and must not be used), and std::process::Command for all tmux operations (the tmux_interface crate is experimental and unnecessary). The npm optionalDependencies distribution pattern, used by esbuild and Biome, is the right approach for making the binary accessible to Node.js developers without requiring Rust toolchain installation.

The primary risks are not architectural but operational: SQLite concurrent write failures when multiple agent hooks fire simultaneously, special character injection via tmux send-keys, an orchestrator infinite hook loop if the skip guard is missing, and stale agent status after crashes. All of these have known, concrete solutions that must be built into Phase 1 — not retrofitted later. Missing any of the Phase 1 safety items before distributing hook configuration to users creates serious footguns.

---

## Key Findings

### Recommended Stack

Squad Station is a synchronous, stateless application — async runtimes (tokio, sqlx) add overhead with zero benefit and are explicitly excluded. The synchronous rusqlite + std::process::Command combination is exactly right for a CLI that runs-acts-exits. The TUI (ratatui 0.30) uses a simple blocking event loop with periodic DB polling — no async required there either.

**Core technologies:**
- `clap 4.5.x`: CLI argument parsing — derive macros map 1:1 to the ~8 subcommands; de-facto standard
- `rusqlite 0.38.0` (bundled feature): SQLite with static linking — zero system dependency, synchronous, correct for stateless CLI
- `rusqlite_migration 2.4.0`: Schema versioning — must be wired up from day one; retrofitting is a breaking change
- `ratatui 0.30` + `crossterm 0.28`: TUI dashboard — active tui-rs fork, cross-platform, version 0.30 is the current major release
- `serde` + `serde-saphyr`: YAML config parsing — serde_yaml is officially archived (March 2024), use serde-saphyr
- `std::process::Command`: tmux operations — the tmux_interface crate is pre-1.0, experimental, and unnecessary for 2 simple commands
- `anyhow` + `thiserror`: Error handling — anyhow at command layer for user-facing messages, thiserror in modules for typed errors
- `tracing` + `tracing-subscriber`: Structured logging — better than env_logger, async-ready for future-proofing
- npm `optionalDependencies` pattern: Distribution — platform-specific binary packages, no postinstall script required; used by esbuild, Biome, Bun

**Explicit exclusions:** tokio, sqlx, Diesel, tui-rs (archived), tmux_interface crate (experimental), serde_yaml (archived), NAPI-RS (wrong tool for standalone binary), daemon/server architecture.

### Expected Features

The feature dependency chain defines a strict build order: `schema → register → send → signal → status`. All UI and distribution features sit on top of this critical path.

**Must have (table stakes):**
- `send` — inject task into named agent via tmux send-keys
- `signal` — agent completion hook with orchestrator skip guard (MUST ship together — skip guard is non-negotiable)
- `status` — query agent lifecycle state (idle/busy/dead) via DB + tmux live check
- `init` — bootstrap agent registry from squad.yml declarative config
- `register` — dynamic agent registration (idempotent, SQLite upsert)
- `list` — enumerate agents with `--json` flag for scripting
- Per-project DB isolation — `~/.agentic-squad/<project>/station.db`
- Idempotency on `send` — message-ID deduplication to prevent double-dispatch
- Human-readable errors with actionable hints and machine-parseable exit codes
- Cross-platform binary (macOS darwin arm64/x64, Linux x64/arm64)

**Should have (differentiators):**
- Provider-agnostic hook adapter — thin per-provider bash scripts, binary stays provider-agnostic
- Auto-generate orchestrator context file — reduces prompt engineering burden
- `view` — split tmux layout of all agent panes
- TUI dashboard (`ui` subcommand) — live agent fleet monitoring
- `--json` output on `list` and `status` for programmatic use

**Defer to v2+:**
- Spec-driven methodology integration
- Agent spawning / lifecycle management (creation of tmux sessions)
- Web UI / browser dashboard
- Cost tracking / token counting
- Git conflict resolution
- Retry/backoff for failed LLM calls

### Architecture Approach

The architecture is a layered stateless CLI with strict module boundaries. The `cli` module parses argv and builds a `Context` struct (project, db_path, config_path) that is passed to all handlers — no global state. The `commands` module orchestrates handlers that call into the `db`, `tmux`, and `config` service modules. The `tui` module is an isolated blocking event loop that reads DB but never writes. The `orchestrator` module generates context files from agent data.

**Major components:**
1. `cli` — clap parsing, Context struct, subcommand dispatch; never touches SQLite or tmux directly
2. `commands` — one file per subcommand (init, send, signal, register, status, view, context, ui); owns business logic
3. `db` — all SQLite access; schema, migrations, typed repositories for agents and message_log
4. `tmux` — thin TmuxAdapter wrapping std::process::Command; no business logic, pure I/O
5. `config` — squad.yml parsing via serde-saphyr; separates config reading from DB persistence
6. `tui` — ratatui event loop, read-only dashboard; component pattern, polling DB every 500ms
7. `orchestrator` — context file generator; depends only on db::models, pure text formatting

**Key patterns:**
- Context struct (not global state) — every handler receives deterministic, testable inputs
- Repository functions take `&Connection` — handler owns transaction boundary, enables atomic multi-table writes
- Thin tmux adapter — command handlers decide what to do; adapter only executes
- Migration-on-open — `open_db()` always runs migrations before returning; no separate migration step

### Critical Pitfalls

1. **SQLITE_BUSY on concurrent hook invocations** — Multiple agents completing simultaneously causes write lock contention. Prevention: enable WAL mode BEFORE migrations run (not inside a migration), set `PRAGMA busy_timeout=5000` on every connection open, use `BEGIN IMMEDIATE` for all writes. Must be in Phase 1.

2. **Orchestrator infinite hook loop** — Hooks fire for ALL sessions including the orchestrator. Without a skip guard in `signal`, the orchestrator signal triggers itself infinitely. Prevention: check current tmux session name against orchestrator name in SQLite; exit 0 silently if matched. Check `stop_hook_active` field for Claude Code specifically. This skip guard MUST ship before any hook configuration is distributed.

3. **tmux send-keys special character injection** — Semicolons, backticks, and escape sequences are parsed by tmux before reaching the shell. Prevention: always use `tmux send-keys -l` (literal mode) for message content; send Enter as a separate command. Must be caught in Phase 1 before any agent workflow testing.

4. **Shell initialization race on session create** — `tmux new-session` returns before the shell (especially with oh-my-zsh, nvm) finishes loading. Injected keys get lost. Prevention: poll `#{pane_current_command}` in a loop until it returns the shell name before sending keys. Add configurable `send_delay_ms` in squad.yml as fallback.

5. **Stale agent status after crash** — Agents that die without a clean hook exit leave status as "busy" forever. Prevention: reconcile DB status against live tmux session state on every `list` invocation; implement `last_seen_at` heartbeat timeout (default 10 minutes = presumed dead).

6. **Executable permissions lost in CI** — GitHub Actions artifact upload/download strips execute bits. Prevention: `chmod 0o755` on binary in postinstall or JS shim; explicit `chmod +x` before creating npm tarball in CI; smoke test that verifies binary is executable post-install.

7. **Rust SIGPIPE panic on piped output** — `squad-station list | head -5` produces a panic backtrace instead of clean exit. Prevention: reset SIGPIPE to SIG_DFL at the top of `main()` using the `libc` crate. One-time fix, add before shipping any commands.

---

## Implications for Roadmap

Based on the combined research, the feature dependency chain and pitfall phase-gating drive a clear 5-phase structure. The ordering is non-negotiable: each phase removes blockers for the next.

### Phase 1: Core Foundation
**Rationale:** Everything depends on the DB schema and the ability to send a message to an agent. The critical safety items (WAL, skip guard, literal send-keys, SIGPIPE) must exist before any hook-based integration or testing begins — shipping without them is actively dangerous.
**Delivers:** A working `init`, `register`, `send`, `signal`, `status`, and `list` flow. The binary is usable for basic orchestration.
**Addresses features:** SQLite schema, `register`, `init` (squad.yml), `send`, `signal` with orchestrator skip guard, `status`, `list` with `--json`, idempotency, per-project DB isolation, human-readable errors.
**Avoids pitfalls:** WAL + busy_timeout + BEGIN IMMEDIATE (P1), PRAGMA WAL before migrations (P2), orchestrator infinite loop skip guard (P9), tmux send-keys -l literal mode (P6), shell init race poll (P5), SIGPIPE handler (P18), errors to stderr (P11), explicit transaction commits (P19), env var DB path override (P20).
**Stack used:** clap, rusqlite (bundled), rusqlite_migration, serde-saphyr, std::process::Command, anyhow, thiserror, tracing.
**Research flag:** Standard patterns — no additional research needed; all technologies are well-documented.

### Phase 2: Agent Lifecycle and Hook Integration
**Rationale:** Once core messaging works, the system must handle the reality of agent crashes, provider-specific hooks, and multi-agent workflows. Stale status and wrong lifecycle detection break Orchestrator scheduling.
**Delivers:** Reliable agent liveness detection, provider-agnostic hook scripts for Claude Code and Gemini CLI, DB reconciliation on `list`, heartbeat timeout.
**Addresses features:** Provider-agnostic hook adapter, auto-generate orchestrator context file (`context` subcommand), `status` liveness refinement using `pane_current_command`.
**Avoids pitfalls:** Stale agent status after crash (P17), agent lifecycle false negatives using `pane_current_command` (P8), provider-specific hook divergence (P10).
**Research flag:** Provider hook schemas may need research — Claude Code's Stop hook JSON payload and Gemini CLI's AfterAgent payload need explicit verification during implementation.

### Phase 3: Views and TUI Dashboard
**Rationale:** Visual tooling is a differentiator but depends on stable core data. The TUI has specific pitfalls (WAL checkpoint starvation from long-lived read connections) that are safe to address after the foundation is solid.
**Delivers:** `view` subcommand (split tmux pane layout), `ui` subcommand (ratatui live dashboard showing idle/busy/dead status).
**Addresses features:** TUI dashboard, `view` split layout.
**Avoids pitfalls:** WAL checkpoint starvation — TUI refresh loop must open short-lived connections per poll, not hold one open persistently (P4). Never use capture-pane output parsing for completion detection (P7).
**Stack used:** ratatui 0.30, crossterm 0.28.
**Research flag:** ratatui 0.30 component architecture is well-documented on ratatui.rs — standard patterns, skip research phase.

### Phase 4: npm Distribution and CI Pipeline
**Rationale:** Distribution is the final unlock. It requires a stable binary (Phases 1-3 complete) and has its own set of critical pitfalls that are ordering-sensitive. Must not be attempted until the binary behavior is stable.
**Delivers:** `npx squad-station` works for any Node.js developer. Cross-compiled binaries for darwin-arm64, darwin-x64, linux-x64, linux-arm64.
**Addresses features:** npm wrapper distribution, cross-platform binary.
**Avoids pitfalls:** Platform packages must publish BEFORE base package (P12), executable permission loss — chmod in JS shim (P13), optionalDependencies as primary (not postinstall) to avoid disabled-scripts environments (P14), native macOS runners for Darwin targets — never cross-compile macOS from Linux (P15), use `windows` not `win32` in npm package names (P16).
**Research flag:** Distribution pipeline is well-documented (Orhun's guide + Sentry guide). Standard patterns — no research phase needed, but follow the exact ordering constraint for publication.

### Phase Ordering Rationale

- **Phase 1 before Phase 2:** You cannot test multi-agent lifecycle behavior without a working send/signal/status core. The skip guard must ship before any hook documentation.
- **Phase 2 before Phase 3:** The TUI displays agent state — that state must be accurate (lifecycle reconciliation) before building the viewer.
- **Phase 3 before Phase 4:** Never ship distribution for an unstable binary. TUI is the last major feature; distribution comes after all features are stable.
- **SQLite safety in Phase 1 is non-negotiable:** The WAL + busy_timeout + BEGIN IMMEDIATE combination must be present before any concurrent hook integration. Retrofitting it after discovering timing bugs is painful and risky.

### Research Flags

Phases needing deeper research during planning:
- **Phase 2:** Provider-specific hook integration — Claude Code Stop hook JSON schema, Gemini CLI AfterAgent event structure, and exact exit-code semantics need verification against current provider documentation before implementation. These may have changed since the research date.

Phases with standard patterns (skip research):
- **Phase 1:** All technologies are well-documented with HIGH confidence sources. Rust + clap + rusqlite + rusqlite_migration patterns are stable.
- **Phase 3:** ratatui 0.30 component architecture is documented on ratatui.rs with official examples.
- **Phase 4:** npm optionalDependencies pattern is well-documented and validated by multiple production projects.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All core crates verified on crates.io as of Jan-Mar 2026; versions confirmed; serde-saphyr is MEDIUM (newer, smaller community) but correct choice |
| Features | HIGH | Corroborated by multiple live projects (Overstory, Agent Deck, NTM, AgentManager); feature dependency chain is logically sound |
| Architecture | HIGH | Patterns verified via official ratatui docs, rusqlite docs, CLI structure guides; module boundaries are clean and testable |
| Pitfalls | HIGH | All critical pitfalls traced to primary sources (GitHub issues, official SQLite docs, tmux issue tracker); not speculative |

**Overall confidence:** HIGH

### Gaps to Address

- **serde-saphyr version:** Research recommends serde-saphyr but notes it's newer with a smaller community. Verify the exact crates.io version before locking in Cargo.toml, and evaluate serde_yml as an alternative if serde-saphyr has compatibility issues.
- **Gemini CLI hook schema:** The exact JSON payload for Gemini CLI's AfterAgent (or equivalent) hook event is not fully documented in the research. Must be verified against current Gemini CLI docs during Phase 2 planning.
- **`status` pattern matching per provider:** FEATURES.md flags this as a complexity hotspot — each provider (Claude Code, Gemini CLI, Codex, Aider) has different idle/busy terminal output patterns. The specific patterns need to be determined empirically during Phase 2.
- **rusqlite_migration WAL interaction:** The migration library's documented issue (#4) with PRAGMAs inside transactions is well-known, but the exact workaround sequence (run WAL pragma first, then call `migrations.to_latest()`) should be verified against the 2.4.0 API before implementation.

---

## Sources

### Primary (HIGH confidence)
- [crates.io — clap 4.5.54](https://crates.io/crates/clap) — version verified
- [crates.io — rusqlite 0.38.0](https://crates.io/crates/rusqlite/) — bundled SQLite 3.49.2 confirmed
- [docs.rs — rusqlite_migration 2.4.0](https://docs.rs/crate/rusqlite_migration/latest) — version and WAL issue confirmed
- [ratatui.rs — v0.30 highlights](https://ratatui.rs/highlights/v030/) — official release notes
- [ratatui.rs — Backends](https://ratatui.rs/concepts/backends/) — crossterm as default
- [ratatui.rs — Component Architecture](https://ratatui.rs/concepts/application-patterns/component-architecture/) — TUI patterns
- [sqlite.org — WAL documentation](https://sqlite.org/wal.html) — WAL mode, checkpoint starvation, busy_timeout
- [Claude Code hooks guide](https://code.claude.com/docs/en/hooks-guide) — hook events, stop_hook_active field
- [Orhun's npm packaging guide](https://blog.orhun.dev/packaging-rust-for-npm/) — distribution pattern
- [Sentry binary publishing guide](https://sentry.engineering/blog/publishing-binaries-on-npm) — optionalDependencies
- [Azure AI Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns) — routing, handoffs

### Secondary (MEDIUM confidence)
- [Overstory — multi-agent orchestration](https://github.com/jayminwest/overstory) — live project reference
- [Agent Deck — terminal session manager](https://github.com/asheshgoplani/agent-deck) — status detection patterns
- [NTM — Named Tmux Manager](https://github.com/Dicklesworthstone/ntm) — agent send, JSON output patterns
- [AgentManager](https://github.com/simonstaton/AgentManager) — message types, inter-agent patterns
- [rusqlite_migration issue #4](https://github.com/cljoly/rusqlite_migration/issues/4) — PRAGMA inside transaction limitation
- [tmux send-keys race — Claude Code issue #23513](https://github.com/anthropics/claude-code/issues/23513) — shell init race documented
- [tmux semicolon parsing issue #1849](https://github.com/tmux/tmux/issues/1849) — send-keys special chars
- [Claude Code infinite loop issue #10205](https://github.com/anthropics/claude-code/issues/10205) — orchestrator hook loop
- [Rust SIGPIPE issue #46016](https://github.com/rust-lang/rust/issues/46016) — SIGPIPE panic
- [Rust ORMs in 2026](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3) — ecosystem consensus

### Tertiary (MEDIUM-LOW confidence)
- [serde-yaml deprecation — users.rust-lang.org thread](https://users.rust-lang.org/t/serde-yaml-deprecation-alternatives/108868) — community consensus on serde-saphyr replacement
- [IttyBitty multi-agent](https://adamwulf.me/2026/01/itty-bitty-ai-agent-orchestrator/) — Manager/Worker patterns

---

*Research completed: 2026-03-06*
*Ready for roadmap: yes*
