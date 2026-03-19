# Squad Station

## What This Is

Squad Station là một stateless CLI binary (Rust + embedded SQLite) hoạt động như trạm trung chuyển messages giữa AI Orchestrator và N agents chạy trong tmux sessions. Provider-agnostic — hỗ trợ bất kỳ AI coding tool nào (Claude Code, Gemini CLI, Codex, Aider...). Người dùng chỉ tương tác với Orchestrator, Station lo việc routing messages, tracking trạng thái agent, cung cấp fleet monitoring qua TUI dashboard, và hỗ trợ orchestrator scale team với intelligence metrics, dynamic cloning, và role templates.

## Core Value

Routing messages đáng tin cậy giữa Orchestrator và agents — gửi task đúng agent, nhận signal khi hoàn thành, notify Orchestrator — tất cả qua stateless CLI commands không cần daemon.

## Requirements

### Validated

- ✓ Orchestrator gửi task đến agent qua `squad-station send` — v1.0
- ✓ Hook-driven signal khi agent hoàn thành (`squad-station signal`) — v1.0
- ✓ Agent registry từ `squad.yml` config (`squad-station init`) — v1.0
- ✓ Dynamic agent registration at runtime (`squad-station register`) — v1.0
- ✓ Multi-project isolation (DB riêng per project) — v1.0
- ✓ Orchestrator skip trong hook (chống infinite loop) — v1.0
- ✓ Agent lifecycle detection (idle/busy/dead) — v1.0
- ✓ Auto-generate orchestrator context file — v1.0
- ✓ TUI dashboard (`squad-station ui`) — v1.0
- ✓ Split tmux view (`squad-station view`) — v1.0
- ✓ Idempotent send/signal (duplicate hook fires safe) — v1.0
- ✓ Message priority levels (normal, high, urgent) — v1.0
- ✓ Peek for pending tasks (`squad-station peek`) — v1.0
- ✓ SQLite WAL mode with busy_timeout (concurrent-safe) — v1.0
- ✓ tmux send-keys literal mode (injection-safe) — v1.0
- ✓ Shell readiness check before prompt injection — v1.0
- ✓ SIGPIPE handler at binary startup — v1.0
- ✓ 4-layer guard on signal command — v1.0
- ✓ Text status overview (`squad-station status`) — v1.0
- ✓ Agent list with status (`squad-station agents`) — v1.0
- ✓ Provider hook scripts (Claude Code + Gemini CLI) — v1.0
- ✓ Message list with filters (`squad-station list`) — v1.0
- ✓ squad.yml config: `project` string, `model`/`description`, removed `command`, `provider`→`tool` — v1.1
- ✓ Messages DB schema: `from_agent`/`to_agent`, `type`, `processing` status, `completed_at` — v1.1
- ✓ Agents DB schema: `model`, `description`, `current_task` FK, `tool` field — v1.1
- ✓ Notification hooks: `claude-code-notify.sh` + `gemini-cli-notify.sh` — v1.1
- ✓ CLI `send --body` flag (positional arg removed) — v1.1
- ✓ Agent naming auto-prefix `<project>-<tool>-<role>` on init — v1.1
- ✓ `context` output includes `model` + `description` per agent — v1.1
- ✓ Signal format standardized to `"<agent> completed <msg-id>"` — v1.1
- ✓ ARCHITECTURE.md updated to reflect actual sqlx + flat module structure — v1.1
- ✓ PLAYBOOK.md rewritten with correct CLI syntax and config format — v1.1
- ✓ GitHub Actions CI/CD cross-compiles Rust binary for 4 targets (darwin-arm64, darwin-x86_64, linux-arm64, linux-x86_64) and creates GitHub Release — v1.2
- ✓ npm package detects platform and downloads correct binary on postinstall — v1.2
- ✓ curl | sh install script as npm-free alternative to install binary — v1.2
- ✓ README.md documents all installation methods with usage quickstart — v1.2
- ✓ `signal` accepts `$TMUX_PANE` env var — zero-arg inline hook, hook shell scripts deprecated — v1.3
- ✓ `antigravity` provider: DB-only orchestrator skips tmux session creation and send-keys notification — v1.3
- ✓ `context` generates `.agent/workflows/squad-delegate.md`, `squad-monitor.md`, `squad-roster.md` — v1.3
- ✓ `init` safely merges hooks into existing `settings.json` with `.bak` backup; fallback instructions when absent — v1.3
- ✓ `inject_body` via `load-buffer`/`paste-buffer` for safe multiline task body delivery — v1.3
- ✓ PLAYBOOK.md rewritten as authoritative v1.3 guide (inline hooks, Antigravity mode, Notification hooks) — v1.3
- ✓ `context` generates single unified `squad-orchestrator.md` replacing 3 fragmented workflow files — v1.4
- ✓ `init` Get Started message references `squad-orchestrator.md` — v1.4
- ✓ DB path moved to `<cwd>/.squad/station.db` (local, no home-dir resolution) — v1.4
- ✓ `dirs` crate removed from dependencies — v1.4
- ✓ `.gitignore` excludes `.squad/`, docs updated for new DB path — v1.4
- ✓ `SQUAD_STATION_DB` env var override preserved through DB path change — v1.4
- ✓ `squad-station init` launches multi-page ratatui TUI wizard when no squad.yml exists — v1.5
- ✓ Wizard collects project name, SDD workflow, orchestrator config, and per-worker config (provider, model, description) — v1.5
- ✓ `init` generates squad.yml from wizard answers (full model ID validation) before registering agents — v1.5
- ✓ Re-init prompt (overwrite/add-agents/abort) when squad.yml already exists; non-interactive TTY guard for CI safety — v1.5
- ✓ Worker-only wizard entry point (`run_worker_only`) for "add agents" re-init path — v1.5
- ✓ Wizard validates inputs with inline error feedback; radio selectors for Provider and Model — v1.5

- ✓ `squad-station` (no args) shows red ASCII title, version, next-step hint, subcommand list — v1.6
- ✓ After `init` completes, ASCII diagram shows all agents (boxes + arrows + status) — v1.6
- ✓ claude-code wizard model options: sonnet, opus, haiku (no version suffixes) — v1.6

- ✓ Bare `squad-station` invocation opens interactive ratatui TUI with BigText pixel-font title, version, auto-exit countdown, and TTY guard — v1.7
- ✓ WelcomePage state machine: Title and Guide pages navigable via Tab/Right; Tab/Left returns to title — v1.7
- ✓ WelcomeAction routing: Enter launches init wizard (no squad.yml) or dashboard (squad.yml exists); Q/Esc/timeout exit silently — v1.7
- ✓ Non-TTY fallback: piped/CI invocation prints static text instead of entering raw mode — v1.7
- ✓ npm postinstall auto-launches `squad-station` via `spawnSync` when `process.stdout.isTTY` — v1.7
- ✓ curl installer auto-launches `squad-station` via `exec` when `[ -t 1 ]`; silent in non-interactive environments — v1.7

- ✓ `init` requires explicit `--tui` flag to enter wizard; bare `init` reads existing squad.yml directly — v1.8-pre

- ✓ Fleet Status metrics in orchestrator context (pending count, busy duration, task-role alignment hints) — v1.8
- ✓ `build_orchestrator_md()` accepts `&[AgentMetrics]` as pure function — metrics fetched externally — v1.8
- ✓ `squad-station clone <agent>` with auto-incremented naming, DB-first + tmux rollback, orchestrator rejection guard — v1.8
- ✓ Clone auto-regenerates squad-orchestrator.md so orchestrator learns about new agents immediately — v1.8
- ✓ 11 role templates in init wizard (8 worker + 3 orchestrator) with split-pane TUI selector — v1.8
- ✓ Template routing hints embedded in Routing Matrix section of squad-orchestrator.md — v1.8
- ✓ Cloned agents appear in TUI dashboard on next poll cycle (existing connect-per-refresh pattern) — v1.8

### Active

(None — planning next milestone)

### Out of Scope

- Task management / workflow logic — đó là việc của Orchestrator AI
- Orchestration decisions / reasoning — đó là việc của AI model
- File sync / code sharing giữa agents — agents work on same codebase via git
- Web UI / browser dashboard — TUI sufficient, complexity not justified
- Git conflict resolution giữa agents — orchestrator should sequence work to avoid
- Agent-to-agent direct messaging — all communication routes through orchestrator
- Offline mode — stateless CLI always needs tmux + DB

## Context

Shipped v1.8 Smart Agent Management. Orchestrator now has intelligence metrics, dynamic cloning, and role templates.
Tech stack: Rust, SQLite (sqlx 0.8), clap 4, ratatui 0.30, crossterm 0.29, tui-big-text 0.8, serde-saphyr, serde_json, owo-colors 3, uuid (temp file naming).
Distribution: npm package (v1.5.7, binaryVersion 1.8) + curl | sh installer, both download pre-built binaries from GitHub Releases. Both install paths auto-launch the welcome TUI in interactive terminals.
CI/CD: GitHub Actions matrix workflow produces 4 musl/darwin binaries on v* tag push.
Providers supported: claude-code, gemini-cli, antigravity (DB-only IDE orchestrator).
Hook registration: inline `squad-station signal $TMUX_PANE` command (scripts in hooks/ deprecated).
Init flow: TUI wizard (ratatui, requires `--tui` flag) with role template selector generates squad.yml from scratch; re-init prompt handles overwrite/add-agents/abort. Post-init prints ASCII agent fleet diagram.
Welcome TUI: bare `squad-station` invocation opens ratatui AlternateScreen with BigText title, 5s countdown, Tab-navigable Quick Guide page; Enter routes to init wizard or dashboard.
Context generation: `.agent/workflows/squad-orchestrator.md` — unified playbook with Fleet Status metrics and Routing Matrix sections.
Clone: `squad-station clone <agent>` creates duplicate agent with auto-incremented name, DB-first + tmux rollback, auto-regenerates orchestrator context.
Safe injection: load-buffer/paste-buffer pattern for multiline task bodies (no shell-injection artifacts).
Database: `.squad/station.db` in project directory (no home-dir dependency, no `dirs` crate). 5 migrations (latest: 0005_routing_hints).
Test suite: 303 tests (all green).

## Constraints

- **Language**: Rust — single binary, zero runtime dependency, cross-compile cho darwin/linux
- **Database**: SQLite embedded — 1 DB file per project tại `<cwd>/.squad/station.db`
- **Architecture**: Stateless CLI — mỗi command chạy xong exit, không daemon, không background process
- **Communication**: tmux send-keys để inject prompt vào agent, tmux capture-pane để đọc output
- **Distribution**: npm package wrapper — download pre-built binary phù hợp platform
- **Repo**: Dedicated repo riêng cho Rust binary (repo hiện tại: squad-station)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust thay vì Go | Binary nhỏ hơn, performance tốt hơn, user preference | ✓ Good — 2,994 LOC, fast compile, single binary |
| Stateless CLI, không daemon | Đơn giản, dễ debug, event-driven qua hook chain | ✓ Good — no process management complexity |
| SQLite embedded per project | Isolation giữa projects, không cần external DB | ✓ Good — WAL mode handles concurrent writes |
| Agent name = tmux session name | Đơn giản hóa lookup, hook tự detect qua TMUX_PANE | ✓ Good — zero-config agent identity |
| npm wrapper distribution | Target audience là developers đã có Node.js | ✓ Good — npm + curl | sh both shipped v1.2 |
| Provider-agnostic design | Không lock-in vào Claude Code hay Gemini CLI | ✓ Good — hooks work for both providers |
| Hook-driven completion | Agent passive, không cần modify agent behavior | ✓ Good — clean separation of concerns |
| sqlx over rusqlite | Already in Cargo.toml, async-native, compile-time SQL checks | ✓ Good — migration system worked well |
| max_connections(1) write pool | Prevents async write-contention deadlock in SQLite | ✓ Good — no busy errors in testing |
| INSERT OR IGNORE for agents | Idempotent registration, safe for duplicate hook fires | ✓ Good — MSG-03 satisfied cleanly |
| connect-per-refresh in TUI | Prevents WAL checkpoint starvation during long TUI sessions | ✓ Good — WAL doesn't grow unbounded |
| Reconciliation loop duplication | Each command file independent, ~10 lines not worth abstraction | ✓ Good — simple, no coupling |
| `--body` flag for `send` | Named flags more discoverable and shell-safe than positional args | ✓ Good — cleaner UX, pattern-matchable |
| Auto-prefix agent naming | Enforces `<project>-<tool>-<role>` convention without manual coordination | ✓ Good — avoids name collisions across projects |
| `provider`→`tool` rename | Matches solution design terminology; aligns with squad.yml and DB | ✓ Good — consistent naming across all layers |
| Notification hooks separate from Stop hooks | Notification fires on permission prompts, not task completion — distinct behavior | ✓ Good — both hook types needed |
| Signal format `"<agent> completed <msg-id>"` | Pattern-matchable string, no JSON parsing needed in orchestrator | ✓ Good — simple, grep-friendly |
| SQUAD_STATION_DB env var in resolve_db_path | Single injection point benefits all commands without per-command changes | ✓ Good — cleaner test isolation |
| musl over gnu for Linux targets | Produces fully static binaries, no glibc dependency — required for install script portability | ✓ Good — runs on any Linux distro |
| cross tool only for linux-arm64 | aarch64-musl requires cross-compilation; native cargo sufficient for darwin and linux-x86_64 | ✓ Good — minimal Docker overhead |
| softprops/action-gh-release@v2 | Idempotent — creates release if absent, appends assets if present; safe for 4 parallel matrix uploads | ✓ Good — race-condition-free releases |
| curl \| sh as npm alternative | Targets users without Node.js; POSIX sh for max portability | ✓ Good — covers non-Node environments |
| Binary naming `squad-station-{os}-{arch}` | Consistent convention consumed by npm postinstall and install script | ✓ Good — both distribution paths aligned |
| Pane ID detection via `starts_with('%')` | tmux pane IDs always use `%` prefix, session names cannot — zero-ambiguity detection | ✓ Good — clean signal arg dispatch |
| `signal` exits 0 silently on pane resolution failure | Hook context — providers must never see errors from hooks (infinite loop risk) | ✓ Good — safe hook behavior |
| `is_db_only()` checks `tool == "antigravity"` | Open string — unknown providers remain tmux providers by default, no config migration needed | ✓ Good — forward-compatible |
| Inline `orch.tool == "antigravity"` in signal.rs | Agent DB struct should not couple to config domain knowledge; check at call site | ✓ Good — clean domain boundaries |
| `context` command is read-only | Removed tmux reconciliation from `context`; DB state only → less side effects | ✓ Good — predictable, idempotent |
| JSON mode guard in `init.rs` | Hook instructions suppressed from stdout when `--json` active — preserves machine-parseable output | ✓ Good — composable CLI |
| `inject_body` uses uuid-named temp file | Prevents concurrent `send` calls from clobbering each other's buffer; cleanup on all code paths | ✓ Good — safe concurrent usage |
| PLAYBOOK.md inline hook as canonical | Shell scripts in `hooks/` deprecated — single install path reduces user confusion | ✓ Good — clearer onboarding |
| Single unified `squad-orchestrator.md` | One file replaces 3 fragmented workflow files — reduces context load for orchestrator | ✓ Good — simpler context loading |
| `build_orchestrator_md` as pub function | Integration tests can import and verify playbook content directly | ✓ Good — testable playbook generation |
| DB at `<cwd>/.squad/station.db` | Data locality — no home-dir resolution, no project-name collision risk | ✓ Good — simpler path, no `dirs` crate |
| No old DB migration | Dev builds only, no production data to preserve — clean break | ✓ Good — zero complexity |
| Wizard as guard clause in init.rs | Minimal diff; existing init path for present squad.yml unchanged | ✓ Good — clean separation, zero coupling |
| `generate_squad_yml` as string builder | Deterministic field ordering; avoids serde_yaml dependency | ✓ Good — simple, readable output |
| `WizardResult` with separate `orchestrator` + `agents` | Orchestrator configured on its own page; role implicit from page context | ✓ Good — cleaner UX, no role selector field |
| `SddWorkflow` enum in wizard | Captures workflow preference early; embedded in squad.yml for orchestrator context | ✓ Good — zero-friction SDD setup |
| `is_terminal()` guard in re-init prompt | crossterm raw mode fails in non-TTY (CI, tests); guard preserves backward compat | ✓ Good — all 201 tests pass unchanged |
| `run_worker_only()` on wizard | Skips Project + OrchestratorConfig pages for add-agents path — no re-entry of existing config | ✓ Good — correct UX for append flow |
| `KeyAction::Cancel` variant | Explicit Esc cancel path for worker-only wizard; doesn't repurpose Continue | ✓ Good — clean intent, no call-site changes |
| `Option<Commands>` in clap Cli struct | Bare invocation no longer errors; None arm routes to welcome screen | ✓ Good — clean pattern, zero coupling to existing subcommands |
| `welcome_content()` as testable private fn | Returns plain string; `print_welcome()` applies color — separates test concerns from terminal state | ✓ Good — 4 tests pass without capturing stdout |
| Short model aliases for ClaudeCode | sonnet/opus/haiku instead of full version strings — decoupled from version churn | ✓ Good — cleaner UX, stored as-is in squad.yml |
| `render_diagram()` returns String | Enables unit testing without stdout capture; `print_diagram()` calls it | ✓ Good — 10 diagram tests with full assertion coverage |
| Workers wrap on 80-char boundary | Prevents diagram overflow on standard terminals | ✓ Good — readable layout for typical agent counts |
| AlternateScreen for welcome TUI | Consistent with existing ui.rs pattern; preserves scrollback buffer | ✓ Good — clean terminal restore on exit |
| `routing_action()` as pure function | WelcomeAction routing extracted to welcome.rs — unit-testable without TTY | ✓ Good — 5 routing tests pass without spawning a terminal |
| WelcomePage enum state machine | Title/Guide pages as enum variant; mutable deadline for per-page countdown reset | ✓ Good — clean dispatch in event loop, no nested match |
| TTY check only for auto-launch | `process.stdout.isTTY` / `[ -t 1 ]` sufficient; no CI env var guards | ✓ Good — correct silent degradation in non-interactive environments |
| `exec` in curl / `spawnSync` in npm | exec replaces shell process cleanly; spawnSync blocks until TUI exits | ✓ Good — correct handoff semantics per install path |
| `build_orchestrator_md()` pure fn with `&[AgentMetrics]` | Metrics fetched externally, pure rendering — testable without DB, INTEL-05 | ✓ Good — integration tests verify output directly |
| Fleet Status after Completion Notification | Orchestrator reads status after understanding completion flow | ✓ Good — correct reading order in context file |
| DB-before-pure-fn pattern in context::run() | All DB queries execute before build_orchestrator_md call | ✓ Good — clean separation, INTEL-05 purity maintained |
| Clone name: strip_clone_suffix only strips -N where N>=2 | Original agent name preserved; -2, -3, -4... for clones | ✓ Good — unambiguous naming |
| Clone DB-first with rollback | Write DB record first, rollback if tmux fails — no orphans | ✓ Good — CLONE-03 satisfied, safe error handling |
| Clones are DB-only (not in squad.yml) | Same as `register` behavior — ephemeral runtime entities | ✓ Good — consistent with existing patterns |
| pub over pub(crate) for clone helpers | Integration tests in tests/ are separate crates, need pub access | ✓ Good — pragmatic visibility choice |
| 11 templates with default_provider=claude-code | Per-provider model mapping in struct, not resolved at runtime | ✓ Good — simple, extensible |
| routing_hints as JSON string in DB | `Option<String>` serialized to JSON array; parsed by build_orchestrator_md | ✓ Good — no schema complexity, serde_json parsing |
| Template selector split-pane layout (45%/55%) | Left: role list, Right: description preview — scannable selection UX | ✓ Good — clear visual hierarchy |
| Routing Matrix after Session Routing section | Orchestrator reads routing knowledge after understanding sessions | ✓ Good — correct context ordering |

---
*Last updated: 2026-03-19 after v1.8 Smart Agent Management milestone*
