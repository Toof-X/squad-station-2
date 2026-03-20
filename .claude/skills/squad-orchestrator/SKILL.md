---
name: squad-orchestrator
description: AI Orchestrator — manage and coordinate squad agents
argument-hint: <task description>
---

# Squad Orchestrator — Task Coordination

Delegate tasks to squad agents by invoking the orchestrator with a task description. The orchestrator will execute a **7-step coordination workflow** to bootstrap configuration, select the appropriate agent, delegate work, monitor completion, and report results.

**You have received a task as input.** Follow this protocol to coordinate agent execution.

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

## EXECUTION PROTOCOL

### STEP 1: CONTEXT CHECK & BOOTSTRAP

Before delegating, you MUST:

1. **Recall Context** — Can you recall:
   - Contents of `squad.yml`
   - Agent list (name, role, model, tmux-session)
   - SDD playbook path

   If NOT → immediately read these files before proceeding.

2. **Validate Setup** — Run:
   ```
   scripts/validate-squad.sh
   ```
   Confirm tmux sessions are alive and playbook paths exist.

3. **Read SDD Playbook** — Load:
   ```
   Read sdd[].playbook file (path from squad.yml)
   ```
   Learn available workflow commands for this project.

**Report:** "✓ Bootstrap complete: [project name], [N agents], SDD: [sdd name]"

---

### STEP 2: ANALYZE TASK & CONSULT SDD

Read the task you received:

```
TASK: <input task argument>
```

Now consult the SDD playbook:
- What workflow commands are available?
- What project state checks are needed?
- What are the available workflow commands?

**Report:** "Task analyzed. Available workflow commands: [list]"

---

### STEP 3: SPEC-DRIVEN DECISION LOOP

Apply the decision framework:

```
┌─────────────────────────────────────────────────────┐
│  1. CONSULT SDD                                     │
│     ✓ Read sdd[].playbook for available commands    │
│     → Check project state using SDD's own method    │
│     → Read docs/ for architecture decisions         │
│                                                      │
│  2. SELECT WORKFLOW COMMAND                          │
│     → Pick the right command for current state      │
│     → This command will guide agent on what to do   │
│                                                      │
│  3. SELECT AGENT                                    │
│     → Match task type → agent role (see below)       │
│     → Implement agent? Brainstorm agent?             │
│                                                      │
│  4. COMPOSE MESSAGE                                 │
│     → Include: workflow command + full context       │
│     → Include: original task description             │
│                                                      │
│  5. DELEGATE                                        │
│     → Send via: scripts/tmux-send.sh                │
│     → Target: agents[].tmux-session                 │
│                                                      │
│  6. MONITOR                                         │
│     → Wait with adaptive timeouts                   │
│     → Check completion via squad-station list       │
│                                                      │
│  7. VERIFY & REPORT                                 │
│     → Read agent output via tmux capture-pane      │
│     → Verify against task requirements              │
│     → Report results to user                        │
└─────────────────────────────────────────────────────┘
```

---

### STEP 4: SELECT AGENT BASED ON TASK TYPE

Match the task to an agent:

```
┌──────────────────────────────┬─────────────────────────────────┐
│  TASK TYPE                   │  AGENT SELECTION                │
├──────────────────────────────┼─────────────────────────────────┤
│  Analysis, architecture,     │  → brainstorm agent             │
│  code review, solution       │     (highest reasoning model)   │
│  design, research            │                                 │
├──────────────────────────────┼─────────────────────────────────┤
│  Implementation, bug fix,    │  → implement agent              │
│  test writing, refactoring   │     (fast execution model)      │
├──────────────────────────────┼─────────────────────────────────┤
│  Complex task requiring      │  → brainstorm FIRST for plan    │
│  both analysis and coding    │  → THEN implement for execution │
└──────────────────────────────┴─────────────────────────────────┘
```

Decision rules:
- If task requires **reasoning before doing** → brainstorm first, implement second.
- If task is **straightforward implementation** → implement directly.
- If **unsure** → brainstorm a brief analysis, then decide.
- For **independent sub-tasks** → delegate to multiple agents in parallel.
- For **dependent sub-tasks** → sequential delegation (each feeding the next).

**Report:** "Agent selected: [agent name] ([role], model: [model])"

---

### STEP 5: COMPOSE & SEND DELEGATION

Prepare the message with:
1. **Workflow command** from the SDD playbook
2. **Full task context** from your reasoning
3. **Original task description**

Send via:
```bash
scripts/tmux-send.sh <agent-tmux-session> "<message>"
```

**Report:** "✓ Task delegated to [agent name]. Message ID: [id if available]"

---

### STEP 6: MONITOR & WAIT

Use **adaptive wait times** based on task complexity:

```
WAIT TIME = base_time × complexity_multiplier

base_time:
  - Confirmation / simple ops     → 10s
  - Interactive Q&A               → 20s
  - Generation (requirements, roadmap, plan) → 60s
  - Execution (code, tests, build) → 90s

complexity_multiplier:
  - Single file / small scope     → 1.0×
  - Multi-file / medium scope     → 1.5×
  - Cross-module / large scope    → 2.0×
```

Check status:
```bash
squad-station list --agent <agent-name> --limit 1
```

**Report:** "Monitoring [agent name]. Wait time: [calculated time]"

---

### STEP 7: VERIFY & RETURN RESULTS

After wait period, verify completion:

```bash
1. Check status:
   squad-station list --agent <agent-name> --limit 1

   IF status == "completed":
     → proceed to read output

   IF status == "processing":
     → wait another interval
     → after 3 checks with no progress, investigate

   IF tmux session is gone:
     → relaunch: scripts/setup-sessions.sh
     → re-send the task

2. Read agent output:
   tmux capture-pane -t <agent-name> -p

3. Verify output against task requirements:
   ✓ Does output match expected deliverables?
   ✓ Are all requirements satisfied?
   ✓ Are there errors to fix?

4. Report results to user:
   - Summary of what agent completed
   - Key outputs/deliverables
   - Any follow-up actions needed
```

**Report:** "✓ Task complete. Agent output: [summary]. Status: [success/needs-review]"

---

## GROUND RULES (CRITICAL)

1. **Bootstrap MUST come first** — Always read squad.yml and validate before delegating.
2. **Every delegation MUST include workflow command** — Never send raw task without SDD context.
3. **You do NOT write code** — You orchestrate; agents implement.
4. **Your workspace is limited to** — `docs/`, `squad.yml`, and coordination commands only.
5. **Use English for all communication** — Between orchestrator and agents.
6. **Maintain session until completion** — Ensure results verify against task requirements.
7. **Report at each step** — Update user on progress through delegation pipeline.

---

## ERROR RECOVERY

| Situation | Action |
|-----------|--------|
| tmux session gone | Relaunch: `scripts/setup-sessions.sh`, re-send task |
| Agent stuck (no progress) | Use `tmux capture-pane` to diagnose, consider re-delegation |
| Task failed | Re-delegate with error context to same or different agent |
| SDD unclear | Re-read the SDD playbook, follow its specific state-check method |
| Multiple independent sub-tasks | Delegate to multiple agents in parallel |
| Complex task (analysis + coding) | Brainstorm first for plan, then implement second |

---

## COMMUNICATION REFERENCE

**Send task to agent:**
```bash
scripts/tmux-send.sh <agent-tmux-session> "<message>"
```

**Read agent output:**
```bash
tmux capture-pane -t <agent-tmux-session> -p
```

**Check agent status:**
```bash
squad-station list --agent <agent-name>
```

**Validate setup:**
```bash
scripts/validate-squad.sh
```

---

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

---

**Now execute the protocol above for the input task. Begin with STEP 1: CONTEXT CHECK & BOOTSTRAP.**
