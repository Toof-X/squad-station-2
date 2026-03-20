# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. It is provider-agnostic: works with Claude Code, Gemini CLI, or any tool. Each project stores its state in a local SQLite database at `.squad/station.db` inside the project directory.

## Installation

### npm (recommended)

```bash
npm install -g squad-station
```

Requires Node.js 14+. Postinstall downloads the native binary for your platform.

### curl

```bash
curl -fsSL https://raw.githubusercontent.com/thientranhung/squad-station/master/install.sh | sh
```

Installs to `/usr/local/bin` (falls back to `~/.local/bin`). Supports macOS and Linux.

### Build from source

```bash
git clone https://github.com/thientranhung/squad-station.git
cd squad-station
cargo build --release
# Binary: target/release/squad-station
```

Requires Rust toolchain. See [Cargo docs](https://doc.rust-lang.org/cargo/getting-started/installation.html).

## Quickstart

**Step 1 — Create `squad.yml`:**

```yaml
project: my-app
orchestrator:
  provider: claude-code
  role: orchestrator
  model: sonnet
agents:
  - name: backend
    provider: gemini-cli
    role: worker
    model: gemini-2.5-pro
```

**Step 2 — Initialize:**

```bash
squad-station init
```

Registers agents from squad.yml and opens their tmux sessions.

**Step 3 — Send a task:**

```bash
squad-station send my-app-gemini-backend --body "Implement the /api/health endpoint"
```

**Step 4 — Signal completion** (run from inside the agent's tmux session or hook):

```bash
squad-station signal my-app-gemini-backend
```

**Step 5 — Check status:**

```bash
squad-station status           # Agent overview: statuses, pending tasks
squad-station list             # List all messages
squad-station reconcile        # Sync agent statuses with live tmux sessions
```

**Step 6 — Self-healing watchdog** (auto-started by init):

```bash
squad-station watch            # Foreground: reconcile + stall detection + nudges
squad-station watch --interval 30  # Custom interval (seconds)
```

See [docs/PLAYBOOK.md](docs/PLAYBOOK.md) for the complete workflow guide.

## Architecture

Squad Station is a stateless Rust CLI. There is no background daemon. Every command opens the SQLite database, reads or writes, and exits.

- `agents` table — registered agents with `tool` (e.g. `claude-code`, `gemini`), role, status, `current_task` FK
- `messages` table — tasks routed to agents with bidirectional `from_agent`/`to_agent` fields, priority (urgent > high > normal), and status lifecycle: `processing → completed`
- tmux sessions — each agent runs in its own named session; `send-keys -l` prevents shell injection; `$SQUAD_AGENT_NAME` env var set at launch
- Provider hooks auto-installed by `init` — detect task completion and call `squad-station signal`
- Watchdog daemon — auto-started by `init`, reconciles sessions, detects stalls, nudges idle orchestrators
- Signal logging — structured logs at `.squad/log/signal.log` for debugging signal flow

## Requirements

Requires: tmux, macOS or Linux (Windows not supported — tmux unavailable).

## License

MIT License
