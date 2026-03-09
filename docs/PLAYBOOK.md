# Squad Station Playbook

A step-by-step guide to orchestrating AI agent squads with squad-station.

---

## Prerequisites

- **tmux** installed and available in PATH
- **squad-station** installed (`npm install -g squad-station` or via curl install script)
- At least one AI coding tool: Claude Code (`claude`) or Gemini CLI (`gemini`)

---

## 1. Define Your Squad

Create a `squad.yml` in your project root.

### Standard CLI Orchestrator

```yaml
project: my-app

orchestrator:
  tool: claude-code
  role: orchestrator
  model: claude-opus-4-5
  description: "Lead orchestrator. Delegates tasks, synthesizes results."

agents:
  - name: frontend
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Frontend specialist"
  - name: backend
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Backend specialist"
```

### IDE Orchestrator (Antigravity)

```yaml
project: my-app

orchestrator:
  tool: antigravity
  role: orchestrator
  description: >
    Orchestrator running inside Antigravity IDE.
    Uses Manager View to poll and monitor tmux worker agents.

agents:
  - name: implement
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Implements features and fixes bugs"
```

**Agent naming convention:** The `name` field acts as a role suffix. The full registered agent name is automatically prefixed as `<project>-<tool>-<role_suffix>`. For example: project `my-app`, tool `claude-code`, name `frontend` → registered as `my-app-claude-code-frontend`.

**Fields:**

| Field | Required | Description |
|-------|----------|-------------|
| `project` | Yes | Project identifier (plain string). Used in DB path and as prefix in agent names. |
| `orchestrator` | Yes | Exactly one orchestrator per squad |
| `agents` | Yes | Array of worker agents (can be empty) |
| `*.name` | Yes | Acts as role suffix; full agent name is auto-prefixed as `<project>-<tool>-<role_suffix>` (e.g., `my-app-claude-code-frontend`) |
| `*.tool` | Yes | Label: `claude-code`, `gemini`, `antigravity`, or any string |
| `*.role` | Yes | `orchestrator` or `worker` |
| `*.model` | No | Model identifier (e.g., `claude-sonnet-4-5`) — shown in context output |
| `*.description` | No | Human-readable description — shown in context output |

> **Note:** The DB path is controlled by the `SQUAD_STATION_DB` environment variable only — it is not set in `squad.yml`.

---

## 2. Launch the Squad

```bash
squad-station init
```

This will:
1. Create the SQLite database
2. Register all agents (names auto-prefixed as `<project>-<tool>-<role_suffix>`)
3. Launch each agent in its own tmux session

**Check the result:**

```bash
squad-station init --json
# {
#   "launched": 3,
#   "skipped": 0,
#   "failed": [],
#   "db_path": "/Users/you/.agentic-squad/my-app/station.db"
# }
```

Re-running `init` is safe — already-running agents are skipped.

**Antigravity note:** When orchestrator tool is `antigravity`, `init` registers the orchestrator in the DB only (no tmux session is created for it) and prints a message confirming DB-only registration. Worker agents still get tmux sessions normally.

---

## 3. Set Up Completion Hooks

Hooks let squad-station know when an agent finishes its work. Without hooks, you must signal manually.

### Automatic Setup

`squad-station init` automatically sets up hooks:
- If a `settings.json` already exists, init merges the hook entry and creates a `.bak` backup before modifying
- If no `settings.json` exists, init prints the hook configuration to stdout for manual setup

### Manual Setup — Claude Code

Add to `.claude/settings.json` (project-level) or `~/.claude/settings.json` (global):

```json
{
  "hooks": {
    "Stop": [
      {
        "type": "command",
        "command": "squad-station signal $TMUX_PANE"
      }
    ]
  }
}
```

### Manual Setup — Gemini CLI

Add to `.gemini/settings.json`:

```json
{
  "hooks": {
    "AfterAgent": [
      {
        "type": "command",
        "command": "squad-station signal $TMUX_PANE"
      }
    ]
  }
}
```

**Notes:**
- `signal` reads `$TMUX_PANE` to identify the agent automatically — no arguments needed beyond the env var
- The command always exits 0 and never blocks the AI tool, even on errors
- `hooks/claude-code.sh` and `hooks/gemini-cli.sh` are deprecated since v1.3 and kept for reference only. Use the inline command above.

---

## 4. Notification Hooks (Optional)

When Claude Code encounters a permission prompt, it fires a `Notification` event. Hook this to alert yourself.

### Claude Code — permission prompt notifications

```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "hooks/claude-code-notify.sh"
          }
        ]
      }
    ]
  }
}
```

### Gemini CLI — notifications

```json
{
  "hooks": {
    "Notification": [
      {
        "type": "command",
        "command": "hooks/gemini-cli-notify.sh"
      }
    ]
  }
}
```

Both notify scripts are included in the `hooks/` directory of the squad-station repo. Make them executable: `chmod +x hooks/claude-code-notify.sh hooks/gemini-cli-notify.sh`. Both scripts always exit 0.

---

## 5. Send Tasks to Agents

Task body is a required named flag (`--body`), not a positional argument.

```bash
# Basic task
squad-station send my-app-claude-code-frontend --body "Build the login page with email/password fields"

# With priority
squad-station send my-app-gemini-backend --body "Fix the auth endpoint" --priority urgent

# JSON output (for scripting)
squad-station send my-app-claude-code-frontend --body "Add form validation" --priority high --json
# {
#   "sent": true,
#   "message_id": "8c2e9e2f-...",
#   "agent": "my-app-claude-code-frontend",
#   "priority": "high"
# }
```

**Priority levels:** `normal` (default), `high`, `urgent`

What happens behind the scenes:
1. Task is stored in the database (status: pending)
2. Agent is marked as "busy"
3. Task text is injected into the agent's tmux session

---

## 6. Monitor Your Squad

### Quick status overview

```bash
squad-station status
# Project: my-app
# DB: /Users/you/.agentic-squad/my-app/station.db
# Agents: 3 -- 2 idle, 1 busy, 0 dead
#
#   my-app-claude-code-orchestrator: idle 5m   |  0 pending
#   my-app-claude-code-frontend: busy 2m       |  1 pending
#   my-app-gemini-backend: idle 10m            |  0 pending
```

### Agent list with live tmux reconciliation

```bash
squad-station agents
# NAME                              ROLE          STATUS            TOOL
# my-app-claude-code-orchestrator   orchestrator  idle 5m           claude-code
# my-app-claude-code-frontend       worker        busy 2m           claude-code
# my-app-gemini-backend             worker        idle 10m          gemini
```

The `agents` command checks tmux to detect crashed sessions:
- Session gone → agent marked **dead**
- Session reappears → agent revived to **idle**

### Message log

```bash
# All messages
squad-station list

# Filter by agent
squad-station list --agent my-app-claude-code-frontend

# Filter by status
squad-station list --status pending

# Limit results
squad-station list --limit 5

# JSON output
squad-station list --agent my-app-gemini-backend --status completed --json
```

### Interactive TUI dashboard

```bash
squad-station ui
```

Controls:
- `j`/`k` or arrow keys — navigate agents
- `Tab` — switch between agent and message panels
- `q` or `Esc` — quit

The dashboard auto-refreshes every 3 seconds.

### tmux tiled view

```bash
squad-station view
```

Opens a tiled tmux layout showing all live agent sessions side by side.

---

## 7. Check Pending Work

```bash
# What's the next task for an agent?
squad-station peek my-app-claude-code-frontend
# [pending] (priority=high) Add form validation
# id: a1b2c3d4-...

# JSON mode
squad-station peek my-app-claude-code-frontend --json
# {
#   "id": "a1b2c3d4-...",
#   "task": "Add form validation",
#   "priority": "high",
#   "status": "pending"
# }

# No pending work
squad-station peek my-app-gemini-backend
# No pending tasks for my-app-gemini-backend
```

Peek returns the highest-priority task first (urgent > high > normal), with oldest-first tie-breaking.

---

## 8. Signal Completion

If hooks are set up, this happens automatically. For manual signaling:

```bash
squad-station signal my-app-claude-code-frontend
# ✓ Signaled completion for my-app-claude-code-frontend (task_id=a1b2c3d4-...)
```

What happens:
1. Most recent pending message is marked "completed"
2. Orchestrator receives a notification in its tmux session:
   `my-app-claude-code-frontend completed a1b2c3d4-...`
3. Agent status resets to "idle"

**Signal format:** The notification injected into the orchestrator's tmux session is a plain string:

```
<agent> completed <msg-id>
```

Example: `my-app-claude-code-frontend completed 8c2e9e2f-1234-...`

Duplicate signals are safe — they silently succeed.

---

## 9. Antigravity IDE Orchestrator Mode

### When to use

Use `tool: antigravity` when you want to run the orchestrator inside an IDE (Antigravity, Cursor, VS Code agent, etc.) rather than as a CLI tmux session.

### What changes with Antigravity

- `init` registers the orchestrator in the DB only — no tmux session is created for it
- `signal` updates the message status in the DB but does NOT inject a notification into a tmux session (there is none)
- The IDE polls completion by calling `squad-station status` or `squad-station list --status completed`
- Worker agents still run as tmux sessions and receive tasks via the normal send path

### squad.yml (full example)

```yaml
project: my-app

orchestrator:
  tool: antigravity
  role: orchestrator
  description: >
    Orchestrator running inside Antigravity IDE.
    Uses Manager View to poll and monitor tmux worker agents.

agents:
  - name: implement
    tool: claude-code
    role: worker
    model: claude-sonnet-4-5
    description: "Implements features and fixes bugs"
```

### IDE workflow

1. Run `squad-station init` — registers orchestrator in DB, launches worker tmux sessions
2. Run `squad-station context` — generates `.agent/workflows/` files for the IDE orchestrator
3. IDE orchestrator reads `.agent/workflows/squad-delegate.md` and `.agent/workflows/squad-roster.md`
4. IDE orchestrator calls `squad-station send <agent> --body "..."` to dispatch tasks
5. IDE orchestrator polls `squad-station status` or `squad-station list --status completed` to detect task completion

### Context files generated by `squad-station context`

- `.agent/workflows/squad-delegate.md` — delegation instructions and exact CLI commands
- `.agent/workflows/squad-monitor.md` — polling/monitoring guidance with behavioral rules
- `.agent/workflows/squad-roster.md` — agent roster with names, models, descriptions

---

## 10. Register Agents at Runtime

Add agents without restarting the squad:

```bash
squad-station register reviewer \
  --role reviewer \
  --tool claude-code
```

This registers the agent in the database but does **not** launch a tmux session. Use this for agents managed externally.

If no `squad.yml` is available, you can point to the database directly:

```bash
SQUAD_STATION_DB=/path/to/station.db squad-station register my-agent --tool claude-code
```

---

## 11. Generate Orchestrator Context

```bash
squad-station context
```

For CLI orchestrators: outputs a Markdown document to stdout with the agent roster and usage examples. Feed this to your orchestrator so it knows which agents are available and how to dispatch tasks.

For IDE orchestrators (Antigravity): also writes three files to `.agent/workflows/`:
- `squad-delegate.md` — delegation instructions and exact CLI commands
- `squad-monitor.md` — monitoring/polling guidance
- `squad-roster.md` — agent roster listing names, models, and descriptions

```
# Squad Station -- Agent Roster

## Available Agents

## my-app-claude-code-frontend (claude-sonnet-4-5)

Frontend specialist

Role: worker | Status: idle

→ squad-station send my-app-claude-code-frontend --body "..."

---

## my-app-gemini-backend

Role: worker | Status: busy

→ squad-station send my-app-gemini-backend --body "..."

---

## Usage

Send a task to an agent:
```
squad-station send <agent> --body "<task description>"
```
```

---

## Workflow Summary

```
┌─────────────────────────────────────────────────────────────┐
│                        Orchestrator                          │
│                                                             │
│  1. Reads context (squad-station context)                   │
│  2. Sends tasks (squad-station send agent --body "task")    │
│  3. Receives signals via tmux notification                  │
│     <agent> completed <msg-id>                              │
│  4. Sends next task or coordinates results                  │
└──────────┬──────────────────────────┬───────────────────────┘
           │                          │
     ┌─────▼─────┐            ┌──────▼──────┐
     │  Worker A  │            │  Worker B   │
     │            │            │             │
     │ Receives   │            │ Receives    │
     │ task via   │            │ task via    │
     │ tmux       │            │ tmux        │
     │            │            │             │
     │ Completes  │            │ Completes   │
     │ work       │            │ work        │
     │            │            │             │
     │ Hook fires │            │ Hook fires  │
     │ signal cmd │            │ signal cmd  │
     └────────────┘            └─────────────┘
```

---

## Command Reference

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `init [config]` | Launch squad from config | `--json` |
| `send <agent> --body <task>` | Send task to agent | `--body`, `--priority`, `--json` |
| `signal <agent>` | Signal task completion | `--json` |
| `peek <agent>` | View next pending task | `--json` |
| `list` | List messages | `--agent`, `--status`, `--limit`, `--json` |
| `register <name>` | Register agent at runtime | `--role`, `--tool`, `--json` |
| `agents` | List agents with status | `--json` |
| `status` | Project overview | `--json` |
| `context` | Generate orchestrator context | — |
| `ui` | Interactive TUI dashboard | — |
| `view` | tmux tiled view | `--json` |

All commands support `--json` for machine-readable output and `--help` for usage details.

---

## Troubleshooting

**Agent shows "dead" status**
The tmux session crashed or was closed. Re-run `squad-station init` to relaunch, or manually start a new tmux session with the agent's name.

**"Agent not found" when sending**
The agent name doesn't match any registered agent. Check `squad-station agents` for the exact names. Remember: full agent names are prefixed as `<project>-<tool>-<role_suffix>`.

**"tmux session not running" when sending**
The agent is registered but its tmux session is down. Re-run `squad-station init` or launch the session manually.

**Hook not firing (shell script)**
Verify the hook path is absolute and the script is executable (`chmod +x hooks/claude-code-notify.sh`). Check that the agent is running inside a tmux session (hooks check `TMUX_PANE`).

**Database locked errors**
Squad-station uses single-writer SQLite. If you see lock errors, ensure only one write operation runs at a time. The 5-second busy timeout handles most concurrent cases.

**Hook not firing (inline command)**
Verify `$TMUX_PANE` is set in the agent session: run `echo $TMUX_PANE` inside the agent tmux session. Also verify `squad-station` is in PATH: `which squad-station`. The inline hook requires no script path — only the binary being accessible.

**Antigravity: orchestrator not receiving completion signals**
This is expected behavior. With `tool: antigravity`, the orchestrator has no tmux session, so `signal` does not inject a tmux notification. Use `squad-station status` or `squad-station list --status completed` to poll for task completion instead.
