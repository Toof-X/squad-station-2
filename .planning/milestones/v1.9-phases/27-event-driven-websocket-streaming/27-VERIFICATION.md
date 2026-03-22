---
phase: 27-event-driven-websocket-streaming
verified: 2026-03-22T11:00:00Z
status: human_needed
score: 9/9 must-haves verified
re_verification: false
human_verification:
  - test: "Verify snapshot first-frame delivery"
    expected: "Browser DevTools WS tab shows first frame as JSON object with type='snapshot', agents array, and messages array on initial connect"
    why_human: "Cannot invoke a real WebSocket handshake from static analysis; server must be running with a live DB"
  - test: "Verify agent status push within 500ms"
    expected: "Killing a tmux session (tmux kill-session -t agent-name) causes an agent_update frame to appear in WS tab within ~1 second — no browser refresh needed"
    why_human: "Requires live tmux + running server + DB with registered agents"
  - test: "Verify message update push within 200ms"
    expected: "Running 'squad-station send agent-name task' causes a message_update frame in WS tab within ~1 second"
    why_human: "Requires live DB interaction while server is running"
  - test: "Verify auto-reconnect and fresh snapshot"
    expected: "Stopping server causes ConnectionStatus to show Disconnected (red dot); restarting causes Connecting then Connected (green dot) and a new snapshot frame in WS tab"
    why_human: "Requires stopping and restarting the server process and observing UI state"
  - test: "Verify delta detection (no spurious frames)"
    expected: "With server running and no DB changes, WS tab receives no frames after the initial snapshot — silence confirms delta detection is working"
    why_human: "Requires monitoring WS tab over time with a live server"
---

# Phase 27: Event-Driven WebSocket Streaming — Verification Report

**Phase Goal:** Browser clients receive real-time state-change events pushed from the server without polling
**Verified:** 2026-03-22T11:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | WS handler sends a full JSON snapshot (agents + messages) as the first frame on connect | VERIFIED | `build_snapshot()` in browser.rs L84-96 queries agents + messages; `handle_socket()` L234-242 calls it and sends before subscribing to broadcast |
| 2 | Background polling tasks detect agent status changes at 500ms intervals | VERIFIED | `poll_agents()` L167-196 uses `tokio::time::interval(Duration::from_millis(500))` |
| 3 | Background polling tasks detect message changes at 200ms intervals | VERIFIED | `poll_messages()` L200-220 uses `tokio::time::interval(Duration::from_millis(200))` |
| 4 | When a DB change is detected, all connected WS clients receive a push event without browser refresh | VERIFIED | `tx.send(serialized)` in both poll_agents (L190) and poll_messages (L214) broadcasts to all subscribers; each handle_socket subscribes with `state.tx.subscribe()` L231 |
| 5 | Delta detection only broadcasts when state actually changed, not on every poll tick | VERIFIED | `agents_changed()` L99-116 and `messages_changed()` L119-132 gate `tx.send()` — poll loops check before broadcasting |
| 6 | Reconcile_agent_statuses runs within the agent polling loop using a separate writable pool | VERIFIED | `reconcile_for_server(pool)` called in `poll_agents()` L177-179 only when `db_write` is Some; `run()` creates separate write pool via `db::connect()` L325-331 |
| 7 | Browser receives full snapshot and renders agent count from live data | VERIFIED | `useSquadWebSocket` hook sets `agents` state on `snapshot` event (L53-54); `App.tsx` passes `agents.length` to `StatusBar` (L27) |
| 8 | When WS connection drops, browser shows Disconnected status and auto-reconnects in 3s | VERIFIED | `ws.onclose` in useSquadWebSocket.ts L68-73 sets status to 'disconnected', wipes agents/messages, schedules `setTimeout(connect, 3000)` |
| 9 | On reconnect, browser wipes stale state and receives fresh full snapshot | VERIFIED | State wipe (setAgents([]), setMessages([])) happens in `ws.onclose` before reconnect; next `snapshot` event from server replaces all state cleanly |

**Score:** 9/9 truths verified (automated)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/commands/browser.rs` | WS broadcast infrastructure, polling tasks, delta detection, snapshot builder | VERIFIED | 395 lines; contains `build_snapshot`, `agents_changed`, `messages_changed`, `reconcile_for_server`, `poll_agents`, `poll_messages`, real `handle_socket` — echo replaced |
| `web/src/hooks/useSquadWebSocket.ts` | Central WS hook managing connection, state, reconnection | VERIFIED | 91 lines; exports `useSquadWebSocket`, `Agent`, `WsMessage`, `ConnectionState` types; full connect/reconnect/wipe lifecycle |
| `web/src/App.tsx` | App component consuming WS data and passing to StatusBar/ConnectionStatus | VERIFIED | Calls `useSquadWebSocket()` L20; passes `status` to `ConnectionStatus` L30 and `agents.length` to `StatusBar` L27 |
| `web/src/components/ConnectionStatus.tsx` | Pure presentational component receiving status prop | VERIFIED | 24 lines; no useEffect, no self-managed WS; accepts `status: ConnectionState` prop directly |
| `web/src/components/StatusBar.tsx` | Status bar showing agent count from WS data, keeps REST for uptime/version | VERIFIED | Accepts `agentCount?: number` prop L16; uses `agentCount ?? status.agents ?? 0` fallback L44; REST polling retained for project name/uptime/version |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `poll_agents` task | `broadcast::Sender` | `tx.send()` on delta detection | WIRED | L190: `let _ = tx.send(serialized);` inside `if agents_changed(...)` |
| `poll_messages` task | `broadcast::Sender` | `tx.send()` on delta detection | WIRED | L214: `let _ = tx.send(serialized);` inside `if messages_changed(...)` |
| `ws_handler` | `broadcast::Receiver` | `state.tx.subscribe()` | WIRED | L231: `let mut rx = state.tx.subscribe();` — critically placed BEFORE `build_snapshot` to prevent race |
| `ws_handler` | `build_snapshot` | initial frame on connect | WIRED | L234: `if let Some(snapshot_json) = build_snapshot(&state).await` — result sent as first WS frame |
| `web/src/App.tsx` | `useSquadWebSocket` hook | hook call at component top | WIRED | L6 import + L20 call: `const { agents, messages: _messages, status } = useSquadWebSocket();` |
| `useSquadWebSocket` | `ws://host/ws` | WebSocket connection | WIRED | L44: `ws = new WebSocket(\`ws://${window.location.host}/ws\`)` |
| `App.tsx` | `ConnectionStatus` | `status=` prop | WIRED | L30: `<ConnectionStatus status={status} />` |
| `App.tsx` | `StatusBar` | `agentCount=` prop | WIRED | L27: `<StatusBar agentCount={agents.length} />` |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| RT-01 | 27-01, 27-02 | axum WS endpoint pushes state-change events to all connected browser clients | SATISFIED | broadcast channel in AppState + `handle_socket` subscribes + `poll_agents`/`poll_messages` broadcast on change |
| RT-02 | 27-01 | Event-driven detection watches tmux panes and DB for state changes | SATISFIED | `agents_changed()` and `messages_changed()` gate broadcasts; `reconcile_for_server()` runs tmux session checks each 500ms tick |
| RT-03 | 27-01 | On WS connect, server sends full topology + message state snapshot as first frame | SATISFIED | `build_snapshot()` called in `handle_socket()` before entering broadcast loop |
| RT-04 | 27-02 | Browser auto-reconnects on WS drop and re-syncs full state | SATISFIED | `ws.onclose` schedules `setTimeout(connect, 3000)` with state wipe; `snapshot` event on reconnect replaces all state |

All 4 requirement IDs (RT-01, RT-02, RT-03, RT-04) declared across the two plans are accounted for. No orphaned requirements found — REQUIREMENTS.md maps exactly these 4 IDs to Phase 27.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `web/src/App.tsx` | 20 | `messages: _messages` (messages received but not passed to any child) | Info | Expected — Phase 28 will wire messages to React Flow nodes; hook correctly populates state |

No TODO/FIXME/PLACEHOLDER comments found. No stub return patterns found. No empty handlers found.

### Human Verification Required

#### 1. Snapshot First-Frame Delivery

**Test:** Start server (`cargo run --features browser -- browser --no-open`), open browser, open DevTools > Network > WS > /ws connection. Inspect first message frame.
**Expected:** First frame is a JSON text frame: `{"type":"snapshot","agents":[...],"messages":[...]}`
**Why human:** Requires a running server connected to a live SQLite DB — cannot simulate a real WebSocket handshake from static analysis.

#### 2. Agent Status Push within 500ms

**Test:** With server running and at least one agent in DB with a live tmux session, run `tmux kill-session -t <agent-name>`. Watch WS tab in DevTools.
**Expected:** An `agent_update` frame appears within approximately 1 second; the agent's status in the JSON payload reflects "dead".
**Why human:** Requires live tmux session and running server with registered agents.

#### 3. Message Update Push within 200ms

**Test:** With server running, run `squad-station send <agent-name> "test task"`. Watch WS tab.
**Expected:** A `message_update` frame appears within approximately 1 second containing the new message.
**Why human:** Requires live DB write-and-detect cycle with running server.

#### 4. Auto-Reconnect and Fresh Snapshot

**Test:** With browser open and showing "Connected" (green dot), stop the server (Ctrl+C). Observe ConnectionStatus. Then restart the server. Observe ConnectionStatus and WS tab.
**Expected:** On stop: red dot "Disconnected" within 3s. On restart: yellow "Connecting..." then green "Connected". A new snapshot frame appears in WS tab.
**Why human:** Requires stopping and restarting the server process while observing browser UI state transitions.

#### 5. Delta Detection Silence

**Test:** Start server with no DB changes happening. Open WS tab after initial snapshot frame. Wait 10-15 seconds.
**Expected:** No additional frames sent to the browser — polling loops detect no changes and stay silent.
**Why human:** Requires time-based observation of live WS traffic; cannot assert absence of frames via static grep.

### Gaps Summary

No automated gaps found. All artifacts are substantive and fully wired. Both compilation checks pass clean (`cargo check --features browser` and `npm run build`). All 13 tests pass.

The only outstanding item is that `messages` data from `useSquadWebSocket` is received but currently aliased as `_messages` in App.tsx — this is an intentional Phase 27 design choice (the plan explicitly states "Keep the static React Flow nodes/edges for now — Phase 28 will make them dynamic"). The messages pipeline is complete end-to-end (server polls, detects changes, broadcasts; hook receives and updates state); the data just has no visual consumer yet. This is not a gap for Phase 27's goal.

Phase 27 automated verification is complete. All 9 observable truths pass and all 4 requirement IDs are satisfied. Awaiting human confirmation of 5 end-to-end runtime behaviors.

---

_Verified: 2026-03-22T11:00:00Z_
_Verifier: Claude (gsd-verifier)_
