---
name: squad-orchestrator
description: AI Orchestrator — delegate tasks to squad agents with direct command syntax
trigger: explicit
---

# Squad Orchestrator — Active Coordination

Delegate tasks to squad agents by invoking the orchestrator with a task description. The orchestrator will execute a **7-step coordination workflow** to bootstrap configuration, select the appropriate agent, delegate work, monitor completion, and report results.

> **Activation:** When invoked as `/squad-orchestrator <task>`, the orchestrator immediately begins executing the full coordination protocol defined in `.claude/commands/squad-orchestrator.md`.

---

## Quick Start — Task Examples

**Simple bug fix:**
```
/squad-orchestrator Fix the failing test in test_integration.rs
```
→ Routes to `implement` agent, sends task, monitors, returns results

**Feature implementation:**
```
/squad-orchestrator Implement Windows path support in config loading
```
→ Routes to `implement` agent, executes, verifies, reports

**Code review/analysis:**
```
/squad-orchestrator Review the signal handling logic for edge cases
```
→ Routes to `brainstorm` agent for deep analysis

**Complex architectural task:**
```
/squad-orchestrator Design a caching strategy for squad-station that works across multiple projects
```
→ Routes to `brainstorm` agent first (design), then `implement` (if needed)

---

## Usage

```
/squad-orchestrator <task description>
```

The task is passed as the argument, and the orchestrator immediately begins execution.

## How It Works — 7-Step Execution Protocol

When you invoke `/squad-orchestrator <task>`, the orchestrator executes:

### **STEP 1: Bootstrap**
- Reads `squad.yml` to load project config, agents, SDD references
- Validates setup via `scripts/validate-squad.sh`
- Reads SDD playbook to learn available workflow commands
- **Report:** "✓ Bootstrap complete: [project], [N agents], SDD: [name]"

### **STEP 2: Analyze Task & Consult SDD**
- Parses the input task
- Consults SDD playbook for available workflow commands
- Checks project state per SDD's own method
- **Report:** "Task analyzed. Available workflow commands: [list]"

### **STEP 3: Spec-Driven Decision Loop**
- Applies the decision framework (Consult → Select Workflow → Select Agent → Compose → Delegate → Monitor → Verify)
- Reads architecture decisions from `docs/` if present
- **Report:** "Decision loop applied. Next: Agent selection"

### **STEP 4: Select Agent**
- Matches task type to agent role:
  - **Analysis/architecture/review** → brainstorm agent (opus model)
  - **Implementation/bug fix/testing** → implement agent (sonnet model)
  - **Complex (both)** → brainstorm first, then implement
- **Report:** "Agent selected: [agent name] ([role], model: [model])"

### **STEP 5: Delegate**
- Composes message with workflow command + full context
- Sends via `scripts/tmux-send.sh <agent-tmux-session> "<message>"`
- **Report:** "✓ Task delegated to [agent name]"

### **STEP 6: Monitor**
- Waits with adaptive timeout (10s-90s based on complexity)
- Checks status: `squad-station list --agent <agent>`
- Recovers if tmux session dies
- **Report:** "Monitoring [agent]. Wait time: [calculated]"

### **STEP 7: Verify & Report**
- Reads agent output: `tmux capture-pane -t <agent> -p`
- Verifies output matches task requirements
- Returns results with summary
- **Report:** "✓ Task complete. Output: [summary]. Status: [success/review]"

## Agent Selection Rules

The orchestrator automatically routes based on task type:

| Task Type | Selected Agent | Model | Execution |
|-----------|---|---|---|
| Bug fixes | implement | sonnet | Direct execution |
| Implementation | implement | sonnet | Direct execution |
| Testing | implement | sonnet | Direct execution |
| Architecture | brainstorm | opus | Direct analysis |
| Code review | brainstorm | opus | Direct analysis |
| Complex (analysis + code) | brainstorm + implement | opus → sonnet | Sequential: analyze first, then code |

## Execution Examples

### Straightforward Implementation Task

```
/squad-orchestrator Fix the bug in src/config.rs where resolve_db_path fails on Windows
```

**Orchestrator executes:**
1. ✓ Bootstraps from squad.yml
2. ✓ Task type: bug fix → routes to `implement` agent
3. ✓ Sends via `squad-station send`
4. ✓ Monitors with adaptive wait (90s for bug fix)
5. ✓ Reads agent output and verifies fix
6. ✓ Reports: "✓ Task complete. Agent implemented fix and ran tests. All passing."

### Complex Architectural Task

```
/squad-orchestrator Design a distributed caching layer that handles multi-project concurrency
```

**Orchestrator executes:**
1. ✓ Bootstraps from squad.yml
2. ✓ Task type: design → routes to `brainstorm` agent first
3. ✓ Sends: "Design a distributed caching layer..."
4. ✓ Monitors: waits 60s for design document
5. ✓ Reviews brainstorm output (architecture design)
6. ✓ If implementation needed, delegates design to `implement` agent
7. ✓ Monitors implementation, verifies, reports results

## Orchestrator Playbook

The coordination protocol is defined in the provider-aware playbook:

- **Claude Code:** `.claude/commands/squad-orchestrator.md` (executable protocol)
- **Gemini CLI:** `.gemini/commands/squad-orchestrator.md` (executable protocol)
- **Fallback:** `.agent/workflows/squad-orchestrator.md` (executable protocol)

The playbook contains the 7-step execution workflow that the orchestrator follows when invoked with a task argument.

## Best Practices

**Task Description:**
- ✓ Keep focused (one clear objective)
- ✓ Provide context (more details = better routing)
- ✓ Be specific (what needs to be done, why, any constraints)

**Monitoring:**
- ✓ Orchestrator automatically monitors completion
- ✓ Adaptive wait times prevent timeout issues
- ✓ Reports progress at each step

**Results:**
- ✓ Orchestrator verifies output matches task requirements
- ✓ Reports success/issues clearly
- ✓ Includes agent output for verification

**Examples of Good Task Descriptions:**
- ❌ "Fix the code" → ✓ "Fix the bug in src/config.rs where resolve_db_path fails on Windows paths"
- ❌ "Implement something" → ✓ "Implement support for Windows path separators in the config loader, ensuring all existing tests pass"
- ❌ "Review code" → ✓ "Review the signal handling in src/commands/signal.rs for potential race conditions with concurrent delegations"

## Ground Rules

1. **Orchestrator handles coordination** — You invoke with a task; orchestrator does the rest
2. **Automatic agent selection** — Task type determines routing (no manual agent selection needed)
3. **Built-in monitoring** — Orchestrator waits and verifies automatically
4. **Full context preservation** — Task context passed through all delegation steps
5. **Error recovery** — Orchestrator handles tmux failures and retries

## References

- **Coordination Protocol:** `.claude/commands/squad-orchestrator.md`
- **Squad Config:** `squad.yml`
- **Agent Setup:** `scripts/setup-sessions.sh`
- **Setup Validation:** `scripts/validate-squad.sh`
- **Task Tracking:** `squad-station list --agent <name>`
- **Manual Delegation:** `scripts/tmux-send.sh <session> "<message>"`
