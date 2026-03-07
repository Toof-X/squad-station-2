# Squad Station

## What This Is

Squad Station là một stateless CLI binary (Rust + embedded SQLite) hoạt động như trạm trung chuyển messages giữa AI Orchestrator và N agents chạy trong tmux sessions. Provider-agnostic — hỗ trợ bất kỳ AI coding tool nào (Claude Code, Gemini CLI, Codex, Aider...). Người dùng chỉ tương tác với Orchestrator, Station lo việc routing messages, tracking trạng thái agent, và cung cấp fleet monitoring qua TUI dashboard và tmux views.

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

### Active

- [ ] npm wrapper distribution
- [ ] Cross-compile CI via GitHub Actions (darwin arm64/amd64, linux amd64/arm64)
- [ ] Support cargo install from source

### Out of Scope

- Task management / workflow logic — đó là việc của Orchestrator AI
- Orchestration decisions / reasoning — đó là việc của AI model
- File sync / code sharing giữa agents — agents work on same codebase via git
- Web UI / browser dashboard — TUI sufficient, complexity not justified
- Git conflict resolution giữa agents — orchestrator should sequence work to avoid
- Agent-to-agent direct messaging — all communication routes through orchestrator
- Offline mode — stateless CLI always needs tmux + DB

## Context

Shipped v1.0 MVP with 2,994 LOC Rust, 58 tests, 0 failures.
Tech stack: Rust, SQLite (sqlx 0.8), clap 4, ratatui 0.26, serde-saphyr, owo-colors 3.
Architecture: Stateless CLI → SQLite WAL → tmux sessions. No daemon, no background process.
Hook system: provider-agnostic shell scripts detect completion via TMUX_PANE and delegate to binary.
Agent liveness: reconciliation loop checks tmux session_exists per agent, updates dead/revive status.
TUI: connect-per-refresh strategy drops read-only pool after each fetch to prevent WAL starvation.

## Constraints

- **Language**: Rust — single binary, zero runtime dependency, cross-compile cho darwin/linux
- **Database**: SQLite embedded — 1 DB file per project tại `~/.agentic-squad/<project>/station.db`
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
| npm wrapper distribution | Target audience là developers đã có Node.js | — Pending (v2) |
| Provider-agnostic design | Không lock-in vào Claude Code hay Gemini CLI | ✓ Good — hooks work for both providers |
| Hook-driven completion | Agent passive, không cần modify agent behavior | ✓ Good — clean separation of concerns |
| sqlx over rusqlite | Already in Cargo.toml, async-native, compile-time SQL checks | ✓ Good — migration system worked well |
| max_connections(1) write pool | Prevents async write-contention deadlock in SQLite | ✓ Good — no busy errors in testing |
| INSERT OR IGNORE for agents | Idempotent registration, safe for duplicate hook fires | ✓ Good — MSG-03 satisfied cleanly |
| connect-per-refresh in TUI | Prevents WAL checkpoint starvation during long TUI sessions | ✓ Good — WAL doesn't grow unbounded |
| Reconciliation loop duplication | Each command file independent, ~10 lines not worth abstraction | ✓ Good — simple, no coupling |

---
*Last updated: 2026-03-06 after v1.0 milestone*
