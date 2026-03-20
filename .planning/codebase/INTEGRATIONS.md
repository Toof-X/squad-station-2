# External Integrations

**Analysis Date:** 2026-03-08

## AI Provider Integrations

Squad Station is **provider-agnostic**. It does not call AI provider APIs directly. Instead, it routes tasks to agents running in tmux sessions and relies on provider-specific hook scripts to signal completion.

**Claude Code:**
- Integration type: Hook script (`hooks/claude-code.sh`)
- Hook event: `Stop` (registered in `.claude/settings.json` or `~/.claude/settings.json`)
- Mechanism: Claude Code invokes `claude-code.sh` after each response; script detects tmux session name and calls `squad-station signal <agent-name>`
- Config: `.claude/settings.json` (project-level), `~/.claude/settings.json` (user-level)
- Provider label in config: `"claude-code"`
- Launch command example: `"claude"`

**Gemini CLI:**
- Integration type: Hook script (`hooks/gemini-cli.sh`)
- Hook event: `AfterAgent` (registered in `.gemini/settings.json`)
- Mechanism: Gemini CLI invokes `gemini-cli.sh` after each agent response; script detects tmux session name and calls `squad-station signal <agent-name>`
- Config: `.gemini/settings.json`
- Provider label in config: `"gemini"`
- Launch command example: `"gemini"`
- Note: Hook must exit 0; exit 2 would trigger Gemini CLI automatic retry

**Adding new providers:**
- Create a hook script in `hooks/` following the pattern in `hooks/claude-code.sh`
- Register it with the provider's hook system
- Use any `provider` label string in `squad.yml` — no built-in provider mappings in the Rust code

## Data Storage

**Databases:**
- Type: SQLite (embedded, no separate server)
- WAL mode enabled (journal_mode = WAL)
- Connection: File path resolved from `squad.yml` → `project.db_path` or default `~/.agentic-squad/<project-name>/station.db`
- Client: `sqlx` 0.8 with async SQLite driver
- Pool: Single-writer (`max_connections=1`), 5-second busy timeout (`SAFE-01`)
- Migrations: Auto-applied from `src/db/migrations/` via `sqlx::migrate!()`

**Migration files:**
- `src/db/migrations/0001_initial.sql` — Creates `agents` and `messages` tables with indexes
- `src/db/migrations/0002_agent_status.sql` — Adds `status` and `status_updated_at` columns to `agents`

**File Storage:**
- Local filesystem only — DB file at `~/.agentic-squad/<project-name>/station.db`
- No remote file storage

**Caching:**
- None — all reads go directly to SQLite

## Authentication & Identity

**Auth Provider:**
- None — Squad Station has no authentication layer
- It is a local CLI tool designed for single-user developer workstations
- Access control is filesystem permissions on `~/.agentic-squad/`

## System Integrations

**tmux (required runtime dependency):**
- Not an API — direct subprocess invocation via `std::process::Command`
- Location: `src/tmux.rs`
- Operations used:
  - `tmux new-session -d -s <name> <command>` — Launch agent sessions
  - `tmux send-keys -t <target> -l <text>` + `tmux send-keys -t <target> Enter` — Send tasks (literal flag `-l` prevents shell injection, `SAFE-02`)
  - `tmux has-session -t <name>` — Check session existence
  - `tmux list-sessions -F #{session_name}` — Enumerate live sessions
  - `tmux kill-window -t <name>` — Remove view windows
  - `tmux new-window`, `tmux split-window`, `tmux select-layout tiled` — Create multi-pane view
  - `tmux list-panes -t $TMUX_PANE -F '#S'` — Detect session name from within hook scripts

## Monitoring & Observability

**Error Tracking:**
- None — errors written to stderr via `eprintln!` and `anyhow` error chains
- Exit code 1 on any unhandled error

**Logs:**
- No structured logging framework
- Human-readable colored output via `owo-colors` for interactive use
- JSON output via `--json` flag for machine-readable / orchestrator consumption

## CI/CD & Deployment

**Distribution:**
- No automated CI/CD pipeline detected
- Release binary built with `cargo build --release`
- Binary placed manually on PATH

**Hosting:**
- Local developer machine only — no cloud deployment

## Environment Configuration

**Required environment variables:**
- None strictly required for normal operation

**Optional environment variables:**
- `SQUAD_STATION_BIN` — Override binary path in hook scripts (default: `squad-station` on PATH)
- `TMUX_PANE` — Set automatically by tmux; used by hook scripts to detect session context

**Secrets:**
- None — no API keys, tokens, or credentials managed by this tool
- AI provider authentication (if any) is handled externally by the provider CLI tools

## Webhooks & Callbacks

**Incoming:**
- None — Squad Station is a CLI, not a server

**Outgoing:**
- None — task delivery is through tmux `send-keys`, not HTTP webhooks

**Hook-based callbacks (not HTTP):**
- `hooks/claude-code.sh` — Called by Claude Code on `Stop` event; calls `squad-station signal`
- `hooks/gemini-cli.sh` — Called by Gemini CLI on `AfterAgent` event; calls `squad-station signal`

---

*Integration audit: 2026-03-08*
