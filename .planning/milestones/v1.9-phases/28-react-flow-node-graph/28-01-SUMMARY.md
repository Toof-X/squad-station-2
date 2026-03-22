---
phase: 28-react-flow-node-graph
plan: 01
subsystem: ui
tags: [react, react-flow, dagre, typescript, websocket, visualization]

requires:
  - phase: 27-event-driven-websocket-streaming
    provides: useSquadWebSocket hook returning agents[], messages[], status

provides:
  - AgentNode: custom React Flow node with status dot, name, role, icon, hover tooltip
  - useGraphLayout: transforms agents[]+messages[] into dagre-positioned nodes/edges
  - Live agent graph wired to WebSocket data in App.tsx

affects:
  - 28-02 (custom animated edge type — consumes edges[] from useGraphLayout)

tech-stack:
  added:
    - "@dagrejs/dagre@2.0.4 — hierarchical graph layout engine"
  patterns:
    - "nodeTypes defined at module level outside React component (critical for React Flow performance)"
    - "Layout key uses agent names only — status-only WS updates don't trigger re-layout"
    - "Separate useMemo for edge animation updates — decoupled from dagre layout"
    - "Orchestrator detection by role string, fallback to message frequency, fallback to first agent"

key-files:
  created:
    - web/src/components/AgentNode.tsx
    - web/src/hooks/useGraphLayout.ts
  modified:
    - web/src/App.tsx
    - web/package.json
    - web/package-lock.json

key-decisions:
  - "AgentNodeData extends Record<string,unknown> to satisfy React Flow NodeProps generic constraint"
  - "Position enum (not string literals) required for sourcePosition/targetPosition in React Flow typed nodes"
  - "Edge type set to 'default' for Plan 01 — Plan 02 adds custom animated edge type"
  - "Edge animation (animated: true) derived from messages with status==='processing' between source/target agents"

patterns-established:
  - "AgentNodeData extends Record<string,unknown>: pattern for custom React Flow node data types"
  - "Layout stability: useMemo keyed on agent name string, not full agent objects, prevents re-layout on status updates"

requirements-completed: [VIZ-01, VIZ-02, UI-03]

duration: 15min
completed: 2026-03-22
---

# Phase 28 Plan 01: React Flow Node Graph Core Summary

**Custom AgentNode component and useGraphLayout hook delivering live dagre-hierarchical agent visualization from WebSocket data, replacing static placeholder graph**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-22T12:30:00Z
- **Completed:** 2026-03-22T12:45:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Custom `AgentNode` React Flow node: status color dot (green/gray/red), crown/gear icon, name, role, hover tooltip with model/status/task/description
- `useGraphLayout` hook: dagre TB layout (rankdir TB, ranksep 120, nodesep 80), orchestrator detection, structural edges with animation flags from in-flight messages
- `App.tsx` fully dynamic: removes all static placeholder nodes/edges, wires `useGraphLayout(agents, messages)` with `nodeTypes` at module level

## Task Commits

Each task was committed atomically:

1. **Task 1: Install dagre, create AgentNode component and useGraphLayout hook** - `69c4c73` (feat)
2. **Task 2: Wire dynamic graph into App.tsx** - `c4590aa` (feat)

## Files Created/Modified

- `web/src/components/AgentNode.tsx` - Custom React Flow node: status dot, icon, name, role, hover tooltip, Handle components
- `web/src/hooks/useGraphLayout.ts` - Transforms agents[]+messages[] into dagre-positioned nodes and edges
- `web/src/App.tsx` - Wires useGraphLayout, nodeTypes at module level, removes static placeholders
- `web/package.json` - Added @dagrejs/dagre dependency
- `web/package-lock.json` - Lock file updated

## Decisions Made

- `AgentNodeData extends Record<string, unknown>` — required to satisfy React Flow's NodeProps generic constraint which demands `data: Record<string, unknown>`
- Used `Position` enum (not string literals like `'top'/'bottom'`) for `sourcePosition`/`targetPosition` in typed React Flow nodes — TypeScript rejected literal strings
- Edge type remains `'default'` — Plan 02 introduces custom animated edge type
- Layout key uses `agents.map(a => a.name).sort().join(',')` — ensures re-layout only when agent set changes, not on every status update

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] AgentNodeData type constraint error**
- **Found during:** Task 2 (wire App.tsx — first full build)
- **Issue:** `AgentNodeData` interface didn't satisfy React Flow's `Record<string, unknown>` constraint on Node data generic, causing 7 TypeScript errors
- **Fix:** Changed `interface AgentNodeData` to `interface AgentNodeData extends Record<string, unknown>`
- **Files modified:** `web/src/components/AgentNode.tsx`
- **Verification:** `npx tsc --noEmit` passes, `npm run build` exits 0
- **Committed in:** `c4590aa` (Task 2 commit)

**2. [Rule 1 - Bug] sourcePosition/targetPosition string literal type error**
- **Found during:** Task 2 (same build run)
- **Issue:** `'top' as const` and `'bottom' as const` not assignable to `Position | undefined` in typed node spreads
- **Fix:** Import `Position` enum from `@xyflow/react`, use `Position.Top` and `Position.Bottom`
- **Files modified:** `web/src/hooks/useGraphLayout.ts`
- **Verification:** TypeScript compiles cleanly
- **Committed in:** `c4590aa` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 type bugs caught during first build)
**Impact on plan:** Both fixes necessary for TypeScript correctness. No scope changes.

## Issues Encountered

None beyond the type errors resolved above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Custom AgentNode and live graph layout complete — Plan 02 can add animated edge type (`edgeTypes`) with custom SVG path and activity indicators
- `edges[]` from `useGraphLayout` already includes `animated: true` and `data.task/priority/timestamp` for processing messages — Plan 02 custom edge can consume this data immediately
- `nodeTypes` module-level pattern established — Plan 02 adds `edgeTypes` at same level

---
*Phase: 28-react-flow-node-graph*
*Completed: 2026-03-22*
