# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. It is provider-agnostic: works with Claude Code, Gemini CLI, or any tool. Each project stores its state in a local SQLite database at `.squad/station.db` inside the project directory.

## Installation

```bash
npx squad-station-2@latest install --tui
```

The interactive TUI guides you through setup — press **y** to launch the init wizard, which generates `squad.yml` and starts your squad automatically.

## Quickstart

**Step 1 — Install and follow the TUI wizard** (see above)

**Step 2 — Open the monitor:**

```bash
squad-station open
```

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
squad-station browser   # Browser dashboard — React Flow graph visualization
squad-station open      # Attach to tmux tiled view of all agent panes
squad-station ui        # TUI dashboard
squad-station status    # Text overview
squad-station list      # Message queue
```

> **Tip:** `squad-station monitor` is the recommended way to observe your agents in real time. It shows live pane output for each agent in a navigable TUI. Use `squad-station browser` for a visual graph of your squad topology.

See [docs/PLAYBOOK.md](docs/PLAYBOOK.md) for the complete workflow guide.

## Commands

| Command | Description |
|---|---|
| `squad-station init` | Launch squad from `squad.yml` — creates DB, tmux sessions, hooks |
| `squad-station init --tui` | Interactive TUI wizard — generate `squad.yml`, then launch |
| `squad-station send <agent> --body "<task>"` | Send a task to an agent |
| `squad-station signal $TMUX_PANE` | Signal agent completed its task |
| `squad-station monitor` | Interactive TUI — live agent pane viewer |
| `squad-station fleet` | Fleet status overview — tasks, busy duration, alignment |
| `squad-station browser` | Browser dashboard — React Flow graph with live WebSocket updates |
| `squad-station clone <agent>` | Clone an agent with auto-incremented naming |
| `squad-station open` | Attach to tmux monitor session |
| `squad-station list` | List messages |
| `squad-station agents` | List agents with live status |
| `squad-station status` | Project and agent summary |
| `squad-station view` | Open tmux tiled view of all agents |
| `squad-station ui` | Interactive TUI dashboard |
| `squad-station close` | Kill all squad tmux sessions |
| `squad-station reset` | Kill sessions, delete DB, relaunch |

## Architecture

Squad Station is a stateless Rust CLI. There is no background daemon. Every command opens the SQLite database, reads or writes, and exits.

- `agents` table — registered agents with `tool` (e.g. `claude-code`, `gemini-cli`), role, model, description, and status
- `messages` table — tasks routed to agents with bidirectional `from_agent`/`to_agent` fields, priority (urgent > high > normal), and a full status lifecycle: `pending → processing → done` (or `failed`)
- tmux sessions — each agent runs in its own named session; `send-keys -l` prevents shell injection; multiline bodies use `load-buffer`/`paste-buffer`
- Inline hooks — `squad-station signal $TMUX_PANE` registered directly in provider stop/completion hooks; no external scripts required
- Browser visualization — `squad-station browser` starts an embedded axum server serving a React Flow SPA with WebSocket live updates

## Providers

| Tool | Provider key | Notes |
|------|-------------|-------|
| Claude Code | `claude-code` | Hook: Stop event |
| Gemini CLI | `gemini-cli` | Hook: AfterAgent event |
| Any IDE (DB-only) | `antigravity` | Skips tmux — orchestrator reads DB directly |

## Requirements

- macOS or Linux (Windows not supported — tmux unavailable)
- tmux

## License

MIT License

---

Based on [thientranhung/squad-station](https://github.com/thientranhung/squad-station).
