# Phase 5: Feature Completion - Research

**Researched:** 2026-03-08
**Domain:** Rust CLI (clap), shell hook scripts, Claude Code hooks, Gemini CLI hooks
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| HOOK-01 | User can register Notification hook for Claude Code | Hook event name, stdin JSON schema, settings.json config pattern documented below |
| HOOK-02 | User can register Notification hook for Gemini CLI | Hook event name, stdin JSON schema, settings.json config pattern documented below |
| CLI-01 | User sends task via `send <agent> --body "task..."` flag syntax | clap rename from positional `task: String` to `#[arg(long)] body: String` — trivial in cli.rs + send.rs |
| CLI-02 | `init` auto-prefixes agent names as `<project>-<tool>-<role>` | init.rs already has stub for this pattern; needs to enforce it when `name` is set in squad.yml |
| CLI-03 | `context` output includes `model` and `description` per agent | context.rs currently ignores agent.model/description; Agent struct already has both fields from Phase 4 |
| SIG-01 | Signal notifications use format `"<agent> completed <msg-id>"` | signal.rs line 78 uses old `[SIGNAL] agent=X...` format; one-line string format change |
</phase_requirements>

---

## Summary

Phase 5 is the final behavioral feature phase before documentation. All 6 requirements are code changes to existing files — no new modules, no schema migrations, no new dependencies. The scope divides cleanly into two plans as stated in the roadmap.

**Plan 05-01** is exclusively about two new shell scripts: `hooks/claude-code-notify.sh` and `hooks/gemini-cli-notify.sh`. These are Notification-event hooks that forward a permission prompt (agent is blocked waiting for user approval) to the orchestrator via tmux. The existing Stop/AfterAgent hooks (`hooks/claude-code.sh`, `hooks/gemini-cli.sh`) are the correct model to follow — same guard pattern, same `exit 0` contract, same binary delegation approach.

**Plan 05-02** touches four separate files: `src/cli.rs` (rename `task` positional to `--body` flag), `src/commands/send.rs` (update function signature), `src/commands/init.rs` (enforce naming convention), `src/commands/context.rs` (add model/description to output), and `src/commands/signal.rs` (change notification format string). All of these are small, targeted changes with existing test infrastructure.

**Primary recommendation:** Implement as two plans in sequence. Plan 05-01 (hooks) is self-contained bash scripting. Plan 05-02 (CLI changes) touches Rust and requires rebuilding the test binary. No new crate dependencies needed.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.5 (already in Cargo.toml) | CLI arg parsing — renaming positional to `--body` flag | Already used throughout the project |
| bash | system | Shell hook scripts | Both Claude Code and Gemini CLI execute hook commands via shell |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| jq | system (optional) | Parse stdin JSON in hooks | Optional — hooks can drain stdin without parsing |
| squad-station binary | current build | Called by hooks for signal/notify logic | All guard logic stays in Rust, not in shell |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `--body` flag | keep positional `task` | Design doc is explicit: `--body` is the locked decision. Positional arg must be removed. |
| Shell hook scripts | Rust subcommand for notify | Shell is correct — providers execute hooks as shell commands |

**Installation:** No new dependencies. Phase 5 uses existing Cargo.toml.

---

## Architecture Patterns

### Recommended Project Structure
```
hooks/
├── claude-code.sh           # existing Stop hook (model for new hooks)
├── claude-code-notify.sh    # NEW: Notification hook for Claude Code (HOOK-01)
├── gemini-cli.sh            # existing AfterAgent hook
└── gemini-cli-notify.sh     # NEW: Notification hook for Gemini CLI (HOOK-02)

src/cli.rs                   # CLI-01: rename task positional → --body flag
src/commands/
├── send.rs                  # CLI-01: update run() signature
├── init.rs                  # CLI-02: enforce <project>-<tool>-<role> naming
├── context.rs               # CLI-03: add model + description columns to output
└── signal.rs                # SIG-01: change notification format string
```

### Pattern 1: Notification Hook Script (Claude Code)

**What:** A shell script registered under the `Notification` event in Claude Code hooks. Fires when Claude Code needs user approval. The hook receives JSON on stdin with `notification_type: "permission_prompt"` and `message` fields.

**When to use:** When an agent's Claude Code instance is blocked waiting for a tool permission — the orchestrator should be notified so the user can respond.

**Exit code contract:**
- Exit 0 always — non-zero exits do not help the user and may interfere with the provider
- Specifically: Claude Code exit 2 is a "block" code that prevents the Stop event from completing (catastrophic). The Notification hook has a different semantic but the safest pattern remains exit 0 always.

**stdin JSON (Claude Code Notification event):**
```json
{
  "hook_event_name": "Notification",
  "notification_type": "permission_prompt",
  "message": "Claude needs permission to use Write",
  "title": "Permission needed",
  "session_id": "...",
  "transcript_path": "...",
  "cwd": "..."
}
```

**Registration in `.claude/settings.json` (project-level or user-level):**
```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/hooks/claude-code-notify.sh"
          }
        ]
      }
    ]
  }
}
```

**Source:** [Claude Code hooks docs](https://code.claude.com/docs/en/hooks) (HIGH confidence, verified via WebSearch cross-reference with multiple community examples)

### Pattern 2: Notification Hook Script (Gemini CLI)

**What:** A shell script registered under the `Notification` event in Gemini CLI hooks. Fires when Gemini CLI emits a `ToolPermission` system alert.

**stdin JSON (Gemini CLI Notification event):**
```json
{
  "hook_event_name": "Notification",
  "notification_type": "ToolPermission",
  "message": "...",
  "details": {},
  "session_id": "...",
  "transcript_path": "...",
  "cwd": "...",
  "timestamp": "..."
}
```

**Exit code contract:**
- Exit 0: success
- Exit 2: system block (blocks the action, uses stderr as rejection reason) — do NOT use for notification hooks
- Other non-zero: warning, CLI continues

**Registration in `.gemini/settings.json`:**
```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/hooks/gemini-cli-notify.sh"
          }
        ]
      }
    ]
  }
}
```

**Source:** [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/) (HIGH confidence from official docs)

### Pattern 3: clap `--body` Flag (CLI-01)

**What:** Change the `Send` variant in `Commands` enum from a positional `task: String` to a named flag `body: String`.

**Current state (cli.rs lines 26-33):**
```rust
Send {
    agent: String,
    task: String,         // positional
    #[arg(long, value_enum, default_value = "normal")]
    priority: Priority,
},
```

**Required state:**
```rust
Send {
    agent: String,
    #[arg(long)]
    body: String,         // --body flag, no longer positional
    #[arg(long, value_enum, default_value = "normal")]
    priority: Priority,
},
```

**Callers that must update:**
- `src/main.rs` line 26: `Send { agent, task, priority }` → `Send { agent, body, priority }`
- `src/commands/send.rs`: function signature `run(agent: String, task: String, ...)` → `run(agent: String, body: String, ...)`; all internal uses of `task` variable rename to `body`
- `tests/test_cli.rs`: tests referencing positional `send agent task` must update to `send agent --body task`

**Breaking change:** Positional `task` is rejected after this change. Any callers using old syntax get a clap parse error. This is intentional per CLI-01.

### Pattern 4: Agent Auto-Prefix on Init (CLI-02)

**What:** When `init` reads `agents[].name` from squad.yml, it auto-derives the full DB name as `<project>-<tool>-<role>`. When `name` is provided in squad.yml, that value is used as the `<role>` component.

**Current behavior (init.rs lines 52-56):**
```rust
let agent_name = agent
    .name
    .clone()
    .unwrap_or_else(|| format!("{}-{}-worker", config.project, agent.tool));
```

This uses `name` verbatim if present, only auto-generating when `name` is absent.

**Required behavior per CLI-02:**
```rust
let role = agent.name.as_deref().unwrap_or(&agent.role);
let agent_name = format!("{}-{}-{}", config.project, agent.tool, role);
```

Same pattern applies to orchestrator registration (init.rs lines 17-21).

**Design reference (SOLUTION-DESIGN.md section 2):**
```
myapp-claude-implement       role=worker
myapp-gemini-brainstorm      role=worker
myapp-claude-docs            role=worker
```
The `name` field in squad.yml (`implement`, `brainstorm`, `docs`) becomes the role suffix, not the full name.

### Pattern 5: Context Output Update (CLI-03)

**What:** `context.rs` currently outputs a table with `| Agent | Role | Status | Send Command |`. Must add `model` and `description` columns (or incorporate them into the Markdown output body).

**Agent struct already has the data** (from Phase 4, AGNT-01):
```rust
pub model: Option<String>,
pub description: Option<String>,
```

**Design reference (SOLUTION-DESIGN.md section 2 context output):**
```
## myapp-claude-implement (Claude Sonnet)
Developer agent. Writes code, fixes bugs, runs tests.
→ squad-station send myapp-claude-implement --body "..."
```

The context output format should match this Markdown template — model in the heading, description as body paragraph, send command using `--body` flag syntax.

**Note:** The context command send examples must also be updated from positional to `--body` flag syntax (catches stale help text).

### Pattern 6: Signal Format Change (SIG-01)

**What:** Change notification string in `signal.rs` from `[SIGNAL] agent=X status=completed task_id=Y` to `<agent> completed <msg-id>`.

**Current (signal.rs line 78):**
```rust
let notification = format!(
    "[SIGNAL] agent={} status=completed task_id={}",
    agent, task_id_str
);
```

**Required:**
```rust
let notification = format!("{} completed {}", agent, task_id_str);
```

**Design reference (SOLUTION-DESIGN.md section 4.1 happy path):**
```
"myapp-claude-implement completed msg-a1b2c3"
```

### Anti-Patterns to Avoid

- **Using `exit 1` in notification hooks:** Both providers may interpret non-zero exits as errors. Use `exit 0` always in notification hooks. The business logic (forwarding to orchestrator) is best-effort.
- **Parsing stdin in hooks beyond what's needed:** The hooks in this project delegate all guard logic to the Rust binary. The notification hook can drain stdin with `cat > /dev/null` and forward based on tmux session name only.
- **Keeping positional `task` alongside `--body`:** Do not add `--body` as an optional alias. CLI-01 requires the positional form to be rejected. Remove `task: String`, replace with `#[arg(long)] body: String`.
- **Using `name` from squad.yml as the full agent name:** CLI-02 explicitly requires auto-prefix. `name: backend` in squad.yml becomes `myapp-claude-backend` in DB, never just `backend`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Hook stdin parsing | Custom JSON parser in bash | `cat > /dev/null` to drain + rely on tmux session name | Guards are in Rust binary; shell only needs session name |
| New Rust notify command | New `notify` subcommand | Reuse `signal` command with guard logic | Signal already handles orchestrator lookup + tmux send-keys |
| Custom notification delivery | HTTP/webhook in hook | `squad-station signal` delegates to binary | All routing stays in Rust; hook scripts stay thin |

**Key insight:** The existing hook architecture (thin shell → binary) is the correct pattern. The Notification hook follows the exact same structure as the Stop/AfterAgent hooks. No new Rust code needed for HOOK-01/HOOK-02.

---

## Common Pitfalls

### Pitfall 1: Notification Hook Calls `signal` (Wrong Command)

**What goes wrong:** The Notification event fires when an agent needs a permission (is paused, not done). Calling `squad-station signal` would mark the task as completed — incorrect.

**Why it happens:** The existing hooks call `signal`. Confusing Notification (approval needed) with Stop/AfterAgent (task done).

**How to avoid:** The Notification hook must forward the permission prompt text to the orchestrator via a different mechanism — a direct `tmux send-keys` to the orchestrator session with the message from stdin, or a separate `squad-station notify` subcommand if one is introduced. Research the design doc: Notification hook purpose is "Agent needs approval → forward to Orchestrator". This requires reading the `message` from stdin JSON and sending it to the orchestrator's tmux session.

**Decision point for planner:** Two implementation options:
1. **Thin hook, no new Rust subcommand:** Hook reads stdin JSON with `jq` (or `python3 -c`), extracts `message`, looks up orchestrator tmux session via `squad-station agents --json | jq`, sends via `tmux send-keys`. Fully in bash.
2. **New `notify` subcommand:** Hook calls `squad-station notify "$AGENT_NAME"` which reads stdin, finds orchestrator, forwards message. Matches existing pattern of delegating to Rust binary.

The existing hooks delegate ALL logic to Rust. Option 2 is more consistent. **However**: introducing a new Rust subcommand adds scope. Option 1 (jq-based bash) is feasible for a notification-only use case. The planner must decide.

**Warning signs:** If the hook calls `signal` on Notification events, agents will show as completed while still waiting for permission.

### Pitfall 2: `--body` Flag Breaks Existing Tests

**What goes wrong:** `tests/test_cli.rs` has `test_cli_send_missing_args_fails` and `test_cli_send_priority_flag_accepts_valid_values` that use positional `send agent task`. After CLI-01, these tests break.

**Why it happens:** Tests were written for positional syntax.

**How to avoid:** Update `test_cli.rs` tests in the same plan as the cli.rs change. Specifically:
- `test_cli_send_missing_args_fails`: still valid (send with no args fails), no change needed
- `test_cli_send_priority_flag_accepts_valid_values`: change `["send", "agent", "task", "--priority", priority]` → `["send", "agent", "--body", "task", "--priority", priority]`

### Pitfall 3: Init Naming Changes Existing Registered Agents

**What goes wrong:** If an existing DB has agents registered with the old verbatim `name` from squad.yml (e.g., `backend`), and init now registers them as `myapp-claude-backend`, the old and new records are different names. `INSERT OR IGNORE` means the old record persists, the new name is never inserted.

**Why it happens:** `insert_agent` uses `INSERT OR IGNORE` with `name` as the unique key.

**How to avoid:** This is a fresh DB for v1.1 (REQUIREMENTS.md explicitly states "Clean migration, no legacy support needed"). Document in PLAYBOOK that users must `rm ~/.agentic-squad/<project>/station.db` to re-init with the new naming convention. No Rust code needed to handle migration.

### Pitfall 4: Context Output Still Shows Old `--body` Send Syntax

**What goes wrong:** `context.rs` currently generates send commands as `squad-station send <agent> "<task>"` (positional). After CLI-01, this is wrong syntax in the generated context file.

**Why it happens:** context.rs hardcodes the old syntax in the usage section.

**How to avoid:** CLI-03 plan must also update the send command examples in context output from positional to `--body` flag syntax.

### Pitfall 5: signal.rs Task ID May Be "unknown"

**What goes wrong:** The signal format `"<agent> completed <msg-id>"` assumes `task_id` is always set. In signal.rs, `task_id_str` defaults to `"unknown"` if the query returns no ID.

**Why it happens:** Race condition or duplicate signal — `rows > 0` but the subsequent SELECT returns nothing.

**How to avoid:** The format change is one line. The `unknown` fallback remains appropriate. The orchestrator design doc shows this as a real task ID, so the fallback is acceptable for edge cases. No additional handling needed.

---

## Code Examples

Verified patterns from existing codebase and official documentation:

### Hook Script Structure (follows existing hooks/claude-code.sh pattern)
```bash
#!/bin/bash
# hooks/claude-code-notify.sh -- Forward permission prompt to orchestrator
# Registered under Notification event in .claude/settings.json
# matcher: "permission_prompt"

# Drain stdin (avoid broken pipe)
NOTIFICATION=$(cat)

# Guard: not in tmux
if [ -z "$TMUX_PANE" ]; then
    exit 0
fi

# Get agent name from current tmux session
AGENT_NAME=$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -n1)
if [ -z "$AGENT_NAME" ]; then
    exit 0
fi

SQUAD_BIN="${SQUAD_STATION_BIN:-squad-station}"
if ! command -v "$SQUAD_BIN" > /dev/null 2>&1; then
    echo "squad-station: warning: binary not found at '$SQUAD_BIN'" >&2
    exit 0
fi

# Extract message from notification JSON (requires jq or python3)
MESSAGE=$(echo "$NOTIFICATION" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('message',''))" 2>/dev/null)
if [ -z "$MESSAGE" ]; then
    exit 0
fi

# Forward to orchestrator (implementation TBD by planner)
# Option A: new `notify` subcommand, Option B: direct tmux in bash
exit 0
```

### CLI-01: clap flag rename (cli.rs)
```rust
Send {
    agent: String,
    #[arg(long)]
    body: String,
    #[arg(long, value_enum, default_value = "normal")]
    priority: Priority,
},
```

### CLI-02: Auto-prefix logic (init.rs)
```rust
// For workers
let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
let agent_name = format!("{}-{}-{}", config.project, agent.tool, role_suffix);

// For orchestrator
let orch_name = config.orchestrator.name
    .as_deref()
    .map(|n| format!("{}-{}-{}", config.project, config.orchestrator.tool, n))
    .unwrap_or_else(|| format!("{}-{}-orchestrator", config.project, config.orchestrator.tool));
```

### SIG-01: Signal format (signal.rs)
```rust
// Change line 78 from:
let notification = format!(
    "[SIGNAL] agent={} status=completed task_id={}",
    agent, task_id_str
);
// To:
let notification = format!("{} completed {}", agent, task_id_str);
```

### Claude Code settings.json Notification hook registration
```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [{ "type": "command", "command": "/path/to/hooks/claude-code.sh" }]
      }
    ],
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [{ "type": "command", "command": "/path/to/hooks/claude-code-notify.sh" }]
      }
    ]
  }
}
```

### Gemini CLI settings.json Notification hook registration
```json
{
  "hooks": {
    "AfterAgent": [
      {
        "hooks": [{ "type": "command", "command": "/path/to/hooks/gemini-cli.sh" }]
      }
    ],
    "Notification": [
      {
        "hooks": [{ "type": "command", "command": "/path/to/hooks/gemini-cli-notify.sh" }]
      }
    ]
  }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `provider` field in agents | `tool` field | Phase 4 (done) | Hook scripts should reference `tool`, not `provider` |
| Positional `task` arg | `--body` flag | Phase 5 (this phase) | Breaking change to CLI |
| Free-form agent name | `<project>-<tool>-<role>` prefix | Phase 5 (this phase) | Existing DBs need re-init |
| `[SIGNAL] agent=X...` format | `"<agent> completed <id>"` | Phase 5 (this phase) | Orchestrator parsing must match |

**Deprecated/outdated:**
- `hooks/claude-code.sh` registered as Stop hook: still valid, not changed in Phase 5
- Positional `send <agent> "<task>"` syntax: deprecated, replaced by `send <agent> --body "<task>"`

---

## Open Questions

1. **Notification hook forward mechanism (HOOK-01/02)**
   - What we know: Hook fires when agent needs permission. The orchestrator must be notified.
   - What's unclear: Whether to introduce a new `notify` Rust subcommand or implement orchestrator lookup in bash using `squad-station agents --json | jq`.
   - Recommendation: The planner should choose. Both are viable. A new `notify` subcommand is more consistent with the existing pattern (guards in Rust, not bash). However it adds a new CLI entry point. Pure bash with `jq` keeps hooks/ self-contained. Given the project's thin-hook philosophy, a new subcommand is the more principled choice.

2. **`jq` dependency for notification hook**
   - What we know: Reading stdin JSON in bash requires `jq` or `python3`. Neither is guaranteed on user systems.
   - What's unclear: Whether to require `jq`, use `python3`, or a new Rust subcommand reading stdin.
   - Recommendation: Use `python3 -c` as the fallback parser (more universally available than `jq`). Alternatively, add a `notify` subcommand to squad-station that reads the message from stdin or a `--message` flag, avoiding the bash JSON parsing dependency entirely.

3. **Notification hook behavior when no orchestrator is registered**
   - What we know: signal.rs already handles `get_orchestrator` returning None silently.
   - What's unclear: Should the notification hook be a no-op when no orchestrator is in DB?
   - Recommendation: Yes — same guard as signal.rs. If no orchestrator found, exit 0 silently.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust test (cargo test) + async tokio |
| Config file | Cargo.toml (test configuration inline) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test && cargo test --test test_cli` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HOOK-01 | `claude-code-notify.sh` exists and exits 0 | smoke (shell) | `bash -n hooks/claude-code-notify.sh` | ❌ Wave 0 |
| HOOK-02 | `gemini-cli-notify.sh` exists and exits 0 | smoke (shell) | `bash -n hooks/gemini-cli-notify.sh` | ❌ Wave 0 |
| CLI-01 | `send agent --body "task"` accepted; positional rejected | unit (clap parse) | `cargo test --test test_cli test_cli_send_body_flag` | ❌ Wave 0 |
| CLI-02 | `init` registers `myapp-claude-backend` from name=backend | integration | `cargo test --test test_commands test_init_agent_name_prefix` | ❌ Wave 0 |
| CLI-03 | `context` output includes model + description | integration | `cargo test --test test_commands test_context_includes_model` | ❌ Wave 0 |
| SIG-01 | Signal notification format is `"<agent> completed <id>"` | unit | `cargo test --test test_commands test_signal_notification_format` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test && ./tests/e2e_cli.sh`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/test_cli.rs` — add `test_cli_send_body_flag_accepted` and `test_cli_send_positional_rejected`; update `test_cli_send_priority_flag_accepts_valid_values` to use `--body`
- [ ] `tests/test_commands.rs` or new `tests/test_init.rs` — `test_init_agent_name_prefix` covering CLI-02
- [ ] `tests/test_commands.rs` — `test_context_includes_model_and_description` covering CLI-03
- [ ] `tests/test_signal.rs` or `tests/test_commands.rs` — `test_signal_notification_format` covering SIG-01
- [ ] Shell syntax check for new hook scripts: `bash -n hooks/claude-code-notify.sh && bash -n hooks/gemini-cli-notify.sh`

*(No framework installation needed — cargo test already runs.)*

---

## Sources

### Primary (HIGH confidence)
- Existing codebase: `src/cli.rs`, `src/commands/send.rs`, `src/commands/signal.rs`, `src/commands/init.rs`, `src/commands/context.rs` — full read, current state documented above
- Existing codebase: `hooks/claude-code.sh`, `hooks/gemini-cli.sh` — structural model for new hooks
- `docs/SOLUTION-DESIGN.md` — authoritative design reference for naming convention, signal format, context output format, CLI syntax
- `docs/GAP-ANALYSIS.md` — gap identification for all 6 requirements
- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks) — Notification event, stdin JSON format, exit codes
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/) — Notification event, stdin JSON format, exit codes

### Secondary (MEDIUM confidence)
- WebSearch results corroborating `notification_type: "permission_prompt"` JSON field for Claude Code Notification hook
- WebSearch results corroborating `notification_type: "ToolPermission"` for Gemini CLI Notification event
- Community examples of Notification hook registration in `settings.json`

### Tertiary (LOW confidence)
- Specific `matcher: "permission_prompt"` syntax in Claude Code settings.json — verified via multiple community examples but official docs page too large to fully parse

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, all existing tools
- Architecture: HIGH — codebase read directly, changes are minimal and surgical
- Hook event names/JSON: MEDIUM — official docs confirmed via WebSearch + community examples; exact field names for edge cases may vary by provider version
- Pitfalls: HIGH — derived from direct code reading and known clap behavior

**Research date:** 2026-03-08
**Valid until:** 2026-04-08 (stable: clap API and provider hook events change slowly)
