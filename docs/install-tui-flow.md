# Flow `npx squad-station-2 install --tui`

## Phase 1: JavaScript (`run.js`)
```
1. installBinary()          → Download binary từ GitHub releases → ~/.local/bin/squad-station
2. skip scaffoldProject()   → Bỏ qua khi --tui (không tạo .squad/ ở cwd)
3. spawnSync("squad-station --tui")  → Chạy binary
```

## Phase 2: Rust binary (`squad-station --tui`)
```
main.rs → welcome TUI → user chọn "Launch Init"
        → commands::init::run(config_path="squad.yml", json=false, tui=true)
```

## Phase 3: Init flow — tuần tự (`init.rs`)

Toàn bộ setup hoàn tất TRƯỚC khi launch tmux sessions.
Khi Claude Code start → mọi thứ đã sẵn sàng (hooks, context, DB, SDD).

```
┌─ Wizard ──────────────────────────────────────────────────────────┐
│  wizard::run()             → User chọn project name, agents...   │
│  create_dir_all(install_dir)  → Tạo thư mục project              │
│  git init -q               → Tạo .git/ (để Claude Code nhận root)│
│  set_current_dir(install_dir) → cd vào project                   │
│  write squad.yml           → Ghi config                          │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 1: Parse config ────────────────────────────────────────────┐
│  load_config(squad.yml)                                          │
│  project_root = config_path.parent() = install_dir               │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 2: Create .squad/ + DB + Install SDD ────────────────────────┐
│  create .squad/            → Tạo thư mục .squad                  │
│  create .squad/sdd/        → SDD playbook (embedded content)      │
│  install_sdd_if_needed()   → Auto-install SDD locally:           │
│    • BMad:  npx bmad-method install --directory . --modules bmm  │
│             --tools claude-code --yes                              │
│    • GSD:   npx get-shit-done-cc@latest --claude --local          │
│    • Superpower: skip (manual /plugin install)                    │
│    → Kiểm tra detect_dirs trước (skip nếu đã cài)               │
│  create .squad/log/        → Cho signal + watchdog logs           │
│  db::connect(.squad/station.db) → SQLite + migrations            │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 3: Register ALL agents ────────────────────────────────────┐
│  insert_agent(orchestrator) → vào DB                             │
│  insert_agent(worker 1)     → vào DB                             │
│  insert_agent(worker 2)     → vào DB                             │
│  Clean stale agents         → xóa agents không còn trong config  │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 4: Install hooks ──────────────────────────────────────────┐
│  → Ghi vào .claude/settings.local.json (trusted, không cần      │
│    approve — chạy ngay lập tức)                                  │
│                                                                   │
│  Hooks cài đặt:                                                  │
│  • Stop         → squad-station signal (task completion)         │
│  • Notification → squad-station notify (permission prompt)       │
│  • PostToolUse  → squad-station notify (AskUserQuestion)         │
│  • SessionStart → squad-station context --inject (auto-inject)   │
│                                                                   │
│  Auto-inject prompt: [Y/n]                                       │
│  → Nếu Yes: cài SessionStart hook                               │
│  → Orchestrator tự động nhận context khi start/resume/compact    │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 5: Generate orchestrator context ──────────────────────────┐
│  context::run()            → Query DB cho full agent list        │
│  → Tạo .claude/commands/squad-orchestrator.md                    │
│  → Claude Code sẽ thấy /squad-orchestrator ngay khi start        │
│                                                                   │
│  Nội dung context:                                               │
│  • Role: orchestrator (coordinate, không code)                   │
│  • Agent roster: tên, model, role, description                   │
│  • SDD playbook reference                                        │
│  • Fleet status metrics                                          │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 6: Start watchdog ─────────────────────────────────────────┐
│  watch::run(30s, daemon)   → Background process                  │
│  → Auto-reconcile agent statuses                                 │
│  → Relaunch dead sessions                                        │
│  → PID saved to .squad/watch.pid                                 │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 7: Launch tmux sessions (CUỐI CÙNG) ───────────────────────┐
│  ══════════════════════════════════════════════════                │
│  MỌI THỨ ĐÃ SẴN SÀNG — Claude Code sẽ thấy:                    │
│  • .squad/station.db       (DB có agents)                        │
│  • .claude/settings.local.json (hooks active)                    │
│  • .claude/commands/squad-orchestrator.md (slash command)         │
│  • .git/                   (project root detection)              │
│  • _bmad/ hoặc .claude/commands/gsd/ (SDD đã cài)              │
│  ══════════════════════════════════════════════════                │
│                                                                   │
│  Launch orchestrator:                                            │
│    tmux new-session "export SQUAD_AGENT_NAME='<name>';           │
│                      claude --dangerously-skip-permissions"      │
│    → SessionStart hook fires → context auto-inject               │
│                                                                   │
│  Launch workers:                                                 │
│    tmux new-session (mỗi worker)                                 │
│    → SessionStart hook fires → skip (không phải orchestrator)    │
│                                                                   │
│  Create monitor session:                                         │
│    tmux multi-pane view                                          │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Step 8: Output + Diagram ───────────────────────────────────────┐
│  "Initialized squad 'xxx' with N agent(s)"                       │
│  "Get Started:" instructions                                     │
│  Reconcile agent statuses                                        │
│  Print agent topology diagram                                    │
│  Write .squad-project-dir marker (for cd prompt)                 │
└───────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Phase 4: Back to JS (run.js) ───────────────────────────────────┐
│  Read .squad-project-dir marker                                  │
│  Prompt "Move to project directory?"                             │
│  Print cd command                                                │
└───────────────────────────────────────────────────────────────────┘
```

## Thứ tự tạo files ở project dir

```
1.  install_dir/                          ← create_dir_all
2.  .git/                                 ← git init -q
3.  squad.yml                             ← write config
4.  .squad/                               ← create directory
5.  .squad/sdd/<name>-playbook.md         ← SDD playbook (embedded)
6.  _bmad/ hoặc .claude/commands/gsd/    ← SDD local install (auto)
7.  .squad/log/                           ← signal + watchdog logs
8.  .squad/station.db                     ← SQLite DB (agents registered)
9.  .claude/settings.local.json           ← hooks (trusted, auto-execute)
10. .claude/commands/squad-orchestrator.md ← full context (from DB)
11. .squad/watch.pid                      ← watchdog daemon
    ══════════════════════════════════════
    MỌI THỨ SẴN SÀNG → LAUNCH SESSIONS
    ══════════════════════════════════════
12. tmux sessions                         ← orchestrator + workers
13. tmux monitor session                  ← multi-pane view
```

## SDD Auto-Install

Khi init, Squad Station tự động cài SDD được chọn nếu chưa có:

| SDD | Detect dirs | Install command | Notes |
|-----|-------------|-----------------|-------|
| BMad | `_bmad/` | `npx bmad-method install --directory . --modules bmm --tools <ide> --yes` | Non-interactive |
| GSD | `.claude/commands/gsd/` | `npx get-shit-done-cc@latest --<provider> --local` | Local install |
| Superpower | — | Manual `/plugin install` | Không tự động được |

Flow:
```
create_sdd_playbook()          → Ghi playbook.md vào .squad/sdd/
install_sdd_if_needed()        → Kiểm tra detect_dirs
  → Đã cài? → skip
  → Chưa cài? → chạy install command (non-interactive)
  → Không có installer? → in hướng dẫn manual
```

Cũng chạy khi re-init (overwrite) và non-TUI init (đọc từ squad.yml).

## Hook behavior khi Claude Code start

```
SessionStart hook fires
  → squad-station context --inject
    → detect_tmux_session() → $SQUAD_AGENT_NAME
    → if orchestrator session → output context to stdout → Claude Code nhận
    → if worker session → skip (workers không nhận orchestrator context)
```

- Hooks ở `.claude/settings.local.json` → trusted, chạy ngay, không cần approve
- Hooks ở `.claude/settings.json` → project-level, cần user approve (KHÔNG dùng)
