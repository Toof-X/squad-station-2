---
phase: 27-event-driven-websocket-streaming
plan: 02
subsystem: ui
tags: [react, typescript, websocket, hooks, tailwind, vite]

# Dependency graph
requires:
  - phase: 27-01
    provides: broadcast WS server sending snapshot/agent_update/message_update JSON frames on /ws

provides:
  - useSquadWebSocket hook managing WS lifecycle (connect, reconnect, state management)
  - ConnectionStatus as pure presentational component driven by status prop
  - StatusBar accepting agentCount prop from WS for real-time count, keeping REST for uptime/version
  - App.tsx wiring useSquadWebSocket() into child components

affects:
  - 28-react-flow-dynamic (will consume agents[] and messages[] from useSquadWebSocket for dynamic React Flow nodes)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Central WS hook pattern: useSquadWebSocket() owned by App, passed as props to presentational children
    - State wipe on disconnect: setAgents([]) + setMessages([]) on ws.onclose, fresh snapshot on reconnect
    - Prop-driven presentational components: ConnectionStatus receives status prop, StatusBar receives agentCount prop
    - WS data preferred over REST: agentCount ?? status.agents fallback preserves REST as backup

key-files:
  created:
    - web/src/hooks/useSquadWebSocket.ts
  modified:
    - web/src/App.tsx
    - web/src/components/ConnectionStatus.tsx
    - web/src/components/StatusBar.tsx

key-decisions:
  - "Central hook owns all WS state (agents, messages, status) and passes down as props — avoids multiple WS connections from child components"
  - "State wipe on disconnect (setAgents([]), setMessages([])) — on reconnect, fresh snapshot from server replaces all state with no stale merge"
  - "StatusBar uses agentCount ?? status.agents ?? 0 fallback — WS real-time preferred, REST as backstop if WS not yet connected"

patterns-established:
  - "useSquadWebSocket(): returns { agents, messages, status } — consumed by App, passed as props to children"
  - "ConnectionStatus({ status }): pure display only, no side effects"
  - "StatusBar({ agentCount? }): WS count when available, REST fallback otherwise"

requirements-completed: [RT-01, RT-04]

# Metrics
duration: ~5min
completed: 2026-03-22
---

# Phase 27 Plan 02: Frontend WebSocket Integration Summary

**React useSquadWebSocket hook consuming server's snapshot/agent_update/message_update events, with pure presentational ConnectionStatus and real-time agent count in StatusBar**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-03-22T10:28:53Z
- **Completed:** 2026-03-22T10:33:00Z
- **Tasks:** 2 of 2 completed (Task 2 human-verified approved)
- **Files modified:** 4

## Accomplishments

- Created `useSquadWebSocket` hook with full WS lifecycle: connect, receive snapshot/agent_update/message_update, reconnect every 3s, wipe state on disconnect
- Refactored `ConnectionStatus` from self-managed WS component to pure presentational component receiving status prop
- Refactored `StatusBar` to accept `agentCount` prop from WS data for real-time count while keeping REST polling for project name, uptime, and version
- Wired `App.tsx` to call `useSquadWebSocket()` once and pass data down to both child components
- npm run build and cargo build --features browser both succeed with zero errors

## Task Commits

1. **Task 1: Create useSquadWebSocket hook, refactor App/ConnectionStatus/StatusBar** - `48d1aaf` (feat)
2. **Task 2: Verify end-to-end WebSocket streaming** - human-verified (approved)

**Plan metadata:** pending (final docs commit)

## Files Created/Modified

- `web/src/hooks/useSquadWebSocket.ts` - Central WS hook with Agent/WsMessage/ConnectionState types, manages connect/reconnect/state
- `web/src/App.tsx` - Calls useSquadWebSocket(), passes status to ConnectionStatus and agentCount to StatusBar
- `web/src/components/ConnectionStatus.tsx` - Pure presentational: removed self-managed WS, now takes status prop
- `web/src/components/StatusBar.tsx` - Accepts agentCount prop; uses WS count preferentially, REST as fallback

## Decisions Made

- Central hook pattern: `useSquadWebSocket()` owned by App, passed as props to presentational children — avoids duplicate WS connections that would occur if each child managed its own connection.
- State wipe on disconnect: `setAgents([])` + `setMessages([])` in `ws.onclose` before scheduling reconnect — on reconnect, fresh snapshot replaces all state with no stale data merge (matches RT-04 requirement).
- `agentCount ?? status.agents ?? 0` fallback in StatusBar — WS real-time count preferred; REST agent count as backstop if WS data hasn't arrived yet.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Frontend WS integration complete and human-verified: snapshot received on connect, agent_update/message_update frames on state change, reconnection with state wipe confirmed
- Phase 28 (React Flow dynamic) can consume `agents` and `messages` arrays from `useSquadWebSocket()` directly — all types and data flow are in place

## Self-Check: PASSED

Files verified: web/src/hooks/useSquadWebSocket.ts, web/src/App.tsx, web/src/components/ConnectionStatus.tsx, web/src/components/StatusBar.tsx all exist. Commit 48d1aaf verified.

---
*Phase: 27-event-driven-websocket-streaming*
*Completed: 2026-03-22*
