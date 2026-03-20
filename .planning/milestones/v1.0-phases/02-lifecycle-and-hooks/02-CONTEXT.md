# Phase 2: Lifecycle and Hooks - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Agent status is always accurate (reconciled against live tmux state), hook scripts handle both Claude Code (Stop event) and Gemini CLI (AfterAgent event), orchestrator never triggers an infinite loop, and a context file generator provides orchestrators with agent roster and usage commands.

Requirements: SESS-03, SESS-04, SESS-05, HOOK-01, HOOK-02, HOOK-03

</domain>

<decisions>
## Implementation Decisions

### Hook delivery method
- Bundled shell scripts that wrap `squad-station signal`
- Users reference these scripts from their provider hook config (Claude Code hooks.json, Gemini CLI settings)
- Scripts contain the 4-layer guard logic (HOOK-03): not-in-tmux check, agent-registered check, orchestrator-skip check, then signal call

### Hook edge case behavior
- Unregistered agent: silent exit 0 (agent might be outside the squad, not an error)
- Not in tmux: silent exit 0 (can't be a managed agent)
- Real errors (binary not found, DB connection failure): stderr warning + exit 0 (debuggable but never fails the provider)
- Orchestrator self-signal (HOOK-01): silently exit 0 to prevent infinite loop

### Context file design
- Output format: Markdown — structured for pasting into AI orchestrator prompts
- Output destination: stdout only (user pipes/redirects as needed)
- Content: agent roster (name, role, current status, per-agent usage commands) plus a general squad-station usage guide
- Self-contained: orchestrator should understand the full system from this file alone

### Status reconciliation
- Agent status includes duration (e.g., "idle 5m", "busy 2m", "dead since 10:30") to help identify stuck agents
- Dead agents auto-revive to idle when their tmux session reappears on next reconciliation

### Claude's Discretion
- Hook script location (repo root hooks/ dir vs generated at init time)
- Single universal hook script vs separate per-provider scripts — based on how different the provider interfaces actually are
- Guard logic placement: shell script vs Rust binary — based on testability and reliability
- Orchestrator detection method: tmux session name check vs DB role lookup
- Status reconciliation timing: on every read command (eager) vs on agents command only (lazy)
- Agent status DB model: dedicated status column vs derived from messages + tmux state

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tmux::session_exists()`: Already checks if a tmux session is alive — directly usable for SESS-04 reconciliation
- `db::agents::list_agents()`: Returns all agents — feeds both `agents` command and `context` command
- `db::agents::get_orchestrator()`: Finds orchestrator by role — usable for HOOK-01 orchestrator detection
- `db::messages::update_status()`: Idempotent status update — signal's existing flow to build guards around

### Established Patterns
- Stateless CLI: every command connects to DB, does work, exits — reconciliation must fit this model
- Single-writer pool (`max_connections(1)`): status updates must be aware of this constraint
- Terminal-aware output: `std::io::IsTerminal` for colored vs plain output — context command should follow same pattern
- JSON output flag: `--json` global flag — new commands should support this

### Integration Points
- `signal.rs`: Needs guard logic added (orchestrator skip, not-in-tmux, unregistered agent) before existing signal flow
- `cli.rs`: New `Agents` and `Context` subcommands to add
- `db/migrations/`: New migration needed if adding status column to agents table
- `db/agents.rs`: Status-related queries (update status, get with status) to add
- Hook scripts: External to Rust binary, reference `squad-station signal` CLI

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-lifecycle-and-hooks*
*Context gathered: 2026-03-06*
