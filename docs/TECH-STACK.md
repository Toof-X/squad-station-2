# Squad Station — Tech Stack Decision

> Source of truth. Based on Obsidian `03. Tech Stack Decision - Squad Station.md`.
> Decision: **Rust** over Go.
> Updated with: `04. Upgrade Design — Antigravity & Hooks Optimization`.

---

## 1. Language Decision

| Criteria | Go | Rust (✓ Chosen) |
|----------|-----|----------------|
| Binary size | ~10MB | ~3-5MB |
| Performance | Good | Better |
| Error handling | `if err != nil` | `Result<T, E>` + `?` |
| Dependencies | Few | Fewer (no runtime) |
| Cross-compile | Easy (native) | Easy (cargo, cross crate) |
| SQLite | CGO required or pure Go | `rusqlite` bundled or `sqlx` |
| User preference | — | ✓ Preferred |

**Reason for choosing Rust:** Smaller binary, better performance, user preference.

## 2. Tech Stack — Confirmed

```
╔══════════════════════════════════════════════════════════════╗
║  ✓ CONFIRMED STACK                                          ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  clap 4          — CLI parsing (derive macros)               ║
║  sqlx 0.8        — Async SQLite (compile-time SQL)           ║
║  ratatui 0.26+   — TUI dashboard                            ║
║  crossterm       — Terminal backend for ratatui              ║
║  serde + serde-saphyr — YAML config parsing                  ║
║  tokio           — Async runtime for sqlx                    ║
║  anyhow          — Error handling (commands layer)           ║
║  thiserror       — Typed errors (db/tmux layer)              ║
║  uuid            — Message IDs                               ║
║  chrono          — Timestamps                                ║
║  owo-colors      — Terminal colors                           ║
║  tracing         — Structured logging                        ║
║  libc            — SIGPIPE handler                           ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

╔══════════════════════════════════════════════════════════════╗
║  ✗ NOT USED — researched and rejected                        ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  rusqlite         — Replaced by sqlx (async, compile-time)   ║
║  Diesel           — ORM overhead for 2-3 tables              ║
║  tui-rs           — Archived, dead project                   ║
║  tmux_interface    — Pre-1.0, unstable                       ║
║  serde_yaml       — Deprecated March 2024                    ║
║  daemon / server  — Constraint: stateless CLI                ║
║  env_logger       — tracing strictly better                  ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

## 3. Module Architecture

```
squad-station/
├── src/
│   ├── main.rs              ← Entry point + SIGPIPE handler
│   ├── cli.rs               ← clap Parser (16 subcommands)
│   ├── config.rs            ← squad.yml parsing, validation, DB path resolution
│   ├── tmux.rs              ← Tmux adapter (send_keys, inject_body, session mgmt)
│   ├── lib.rs               ← Re-exports for test access
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── init.rs          ← squad.yml → DB + tmux sessions + context + monitor
│   │   ├── send.rs          ← Task → agent (DB + tmux inject)
│   │   ├── signal.rs        ← Hook completion (guard chain + orch skip)
│   │   ├── notify.rs        ← Mid-task HITL notification (no status change)
│   │   ├── peek.rs          ← Pending task check (priority-ordered)
│   │   ├── list.rs          ← Message list + filters
│   │   ├── register.rs      ← Dynamic agent registration
│   │   ├── agents.rs        ← Agent list (with tmux reconciliation)
│   │   ├── context.rs       ← Generate orchestrator context (provider-specific)
│   │   ├── status.rs        ← Squad overview
│   │   ├── ui.rs            ← TUI dashboard (ratatui)
│   │   ├── view.rs          ← Split tmux layout
│   │   ├── close.rs         ← Kill all squad tmux sessions
│   │   ├── reset.rs         ← Kill + delete DB + optionally relaunch
│   │   ├── clean.rs         ← Delete DB file only
│   │   └── helpers.rs       ← Shared: colorize, format_status, reconcile
│   └── db/
│       ├── mod.rs           ← connect() + migrations
│       ├── agents.rs        ← Agent CRUD
│       ├── messages.rs      ← Message CRUD
│       └── migrations/      ← 4 migration files (0001–0004)
├── hooks/
│   ├── claude-code.sh       ← Completion hook for Claude Code agents
│   ├── gemini-cli.sh        ← Completion hook for Gemini CLI agents
│   ├── claude-code-notify.sh ← Notification hook (permission prompts)
│   ├── gemini-cli-notify.sh  ← Notification hook (Gemini)
│   └── test-notify-hooks.sh  ← Hook testing utility
├── scripts/
│   ├── _common.sh           ← Shared helpers (provider/model validation)
│   ├── setup-sessions.sh    ← Create tmux sessions from squad.yml
│   ├── teardown-sessions.sh ← Tear down sessions
│   ├── tmux-send.sh         ← Send text to tmux session
│   └── validate-squad.sh    ← Validate squad.yml
├── Cargo.toml
├── squad.yml                ← User config
└── tests/
```

**Module dependency flow:**
```
cli → commands → db + tmux + config
                  ↓         ↓
                sqlx    std::process::Command
```

## 4. Roadmap — 3 Phases

```
Phase 1                    Phase 2                    Phase 3
CORE FOUNDATION            LIFECYCLE & HOOKS          VIEWS & TUI
━━━━━━━━━━━━━━━━━━          ━━━━━━━━━━━━━━━━━━          ━━━━━━━━━━━━━

┌────────────────┐        ┌────────────────┐        ┌────────────────┐
│ • DB schema    │        │ • Agent status │        │ • status cmd   │
│ • init         │───────►│   idle/busy/dead│───────►│ • agents cmd   │
│ • register     │        │ • Liveness     │        │ • TUI dashboard│
│ • send + signal│        │   reconciliation│       │ • Split tmux   │
│ • list + peek  │        │ • Hook scripts │        │   view         │
│ • Priority     │        │   Claude + Gem │        │                │
│ • Idempotency  │        │ • Orch skip    │        │                │
│ • WAL + safety │        │ • Context gen  │        │                │
└────────────────┘        └────────────────┘        └────────────────┘
                                                            │
                                                            ▼
Phase 4
ANTIGRAVITY & HOOKS OPT
━━━━━━━━━━━━━━━━━━━━━━━

┌──────────────────────┐
│ • Centralized hooks    │
│   (no shell scripts)  │
│ • Antigravity provider │
│ • Conditional notify   │
│   skip in signal.rs   │
│ • .agent/workflows/    │
│   context generation  │
│ • Safe tmux injection  │
│   (Rust adapter)      │
│ • Setup-hooks merge    │
└──────────────────────┘
```

## 5. Safety Checklist

| # | Safety Item | Pitfall | Severity |
|---|------------|---------|----------|
| 1 | WAL mode + busy_timeout=5000 | Concurrent writes → SQLITE_BUSY | CRITICAL |
| 2 | max_connections(1) write pool | Write-contention deadlock | CRITICAL |
| 3 | Orchestrator skip guard in `signal` | Infinite hook loop | CRITICAL |
| 4 | `tmux send-keys -l` (literal mode) | Special char injection | HIGH |
| 5 | Shell readiness poll before inject | Keys lost due to shell not loaded | HIGH |
| 6 | SIGPIPE handler at main() | Panic on pipe | MEDIUM |
| 7 | Migration on every open | Schema drift between versions | MEDIUM |
| 8 | Idempotent send (INSERT OR IGNORE) | Double-dispatch from duplicate hooks | HIGH |
| 9 | Safe multiline tmux injection | Shell escaping breaks with long prompts; use `load-buffer`/`paste-buffer` | HIGH |
| 10 | Antigravity skip-notify in `signal.rs` | IDE Orchestrator does not receive `tmux send-keys`; must check provider at runtime | HIGH |

## 6. Confirmed Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust over Go | Smaller binary, performance, user preference | ✓ Confirmed |
| sqlx over rusqlite | Async-native, compile-time SQL checks | ✓ Confirmed |
| Stateless CLI, no daemon | Simple, debuggable, event-driven | ✓ Confirmed |
| SQLite embedded per project (`.squad/station.db` in project dir) | Isolation, no external DB needed, data lives with project | ✓ Confirmed |
| Agent name = tmux session name | Simple, hook auto-detects | ✓ Confirmed |
| Provider-agnostic design | No lock-in | ✓ Confirmed |
| Hook-driven completion | Agent passive, clean separation | ✓ Confirmed |
| Dedicated repo for binary | `squad-station` dedicated repo | ✓ Confirmed |
| Centralized hooks via CLI | `squad-station signal $TMUX_PANE` replaces shell scripts | ✓ Confirmed |
| Antigravity IDE support | IDE-based orchestrator with polling, no notify | ✓ Confirmed |
| Context via `.agent/workflows/` | For IDE orchestrators (Antigravity); CLI uses .md | ✓ Confirmed |
| Safe tmux injection in Rust | `tmux::adapter` with `load-buffer`/`paste-buffer` | ✓ Confirmed |

---
*Source: Obsidian/1-Projects/Agentic-Coding-Squad/03. Tech Stack Decision - Squad Station.md*
*Supersedes Go references in 02. Solution Design (sections 13, 15)*
*Updated with: 04. Upgrade Design — Antigravity & Hooks Optimization*
*Updated with: 05. Local DB — `.squad/station.db` in project directory*
