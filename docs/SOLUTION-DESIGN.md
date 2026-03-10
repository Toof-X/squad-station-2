# Squad Station — Solution Design

> Source of truth. Based on Obsidian `02. Solution Design - Squad Station.md`.
> Confirmed decisions: **Rust** (not Go), **sqlx** (not rusqlite).
> Updated with: `04. Upgrade Design — Antigravity & Hooks Optimization`.
> Updated with: `05. Local DB — .squad/station.db in project directory`.
> Go-specific sections replaced by `TECH-STACK.md`.

---

## 1. Core Concept

User only communicates with **Orchestrator**. Orchestrator reasons, makes decisions, sends tasks to **Station**. Station routes to the correct agent via tmux. Agent completes work, **hook automatically** reports to Station. Orchestrator uses **tmux capture-pane** to read actual output.

```
                    ┌───────────────┐
                    │     USER      │
                    └───────┬───────┘
                            │ conversation
                            ▼
                    ┌───────────────┐
                    │ ORCHESTRATOR  │  ← any AI tool
                    │ (tmux session)│
                    └───────┬───────┘
                            │
              ┌─────────────▼─────────────┐
              │       SQUAD STATION        │
              │  (Rust binary + SQLite)    │
              │                            │
              │  • Receive task from Orch  │
              │  • Route to agent (tmux)   │
              │  • Receive signal from hook│
              │  • Notify Orch             │
              │  • Track state             │
              └──────┬──────┬──────┬───────┘
                     │      │      │
                     ▼      ▼      ▼
              ┌──────┐ ┌──────┐ ┌──────┐
              │Agent │ │Agent │ │Agent │
              │  A   │ │  B   │ │  C   │
              │(tmux)│ │(tmux)│ │(tmux)│
              └──────┘ └──────┘ └──────┘
```

**Key points:**
- Agent is **completely passive** — does not know about Station, does not send results
- **Hook** is an external layer on the agent, auto-detects Stop/AfterAgent events → reports to Station
- Orchestrator **captures output** from agent session (`tmux capture-pane -t <agent> -p`)
- Station is a **stateless CLI** — exits after completion, no background process
- **IDE Orchestrator mode** (e.g. Antigravity): Station only tracks state in DB; the IDE polls tmux workers directly and does not need `tmux send-keys` notification

### Two-directional communication

```
  OUTBOUND (delegate task):
    Orchestrator ──► Station ──► tmux send-keys ──► Agent
                     (save DB)   (inject prompt)

  INBOUND (report completion):
    Agent done ──► Hook auto-fires ──► Station ──► (conditional) notify Orchestrator
                   (agent is unaware)  (save DB)   │
                                                    ├─ CLI Orch: tmux send-keys
                                                    └─ IDE Orch: skip (DB update only)
                                                         │
                                          Orchestrator ──► tmux capture-pane
                                          (reads raw output from agent session)
```

## 2. Config File — `squad.yml`

```yaml
project: myapp

orchestrator:
  provider: claude
  model: opus
  description: >
    Main orchestrator. Receives requests from user, reasons,
    delegates tasks to agents, reads results, synthesizes.

# Alternative: IDE-based Orchestrator
# orchestrator:
#   provider: antigravity
#   description: >
#     Orchestrator running inside Antigravity IDE.
#     Uses Manager View to poll and monitor tmux worker agents.

agents:
  - name: implement
    provider: claude
    model: sonnet
    description: >
      Developer agent. Writes code, fixes bugs, runs tests.
  - name: brainstorm
    provider: gemini
    model: gemini-2.5-pro
    description: >
      Architect & reviewer. Designs architecture, reviews code.
  - name: docs
    provider: claude
    model: haiku
    description: >
      Technical writer. Writes documentation.
  - name: test
    provider: claude
    model: sonnet
    description: >
      QA agent. Writes test cases, runs test suites.
```

### Required fields

| Field | Type | Description |
|-------|------|-------------|
| `project` | string | Project name, used for agent name prefix |
| `orchestrator` | object | Exactly 1 orchestrator per squad |
| `orchestrator.provider` | string | AI tool label (`claude`, `gemini`, `antigravity`, etc.) |
| `orchestrator.model` | string | AI model name |
| `orchestrator.description` | string | Role description, used for context generation |
| `agents` | array | List of worker agents |
| `agents[].name` | string | Role name (will be prefixed with `<project>-<provider>-`) |
| `agents[].provider` | string | AI tool label |
| `agents[].model` | string | AI model name |
| `agents[].description` | string | Capability description |

### Initialization from config

```
  $ squad-station init

  Read squad.yml
      │
      ├── Create .squad/station.db (in project directory)
      │
      ├── Register agents in DB (role = orchestrator | worker):
      │   myapp-claude-orchestrator    role=orchestrator  ← hook will SKIP
      │   myapp-claude-implement       role=worker
      │   myapp-gemini-brainstorm      role=worker
      │   myapp-claude-docs            role=worker
      │   myapp-claude-test            role=worker
      │
      ├── Create tmux sessions with interactive shells
      │   └── If orchestrator.provider = antigravity:
      │       → SKIP tmux session for orchestrator (IDE manages itself)
      │       → Only create tmux sessions for worker agents
      │
      ├── ⚠️  User must manually launch AI tools in sessions:
      │   tmux send-keys -t <session-name> "<tool-command>" Enter
      │   (e.g., "claude" for Claude Code, "gemini" for Gemini CLI)
      │
      └── AI tools stay running, receive tasks via squad-station send
      │
      └── Generate orchestrator context
          ├── CLI Orchestrator: write context .md file
          └── IDE Orchestrator: generate .agent/workflows/ files
```

### Orchestrator context — auto-generated

Station creates a context file so orchestrator knows its available agents:

**For CLI Orchestrators** — generates a Markdown context file:

```
  # Squad Agents

  You are the orchestrator for project myapp.
  You have the following agents to delegate tasks to:

  ## myapp-claude-implement (Claude Sonnet)
  Developer agent. Writes code, fixes bugs, runs tests.
  → squad-station send myapp-claude-implement --body "..."

  ## myapp-gemini-brainstorm (Gemini 2.5 Pro)
  Architect & reviewer. Designs architecture, reviews code.
  → squad-station send myapp-gemini-brainstorm --body "..."

  ## How to delegate
  1. Reasoning → select appropriate agent
  2. squad-station send <agent> --body "<task>"
  3. Wait for notification when agent completes
  4. tmux capture-pane -t <agent> -p to read result
  5. Reasoning → continue or report to user
```

**For IDE Orchestrators (Antigravity)** — generates `.agent/workflows/` files:

```
  .agent/workflows/
  ├── squad-delegate.md       ← Workflow: how to delegate tasks to agents
  ├── squad-monitor.md        ← Workflow: how to poll/monitor worker sessions
  └── squad-roster.md         ← Agent list with names, models, descriptions
```

Workflow files include **behavioral rules** that reinforce orchestrator discipline:
- Remind the IDE agent to delegate rather than do work itself
- Survive context compression by placing rules at the top of workflow files
- Provide exact `squad-station send` and `tmux capture-pane` commands

## 3. Distinguishing Squad Agent vs Independent Agent

```
  AGENT IN SQUAD                     INDEPENDENT AGENT

  Tmux session:                      Tmux session:
  "myapp-claude-implement"           "my-personal-claude"
  (registered in DB)                 (NOT in DB)

  Agent stops → Hook fires           Agent stops → Hook fires
  → squad-station signal             → squad-station signal
    "myapp-claude-implement"            "my-personal-claude"
  → FOUND in DB ✓                    → NOT FOUND → SKIP ✓
  → Update completed + notify orch   → Exit 0, silent
```

**4-layer guard in hook:**
1. Not in tmux → exit (user uses AI tool from regular terminal)
2. Agent not registered in Station → exit (instance outside squad)
3. Agent has role = `orchestrator` → exit (prevent loop)
4. No task processing → exit (agent in squad but chatting freely)

## 4. Real-world Scenarios

### 4.1 Happy path — delegate task, receive result

```
  Orchestrator:
  > squad-station send myapp-claude-implement \
    --body "Implement JWT auth module..."

  ✓ Message msg-a1b2c3 created
  ✓ Injected into myapp-claude-implement

  ... (wait) ...

  "myapp-claude-implement completed msg-a1b2c3"

  > tmux capture-pane -t myapp-claude-implement -p
  > 12 tests passed, 3 files created ✓
```

### 4.2 Multi-agent — implement then review

```
  Orchestrator ──► send implement "Implement auth"
                   ... wait ...
               ◄── "implement done" → capture output
               ──► send brainstorm "Review auth code"
                   ... wait ...
               ◄── "brainstorm done" → capture output
               → Synthesize for user
```

### 4.3 Parallel — send to N agents simultaneously

```
  Orchestrator ──► send implement  (write code)
               ──► send test       (write tests)
               ──► send docs       (write docs)
               ... wait for all ...
               ◄── "docs done"
               ◄── "test done"
               ◄── "implement done"
               → Synthesize
```

### 4.4 Agent dies mid-work

```
  Orchestrator ──► send implement
                   ... wait too long ...
               ──► squad-station status
               ◄── implement: DEAD (tmux session does not exist)
               → Reasoning: Restart or reassign
```

### 4.5 Multiple projects simultaneously

```
  Project myapp:     DB: /path/to/myapp/.squad/station.db
  Project api-svc:   DB: /path/to/api-svc/.squad/station.db
  → Fully isolated, separate DBs in each project directory, different agent name prefixes
```

## 5. Workflow Diagrams

### 5.1 Delegate task

```
  Orchestrator                   Station                       Agent
      │                             │                             │
      │  ① squad-station send       │                             │
      │    <agent-name>              │                             │
      │    --body "task prompt"      │                             │
      │  ──────────────────────►     │                             │
      │                             │  INSERT message             │
      │                             │  (status=processing)        │
      │                             │  UPDATE agent → busy        │
      │  ◄── message_id            │  + current_task             │
      │                             │                             │
      │                             │  tmux send-keys ───────────►│
      │                             │  (inject task prompt)        │
      │                             │                             │
      │                             │                             │  ② Works
```

### 5.2 Completion

```
  Orchestrator                   Station                       Agent
      │                             │                             │
      │                             │                             │  ③ Agent stops
      │                             │                             │  ④ HOOK fires
      │                             │  ◄─────────────────────────│  ⑤ squad-station signal
      │                             │                             │
      │                             │  SELECT FROM messages       │
      │                             │  WHERE to_agent=<agent>     │
      │                             │  AND status='processing'    │
      │                             │                             │
      │                             │  UPDATE msg → completed     │
      │                             │  UPDATE agent → idle        │
      │                             │                             │
      │                             │  Check: orchestrator.provider│
      │                             │  ├─ CLI: tmux send-keys     │
      │  ① tmux send-keys ◄─────  │  │  (notify orchestrator)    │
      │  "<agent> completed         │  │                           │
      │   <message_id>"             │  │                           │
      │                             │  └─ IDE (Antigravity):       │
      │                             │     skip notify             │
      │                             │     (IDE polls tmux)        │
      │                             │                             │
      │  ② tmux capture-pane      │                             │
      │    (read raw output)       │                             │
      │  ③ Reasoning/summarize    │                             │
```

## 6. Hook System

### 6.1 Centralized Hook Architecture

All hook logic is centralized in the `squad-station signal` CLI command. Provider hook configs declare a **single command** — no external shell scripts needed.

```json
// .claude/settings.json or .gemini/settings.json
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

When the hook fires, `squad-station signal` handles **all** logic:
1. Detect tmux session name from `$TMUX_PANE`
2. Look up agent in SQLite DB
3. Guard: not in DB → exit 0 (not a squad agent)
4. Guard: role = orchestrator → exit 0 (prevent loop)
5. Guard: no task processing → exit 0 (agent chatting freely)
6. Update message status → completed, agent status → idle
7. **Check orchestrator provider** (runtime, in `signal.rs`):
   - CLI provider → `tmux send-keys` to notify orchestrator
   - IDE provider (antigravity) → skip notification (DB update only)

### 6.2 Each provider needs 2 hook events

| Event | Claude Code | Gemini CLI | Purpose |
|-------|-------------|------------|---------|
| **Stop / AfterAgent** | `Stop` | `AfterAgent` | Task completed → signal Station |
| **Notification** | `Notification` (matcher: `permission_prompt`) | `Notification` | Agent needs approval → forward to Orchestrator |

### 6.3 Orchestrator skip guard (in `signal.rs`)

```
  Station checks DB:
    SELECT role FROM agents WHERE name = $AGENT_NAME
    → role = 'orchestrator' → SKIP, exit 0
    → role = 'worker'       → UPDATE completed
                                     │
                            SELECT provider FROM orchestrator
                            ├─ CLI  → tmux send-keys (notify)
                            └─ IDE  → return Ok(()) (no notify)
```

### 6.4 Setup Hooks Strategy

The `squad-station init` command (or a future `setup-hooks` subcommand) handles hook configuration:

```
  squad-station init
      │
      ├── For each provider (claude, gemini):
      │   ├── Check if settings.json exists
      │   │   ├─ YES: Parse existing JSON
      │   │   │       ├─ hooks key exists? Merge/append squad-station hook
      │   │   │       └─ Create backup: settings.json.bak
      │   │   └─ NO:  Print instructions to stdout:
      │   │           "Add this to .claude/settings.json: {...}"
      │   │
      │   └── Never overwrite user settings — only merge hook entries
```

### 6.5 Safe Tmux Injection (Multiline)

Sending long prompts/context via `tmux send-keys` is error-prone (shell escaping, special chars, multiline).

**Solution:** Rust-native `tmux::adapter` module handles safe injection:
- Write prompt content to a temp file
- Use `tmux load-buffer` + `tmux paste-buffer` for safe multiline transfer
- Or use heredoc-style injection via `tmux send-keys`
- `squad-station send` automatically uses the safe adapter

```
  squad-station send <agent> --body "long multiline prompt..."
      │
      ├── Write body to /tmp/squad-station-msg-<uuid>
      ├── tmux load-buffer /tmp/squad-station-msg-<uuid>
      ├── tmux paste-buffer -t <agent-session>
      └── Cleanup temp file
```

## 7. Station is a Stateless CLI

Every command exits after completion. No daemon, no background process.

```
  $ squad-station send ...       ← runs ~50ms, writes DB + tmux inject, exits
  $ squad-station signal ...     ← runs ~50ms, updates DB + notify, exits
  $ squad-station status         ← runs ~10ms, queries DB + prints, exits
```

## 8. CLI Reference — All Available Commands

### Core Commands

**`squad-station init`**
- Initialize project with squad.yml, launch agent tmux sessions, register agents
- Reads `squad.yml` config from project root
- Creates `.squad/` directory and `station.db` SQLite database
- Sets up tmux sessions for each agent
- Usage: `squad-station init`

**`squad-station send <agent-name> --body "<task>"`**
- Send task/prompt to an agent's tmux session
- Writes message to database with status `processing`
- Injects message into agent session via tmux (safe multiline handling)
- Usage: `squad-station send implement --body "Fix the bug in src/config.rs"`

**`squad-station signal <agent-name> [message-id]`**
- Mark agent task as completed (called by hook after agent finishes)
- Updates message status from `processing` to `completed`
- Notifies orchestrator (CLI mode) or updates DB only (IDE mode)
- Usage: `squad-station signal implement <id>`

**`squad-station list [--agent <name>] [--limit N]`**
- List messages from database with status and timestamps
- Filter by agent name with `--agent` flag
- Limit results with `--limit` flag (default: all)
- Usage: `squad-station list --agent implement --limit 5`

**`squad-station agents`**
- List all registered agents with current status and reconciliation
- Shows: name, role, provider, model, current status (idle/busy/dead)
- Reconciles against live tmux sessions
- Usage: `squad-station agents`

**`squad-station status`**
- Show project summary: agents, pending tasks, completed tasks
- Supports `--json` flag for machine-readable output
- Usage: `squad-station status` or `squad-station status --json`

**`squad-station register <name> <provider> <role> <tmux-session>`**
- Register a new agent at runtime (without restarting)
- Adds agent to database for message routing
- Usage: `squad-station register new-agent claude-code worker squad-new-agent`

**`squad-station context`**
- Generate provider-aware orchestrator context file
- Creates `.claude/commands/squad-orchestrator.md` (or provider-specific path)
- Updates with squad.yml config, agent list, SDD playbook reference
- Usage: `squad-station context`

### Cleanup Commands

**`squad-station close`**
- Kill all agent tmux sessions defined in squad.yml
- Shows killed/skipped session counts and remaining sessions
- Useful for stopping agents between tasks
- Usage: `squad-station close`

**`squad-station reset [--no-relaunch]`**
- Full cleanup: kill all sessions + delete database + relaunch
- Default: automatically relaunches sessions after cleanup
- Use `--no-relaunch` to skip session restart
- Useful for resetting to clean state
- Usage: `squad-station reset` or `squad-station reset --no-relaunch`

**`squad-station clean [-y | --yes]`**
- Delete local `.squad/station.db` database only
- Prompts for confirmation before deletion
- Use `-y` or `--yes` flag to skip confirmation
- Useful for clearing message history without restarting sessions
- Usage: `squad-station clean` or `squad-station clean -y`

### UI & Monitoring

**`squad-station ui`**
- Launch interactive TUI dashboard with keybindings
- View agent status, messages, task history in real-time
- Supports focus navigation and message filtering
- Usage: `squad-station ui`

**`squad-station view`**
- Open tmux tiled window showing all live agent sessions side-by-side
- Useful for monitoring multiple agents simultaneously
- Usage: `squad-station view`

**`squad-station peek`**
- Peek at an agent's next pending task from database
- Usage: `squad-station peek <agent-name>`

---

## 9. Agent Naming Convention

Agent name = Tmux session name. Format: `<project>-<provider>-<role>`

```
  Pattern:  <project>-<provider>-<role>

  project   = project name (from squad.yml)
  provider  = gemini, claude, aider, codex, ...
  role      = orchestrator, implement, brainstorm, test, docs, ...

  Agent name = tmux session name
  → Hook detects agent via: tmux display-message -p '#S'
  → Station looks up tmux session via: agent name
```

## 10. Data Model

```
station.db  (1 file per project: <project-root>/.squad/)

┌──────────────────────────────────────────────────┐
│ messages                                          │
│                                                   │
│  id              TEXT PK          (UUID)          │
│  from_agent      TEXT             (sender)        │
│  to_agent        TEXT             (recipient)     │
│  type            TEXT             (task / signal)  │
│  priority        TEXT             (normal/high/    │
│                                    urgent)        │
│  status          TEXT             (processing /    │
│                                    completed /    │
│                                    failed)        │
│  body            TEXT             (task prompt)    │
│  created_at      DATETIME                         │
│  completed_at    DATETIME                         │
└──────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────┐
│ agents                                            │
│                                                   │
│  name            TEXT PK          (<proj>-<prov>  │
│                                    -<role>)       │
│  role            TEXT             (orchestrator /  │
│                                    worker)        │
│  tool            TEXT             (claude-code,    │
│                                    gemini-cli)    │
│  model           TEXT             (sonnet, opus)   │
│  description     TEXT             (from squad.yml) │
│  status          TEXT             (idle/busy/dead) │
│  current_task    TEXT FK          (→ messages.id)  │
│  last_heartbeat  DATETIME                         │
└──────────────────────────────────────────────────┘

Relationships:
  agents.current_task ──► messages.id
  Hook only needs signal <agent-name>, does NOT need to know task ID
```

## 11. Message Lifecycle

```
                    Station send             Hook signal
                    + tmux inject            (agent done)
                      to agent
  send ───────► PROCESSING ──────────────────────► COMPLETED
                     │
                     │ abort / fail / timeout
                     ▼
                  FAILED / ABORTED
```

## 11. CLI Interface

```
squad-station — Message routing for AI agent squads

  Setup:
    init                              Read squad.yml, create station
    register <agent> [--tool] [--model] [--role] [--description]

  Info:
    status                            Quick summary
    agents                            List agents
    list [--agent] [--status] [--limit]

  Messaging:
    send <to> --body "..."            Send task + inject tmux
         [--type] [--priority]

  Signal:
    signal <agent>                    Hook reports completed/failed
           [--status completed|failed]
    peek <agent>                      Any pending task?

  Display:
    ui                                Dashboard TUI
    view                              Split tmux view

  Flags:
    --json                            Machine-readable output (global)
```

## 12. Dashboard

### `squad-station ui`

```
  ┌─────────────────────────────────────────────────────────────┐
  │  SQUAD STATION — myapp                                      │
  │                                                              │
  │  AGENTS                          STATUS    CURRENT TASK      │
  │  myapp-gemini-orchestrator       ● idle    —                 │
  │  myapp-claude-implement          ● busy    msg-a1b2 (auth)   │
  │  myapp-claude-brainstorm         ● idle    —                 │
  │                                                              │
  │  RECENT MESSAGES                                             │
  │  msg-a1b2  orch → implement    processing  2m ago            │
  │  msg-c3d4  orch → brainstorm   completed   5m ago            │
  │                                                              │
  │  [1-5] attach  [r] refresh  [q] quit                         │
  └─────────────────────────────────────────────────────────────┘
```

### `squad-station view`

Split tmux layout showing all live agent sessions side by side.

## 13. Methodology Integration (v2 Roadmap)

```yaml
# squad.yml — v2 will add methodology field
project: myapp
methodology: bmad          # bmad | superpower | speckit | openspec | none
```

> **v1:** User sets up methodology manually. Station only handles agent coordination.
> **v2:** Integrate methodology into squad.yml + auto-install on init.

---
*Source: Obsidian/1-Projects/Agentic-Coding-Squad/02. Solution Design - Squad Station.md*
*Updated: Rust confirmed (supersedes Go references in original)*
*Updated with: 04. Upgrade Design — Antigravity & Hooks Optimization*
*Updated with: 05. Local DB — `.squad/station.db` in project directory (replaces `~/.agentic-squad/`)*
