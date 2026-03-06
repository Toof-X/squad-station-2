# Squad Station

## What This Is

Squad Station là một stateless CLI binary (Rust + embedded SQLite) hoạt động như trạm trung chuyển messages giữa AI Orchestrator và N agents chạy trong tmux sessions. Provider-agnostic — hỗ trợ bất kỳ AI coding tool nào (Claude Code, Gemini CLI, Codex, Aider...). Người dùng chỉ tương tác với Orchestrator, Station lo việc routing messages và tracking trạng thái.

## Core Value

Routing messages đáng tin cậy giữa Orchestrator và agents — gửi task đúng agent, nhận signal khi hoàn thành, notify Orchestrator — tất cả qua stateless CLI commands không cần daemon.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Orchestrator gửi task đến agent qua `squad-station send`
- [ ] Hook-driven signal khi agent hoàn thành (`squad-station signal`)
- [ ] Agent registry từ `squad.yml` config (`squad-station init`)
- [ ] Dynamic agent registration at runtime (`squad-station register`)
- [ ] Multi-project isolation (DB riêng per project)
- [ ] Orchestrator skip trong hook (chống infinite loop)
- [ ] Agent lifecycle detection (idle/busy/dead)
- [ ] Auto-generate orchestrator context file
- [ ] TUI dashboard (`squad-station ui`)
- [ ] Split tmux view (`squad-station view`)
- [ ] npm wrapper distribution

### Out of Scope

- Task management / workflow logic — đó là việc của Orchestrator AI
- Orchestration decisions / reasoning — đó là việc của AI model
- File sync / code sharing giữa agents
- Web UI / browser dashboard
- Spec-driven methodology integration (v2)
- Git conflict resolution giữa agents

## Context

- Dự án giải quyết vấn đề điều phối nhiều AI coding agents làm việc song song trên cùng codebase
- Kiến trúc: User → Orchestrator (any AI tool) → Station (CLI) → Agents (tmux sessions)
- Agent hoàn toàn passive — không biết Station tồn tại, hook layer bên ngoài tự detect và signal
- Orchestrator tự capture-pane để đọc raw output từ agent session
- Hook system phải xử lý đúng cho cả Claude Code (Stop event) lẫn Gemini CLI (AfterAgent event)
- Naming convention: `<project>-<provider>-<role>` = tmux session name = agent identity
- Tài liệu requirements chi tiết có tại Obsidian vault: `1-Projects/Agentic-Coding-Squad/`

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
| Rust thay vì Go | Binary nhỏ hơn, performance tốt hơn, user preference | — Pending |
| Stateless CLI, không daemon | Đơn giản, dễ debug, event-driven qua hook chain | — Pending |
| SQLite embedded per project | Isolation giữa projects, không cần external DB | — Pending |
| Agent name = tmux session name | Đơn giản hóa lookup, hook tự detect qua `tmux display-message -p '#S'` | — Pending |
| npm wrapper distribution | Target audience là developers đã có Node.js, dễ cài đặt | — Pending |
| Provider-agnostic design | Không lock-in vào Claude Code hay Gemini CLI | — Pending |
| Hook-driven completion | Agent passive, không cần modify agent behavior | — Pending |

---
*Last updated: 2026-03-06 after initialization*
