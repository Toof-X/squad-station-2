# Phase 28: React Flow Node Graph - Context

**Gathered:** 2026-03-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the React Flow node-graph SPA that visualizes the agent fleet in real time. Each agent is a styled node with icon, name, and color-coded status. Edges between orchestrator and agents animate with crawling dots while messages are in-flight. The graph uses hierarchical auto-layout (orchestrator on top, workers below) with pan/zoom for scalability. Dark/light theme toggle persists in localStorage. Connection status indicator already exists from Phase 27 — enhance as needed.

</domain>

<decisions>
## Implementation Decisions

### Node visual design & information density
- Each node shows: **icon** (visual identifier), **agent name**, and **color-coded status dot/badge**
- Status color coding: **green = busy** (actively working), **gray = idle** (waiting), **red = dead** (session gone)
- Detailed info (role, model, description, current task, routing hints) appears in a **tooltip on hover** — not cluttering the node face
- Keep nodes compact — the graph should be scannable at a glance

### Edge animation & message display
- Use **crawling dots** animation style on edges to indicate message flow (in-flight/processing messages)
- Animation runs continuously while a message has `status: "processing"` between two agents; stops when message completes
- For MVP: simple is best — one animated edge per in-flight message, no stacking/multiplexing of concurrent messages on the same edge
- Edge labels or tooltips show message task text, priority, and timestamp per VIZ-04

### Theme toggle behavior
- Theme toggle button lives in the **top-right corner** of the UI (near the existing ConnectionStatus indicator)
- Theme preference persists in **localStorage** — survives page refresh and reconnection
- Light theme provides a contrasting color palette suitable for the node graph (details at Claude's discretion)
- Default theme remains **dark** (matches current `bg-gray-900` baseline from Phase 26)

### Graph layout & spacing
- **Hierarchical layout**: orchestrator node at top, worker nodes arranged below
- Layout derived automatically from squad.yml topology (orchestrator role vs worker roles) — no manual positioning
- Graph is **pannable and zoomable** (React Flow built-in controls) for scalability with many agents
- `fitView` on initial load so the entire topology is visible without manual adjustment
- Should remain usable with 10-20+ agents — workers wrap or space evenly below orchestrator

### Claude's Discretion
- Exact React Flow node component implementation (custom node type vs. styled default)
- Tooltip library choice or implementation (React Flow built-in vs. external)
- Crawling dots animation technique (CSS animation, React Flow edge type, or custom SVG)
- Hierarchical layout algorithm (manual position calculation vs. dagre/elkjs layout library)
- Light theme exact color palette
- Component file structure and naming
- How edges are derived from messages array (mapping logic)
- Memoization and re-render optimization strategy
- Whether to use React Flow's built-in minimap or controls panel

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — Phase 28 requirements: VIZ-01, VIZ-02, VIZ-03, VIZ-04, UI-02, UI-03

### Roadmap
- `.planning/ROADMAP.md` — Phase 28 success criteria (5 items) define the acceptance bar

### Prior phase context
- `.planning/milestones/v1.9-phases/25-architecture-research/25-CONTEXT.md` — Architecture decisions: React Flow v12, Vite React-TS, rust-embed pipeline
- `.planning/milestones/v1.9-phases/26-axum-server-and-cli-command/26-CONTEXT.md` — Server structure, SPA serving, Tailwind v4 setup
- `.planning/milestones/v1.9-phases/27-event-driven-websocket-streaming/27-CONTEXT.md` — WS event schema (snapshot/agent_update/message_update), reconnection behavior, state wipe on disconnect

### Existing code to build on
- `web/src/App.tsx` — Current SPA shell with static placeholder nodes; replace with dynamic WS-driven React Flow graph
- `web/src/hooks/useSquadWebSocket.ts` — Provides `agents[]`, `messages[]`, `status` via WS; Agent and WsMessage interfaces defined here
- `web/src/components/ConnectionStatus.tsx` — WS connection indicator (connected/disconnected/connecting) with colored dot; already satisfies UI-03 core behavior
- `web/src/components/StatusBar.tsx` — Project info bar with agent count, uptime, version; stays as-is or receives minor updates
- `web/src/main.tsx` — React entry point
- `web/package.json` — `@xyflow/react` v12 and Tailwind v4 already installed

### Data shapes (from useSquadWebSocket.ts)
- `Agent`: id, name, tool, role, status, status_updated_at, model, description, current_task, routing_hints
- `WsMessage`: id, agent_name, from_agent, to_agent, msg_type, task, status, priority, created_at, updated_at, completed_at, thread_id
- `ConnectionState`: 'connecting' | 'connected' | 'disconnected'

</canonical_refs>

<code_context>
## Existing Code Insights

### Assets to replace
- Static `initialNodes` and `initialEdges` arrays in `web/src/App.tsx:8-17` — Replace with dynamically generated nodes/edges derived from WS `agents[]` and `messages[]` data

### Assets to extend
- `App.tsx` — Add theme state (localStorage-backed), theme toggle button, dynamic node/edge generation from WS data
- `ConnectionStatus.tsx` — Already functional for UI-03; may need minor styling updates for theme support
- `StatusBar.tsx` — May need theme-aware styling

### Reusable patterns
- `useSquadWebSocket()` hook — Central data source; all node/edge state derives from `agents` and `messages` arrays
- Tailwind v4 utility classes — Used throughout existing components; extend for theme variants
- React Flow `fitView` prop — Already used in App.tsx; keep for initial load

### Integration points
- `web/src/` — All changes are frontend-only; no Rust server changes needed for Phase 28
- `web/package.json` — May need layout library (dagre/elkjs) if auto-layout requires it
- No build.rs or Cargo.toml changes expected — frontend-only phase

</code_context>

<specifics>
## Specific Ideas

- Derive nodes from `agents[]`: each agent becomes a React Flow node; orchestrator identified by `role` containing "orchestrator" or by being the agent that appears in `from_agent` fields
- Derive edges from `messages[]`: each message with `status === "processing"` creates an animated edge from `from_agent` to `to_agent`; completed messages can show as static edges briefly or be removed
- Theme toggle can use a simple React state + `localStorage.getItem/setItem` + a CSS class on the root `<div>` to flip Tailwind dark/light variants
- The crawling dots animation can leverage React Flow's built-in `animated` edge prop as a starting point, or use a custom CSS animation on SVG path for more control

</specifics>

<deferred>
## Deferred Ideas

None captured during discussion.

</deferred>

---

*Phase: 28-react-flow-node-graph*
*Context gathered: 2026-03-22*
