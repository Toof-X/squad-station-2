# Phase 1: Core Foundation - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Stateless CLI binary with DB schema, agent registration from squad.yml, send/signal messaging via tmux, message queries, and all safety primitives (WAL, literal send-keys, shell readiness, SIGPIPE). Users can init a squad, send tasks, receive completion signals, and query messages. Hook scripts, agent lifecycle detection, and views are separate phases.

</domain>

<decisions>
## Implementation Decisions

### squad.yml Config Format
- Structured format: each agent has name, provider (label only), role, and explicit `command` field for launch
- Dedicated top-level `orchestrator:` section, separate from `agents:` list — structurally distinct from workers
- Include project-level config: project name, DB path, tmux settings alongside agent definitions
- Provider field is purely a label (e.g., "claude-code", "gemini") — no built-in provider-to-command mappings. Actual launch always via explicit `command` field

### CLI Output & Feedback
- Minimal by default: action + result only (e.g., `✓ Sent task to frontend-agent` or `✗ Agent not found: backend`)
- `--json` flag available on all commands for machine-readable structured output
- `list` command uses table format with aligned columns (like `docker ps` / `kubectl get pods`)
- Terminal colors with auto-detect: enabled by default, auto-disabled when piped or `NO_COLOR` env set

### Notification Delivery
- Signal notifies orchestrator via tmux send-keys into orchestrator session
- Notification format is structured: `[SIGNAL] agent=frontend status=completed task_id=42` — machine-parseable by orchestrator AI
- Completion event only — no output capture. Orchestrator uses `capture-pane` separately if it needs agent output
- Signals queue in DB regardless of orchestrator availability. If orchestrator session not running, notification is persisted and can be retrieved via `peek`/`list` on next check

### Error Handling
- Invalid targets (agent not found, dead tmux session) fail with clear error message and exit non-zero
- Simple exit codes: 0=success, 1=any error. Error type communicated via stderr message
- `init` partial failure: continue launching what works, report which agents failed and why. Exit 1 only if all agents failed
- `init` is idempotent: re-running skips already-running agents, only launches missing ones. Safe to retry after partial failures

### Claude's Discretion
- DB schema design and migration approach
- Rust module structure and code organization
- Exact table column layout for `list` output
- Color choices for status indicators
- Internal error types and error message wording
- Specific SQLite configuration details beyond WAL + busy_timeout

</decisions>

<specifics>
## Specific Ideas

- Naming convention from PROJECT.md: `<project>-<provider>-<role>` = tmux session name = agent identity
- Orchestrator should be structurally separate in config because it behaves fundamentally differently (receives signals, doesn't get tasks sent to it)
- Signal format should be parseable by any AI model acting as orchestrator — structured key=value pairs

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — fresh project with only boilerplate main.rs

### Established Patterns
- Cargo.toml has dependencies pre-selected: clap (derive), tokio, sqlx (sqlite), serde/serde_json, anyhow, chrono, uuid
- Note: STATE.md decisions reference rusqlite but Cargo.toml has sqlx — planner should reconcile this during planning

### Integration Points
- src/main.rs is the entry point — needs clap CLI argument parsing wired in
- No existing modules, everything built from scratch

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-core-foundation*
*Context gathered: 2026-03-06*
