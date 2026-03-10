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
  tool: claude-code
  role: orchestrator
agents:
  - name: backend
    tool: gemini
    role: worker
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

**Step 5 — List pending tasks:**

```bash
squad-station list
```

See [docs/PLAYBOOK.md](docs/PLAYBOOK.md) for the complete workflow guide.

## Architecture

Squad Station is a stateless Rust CLI. There is no background daemon. Every command opens the SQLite database, reads or writes, and exits.

- `agents` table — registered agents with `tool` (e.g. `claude-code`, `gemini`), role, command, status
- `messages` table — tasks routed to agents with bidirectional `from_agent`/`to_agent` fields, priority (urgent > high > normal), and a full status lifecycle: `pending → processing → done` (or `failed`)
- tmux sessions — each agent runs in its own named session; `send-keys -l` prevents shell injection
- Hooks in `hooks/` detect task completion per tool and call `squad-station signal`

## Requirements

Requires: tmux, macOS or Linux (Windows not supported — tmux unavailable).

## License

MIT License
