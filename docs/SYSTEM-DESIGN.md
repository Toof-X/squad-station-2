# Squad Station — System Design

> System design document for Squad Station.
> Implementation language: Rust. References: Gastown (Mayor/Hook pattern), Overstory (SQLite WAL).

---

## 1. System Overview

You communicate with the **Orchestrator** via CLI terminal.
The Orchestrator understands requests, automatically selects a workflow (GSD/BMAD/...), and coordinates the **Worker agents**.
All communication goes through **Squad Station** — a central post office that knows how to deliver messages to the correct tmux session.

```
You ←→ Orchestrator (tmux)
              ↕  squad-station send/signal
         Squad Station (Rust CLI + SQLite)
              ↕  tmux inject / hook notify
     [implement] [brainstorm] [agent N] (tmux)
```

**Core Principles:**
- Orchestrator = your clone on the machine. It acts as your HITL proxy.
- Squad Station = post office. No brain. Only knows `from`, `to`, `body`.
- Agents = passive. They don't know each other. Receive tasks via tmux, report completion via hooks.
- SDD (GSD/BMAD) = workflow knowledge. Lives in the orchestrator's auto-generated context file.

---

## 2. squad.yml — System Declaration

Each project has 1 `squad.yml` file. This is the only config the user needs to write.

```yaml
project: my-app

sdd:
  - name: get-shit-done
    playbook: "/path/to/GSD/Playbook.md"
  - name: bmad
    playbook: "/path/to/BMAD/Playbook.md"

orchestrator:
  provider: claude-code       # claude-code | gemini-cli | antigravity
  role: orchestrator
  model: opus                 # optional: opus | sonnet | haiku (for claude-code)
  description: "Team lead. Acts on behalf of the user to coordinate all agents."

agents:
  - name: implement
    provider: claude-code
    role: worker
    model: sonnet
    description: "Senior developer. Writes code, fixes bugs, runs tests."

  - name: brainstorm
    provider: claude-code
    role: worker
    model: opus
    description: "Tech lead. Analyzes, designs, reviews."

  - name: tester
    provider: gemini-cli
    role: worker
    model: gemini-3.1-pro-preview
    description: "QA engineer. Writes tests, verifies acceptance criteria."
```

**Rules:**
- `project` → prefix for all tmux session names
- `sdd` → optional list of workflow frameworks (the Orchestrator auto-selects)
- `orchestrator` → exactly 1
- `agents` → N workers, each with its own tmux session
- Agent tmux session name = `<project>-<name>` (e.g., `my-app-implement`)
- Orchestrator tmux session name = `<project>-orchestrator`
- Session names are sanitized: `.`, `:`, `"` → `-`
- `#[serde(deny_unknown_fields)]` — unknown fields in agent config are rejected

**Provider validation:**
- Known providers: `claude-code`, `gemini-cli`, `antigravity`
- Unknown providers: warn to stderr but proceed (extensibility)
- Model validation per provider:
  - `claude-code` → `opus`, `sonnet`, `haiku`
  - `gemini-cli` → `gemini-3.1-pro-preview`, `gemini-3-flash-preview`
  - `antigravity` → no model validation (DB-only mode, no tmux)

---

## 3. Bootstrap — `squad-station init`

Run once at project start.

```
squad-station init
```

**Execution order:**

```
1. Read squad.yml (walks up directory tree if not in CWD)
2. Validate provider + model for all agents
3. Create .squad/station.db (SQLite WAL mode)
4. Register all agents in DB (INSERT OR IGNORE — idempotent)
5. Create tmux sessions:
   - <project>-orchestrator  (1 session, working dir: .squad/orchestrator/)
   - <project>-implement     (1 session per agent, working dir: project root)
   - <project>-brainstorm
   - <project>-tester
   - Skip tmux for antigravity agents (DB-only mode)
6. Launch AI tool in each session:
   - claude-code → claude --dangerously-skip-permissions --model <model>
   - gemini-cli  → gemini --model <model>
7. Generate orchestrator context file → .squad/orchestrator/<PROVIDER_FILE>
8. Create monitor session (<project>-monitor):
   - Tiled panes, one per agent (orchestrator + workers)
   - Each pane is an interactive nested tmux attach
   - Attach via: tmux attach -t <project>-monitor
9. Print "Get Started" message with provider-specific CLI invocation
```

### 3.1 Orchestrator Context — Provider-Specific Slash Command

The `context` command generates a slash command file that the Orchestrator loads via `/squad-orchestrator`.

| Provider | Generated file |
|----------|----------------|
| claude-code | `.claude/commands/squad-orchestrator.md` |
| gemini-cli  | `.gemini/commands/squad-orchestrator.toml` |
| other       | `.claude/commands/squad-orchestrator.md` (fallback) |

**Content structure (auto-generated):**

```markdown
## PRE-FLIGHT — Execute IMMEDIATELY before any task
(read SDD playbooks, verify agents, check project root)

## Completion Notification (Automatic)
(signal-based wait protocol, anti-polling rules)

## Context Management — /clear
(when to send /clear to agents, re-injection guidance)

## Session Routing
(which agent for which task type, routing rules)

## SDD Orchestration
(playbook-driven task delegation, if SDD configured)

## Sending Tasks
(send command templates per worker agent)

## Full Context Transfer
(capture-pane instructions for cross-agent handoff)

## Workflow Completion Discipline
(never interrupt, wait for signal)

## QA Gate
(6-step checklist after receiving signal, includes /clear decision)

## Agent Roster
(table: Agent | Model | Role | Description)
```

### 3.2 Context Auto-Inject (SessionStart Hook)

During `squad-station init`, the user is asked whether to enable auto-injection.
When enabled, a `SessionStart` hook is installed that runs `squad-station context --inject`
on every session start, resume, or compact/clear.

The `--inject` flag:
1. Detects the current tmux session name via `tmux display-message -p '#S'`
2. Compares against the orchestrator session name from `squad.yml`
3. If not the orchestrator → silent exit 0 (workers get no injection)
4. If the orchestrator → outputs content to stdout in provider-appropriate format:
   - Claude Code: raw markdown (stdout is injected as context)
   - Gemini CLI: JSON `{"hookSpecificOutput":{"additionalContext":"..."}}`

---

## 4. Squad Station — Message Hub

### 4.1 Role

Squad Station is a **post office**. No business logic. It only knows:
- Receive message: `from`, `to`, `body`, `priority`, `thread_id`
- Write to DB
- Inject into the correct tmux session for `to`
- Receive hook signal → mark done → notify orchestrator

### 4.2 Data Model

```sql
-- station.db (1 file per project: .squad/station.db)
-- Env override: SQUAD_STATION_DB

-- ── Agents ──────────────────────────────────────────────────

CREATE TABLE agents (
  id              TEXT PRIMARY KEY,
  name            TEXT NOT NULL UNIQUE,
  tool            TEXT NOT NULL DEFAULT '',   -- provider label (renamed from "provider" in migration 0003)
  role            TEXT NOT NULL DEFAULT 'worker', -- orchestrator | worker
  command         TEXT NOT NULL,              -- legacy field (empty string)
  created_at      TEXT NOT NULL,
  status          TEXT NOT NULL DEFAULT 'idle', -- idle | busy | dead
  status_updated_at TEXT NOT NULL,
  model           TEXT DEFAULT NULL,
  description     TEXT DEFAULT NULL,
  current_task    TEXT DEFAULT NULL           -- FK → messages.id
    REFERENCES messages(id)
);

-- ── Messages ────────────────────────────────────────────────

CREATE TABLE messages (
  id           TEXT PRIMARY KEY,              -- UUID
  agent_name   TEXT NOT NULL,                 -- legacy column (kept for backward compat)
  from_agent   TEXT DEFAULT NULL,             -- sender (orchestrator name or NULL)
  to_agent     TEXT DEFAULT NULL,             -- recipient (agent name)
  type         TEXT NOT NULL DEFAULT 'task_request', -- task_request | task_completed | notify
  thread_id    TEXT DEFAULT NULL,             -- group related messages
  task         TEXT NOT NULL,                 -- body/content
  status       TEXT NOT NULL DEFAULT 'processing', -- processing | completed
  priority     TEXT NOT NULL DEFAULT 'normal',     -- normal | high | urgent
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  completed_at TEXT DEFAULT NULL,
  FOREIGN KEY (agent_name) REFERENCES agents(name)
);

-- ── Indexes ─────────────────────────────────────────────────

CREATE INDEX idx_messages_agent_status ON messages(agent_name, status);
CREATE INDEX idx_messages_priority ON messages(priority, created_at);
CREATE INDEX idx_messages_direction ON messages(from_agent, to_agent);
CREATE INDEX idx_messages_thread ON messages(thread_id);

PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;
```

**Key design decisions:**
- Single-writer SQLite pool (`max_connections=1`) with 5s busy_timeout
- `send-keys -l` (literal mode) to prevent shell injection via tmux
- Idempotent agent registration (`INSERT OR IGNORE`)
- Idempotent signal handling (only updates oldest processing message by priority)
- DB column `tool` (not `provider`) — renamed in migration 0003 for clarity

### 4.3 Message Flow — Detailed

**Basic flow: Orchestrator → Agent → Done**

```
Orchestrator calls:
  squad-station send my-app-implement --body "Build login page"
    → write to messages: from=orchestrator, to=implement, type=task_request, status=processing
    → update agents: implement.status=busy, current_task=<msg-id>
    → tmux send-keys -t my-app-implement "Build login page" Enter
      (multiline bodies use load-buffer + paste-buffer via temp file)

Agent working...

Agent stops → Hook fires automatically:
  squad-station signal my-app-implement
    → GUARD: skip if agent is orchestrator (prevent loop)
    → update messages: status=completed, completed_at=now
    → update agents: implement.status=idle
    → tmux send-keys -t my-app-orchestrator "my-app-implement completed <msg-id>" Enter

Orchestrator receives notification → reads results:
  tmux capture-pane -t my-app-implement -p
    → decides next step
```

**HITL flow: Agent needs input from Orchestrator**

```
Agent is working, encounters an issue needing a decision:
  squad-station notify --body "Need confirmation: use JWT or sessions?"
    → auto-detect agent name from tmux session (or --agent explicit)
    → verify agent exists + is not orchestrator
    → tmux send-keys -t orchestrator "[SQUAD INPUT NEEDED] Agent 'implement': Need confirmation..."
    → does NOT change message/agent status

Orchestrator receives → thinks → asks user if needed → sends back:
  squad-station send my-app-implement --body "Use JWT. Expiry 24h."
    → tmux send-keys -t my-app-implement "Use JWT. Expiry 24h." Enter

Agent continues working...
```

**Threaded flow: Grouping related messages**

```
Orchestrator starts a thread:
  squad-station send my-app-implement --body "Build auth backend"
    → returns message ID (e.g. abc-123), auto-generates thread_id

Follow-up in same thread:
  squad-station send my-app-implement --body "Also add rate limiting" --thread abc-123
    → thread_id=abc-123 groups both messages

Query by thread:
  squad-station list --agent my-app-implement
    → shows messages grouped by thread
```

**Multi-agent flow: Orchestrator fan-out**

```
Orchestrator decides to run in parallel:
  squad-station send my-app-implement --body "Build auth backend"
  squad-station send my-app-brainstorm --body "Design API spec"

Both agents work in parallel...

Implement finishes first:
  squad-station signal → notify orchestrator

Brainstorm finishes:
  squad-station signal → notify orchestrator

Orchestrator receives both → synthesizes → next step
```

### 4.4 CLI Interface

```
squad-station — Message routing and orchestration for AI agent squads

Setup:
  init [config]                       Read squad.yml, create DB, tmux sessions, context,
                                      and monitor session with interactive panes
                                      (default: squad.yml, walks up dir tree)
  context [--inject]                  Generate orchestrator context file per provider
                                      --inject: output to stdout for SessionStart hook

Messaging:
  send <agent> --body "..."           Assign task to agent
       [--priority normal|high|urgent]
       [--thread <id>]                Group messages into a thread

  notify --body "..."                 Agent mid-task notification to orchestrator
         [--agent <name>]             (auto-detect from tmux if omitted)

  signal [agent]                      Hook reports agent has finished
                                      (accepts name or tmux pane ID like %3)
                                      (auto-detect from $TMUX_PANE if omitted)

  peek <agent>                        View highest-priority pending task for agent

Info:
  status                              Project overview + agent summary
  agents                              List agents with reconciled tmux status
  list [--agent] [--status] [--limit] List messages with filters

Display:
  ui                                  Interactive TUI dashboard (ratatui)
  view                                Split tmux tiled view of all live sessions

Lifecycle:
  register <name> [--role] [--tool]   Register agent at runtime
  clean [config] [-y]                 Kill all squad tmux sessions + delete DB
  reset [config] [--no-relaunch]      Clean + optionally relaunch (re-init)

Flags:
  --json                              Machine-readable output (global)
```

---

## 5. Hook System

### 5.1 Mechanism

Hooks are mechanisms that automatically notify Squad Station when an agent finishes a work session.
The agent **does not know** Squad Station exists. Hooks are an external layer.

Hook logic is embedded in `signal.rs` — there is no separate `hooks/` directory.

### 5.2 Signal Guard Chain

The `signal` command implements multiple guards to ensure safe, idempotent operation:

```
Agent stops → Hook fires → squad-station signal [agent]
                                  ↓
                    GUARD-03: No agent name? → silent exit 0
                    GUARD-02: Config/DB failure? → warn to stderr, exit 0
                    GUARD-03: Agent not found in DB? → silent exit 0
                    GUARD-04: Agent role = orchestrator? → silent exit 0
                                  ↓
                    Update message: processing → completed
                    Find orchestrator → send notification via tmux
                                  ↓
                    Always returns 0 (never fails the provider)
```

### 5.3 Config per provider

`squad-station init` auto-installs all hooks below. The inline command pattern uses
`$(tmux display-message -p '#S')` to resolve the agent name from the tmux session.

**Claude Code** (`.claude/settings.json`) — 4 hook events (+ optional SessionStart):

| Event | Matcher | Command | Purpose |
|-------|---------|---------|---------|
| `Stop` | `*` | `squad-station signal ...` | Agent finished turn → signal completion |
| `Notification` | `permission_prompt` | `squad-station notify ...` | Permission dialog blocking agent |
| `Notification` | `elicitation_dialog` | `squad-station notify ...` | MCP server input form blocking agent |
| `PostToolUse` | `AskUserQuestion` | `squad-station notify ...` | Agent asking clarifying question |
| `SessionStart` | `*` | `squad-station context --inject` | Auto-inject orchestrator context (opt-in) |

```json
{
  "hooks": {
    "Stop": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "squad-station signal $(tmux display-message -p '#S')" }] }
    ],
    "Notification": [
      { "matcher": "permission_prompt", "hooks": [{ "type": "command", "command": "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')" }] },
      { "matcher": "elicitation_dialog", "hooks": [{ "type": "command", "command": "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')" }] }
    ],
    "PostToolUse": [
      { "matcher": "AskUserQuestion", "hooks": [{ "type": "command", "command": "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')" }] }
    ],
    "SessionStart": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "squad-station context --inject" }] }
    ]
  }
}
```

The `SessionStart` hook is **opt-in** — installed only if the user says yes during `squad-station init`.
The `--inject` command internally guards on the orchestrator session name, so workers sharing the
same settings file receive no injection.

**Gemini CLI** (`.gemini/settings.json`) — 2 hook events (+ optional SessionStart):

| Event | Matcher | Command | Purpose |
|-------|---------|---------|---------|
| `AfterAgent` | `*` | `squad-station signal ...` | Agent finished turn → signal completion |
| `Notification` | `*` | `squad-station notify ...` | Any notification (permissions, alerts) |
| `SessionStart` | `*` | `squad-station context --inject` | Auto-inject orchestrator context (opt-in) |

```json
{
  "hooks": {
    "AfterAgent": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "squad-station signal $(tmux display-message -p '#S')" }] }
    ],
    "Notification": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')" }] }
    ],
    "SessionStart": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "squad-station context --inject" }] }
    ]
  }
}
```

**Antigravity:** No hooks needed — DB-only polling mode (no tmux sessions).

### 5.4 Orchestrator Resolution

`get_orchestrator()` returns the best orchestrator when multiple exist in the DB
(e.g. stale records from a previous `init` with a different project name):

```sql
SELECT * FROM agents WHERE role = 'orchestrator'
ORDER BY CASE WHEN status = 'dead' THEN 1 ELSE 0 END, created_at DESC
LIMIT 1
```

Non-dead orchestrators are always preferred. Among equals, the most recently created wins.

---

## 6. Agent Isolation — Solving the Context Hierarchy Problem

### 6.1 Problem

Claude Code reads CLAUDE.md following a hierarchy from root downward.
If orchestrator instructions are placed at the project root → worker agents will also read them → contamination occurs.

### 6.2 Solution — Subdirectory Isolation

**Orchestrator** runs at `.squad/orchestrator/` → reads its own context file.
**Worker agents** run at `project-root/` → read the project's own CLAUDE.md (uncontaminated).

```bash
# Bootstrap creates:
tmux new-session -s my-app-orchestrator -c ".squad/orchestrator"
# → Claude Code reads .squad/orchestrator/CLAUDE.md (orchestrator playbook)

tmux new-session -s my-app-implement -c "."
# → Claude Code reads ./CLAUDE.md (project instructions, not orchestrator)
```

The orchestrator knows the project root because the generated context file contains:
```markdown
## Project
Codebase at: /absolute/path/to/project
```

---

## 7. Reference from Gastown

| Gastown concept | Squad Station equivalent | Notes |
|----------------|-------------------------|-------|
| Mayor | Orchestrator | Gastown: Mayor = global coordinator. Squad Station: Orchestrator = project coordinator |
| Polecat | Worker agent | Gastown: persistent identity. Squad Station: simpler |
| Hook | Hook (signal.rs) | Same — agent is passive, hook fires automatically |
| Beads | SQLite messages | Gastown: git-backed Dolt. Squad Station: SQLite WAL (simpler) |
| `gt nudge` | `squad-station notify` | Real-time messaging without going through mail queue |
| `gt sling` | `squad-station send` | Assign work to agent |
| `gt mayor attach` | `tmux attach -t <project>-orchestrator` | You jump in to talk |
| Convoy | thread_id in messages | Group related tasks |
| `gt prime` | orchestrator context auto-loads | Context recovery after compaction |

---

## 8. Directory Structure After `squad-station init`

```
my-app/
├── squad.yml                    ← user config (only file user writes)
├── CLAUDE.md                    ← project instructions (user's own, NOT generated)
├── .squad/
│   ├── station.db               ← SQLite WAL message hub
│   ├── station.db-wal           ← WAL file
│   └── station.db-shm           ← shared memory
├── .claude/
│   ├── settings.json            ← hooks (auto-installed by init)
│   └── commands/
│       └── squad-orchestrator.md ← orchestrator slash command (auto-generated)
├── .gemini/                     ← (if gemini-cli provider)
│   ├── settings.json            ← hooks
│   └── commands/
│       └── squad-orchestrator.toml
└── (project codebase)

tmux sessions created:
  my-app-orchestrator              ← orchestrator AI session
  my-app-implement                 ← worker AI session
  my-app-brainstorm                ← worker AI session
  my-app-monitor                   ← interactive monitor (tiled panes for all agents)
```

---

## 9. Runtime Loop — Step by Step

```
1. You: tmux attach -t my-app-orchestrator
   → jump into the Orchestrator's session

2. You say: "Add Google login feature"
   → Orchestrator reads its context file → knows to use GSD workflow
   → Orchestrator reads GSD Playbook → knows what to do

3. Orchestrator thinks → selects agent:
   squad-station send my-app-brainstorm --body "Design the OAuth Google login flow"
   → station.db: message inserted, type=task_request, status=processing
   → agent.status=busy, current_task=<msg-id>
   → tmux inject into my-app-brainstorm

4. Brainstorm agent works (Orchestrator waits for hook signal)

5. Brainstorm stops → Hook fires:
   squad-station signal my-app-brainstorm
   → guard checks pass (not orchestrator, agent exists)
   → message status=completed, completed_at=now
   → agent.status=idle
   → tmux send-keys to orchestrator: "my-app-brainstorm completed <id>"

6. Orchestrator receives signal → reads results:
   tmux capture-pane -t my-app-brainstorm -p
   → reads the design → decides next step

7. Orchestrator assigns to implement:
   squad-station send my-app-implement --body "Implement OAuth: [design from brainstorm]"

8. ... (repeats until complete)

9. Orchestrator reports to you:
   "Done. Implemented Google OAuth. Files: auth/google.ts, config/oauth.ts. Tests: pass."
```

---

## 10. Architecture Modules

```
src/
├── main.rs          ← Entry: SIGPIPE handler (SAFE-04), async tokio runtime, command dispatch
├── cli.rs           ← clap-based arg parsing (Commands enum, 15 subcommands)
├── config.rs        ← YAML config loading, validation, DB path resolution, session name sanitization
├── tmux.rs          ← tmux operations: send-keys (literal), inject-body (multiline via buffer),
│                       session management, view creation, pane-to-session resolution
├── commands/
│   ├── mod.rs       ← module declarations
│   ├── init.rs      ← bootstrap: config → DB → register agents → tmux sessions → hooks → context
│   ├── send.rs      ← task dispatch: validate → DB write → agent busy → tmux inject
│   ├── signal.rs    ← hook handler: guard chain → mark complete → notify orchestrator
│   ├── notify.rs    ← mid-task HITL notification (no status change)
│   ├── peek.rs      ← highest-priority pending message for agent
│   ├── list.rs      ← message listing with filters
│   ├── register.rs  ← runtime agent registration
│   ├── agents.rs    ← agent listing with tmux status reconciliation
│   ├── context.rs   ← generate orchestrator context file; --inject outputs to stdout for hooks
│   ├── status.rs    ← project + agent status summary
│   ├── ui.rs        ← ratatui TUI dashboard (drops pool after each fetch to prevent WAL starvation)
│   ├── view.rs      ← tmux tiled view of agent sessions
│   ├── clean.rs     ← kill all squad tmux sessions + delete DB
│   ├── reset.rs     ← clean + optionally relaunch (re-init)
│   ├── freeze.rs    ← freeze/unfreeze agent task dispatch
│   └── helpers.rs   ← shared: colorize_agent_status, format_status_with_duration, reconcile
└── db/
    ├── mod.rs       ← SQLite pool setup (max_connections=1, busy_timeout=5s)
    ├── agents.rs    ← Agent CRUD, get_orchestrator (prefers non-dead), reconciliation
    ├── messages.rs  ← Message CRUD, priority ordering, idempotent completion
    └── migrations/
        ├── 0001_initial.sql       ← agents + messages tables
        ├── 0002_agent_status.sql  ← agent status + status_updated_at
        ├── 0003_v11.sql           ← provider→tool rename, model, description, from/to routing, message type
        └── 0004_thread_id.sql     ← thread_id for grouping related messages
```

---

## 11. Release History

| Version | Highlights |
|---------|------------|
| v0.5.1 | First public release: npm package, colored init, provider-agnostic hooks, full messaging pipeline |
| v0.5.3 | PostToolUse hook (AskUserQuestion), elicitation_dialog support, orchestrator resolution fix |
| v0.5.5 | Context auto-inject (SessionStart hook), /clear management, simplified CLI (close removed, clean = kill + delete) |

---

*Implementation language: Rust (decided in TECH-STACK.md)*
*171 tests: cargo test (unit + integration, all async tokio)*
