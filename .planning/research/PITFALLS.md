# Pitfalls Research

**Domain:** Stateless CLI with SQLite + tmux — adding agent templates, orchestrator intelligence metrics, and dynamic agent cloning
**Researched:** 2026-03-19
**Confidence:** HIGH — all findings grounded in direct codebase inspection (`signal.rs`, `db/agents.rs`, `tmux.rs`, `config.rs`, `context.rs`, `ui.rs`) and the existing CONCERNS.md audit

---

## Critical Pitfalls

### Pitfall 1: Clone Name Collision — DB Sees Only DB, Not tmux Reality

**What goes wrong:**
`squad-station clone <agent>` must derive a unique name like `proj-claude-code-implement-2`. A naive implementation queries the DB for the highest existing suffix and increments by one. After a re-init with overwrite (`delete_all_agents()` clears the DB), the DB resets but orphaned tmux sessions `<name>-2`, `<name>-3` still exist. The next `clone` derives `-2`, then calls `tmux new-session -d -s proj-claude-code-implement-2`. tmux returns exit code 1 (session already exists). The clone command either errors out or silently no-ops, but the orchestrator believes a new agent was registered.

**Why it happens:**
Agent name = tmux session name is a foundational design decision. The DB and tmux state can diverge after a re-init. The DB reflects declared intent; tmux reflects live reality. The auto-increment logic only has visibility into the DB.

**How to avoid:**
The clone command must check both `db::agents::get_agent(pool, candidate_name)` AND `tmux::session_exists(candidate_name)` before committing to a derived name. Keep incrementing until a name is free in both. Explicitly document: `squad-station clone` does not kill existing sessions; use `squad-station close <clone>` to fully remove a clone.

**Warning signs:**
- `tmux new-session` returns non-zero during a clone attempt
- `squad-station agents` shows clones as `dead` (DB says they exist, tmux sessions are gone)
- Re-init with overwrite followed by clone produces "session already exists" errors

**Phase to address:**
Phase implementing the `clone` command. Double-check logic must be specified in the implementation plan before the name resolution loop is written.

---

### Pitfall 2: Clone Partially Succeeds — tmux Session Created But DB Registration Failed

**What goes wrong:**
The clone command has two sequential steps: (1) register the agent in the DB, (2) launch a new tmux session. If the implementation reverses the order — launch tmux first, then write to DB — and the DB write fails (busy_timeout under write contention, or a bug), the tmux session is running with no DB record. The stop hook fires `squad-station signal <clone-name>`, hits GUARD 3 (`get_agent` returns `None`), silently exits. The orchestrator is never notified of the clone's task completion. The clone works in tmux but is invisible to the squad.

Alternatively: if DB registration succeeds but `tmux new-session` fails, the DB contains a ghost record that the TUI shows as `idle` but the session does not exist. This will flip to `dead` on next reconciliation, but the user sees a phantom agent in the list.

**Why it happens:**
tmux and SQLite are separate systems with no shared transaction boundary. Developers write the "happy path" sequentially without considering partial failure. The existing pattern in `register.rs` does DB-only registration with no tmux involvement, so there is no established precedent for the two-step pattern.

**How to avoid:**
Always register in DB first, launch tmux second. If the DB write fails, return an error before touching tmux. If the tmux launch fails after a successful DB write, immediately call `db::agents::delete_agent_by_name(pool, &clone_name)` (a compensating transaction). Log the compensating action to stderr. This is the closest thing to atomicity achievable without a shared transaction.

**Warning signs:**
- `squad-station agents` shows a clone as `idle` but `tmux ls` does not show its session
- Clone completes a task but no `[SQUAD SIGNAL]` notification reaches the orchestrator
- The DB contains agents with suffix names that do not correspond to live tmux sessions

**Phase to address:**
Phase implementing the `clone` command. The DB-first ordering and compensating rollback must be explicit requirements in the implementation plan.

---

### Pitfall 3: Clone Does Not Update squad-orchestrator.md — Orchestrator Never Learns About the Clone

**What goes wrong:**
After `squad-station clone <agent>` successfully registers the clone and launches its session, the orchestrator's routing instructions in `squad-orchestrator.md` still only list the original agents. The orchestrator never routes tasks to the clone. The clone idles. Workload is not distributed. From the user's perspective, cloning did nothing useful.

**Why it happens:**
`context` is a separate, read-only command that regenerates `squad-orchestrator.md` from the current DB state. It is not called automatically after `clone`. The orchestrator loads the playbook once at session start via `/squad-orchestrator`. Without a re-invocation, the orchestrator has no mechanism to detect new agents.

**How to avoid:**
The `clone` command must call the `context` regeneration logic internally as its final step — equivalent to running `squad-station context` after the agent is registered. The `build_orchestrator_md` function is already exported from `context.rs` as a `pub fn`; call it directly rather than shelling out to the binary. Additionally, the clone command output must instruct the user: "Orchestrator playbook updated at `.claude/commands/squad-orchestrator.md`. Reload `/squad-orchestrator` in your orchestrator session."

**Warning signs:**
- `squad-orchestrator.md` contains only original agents after a clone
- TUI shows the clone as `idle` but the orchestrator never sends it tasks
- No call to `build_orchestrator_md` or equivalent in the clone command's implementation

**Phase to address:**
Phase implementing the `clone` command. The auto-context-regeneration must be a stated requirement in the plan, not an afterthought.

---

### Pitfall 4: Metrics Data Is Stale the Moment squad-orchestrator.md Is Written

**What goes wrong:**
The orchestrator intelligence feature computes task-role alignment scores, busy time, and messages-per-agent counts and embeds them as a static table in `squad-orchestrator.md`. By the time the orchestrator reads this file — potentially minutes or many tasks later — the data is stale. The orchestrator makes routing decisions ("agent A is overloaded, send to agent B") based on past state, causing misrouting in the opposite direction (avoiding an agent that has since become idle).

**Why it happens:**
`squad-station context` is a stateless snapshot command by design (the decision to make it read-only was explicit in v1.0). There is no push mechanism, no daemon, no file watcher. The orchestrator loads the playbook once. Static metrics in a once-loaded document are definitionally stale.

**How to avoid:**
Do not embed pre-computed metric values in `squad-orchestrator.md`. Instead, embed **CLI commands** the orchestrator must run to get live data:

```
## Workload Check (run before routing a task)
\`\`\`bash
squad-station status --json
squad-station agents
\`\`\`
```

If a static snapshot is valuable (e.g., at session initialization), include it with a clearly labeled timestamp and an advisory TTL:

```
<!-- Snapshot generated: 2026-03-19T10:00:00Z — refresh by running `squad-station context` -->
```

The orchestrator is an AI with tool access. Give it commands to run, not data to memorize.

**Warning signs:**
- `squad-orchestrator.md` contains a table of metrics with no timestamp
- `squad-orchestrator.md` contains no CLI command to re-query current workload
- Orchestrator systematically avoids idle agents because the stale metrics label them busy

**Phase to address:**
Phase implementing orchestrator intelligence data. This is a UX design decision — the generated playbook text must be drafted carefully before any metric computation code is written.

---

### Pitfall 5: busy_time Metric Resets on Re-Init — Looks Like Zero for All Agents

**What goes wrong:**
Computing "busy time" uses `status_updated_at` timestamps from the `agents` table. When a user runs re-init with overwrite, `delete_all_agents()` clears the table and `insert_agent()` re-inserts all agents with fresh `status_updated_at = now`. All agents show 0 busy time immediately after re-init, even if they have been running for hours. The metrics report an agent fleet that looks freshly started, regardless of actual runtime.

More subtly: `insert_agent` uses `ON CONFLICT(name) DO UPDATE SET tool = excluded.tool, role = excluded.role, model = excluded.model, description = excluded.description`. It does NOT reset `status_updated_at`. But the overwrite re-init path calls `delete_all_agents()` followed by fresh inserts — so `status_updated_at` is reset to the re-init timestamp.

**Why it happens:**
`status_updated_at` was designed to track the most recent status transition, not total accumulated busy time. Using it as a busy-time proxy works for "how long has this agent been in its current state" but is unreliable across re-inits and status updates triggered by duplicate signals.

**How to avoid:**
Define busy_time explicitly as "elapsed time since `status_updated_at` if current status is `busy`." Document clearly in the generated playbook that this metric represents "time in current state since last transition," not "total busy time since deployment." Do NOT attempt to compute historical busy time from existing schema — it is not available. For v1.8, this simple metric is sufficient and honest. Defer trend analytics (total busy time over 24h) to a future milestone that would require an `agent_events` table.

**Warning signs:**
- All agents show 0 busy time immediately after a re-init
- An agent that has been idle for 3 hours shows 0 busy time after a status update from a duplicate signal
- Metrics claim an agent is "newly idle" when it has been idle since deployment

**Phase to address:**
Phase implementing orchestrator intelligence data. The metric definition and its limitations must be documented in the generated playbook to prevent orchestrator misinterpretation.

---

### Pitfall 6: Agent Template Suggests Model Not in Validation Allowlist

**What goes wrong:**
Agent role templates embed suggested model identifiers (e.g., for an "Architect" role, suggest `claude-opus`). When the wizard pre-fills the model field from the template and the user proceeds, the model string flows through `generate_squad_yml` into the YAML and then through `config::validate()` at init time. If the template embeds a model string not in `VALID_PROVIDERS` / `valid_models_for()` — for example `"claude-opus-4"` instead of the valid alias `"opus"` — validation rejects it. The user selected a template that Squad Station itself offered and immediately gets a cryptic validation error.

This is especially insidious because the templates are compiled into the binary. A mismatch between template model strings and the validation allowlist cannot be caught at runtime — only by tests.

**Why it happens:**
The model validation allowlist in `src/config.rs` is maintained separately from the template definitions in the wizard. If a new model alias is added to the allowlist (e.g., `"claude-sonnet-4-7"`) but existing templates still reference an old format, or vice versa, the two diverge silently until a user hits the validation error.

**How to avoid:**
Templates must reference model aliases drawn from the same `valid_models_for()` function used in `validate_agent_config`. The simplest approach: templates do not specify a model at all — they set role and description, and leave the model field empty for the user to choose from the radio selector (which is already bound to the allowlist). If templates do suggest models, embed a test that calls `validate_agent_config` on each template's generated agent config and asserts it passes. This test runs in CI and catches drift before release.

**Warning signs:**
- No test validates template-generated configs against `validate_agent_config`
- Template embeds a string literal for a model (e.g., `"claude-opus"`) rather than referencing the allowlist constant
- A template passes wizard smoke tests but fails when `init` writes squad.yml and re-validates

**Phase to address:**
Phase implementing agent role templates in wizard. Template validation against the allowlist must be a test requirement, not an afterthought.

---

### Pitfall 7: Cloning the Orchestrator Creates a Routing Loop

**What goes wrong:**
If the `clone` command does not explicitly reject orchestrator agents, a user (or a confused orchestrator) can run `squad-station clone <orchestrator-name>`. The clone is registered with `role = "orchestrator"`. Now there are two orchestrators in the DB. GUARD 4 in `signal.rs` checks `agent_record.role == "orchestrator"` and silently exits for both. Neither orchestrator receives task completion signals. More dangerously: `get_orchestrator` in `db/agents.rs` returns the most recent non-dead orchestrator. The original orchestrator may stop receiving signals if the cloned orchestrator appears "more recent." The routing chain silently breaks.

**Why it happens:**
The `clone` command will copy the source agent's `role` field from the DB. The DB does not enforce any uniqueness constraint on `role = "orchestrator"`. There is no guard at the DB level or in the signal chain to detect duplicate orchestrators at routing time.

**How to avoid:**
The `clone` command must explicitly reject agents with `role == "orchestrator"` with a clear error message: "Cannot clone the orchestrator agent. Only worker agents can be cloned." This is a one-line guard at the top of the clone handler, before any DB writes.

**Warning signs:**
- `squad-station agents` shows two agents with `role = "orchestrator"`
- `squad-station signal <worker>` succeeds (rows > 0) but no notification reaches the orchestrator
- `get_orchestrator` query is returning the wrong agent (cloned orchestrator shadows the original)

**Phase to address:**
Phase implementing the `clone` command. The orchestrator rejection guard must be the first guard in the clone handler.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Auto-increment clone names by scanning DB only | Simple implementation | Diverges from tmux reality after re-init; causes collision bugs | Never — always check tmux too |
| Launch tmux session before DB registration in clone | Fewer rollback cases | Orphaned sessions if DB write fails; signal chain breaks silently | Never — DB first, always |
| Skip context regeneration after clone | Simpler clone command | Orchestrator never learns about new clones; cloning has no effect on routing | Never — regeneration is mandatory |
| Embed static metric tables in squad-orchestrator.md | Easier to read at a glance | Stale by the time the orchestrator reads it; causes misrouting | Never — embed CLI commands, not values |
| Templates hard-code model strings as string literals | Fast to write | Drift from validation allowlist; user sees validation errors on their own selections | Never — reference the allowlist or omit the model |
| Use status_updated_at as proxy for total busy time | No new schema needed | Resets on re-init; not historical; misleads on long-running squads | Acceptable for v1.8 with documented caveats in the playbook |
| Allow cloning any agent including orchestrators | No special-case code | Two orchestrators break the signal routing chain silently | Never — orchestrator clone must be explicitly rejected |

---

## Integration Gotchas

Common mistakes when connecting new features to the existing system.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| clone command → signal chain | Launch tmux session before DB registration | Register in DB first; roll back DB record if tmux launch fails |
| clone command → orchestrator context | Assume orchestrator will reload `/squad-orchestrator` manually | Auto-regenerate squad-orchestrator.md inside the clone command as the last step |
| metrics → context command | Compute metric values and embed as static table | Embed CLI commands for live re-query; timestamp any static snapshot |
| templates → config validation | Template suggests model not in `valid_models_for()` allowlist | Templates omit model field, or reference the allowlist constant; CI test validates |
| clone → name resolution | Check DB only for name collision | Check both `get_agent(pool, candidate)` and `tmux::session_exists(candidate)` |
| clone → orchestrator role | Allow cloning any agent | Reject `role == "orchestrator"` with clear error before any DB writes |
| clone → TUI live update | TUI must poll at a different cadence for new agents | TUI already polls DB every 3s via connect-per-refresh; clone registers in DB; TUI picks it up automatically within one cycle |
| busy_time metric → re-init | Assume status_updated_at persists across re-inits | Document the reset behavior; define metric as "time in current state" not "total runtime" |

---

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `tmux has-session` called once per agent during reconciliation (pre-existing issue) | `agents`, `status`, `context` commands slow with many agents | Use `list_live_session_names()` once + HashSet lookup (flagged in CONCERNS.md) | 10+ agents |
| Context regeneration called once per clone in a rapid clone loop | Slow if user clones 5 agents sequentially | Each clone triggers one `build_orchestrator_md` call; cost is proportional to agent count (already O(N)) | Not a concern at v1.8 agent counts (<10) |
| Metrics query runs a full table scan per `context` call | `context` slow for large agent fleets | Metrics are derived from existing `list_agents` result — no extra query needed if implementation shares the data | Not a trap if implementation reuses the existing query |
| Clone creates many agents that all signal orchestrator simultaneously | Rapid-fire `send_keys_literal` calls overlap in the orchestrator session | Each `send_keys_literal` call includes a 2s sleep (built into `tmux.rs`); naturally serialized by single-writer pool | 5+ agents completing simultaneously |

---

## Security Mistakes

Domain-specific security issues.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Clone name derived from user input without sanitization | Spaces or tmux-unsafe chars in the derived name break session targeting | Apply `config::sanitize_session_name()` to any derived clone name before passing to `tmux::launch_agent` |
| Template description field contains markdown or backticks | Markdown injection could confuse orchestrator's parsing of squad-orchestrator.md | Template descriptions are plain text only; no markdown formatting, no backtick blocks |
| Clone command allows cloning the orchestrator | Two orchestrators break signal routing silently | Reject `role == "orchestrator"` at the start of the clone handler |
| Metrics expose internal agent state to squad-orchestrator.md | Low risk (same file already contains agent names/descriptions) | No new exposure beyond what the existing `context` command generates |

---

## UX Pitfalls

Common user experience mistakes in this domain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Template list has no guidance on when to use each role | User picks "Architect" for a 2-agent squad that doesn't need one | Each template includes a one-line "Use when:" hint in the wizard list item |
| Clone succeeds but orchestrator routing is unchanged | User expects load balancing; nothing changes | Print explicit confirmation: "Clone registered. Playbook updated. Reload `/squad-orchestrator` in your orchestrator session." |
| Metrics in playbook presented as routing rules, not hints | Orchestrator over-trusts stale data; refuses to route to an idle agent | Frame as "run this command to check current load" not "this agent is busy" |
| TUI shows clone agents indistinguishable from originals | User cannot identify which agents are clones | Derive clone status from naming convention (suffix `-2`, `-3`); or add a "clone of X" note in the role/description field |
| No confirmation prompt before creating a clone | User accidentally clones the wrong agent | Print what will be cloned and prompt for confirmation, or at minimum print "Created clone: <name>" with rollback instructions |

---

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Clone command — name resolution:** Does the implementation check `tmux::session_exists(candidate)` in addition to querying the DB? Verify with a test where a tmux session exists but no DB record for that name.
- [ ] **Clone command — DB-first ordering:** Is the DB `insert_agent` call made BEFORE `tmux::launch_agent`? Verify by code review of the implementation order.
- [ ] **Clone command — rollback:** If `tmux::launch_agent` fails after DB registration, is the DB record removed? Verify with a unit test that simulates tmux failure.
- [ ] **Clone command — orchestrator rejection:** Does `clone <orchestrator-name>` return a non-zero exit and clear error message? Verify with an integration test.
- [ ] **Clone command — context regeneration:** Is `squad-orchestrator.md` updated after a successful clone? Verify by reading the file after a clone and confirming the clone's name appears.
- [ ] **Agent templates — allowlist compliance:** Does every template's suggested model pass `validate_agent_config()`? Verify with a CI test that runs each template through config validation.
- [ ] **Orchestrator intelligence — no static values:** Does the generated `squad-orchestrator.md` contain CLI commands for live queries, not just a static metric table? Read the generated file and confirm.
- [ ] **Orchestrator intelligence — timestamp:** If any static metric snapshot is included, does it have a generated-at timestamp? Verify by reading the generated file.
- [ ] **TUI live update:** After a clone, does the TUI show the new agent within one refresh cycle (3 seconds)? The TUI polls DB; the clone registers in DB; this should work automatically. Verify by inspection.
- [ ] **busy_time caveats:** Does the generated playbook include a note explaining that busy_time resets on re-init and represents "time in current state," not "total runtime"? Verify by reading the generated playbook text.
- [ ] **Signal roundtrip for clone:** After a clone completes a task, does the orchestrator receive a `[SQUAD SIGNAL]`? Full roundtrip: DB registration → hook fires → `signal` finds DB record → `get_orchestrator` succeeds → `send_keys_literal` to orchestrator.

---

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Clone name collision with orphaned tmux session | LOW | `tmux kill-session -t <name>` then re-run `squad-station clone` |
| Ghost DB record (clone in DB, no tmux session) | LOW | Runs `squad-station agents` — reconciliation marks it dead; optionally run `squad-station clean` or `reset` to purge dead agents |
| Orchestrator has stale playbook (no clone in routing) | LOW | Run `squad-station context` manually; orchestrator reloads `/squad-orchestrator` |
| Two orchestrators in DB (orchestrator was cloned) | HIGH | Kill clone session (`tmux kill-session -t <clone>`); no CLI command to delete a single agent record — requires direct SQLite or a future `squad-station remove` command; run `squad-station context` to rebuild playbook |
| Template validation error at init time | LOW | Edit the template's model field in squad.yml manually to a valid alias; re-run `init` without `--tui` to re-validate |
| Metrics mislead orchestrator (stale data misrouting) | MEDIUM | Run `squad-station status` to get current state; manually inform orchestrator via a direct send; adjust playbook text to instruct live re-query |

---

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Clone name collision (DB vs. tmux) | Phase: `clone` command | Integration test: create orphaned tmux session, run `clone`, verify name skipped |
| Partial clone success (tmux without DB, or DB without tmux) | Phase: `clone` command | Unit test: simulate tmux failure after DB write; verify DB record removed |
| Missing context regeneration after clone | Phase: `clone` command | Integration test: run `clone`, read `squad-orchestrator.md`, verify clone name present |
| Cloning the orchestrator | Phase: `clone` command | Unit test: `clone <orchestrator-name>` returns error, no DB write |
| Stale metrics in orchestrator playbook | Phase: orchestrator intelligence data | Manual review: generated file contains CLI commands for live re-query, not static values |
| busy_time misleads after re-init | Phase: orchestrator intelligence data | Generated playbook includes caveats; no test needed |
| Template model drift from validation allowlist | Phase: agent role templates | CI test: each template's agent config passes `validate_agent_config()` |
| Clone agent invisible to TUI | Phase: TUI live update | Integration test: register clone in DB, wait one TUI refresh cycle, verify agent appears |
| Signal lost from clone (unregistered agent) | Phase: `clone` command | E2E or integration test: full roundtrip from clone registration to signal notification |

---

## Sources

- `src/commands/signal.rs` — GUARD 3 (missing agent = silent exit) and GUARD 4 (orchestrator self-signal) document the exact silent-failure modes for unregistered agents and duplicate orchestrators
- `src/db/agents.rs` — `insert_agent` upsert semantics (ON CONFLICT DO UPDATE); `delete_all_agents` for re-init; `status_updated_at` behavior; `get_orchestrator` role-based lookup (shows dual-orchestrator risk)
- `src/tmux.rs` — `launch_agent` ordering; `session_exists` availability; `list_live_session_names` for bulk session detection
- `src/config.rs` — `VALID_PROVIDERS`, `valid_models_for()`, `sanitize_session_name()` — defines validation and naming constraints templates must conform to
- `src/commands/context.rs` — `build_orchestrator_md` is a `pub fn` (callable from clone command); stateless snapshot design (no auto-refresh mechanism)
- `src/commands/ui.rs` — connect-per-refresh pattern (3s interval, DB-only); TUI auto-picks up new DB records without special handling
- `.planning/codebase/CONCERNS.md` — Pre-existing audit: reconciliation loop duplication, tmux/DB sync risks, single-writer pool limits, `status_updated_at` clock fragility, agent-name-as-FK risk
- `.planning/PROJECT.md` — "agent name = tmux session name" key decision; stateless CLI constraint; `context` is read-only (reconciliation removed to reduce side effects); `delete_all_agents` on overwrite re-init

---

*Pitfalls research for: Squad Station v1.8 — agent templates, orchestrator intelligence metrics, dynamic agent cloning*
*Researched: 2026-03-19*
