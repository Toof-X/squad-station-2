# Squad Station

Message routing and orchestration for AI agent squads — stateless CLI, no daemon.

Squad Station routes messages between an AI orchestrator and N agents running in tmux sessions. Provider-agnostic: works with Claude Code, Gemini CLI, or any AI tool.

## Install

```bash
# Install binary and scaffold project files
npx squad-station-2@latest install

# Same, but launch the interactive welcome TUI after install
npx squad-station-2@latest install --tui
```

This downloads the `squad-station` binary to your system and scaffolds project files:

```
.squad/
├── sdd/                          # SDD playbooks
│   ├── gsd-playbook.md
│   ├── bmad-playbook.md
│   └── superpowers-playbook.md
└── examples/                     # Example configs
    ├── orchestrator-claude.yml
    └── orchestrator-gemini.yml
```

## Quickstart

```bash
# 1. Copy an example config
cp .squad/examples/orchestrator-claude.yml squad.yml

# 2. Edit — set project name, providers, models
vi squad.yml

# 3. Launch the squad
squad-station init
```

## Example `squad.yml`

```yaml
project: my-app

sdd:
  - name: get-shit-done
    playbook: ".squad/sdd/gsd-playbook.md"

orchestrator:
  provider: claude-code
  role: orchestrator
  model: haiku
  description: Team leader, coordinates tasks for agents

agents:
  - name: implement
    provider: claude-code
    role: worker
    model: sonnet
    description: Senior coder, coding, fixing bugs

  - name: brainstorm
    provider: claude-code
    role: worker
    model: opus
    description: Technical Lead, planner, analysis, code reviews
```

## Commands

| Command | Description |
|---|---|
| `squad-station init` | Launch squad from `squad.yml` — creates DB, tmux sessions, hooks |
| `squad-station send <agent> --body "<task>"` | Send a task to an agent |
| `squad-station signal <agent>` | Signal agent completed its task |
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
- Node.js 14+ (for `npx install` only)

## Build from source

```bash
git clone https://github.com/Toof-X/squad-station-2.git
cd squad-station-2 && cargo build --release
```

## License

MIT
