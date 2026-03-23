# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. Provider-agnostic: works with Claude Code, Gemini CLI, or any AI tool.

## Install

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

**Step 4 — Monitor your fleet:**

```bash
squad-station monitor   # Interactive TUI — live agent pane viewer (recommended)
squad-station fleet     # Fleet overview — tasks, busy duration, alignment per agent
squad-station browser   # Browser dashboard — React Flow graph visualization
squad-station open      # Attach to tmux tiled view of all agent panes
squad-station ui        # TUI dashboard
squad-station status    # Text overview
squad-station list      # Message queue
```

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

## Requirements

- macOS or Linux
- tmux

## License

MIT
