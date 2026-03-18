# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. It is provider-agnostic: works with Claude Code, Gemini CLI, or any tool. Each project stores its state in a local SQLite database at `.squad/station.db` inside the project directory.

## Installation

### npx (recommended)

```bash
# Install binary and scaffold project files
npx squad-station-2@latest install

# Same, but launch the interactive welcome TUI after install
npx squad-station-2@latest install --tui
```

Requires Node.js 14+. Downloads the native binary for your platform to `/usr/local/bin` (falls back to `~/.local/bin`) and scaffolds `.squad/` project files.

### Build from source

```bash
git clone https://github.com/Toof-X/squad-station-2.git
cd squad-station-2
cargo build --release
# Binary: target/release/squad-station
```

Requires Rust toolchain. See [Cargo docs](https://doc.rust-lang.org/cargo/getting-started/installation.html).

## First Run

Running `squad-station` with no arguments opens an interactive welcome screen:

```
SQUAD-STATION
v0.x.x

Multi-agent orchestration for AI coding

  Commands:
    init        Initialize squad from config
    send        Send a task to an agent
    ...

  Ok to proceed? (y)
● ○  Tab: Guide  Q: Quit  25s
```

- **y / Enter** — launch the init wizard (or open the dashboard if already configured)
- **Tab** — open the Quick Guide
- **Q / Esc** — exit
- Timeout — exits silently with no action

The title scales with terminal width: full pixel font on wide terminals, compact on narrow.

## Quickstart

**Step 1 — Run the init wizard:**

```bash
squad-station init
```

The interactive TUI wizard collects your project name, SDD workflow, orchestrator config, and worker agent configs. It generates `squad.yml` and registers all agents automatically.

**Step 2 — Send a task:**

```bash
squad-station send my-app-claude-code-backend --body "Implement the /api/health endpoint"
```

**Step 3 — Signal completion** (from inside the agent's tmux session via hook):

```bash
squad-station signal $TMUX_PANE
```

**Step 4 — Monitor your fleet:**

```bash
squad-station ui      # TUI dashboard
squad-station status  # text overview
squad-station list    # message queue
```

See [docs/PLAYBOOK.md](docs/PLAYBOOK.md) for the complete workflow guide.

## Architecture

Squad Station is a stateless Rust CLI. There is no background daemon. Every command opens the SQLite database, reads or writes, and exits.

- `agents` table — registered agents with `tool` (e.g. `claude-code`, `gemini-cli`), role, model, description, and status
- `messages` table — tasks routed to agents with bidirectional `from_agent`/`to_agent` fields, priority (urgent > high > normal), and a full status lifecycle: `pending → processing → done` (or `failed`)
- tmux sessions — each agent runs in its own named session; `send-keys -l` prevents shell injection; multiline bodies use `load-buffer`/`paste-buffer`
- Inline hooks — `squad-station signal $TMUX_PANE` registered directly in provider stop/completion hooks; no external scripts required

## Providers

| Tool | Provider key | Notes |
|------|-------------|-------|
| Claude Code | `claude-code` | Hook: Stop event |
| Gemini CLI | `gemini-cli` | Hook: AfterAgent event |
| Any IDE (DB-only) | `antigravity` | Skips tmux — orchestrator reads DB directly |

## Requirements

Requires: tmux, macOS or Linux (Windows not supported — tmux unavailable).

## License

MIT License

---

Based on [thientranhung/squad-station](https://github.com/thientranhung/squad-station).
