# Phase 23: Dynamic Agent Cloning - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

New `squad-station clone <agent-name>` CLI command that duplicates an existing agent (same role/model/description) with an auto-incremented name, registers it in DB, launches a tmux session, and auto-regenerates `squad-orchestrator.md`. Orchestrator cloning is rejected. No changes to squad.yml — clones are runtime-only (same as `register` behavior).

</domain>

<decisions>
## Implementation Decisions

### Name auto-increment
- Append `-N` suffix: first clone gets `-2`, next gets `-3`, etc.
- No gap-filling — always increment from highest existing N (monotonically increasing)
- Check uniqueness against both DB agents table AND live tmux sessions (handles orphaned sessions)
- If source agent name already ends with `-N` (it's itself a clone), strip the suffix and use the original base name — all clones are siblings, not nested (e.g., cloning `worker-3` produces `worker-4`, not `worker-3-2`)

### Failure rollback
- DB-first ordering: insert agent record, then launch tmux session
- If tmux launch fails, DELETE the agent record from DB entirely (not mark as dead) — no orphaned records
- Source agent must exist in DB; live tmux session NOT required — source is a config template, not a running dependency
- DB-only agents (tool='antigravity') can be cloned — clone inherits tool, no tmux session launched

### CLI interface
- Concise one-liner output: `Cloned myproject-cc-worker → myproject-cc-worker-2`
- Second line for context regen: `Regenerated squad-orchestrator.md`
- Orchestrator rejection: `Error: cannot clone orchestrator agent` + exit code 1 (CLONE-04)
- Support `--json` flag: output `{"cloned": true, "source": "...", "name": "..."}`
- Source agent specified by exact name only — no partial/fuzzy matching (consistent with send, peek)

### Context regeneration
- Call `context::run()` directly after successful clone — reuses existing logic, no duplication
- Show "Regenerated squad-orchestrator.md" confirmation in human output
- Context regen is best-effort: if it fails, warn but don't fail the clone itself — user can run `squad-station context` manually
- In `--json` mode, include `context_regenerated: true/false` in output

### Claude's Discretion
- New `delete_agent()` DB function design (or inline DELETE query)
- Exact regex/logic for stripping `-N` suffix from source name to find base
- How to derive launch command for clone (read from DB agent's `tool` + `model` fields, reuse `get_launch_command` pattern from init.rs)
- Error message wording for "source agent not found" case

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — CLONE-01 through CLONE-06 define the six cloning requirements

### Existing implementation (clone reuses these patterns)
- `src/commands/register.rs` — DB-only agent registration pattern (INSERT OR IGNORE)
- `src/commands/init.rs` lines 239-274 — Full init pattern: `insert_agent()` → `session_exists()` → `launch_agent_in_dir()` with `get_launch_command()`
- `src/commands/init.rs` fn `get_launch_command()` (line 599) — Builds tmux launch command from agent tool + model
- `src/commands/context.rs` fn `run()` — Context regeneration entry point (clone calls this directly)
- `src/db/agents.rs` — Agent struct, `insert_agent()`, `get_agent()`, `list_agents()`
- `src/tmux.rs` — `launch_agent_in_dir()`, `session_exists()`, `list_live_session_names()`
- `src/cli.rs` — `Commands` enum where `Clone` subcommand must be added
- `src/config.rs` — `sanitize_session_name()`, `resolve_db_path()`, `find_project_root()`

### Prior phase context
- `.planning/phases/22-orchestrator-intelligence-data/22-CONTEXT.md` — `build_orchestrator_md()` purity constraint (INTEL-05); clone must not break this when calling `context::run()`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `db::agents::insert_agent(pool, name, tool, role, model, description)` — INSERT with ON CONFLICT UPDATE; directly usable for clone registration
- `db::agents::get_agent(pool, name)` — Fetch source agent to copy config from
- `tmux::launch_agent_in_dir(session_name, command, start_dir)` — Launch tmux session at project root
- `tmux::session_exists(session_name)` — Check for tmux session collision
- `tmux::list_live_session_names()` — List all tmux sessions for uniqueness scan
- `config::sanitize_session_name()` — Ensure session name is tmux-safe
- `context::run()` — Full context regeneration including metrics (Phase 22)

### Established Patterns
- DB-first then tmux launch in init.rs (lines 243-272) — clone follows same ordering
- `get_launch_command()` builds tool-specific command from agent config — clone needs to reconstruct this from DB agent fields
- Global `--json` flag on Cli struct — all commands check `json` parameter
- `register.rs` shows minimal DB-only registration pattern — clone extends this with tmux + context regen

### Integration Points
- `cli.rs::Commands` enum — add `Clone { agent: String }` variant
- `src/commands/mod.rs` — add `pub mod clone;`
- `src/main.rs` — add match arm for `Commands::Clone` dispatching to `clone::run()`
- `context::run()` — called at end of clone for CLONE-05

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

- `clone --n <count> <agent>` batch clone shorthand — tracked as CLONE-07 in REQUIREMENTS.md v2 section
- Clone count limit guardrail — tracked as CLONE-08 in REQUIREMENTS.md v2 section

</deferred>

---

*Phase: 23-dynamic-agent-cloning*
*Context gathered: 2026-03-19*
