# Phase 27: Event-Driven WebSocket Streaming - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the Phase 26 WebSocket echo handler with real event-driven streaming. The server watches DB state changes at fixed polling intervals, detects deltas, and pushes events to all connected browser clients via `tokio::sync::broadcast`. On connect, clients receive a full JSON snapshot; subsequent pushes are partial updates. The browser auto-reconnects and re-syncs via fresh snapshot. No tmux pane content watching — purely DB-driven state detection.

</domain>

<decisions>
## Implementation Decisions

### WebSocket event schema & snapshot content
- On WebSocket connect, server sends a **full JSON snapshot** containing all agents (with status, role, model, etc.) and all relevant messages (in-flight/recent)
- After the initial snapshot, server sends **partial updates** — only the DB rows that changed since the last push
- Keep the schema simple: events are JSON objects with a `type` field (`snapshot` vs `update`) and the relevant data payload
- Payload structure mirrors DB row shapes (Agent struct, Message struct) — no transformation layer, frontend adapts to the DB schema

### State change detection — DB only
- Agent status detection relies **purely on DB `status` field changes** — no tmux pane content watching
- The existing `reconcile_agent_statuses()` pattern (session existence checks) runs within the polling loop to keep DB status current
- Polling intervals (from Phase 25 decision): **500ms for agent status, 200ms for messages**
- Delta detection: compare current DB state against previous snapshot; only broadcast when something actually changed
- `tokio::sync::broadcast` for multi-client fan-out (from Phase 25 decision)
- `tokio::task::spawn_blocking` for tmux session existence checks within the polling loop

### Reconnection & staleness UX
- On WebSocket disconnect, browser shows a simple **"Reconnecting..."** warning via the existing `ConnectionStatus` component
- On reconnect, client **wipes its entire local state** and loads the fresh full snapshot from the server — no incremental merge or diff reconciliation
- This avoids complex client-side state merging logic; the snapshot is small enough (agent count is typically < 20) that full reload is cheap
- Existing 3s reconnect timer in `ConnectionStatus` is acceptable; no exponential backoff needed

### Claude's Discretion
- Exact JSON field names and event type strings
- How the polling loop is structured (single task vs separate tasks for agents/messages)
- broadcast channel capacity and lagged-receiver handling
- How `reconcile_agent_statuses()` is adapted for the server context (read-only pool + separate write for status updates, or reconciliation delegated to CLI commands)
- Whether `/api/status` endpoint is kept alongside WS or replaced
- Internal state diffing implementation (hash comparison, timestamp comparison, etc.)
- Test strategy for the streaming infrastructure

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — Phase 27 requirements: RT-01, RT-02, RT-03, RT-04

### Roadmap
- `.planning/ROADMAP.md` — Phase 27 success criteria (5 items) define the acceptance bar

### Prior phase context
- `.planning/milestones/v1.9-phases/25-architecture-research/25-CONTEXT.md` — Architecture decisions: polling intervals, broadcast channel, spawn_blocking for tmux
- `.planning/milestones/v1.9-phases/26-axum-server-and-cli-command/26-CONTEXT.md` — Server structure, feature gating, SPA content decisions

### Existing code to build on
- `src/commands/browser.rs` — Current server with WS echo handler (`ws_handler`, `handle_socket`), `AppState`, `bind_listener`, `shutdown_signal` — WS handler will be replaced
- `web/src/components/ConnectionStatus.tsx` — Auto-reconnect logic (3s timer), status indicator — will consume real WS events
- `web/src/components/StatusBar.tsx` — Currently polls `/api/status` every 10s — may switch to WS-driven data
- `web/src/App.tsx` — Static React Flow nodes/edges — Phase 28 will make these dynamic using WS data

### State detection patterns
- `src/commands/helpers.rs:reconcile_agent_statuses()` — Reconciles agent status against tmux session existence; reference pattern for the server's polling loop
- `src/db/agents.rs` — `Agent` struct (fields: name, tool, role, status, model, description, current_task, routing_hints) — shapes the snapshot payload
- `src/db/messages.rs` — `Message` struct (fields: from_agent, to_agent, msg_type, task, status, priority, created_at, completed_at) — shapes the snapshot payload
- `src/tmux.rs` — `session_exists()` used by reconciliation; server will call this via spawn_blocking

### Build & distribution
- `Cargo.toml` — `tokio::sync::broadcast` is already available via tokio dependency; no new deps expected for Phase 27
- `web/package.json` — Frontend deps; may need no changes if WS handling stays in existing components

</canonical_refs>

<code_context>
## Existing Code Insights

### Assets to replace
- `ws_handler()` + `handle_socket()` in `src/commands/browser.rs:77-92` — Echo handler; replace with real event streaming that subscribes to broadcast channel

### Assets to extend
- `AppState` in `src/commands/browser.rs:21-26` — Needs broadcast sender added (e.g., `tx: broadcast::Sender<Event>`)
- `run()` in `src/commands/browser.rs:118-182` — Needs to spawn the polling/detection background task before starting axum server
- `ConnectionStatus.tsx` — Already handles connect/disconnect/reconnect; needs to process incoming JSON messages and pass data up to App

### Reusable patterns
- `reconcile_agent_statuses()` in `src/commands/helpers.rs:10-35` — Checks session existence in parallel, updates DB status; polling loop needs similar logic but on a 500ms interval
- `list_agents()` in `src/db/agents.rs` — Fetches all agents; used to build snapshot
- `get_messages()` in `src/db/messages.rs` — Fetches messages with filters; used to build snapshot (filter for in-flight/recent)
- Connect-per-refresh pattern from `src/commands/ui.rs` — Server's persistent read-only pool is the alternative; no need to reconnect per poll

### Integration points
- `src/commands/browser.rs` — All server-side changes happen here (new module files acceptable)
- `web/src/` — Frontend WS message handling, state management, passing data to React Flow components
- `src/db/mod.rs` — `connect_readonly()` already exists; server uses this for polling reads

</code_context>

<specifics>
## Specific Ideas

- The polling task can maintain a local cache of the last-seen agent list and message list; on each poll cycle, compare against fresh DB query results and only broadcast if there's a diff
- The full snapshot event should include both `agents` array and `messages` array in a single JSON frame — the client needs both to render the graph
- Consider filtering messages in the snapshot to only include active/recent ones (e.g., `processing` status + last N completed) rather than the entire message history
- The broadcast channel capacity can be small (e.g., 16-32) since events are frequent but small; lagged receivers should skip to latest state

</specifics>

<deferred>
## Deferred Ideas

None captured during discussion.

</deferred>

---

*Phase: 27-event-driven-websocket-streaming*
*Context gathered: 2026-03-22*
