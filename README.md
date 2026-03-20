# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. It is provider-agnostic: works with Claude Code, Gemini CLI, or any tool. Each project stores its state in a local SQLite database at `.squad/station.db` inside the project directory.

## Installation

### npx (recommended)

```bash
npx squad-station-2@latest install --tui
```

![Welcome TUI](docs/assets/welcome-tui.png)

Requires Node.js 14+. Downloads the native binary for your platform to `/usr/local/bin` (falls back to `~/.local/bin`) and scaffolds `.squad/` project files.

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

**Step 1 — Create `squad.yml`:**

Copy an example config and edit it, or use the interactive wizard:

```bash
# Option A — copy and edit an example
cp .squad/examples/orchestrator-claude.yml squad.yml
vi squad.yml

# Option B — interactive TUI wizard (generates squad.yml for you)
squad-station init --tui
```

**Step 2 — Launch the squad:**

```bash
squad-station init
```

Reads `squad.yml`, creates the SQLite database, launches tmux sessions, and installs hooks.

**Step 3 — Send a task:**

```bash
squad-station send my-app-claude-code-backend --body "Implement the /api/health endpoint"
```

**Step 4 — Signal completion** (from inside the agent's tmux session via hook):

```bash
squad-station signal $TMUX_PANE
```

**Step 5 — Monitor your fleet:**

```bash
squad-station monitor   # Interactive TUI — live agent pane viewer (recommended)
squad-station fleet     # Fleet overview — tasks, busy duration, alignment per agent
squad-station open      # Attach to tmux tiled view of all agent panes
squad-station ui        # TUI dashboard
squad-station status    # Text overview
squad-station list      # Message queue
```

> **Tip:** `squad-station monitor` is the recommended way to observe your agents in real time. It shows live pane output for each agent in a navigable TUI. Use `squad-station fleet` for a quick summary of pending tasks and agent alignment.

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
