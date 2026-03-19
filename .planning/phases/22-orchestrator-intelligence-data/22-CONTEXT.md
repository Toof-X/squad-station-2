# Phase 22: Orchestrator Intelligence Data - Context

**Gathered:** 2026-03-19
**Status:** Ready for planning

<domain>
## Phase Boundary

The `squad-station context` command produces a `squad-orchestrator.md` that includes live fleet metrics (pending message count, busy duration, task-role alignment) so the orchestrator AI can detect overload and misrouting without guessing. No new CLI subcommands — this extends the existing `context` command's output.

</domain>

<decisions>
## Implementation Decisions

### Fleet Status section placement
- New "Fleet Status" section goes **after PRE-FLIGHT**, before Session Routing
- Orchestrator sees metrics immediately after reading the playbook, informing routing decisions

### Fleet Status table format
- Markdown table with columns: Agent | Pending | Busy For | Alignment
- Pre-computed snapshot values at generation time (not commands-only)
- Blockquote below table with re-query commands for live data
- Brief routing hints (3 bullets) between table and re-query blockquote
- Only show workers (exclude orchestrator row) — same filter as Session Routing
- Exclude dead agents — only idle/busy agents appear in Fleet Status

### Routing hints content
- "Prefer agents with 0 pending tasks"
- "⚠️ alignment = task may be misrouted — verify before sending"
- "Re-query if this context is >5 minutes old"

### Re-query commands
- All four commands embedded in blockquote:
  - `squad-station agents` — agent status + busy duration
  - `squad-station list --status processing` — pending queue
  - `squad-station status` — fleet overview
  - `squad-station context` — regenerate this file with fresh data

### Task-role alignment
- Check **most recent task only** (currently-processing or last-completed) per agent
- **Simple word intersection** — tokenize task body and agent description (split whitespace, lowercase, dedup), compute overlap ratio after filtering stop words
- Zero overlap → ⚠️ warning; any overlap → ✅
- No external NLP crate — pure Rust string operations
- When no recent task exists (idle, never assigned) → show "—" (em dash)
- Warning format: `⚠️ 'fix CSS grid...' → backend` — first few words of task body + agent role

### Busy-time data source
- Use **existing `status_updated_at`** column — no new migration needed
- When `status == "busy"`: duration = now - status_updated_at
- When `status != "busy"`: show "idle"
- Human-friendly format: "5m", "1h 23m", "2d 4h"

### Pending message count
- Use existing `count_processing()` from `db/messages.rs` per agent
- Show integer count in Pending column

### build_orchestrator_md() purity (INTEL-05)
- Function remains pure — metrics struct passed as parameter
- Caller (context.rs `run()`) fetches metrics from DB, computes alignment, passes to builder
- New parameter: metrics slice/struct alongside existing `agents`, `project_root`, `sdd_configs`

### Claude's Discretion
- Stop word list for alignment tokenization
- Exact truncation length for task body in warning text
- Duration formatting edge cases (e.g., <1 minute → "just now" or "<1m")
- Internal metrics struct design (FleetMetrics, AgentMetrics, etc.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — INTEL-01 through INTEL-05 define the five metrics requirements

### Existing implementation
- `src/commands/context.rs` — Current `build_orchestrator_md()` pure function and `run()` caller
- `src/db/agents.rs` — Agent struct with `status_updated_at`, `description`, `role` fields
- `src/db/messages.rs` — `count_processing()` for pending count, `peek_message()` for most recent task
- `src/commands/helpers.rs` — `format_status_with_duration()` existing duration formatting pattern

### Architecture decisions
- `.planning/STATE.md` §Blockers/Concerns — flags wording correctness and busy_since decision (resolved: use status_updated_at)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `db::messages::count_processing(pool, agent_name)` — already counts processing messages per agent, directly usable for Pending column
- `db::messages::peek_message(pool, agent_name)` — retrieves most recent processing message, usable for alignment check
- `commands::helpers::format_status_with_duration(status, status_updated_at)` — existing duration formatting pattern to follow/reuse
- `Agent` struct already has `status`, `status_updated_at`, `role`, `description` — all fields needed for metrics

### Established Patterns
- `build_orchestrator_md()` is a pure string-builder function taking data slices — new metrics follow this pattern
- Workers filtered via `agents.iter().filter(|a| a.role != "orchestrator")` — reuse for Fleet Status
- Output dual-mode (JSON/human) in command `run()` functions — context command currently only writes file, no JSON mode needed

### Integration Points
- `context.rs::run()` calls `build_orchestrator_md()` — this is where DB queries for metrics will be added before the call
- Fleet Status section inserts into the markdown string between PRE-FLIGHT and Session Routing sections
- No changes to DB schema — all data already exists

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 22-orchestrator-intelligence-data*
*Context gathered: 2026-03-19*
