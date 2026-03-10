---
description: Delegate tasks to squad agents with direct command syntax
allowed-tools: Bash, Read, Grep, Glob
argument-hint: <task description>
---

# Squad Orchestrator

Delegate tasks to squad agents in the squad-station project. The orchestrator automatically bootstraps configuration and routes your task to the appropriate agent.

## Usage

```
/squad-orchestrator <task description>
```

## Examples

```
/squad-orchestrator Fix the failing test in test_integration.rs

/squad-orchestrator Implement Windows path support in config loading

/squad-orchestrator Review the signal handling logic for edge cases
```

## How It Works

1. **Bootstrap:** Reads `squad.yml` to load:
   - Project configuration
   - Orchestrator role, model, provider
   - Available agents (name, role, model, tmux-session)
   - SDD (Spec-Driven Development) playbook references

2. **Consult Playbook:** Reads the generated `squad-orchestrator.md` which contains:
   - Delegation workflow (how to send tasks)
   - Monitoring protocol (how to track completion)
   - Agent selection matrix (which agent for which task type)
   - Registered agents and their capabilities

3. **Select Agent:** Routes your task based on type:
   - **Bug fixes, implementation, testing** → `implement` agent (fast execution, focused on code)
   - **Architecture, design, code review, analysis** → `brainstorm` agent (high reasoning model)
   - **Complex tasks** → `brainstorm` first for analysis, then `implement` for execution

4. **Delegate & Monitor:**
   - Sends task via `squad-station send <agent>`
   - Waits for completion via signal-based monitoring
   - Reads agent output from tmux session
   - Returns results

## Workflow

### Simple Implementation Task

```
/squad-orchestrator Fix the bug in src/config.rs where resolve_db_path fails on Windows
```

The orchestrator will:
1. Route to `implement` agent (straightforward fix)
2. Send the task via `squad-station send`
3. Wait for completion signal
4. Read and display agent output

### Complex Task Requiring Analysis

```
/squad-orchestrator Design a caching strategy for squad-station that works across multiple projects
```

The orchestrator will:
1. Route to `brainstorm` agent for analysis first
2. Wait for architecture/design document
3. Review the design
4. If coding needed, delegate to `implement` agent with the design context

## Playbook Location

The orchestrator uses provider-aware playbooks:

- **Claude Code:** `.claude/commands/squad-orchestrator.md` (auto-generated)
- **Gemini CLI:** `.gemini/commands/squad-orchestrator.md` (auto-generated)
- **Fallback:** `.agent/workflows/squad-orchestrator.md` (auto-generated)

## Agent Selection Rules

| Task Type | Agent | Model | When to Use |
|-----------|-------|-------|-------------|
| Bug fix | implement | sonnet | Specific, focused issue |
| Feature implementation | implement | sonnet | Clear requirements exist |
| Code review | brainstorm | opus | Critical analysis needed |
| Architecture design | brainstorm | opus | New system or major refactor |
| Testing | implement | sonnet | Writing or fixing tests |
| Complex (both) | brainstorm → implement | opus → sonnet | Sequential analysis then coding |

## Tips

- **Keep tasks focused:** One clear objective per delegation
- **Provide context:** More details help the agent understand requirements
- **Monitor actively:** Use `squad-station list --agent <name>` to track progress
- **Verify results:** Always check agent output via `tmux capture-pane -t <agent-name> -p`
- **Sequential complex tasks:** For multi-step work, delegate to brainstorm first, then implement

## Configuration

The task routing is based on your `squad.yml` configuration. Each agent has:
- `name`: Agent identifier (e.g., `implement`, `brainstorm`)
- `role`: Job function (e.g., `worker`, `orchestrator`)
- `model`: Claude model to use (haiku, sonnet, opus)
- `provider`: Where agent runs (claude-code, gemini-cli, etc.)
- `tmux-session`: Session name for background execution
- `description`: Agent capabilities

Run `squad-station agents` to see current configuration.

## Troubleshooting

**Agent not responding?**
```
tmux list-sessions                    # Check if tmux sessions are alive
scripts/setup-sessions.sh             # Relaunch agent sessions
squad-station list --agent <name>     # Check message status
```

**Need to see agent output?**
```
tmux capture-pane -t <agent-name> -p  # Display tmux pane contents
```

**Check configuration?**
```
squad-station agents                  # List registered agents
scripts/validate-squad.sh             # Validate squad.yml
```

## See Also

- `squad.yml` — Project configuration and agent setup
- `squad-station init` — Initialize a squad project
- `squad-station send` — Manually send task to agent
- `squad-station signal` — Signal task completion
- `scripts/setup-sessions.sh` — Set up agent tmux sessions
- `.planning/quick/` — Track quick tasks with GSD
