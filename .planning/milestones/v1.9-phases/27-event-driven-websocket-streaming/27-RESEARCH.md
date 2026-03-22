# Phase 27: Event-Driven WebSocket Streaming - Research

**Researched:** 2026-03-22
**Domain:** axum WebSocket streaming, tokio broadcast channels, delta-detection polling
**Confidence:** HIGH

## Summary

Phase 27 replaces the echo WebSocket handler from Phase 26 with real event-driven streaming. The server runs background polling tasks that detect DB state changes (agent status, messages) and tmux session existence, then broadcast deltas to all connected browser clients via `tokio::sync::broadcast`. On connect, each client receives a full JSON snapshot; subsequent pushes are partial updates containing only changed rows.

The architecture is straightforward: axum's built-in WebSocket support handles upgrades, `tokio::sync::broadcast` handles multi-client fan-out, and background `tokio::spawn` tasks poll the DB at fixed intervals (500ms agents, 200ms messages). The primary complexity is in the delta-detection logic (comparing current DB state against cached previous state) and the read-only vs writable pool tension for tmux reconciliation.

**Primary recommendation:** Use a single broadcast channel carrying serialized JSON strings. Background polling tasks own the delta detection and serialize events before broadcasting. Each WS connection subscribes to the broadcast channel and forwards messages verbatim. Keep the server's DB pool read-only; add a separate single-connection writable pool solely for tmux status reconciliation.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- On WebSocket connect, server sends a **full JSON snapshot** containing all agents and all relevant messages
- After initial snapshot, server sends **partial updates** -- only DB rows that changed since last push
- Events are JSON objects with a `type` field (`snapshot` vs `update`) and relevant data payload
- Payload structure mirrors DB row shapes (Agent struct, Message struct) -- no transformation layer
- Agent status detection relies **purely on DB status field changes** -- no tmux pane content watching
- Existing `reconcile_agent_statuses()` pattern runs within the polling loop
- Polling intervals: **500ms for agent status, 200ms for messages**
- `tokio::sync::broadcast` for multi-client fan-out
- `tokio::task::spawn_blocking` for tmux session existence checks
- On reconnect, client **wipes entire local state** and loads fresh full snapshot
- Existing 3s reconnect timer in ConnectionStatus is acceptable

### Claude's Discretion
- Exact JSON field names and event type strings
- Single polling task vs separate tasks for agents/messages
- Broadcast channel capacity and lagged-receiver handling
- How `reconcile_agent_statuses()` is adapted (read-only pool + separate write, or delegated to CLI)
- Whether `/api/status` endpoint is kept alongside WS or replaced
- Internal state diffing implementation (hash comparison, timestamp comparison, etc.)
- Test strategy for the streaming infrastructure

### Deferred Ideas (OUT OF SCOPE)
None captured during discussion.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| RT-01 | axum WebSocket endpoint pushes state-change events to all connected browser clients | Broadcast channel pattern with per-connection subscriber tasks; verified via axum chat example |
| RT-02 | Event-driven detection watches tmux panes and DB for state changes | Background polling tasks with delta detection; reconcile_agent_statuses() adapted for server context |
| RT-03 | On WebSocket connect, server sends full topology + message state snapshot as initial frame | ws_handler sends snapshot from DB before subscribing to broadcast channel |
| RT-04 | Browser auto-reconnects on WebSocket drop and re-syncs full state | Existing ConnectionStatus.tsx 3s reconnect; client wipes state and receives fresh snapshot |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.7 | WebSocket upgrade, routing | Already in Cargo.toml with `ws` feature |
| tokio | 1.37 | broadcast channel, spawn, timers | Already in Cargo.toml with `full` features |
| serde / serde_json | 1.0 | Event serialization | Already in Cargo.toml; Agent/Message already derive Serialize |
| sqlx | 0.8 | DB polling reads | Already in Cargo.toml |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| futures | 0.3 | join_all for parallel tmux checks | Already in Cargo.toml; used by reconcile pattern |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tokio::sync::broadcast | tokio::sync::watch | watch only keeps latest value; broadcast preserves event history up to capacity -- broadcast is correct for event stream |
| Polling DB | SQLite update hooks | SQLite hooks fire on the writer side (CLI process), not the reader; cross-process notification would need IPC -- polling is simpler and sufficient |

**Installation:**
```bash
# No new dependencies needed -- all libraries already in Cargo.toml
```

## Architecture Patterns

### Recommended Project Structure
```
src/commands/
  browser.rs          # Extended: AppState gains broadcast sender, run() spawns polling tasks
  browser/            # Alternative: split into submodules if browser.rs grows too large
    mod.rs            # Re-exports, AppState, run()
    events.rs         # Event types, serialization, snapshot building
    polling.rs        # Background polling tasks, delta detection
    ws.rs             # WebSocket handler, per-connection subscriber
```

### Pattern 1: Broadcast Channel with Per-Connection Subscriber
**What:** Store `broadcast::Sender<String>` in AppState. Each WS connection calls `tx.subscribe()` to get its own receiver. A spawned task reads from the receiver and writes to the WS sender.
**When to use:** Always -- this is the canonical axum pattern for fan-out.
**Example:**
```rust
// Source: axum examples/chat pattern (verified via GitHub)
use tokio::sync::broadcast;

#[derive(Clone)]
struct AppState {
    db: Option<sqlx::SqlitePool>,
    db_write: Option<sqlx::SqlitePool>,  // For reconciliation writes only
    project_name: String,
    started_at: Instant,
    tx: broadcast::Sender<String>,       // Events broadcast as pre-serialized JSON
}

// In run(), before starting server:
let (tx, _rx) = broadcast::channel::<String>(32);

// WS handler:
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 1. Send initial snapshot
    let snapshot = build_snapshot(&state).await;
    let json = serde_json::to_string(&snapshot).unwrap();
    let _ = ws_sender.send(Message::Text(json.into())).await;

    // 2. Subscribe to broadcast channel
    let mut rx = state.tx.subscribe();

    // 3. Forward broadcast events to this WS client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // 4. Listen for client close
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
```

### Pattern 2: Delta-Detection Polling Loop
**What:** Background task polls DB at fixed intervals, compares against cached state, broadcasts only when changes detected.
**When to use:** For RT-02 -- state change detection.
**Example:**
```rust
async fn poll_agents(
    db: sqlx::SqlitePool,
    db_write: Option<sqlx::SqlitePool>,
    tx: broadcast::Sender<String>,
) {
    let mut cached_agents: Vec<Agent> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        interval.tick().await;

        // Reconcile tmux status (requires writable pool)
        if let Some(ref write_pool) = db_write {
            // session_exists() shells out to tmux -- use spawn_blocking
            let pool = write_pool.clone();
            let _ = tokio::task::spawn_blocking(move || {
                // Note: reconcile is async, need a runtime handle
                // Better: run session checks with spawn_blocking individually
            });
            // Actually: reconcile_agent_statuses is async, call it directly
            // but wrap individual tmux calls in spawn_blocking
            let _ = reconcile_statuses_for_server(&pool).await;
        }

        // Read current state
        let current = match db::agents::list_agents(&db).await {
            Ok(agents) => agents,
            Err(_) => continue,
        };

        // Compare against cache
        if agents_changed(&cached_agents, &current) {
            let event = json!({
                "type": "agent_update",
                "agents": current,
            });
            let _ = tx.send(serde_json::to_string(&event).unwrap());
            cached_agents = current;
        }
    }
}
```

### Pattern 3: Snapshot Building
**What:** Build full state snapshot from DB for initial WS frame.
**When to use:** On every new WS connection and on reconnect.
**Example:**
```rust
#[derive(Serialize)]
struct Snapshot {
    #[serde(rename = "type")]
    event_type: String,  // "snapshot"
    agents: Vec<Agent>,
    messages: Vec<Message>,
}

async fn build_snapshot(state: &AppState) -> Snapshot {
    let agents = match &state.db {
        Some(pool) => db::agents::list_agents(pool).await.unwrap_or_default(),
        None => vec![],
    };
    let messages = match &state.db {
        Some(pool) => db::messages::list_messages(pool, None, None, 100)
            .await.unwrap_or_default(),
        None => vec![],
    };
    Snapshot {
        event_type: "snapshot".to_string(),
        agents,
        messages,
    }
}
```

### Anti-Patterns to Avoid
- **Sending binary WS frames:** Use Text frames with JSON -- the browser side uses `JSON.parse()` and binary adds complexity for no benefit.
- **Holding DB pool across await points in WS handler:** The snapshot query should complete before subscribing to broadcast. Don't interleave DB reads with WS sends.
- **Broadcasting the raw DB query result without diffing:** This wastes bandwidth and forces clients to do diffing. The server should diff and only send when state actually changed.
- **Using a single read-only pool for reconciliation writes:** `connect_readonly` sets `read_only(true)` which will reject writes. A separate writable pool is needed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Multi-client fan-out | Custom Vec of senders | `tokio::sync::broadcast` | Handles subscribe/unsubscribe, backpressure, lagged receiver cleanup |
| WebSocket upgrade | Manual HTTP upgrade | `axum::extract::ws::WebSocketUpgrade` | Handles protocol negotiation, headers, upgrade response |
| WS stream splitting | Manual message routing | `socket.split()` from futures | Clean separation of send/receive halves for concurrent tasks |
| Periodic polling | Manual sleep loops | `tokio::time::interval` | Handles drift, missed ticks correctly |
| JSON serialization | Manual string building | `serde_json::to_string` | Agent/Message already derive Serialize |

**Key insight:** The entire server-side WS infrastructure is ~150 lines of glue code connecting existing pieces (axum WS, broadcast channel, DB queries, serde). No novel algorithms needed.

## Common Pitfalls

### Pitfall 1: Read-Only Pool Cannot Write
**What goes wrong:** `reconcile_agent_statuses()` calls `update_agent_status()` which writes to DB. The server's pool from `connect_readonly()` has `read_only(true)` and will reject writes.
**Why it happens:** Phase 26 intentionally used read-only to avoid contending with CLI's single-writer pool.
**How to avoid:** Create a separate writable pool with `max_connections(1)` for status reconciliation only. Or better: adapt the reconciliation to use a dedicated writable connection that opens/closes per reconciliation cycle to minimize WAL contention.
**Warning signs:** SQLite error "attempt to write a readonly database" at runtime.

**Recommendation:** Create a second pool via `db::connect()` (the existing writable connector) for reconciliation writes. This is safe because reconciliation writes are infrequent (every 500ms, only when status changes) and the single-writer pool has a 5s busy_timeout.

### Pitfall 2: Broadcast Channel Lagged Receivers
**What goes wrong:** If a client is slow to read, `rx.recv()` returns `RecvError::Lagged(n)`. If not handled, the receive loop breaks and the client disconnects.
**Why it happens:** Broadcast channel has finite capacity (e.g., 32). If a receiver doesn't consume fast enough, oldest messages are dropped.
**How to avoid:** Handle `RecvError::Lagged` by continuing the loop (skip missed messages). The next agent or message update will contain the current full state of changed rows anyway. For this use case, missing intermediate updates is acceptable.
**Warning signs:** Clients randomly disconnecting under load.

```rust
loop {
    match rx.recv().await {
        Ok(msg) => {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
        Err(broadcast::error::RecvError::Lagged(_)) => {
            // Skip missed messages -- next update will have current state
            continue;
        }
        Err(broadcast::error::RecvError::Closed) => break,
    }
}
```

### Pitfall 3: tmux Calls Blocking the Tokio Runtime
**What goes wrong:** `session_exists()` shells out to `tmux has-session` via `tokio::process::Command`. While this is async, running many in parallel can pressure the runtime.
**Why it happens:** Each agent requires a separate tmux process spawn.
**How to avoid:** The existing `reconcile_agent_statuses()` already uses `futures::future::join_all` for parallel checks, which is fine. The CONTEXT.md mentions `spawn_blocking` but `tokio::process::Command` is already non-blocking. No change needed here -- the existing pattern works.
**Warning signs:** High latency spikes during reconciliation with many agents.

### Pitfall 4: Race Between Snapshot and Broadcast Subscription
**What goes wrong:** If a state change happens between sending the snapshot and subscribing to the broadcast channel, the client misses that update.
**Why it happens:** Subscribe-then-snapshot ordering matters.
**How to avoid:** Subscribe to the broadcast channel FIRST, then build and send the snapshot, then start forwarding broadcast messages. Any events that arrive during snapshot building will be queued in the broadcast receiver.

```rust
// Correct order:
let mut rx = state.tx.subscribe();  // 1. Subscribe first
let snapshot = build_snapshot(&state).await;  // 2. Build snapshot
ws_sender.send(snapshot_msg).await;  // 3. Send snapshot
// 4. Now drain rx -- events during snapshot build are queued
```

### Pitfall 5: Frontend State Not Clearing on Reconnect
**What goes wrong:** If the client doesn't wipe state on reconnect, it may show stale data merged with fresh snapshot data.
**Why it happens:** The existing ConnectionStatus component reconnects but doesn't communicate state reset to App.
**How to avoid:** The WS message handler in React should reset all state when it receives a `snapshot` event type. This naturally happens if the snapshot handler does `setAgents(snapshot.agents)` and `setMessages(snapshot.messages)` (full replacement, not merge).

## Code Examples

### Event Type Definitions (Rust)
```rust
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
enum WsEvent {
    #[serde(rename = "snapshot")]
    Snapshot {
        agents: Vec<crate::db::agents::Agent>,
        messages: Vec<crate::db::messages::Message>,
    },
    #[serde(rename = "agent_update")]
    AgentUpdate {
        agents: Vec<crate::db::agents::Agent>,
    },
    #[serde(rename = "message_update")]
    MessageUpdate {
        messages: Vec<crate::db::messages::Message>,
    },
}
```

### Delta Detection (Rust)
```rust
fn agents_changed(prev: &[Agent], curr: &[Agent]) -> bool {
    if prev.len() != curr.len() {
        return true;
    }
    // Compare by status and status_updated_at -- the fields that change
    for (p, c) in prev.iter().zip(curr.iter()) {
        if p.name != c.name || p.status != c.status
            || p.status_updated_at != c.status_updated_at
            || p.current_task != c.current_task
        {
            return true;
        }
    }
    false
}

fn messages_changed(prev: &[Message], curr: &[Message]) -> bool {
    if prev.len() != curr.len() {
        return true;
    }
    for (p, c) in prev.iter().zip(curr.iter()) {
        if p.id != c.id || p.status != c.status || p.updated_at != c.updated_at {
            return true;
        }
    }
    false
}
```

### Frontend WS Hook (TypeScript)
```typescript
// Source: React WebSocket pattern
import { useEffect, useState, useRef, useCallback } from 'react';

interface Agent {
  name: string;
  tool: string;
  role: string;
  status: string;
  status_updated_at: string;
  model: string | null;
  description: string | null;
  current_task: string | null;
}

interface Message {
  id: string;
  from_agent: string | null;
  to_agent: string | null;
  msg_type: string;
  task: string;
  status: string;
  priority: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

type WsEvent =
  | { type: 'snapshot'; agents: Agent[]; messages: Message[] }
  | { type: 'agent_update'; agents: Agent[] }
  | { type: 'message_update'; messages: Message[] };

function useSquadWebSocket() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [status, setStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');

  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    function connect() {
      setStatus('connecting');
      ws = new WebSocket(`ws://${window.location.host}/ws`);

      ws.onopen = () => setStatus('connected');

      ws.onmessage = (event) => {
        const data: WsEvent = JSON.parse(event.data);
        switch (data.type) {
          case 'snapshot':
            setAgents(data.agents);
            setMessages(data.messages);
            break;
          case 'agent_update':
            setAgents(data.agents);
            break;
          case 'message_update':
            setMessages(data.messages);
            break;
        }
      };

      ws.onclose = () => {
        setStatus('disconnected');
        // Wipe state on disconnect (will be refreshed on reconnect snapshot)
        setAgents([]);
        setMessages([]);
        reconnectTimer = setTimeout(connect, 3000);
      };

      ws.onerror = () => ws?.close();
    }

    connect();
    return () => {
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (ws) { ws.onclose = null; ws.close(); }
    };
  }, []);

  return { agents, messages, status };
}
```

### Integrating WS Hook into ConnectionStatus
```typescript
// ConnectionStatus becomes the WS connection manager and data provider
// App.tsx lifts state up or uses a context provider

// Option A: Lift WS hook to App.tsx
export default function App() {
  const { agents, messages, status } = useSquadWebSocket();

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      <div className="flex items-center justify-between border-b border-gray-700">
        <StatusBar agentCount={agents.length} />
        <ConnectionStatus status={status} />
      </div>
      <div className="flex-1">
        <ReactFlow nodes={buildNodes(agents)} edges={buildEdges(messages)} fitView />
      </div>
    </div>
  );
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Server-Sent Events | WebSocket bidirectional | N/A | WS allows future client-to-server commands |
| Full state on every push | Delta detection + partial updates | Design decision | Reduces bandwidth, especially with many messages |
| Poll from browser | Server push via WS | Phase 27 | Eliminates 10s polling in StatusBar; real-time updates |

## Critical Design Decision: Reconciliation and Writable Pool

The server currently uses `connect_readonly()` which sets `read_only(true)`. The `reconcile_agent_statuses()` function needs to WRITE status updates (dead/idle transitions based on tmux session existence).

**Recommendation:** Add a second pool via `db::connect()` with `max_connections(1)` for reconciliation writes. This is the simplest approach that reuses existing code. The 5s `busy_timeout` handles contention with CLI writes gracefully.

**Alternative:** Skip reconciliation in the server entirely; rely on CLI commands (status, list, etc.) to trigger reconciliation. This is simpler but means the browser shows stale status until someone runs a CLI command. Given the 500ms polling interval, this defeats the purpose of real-time updates.

**Decision: Use separate writable pool.** The reconciliation is the mechanism that makes agent status accurate. Without it, the browser would show agents as "idle" even after their tmux session dies.

## Message Filtering for Snapshot

The full message history can grow large. For the snapshot and updates, filter to:
- All messages with `status = 'processing'` (in-flight)
- Recent completed messages (last 50 or last hour)

This keeps payloads small while giving the browser enough context for edge rendering.

```rust
// For snapshot: processing + recent completed
let messages = db::messages::list_messages(pool, None, None, 100).await?;
// list_messages already orders by created_at DESC with limit
```

## Open Questions

1. **Should StatusBar keep polling /api/status or switch to WS data?**
   - What we know: StatusBar currently polls every 10s for project name, agent count, uptime, version
   - What's unclear: Whether to keep this separate endpoint or derive everything from WS data
   - Recommendation: Keep `/api/status` for now. StatusBar can derive agent count from WS data but still needs uptime/version from the REST endpoint. Phase 28 can consolidate. This avoids scope creep in Phase 27.

2. **Single polling task vs separate tasks for agents and messages?**
   - What we know: Agents poll at 500ms, messages at 200ms -- different intervals
   - Recommendation: Use two separate `tokio::spawn` tasks, one for each interval. Simpler than a single task managing two timers.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (tokio async) |
| Config file | Cargo.toml [dev-dependencies] |
| Quick run command | `cargo test --features browser` |
| Full suite command | `cargo test` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RT-01 | Broadcast channel sends events to subscribers | unit | `cargo test --features browser test_broadcast` | No -- Wave 0 |
| RT-02 | Delta detection identifies changed agents/messages | unit | `cargo test --features browser test_delta` | No -- Wave 0 |
| RT-03 | Snapshot builds correct JSON from DB state | unit | `cargo test --features browser test_snapshot` | No -- Wave 0 |
| RT-04 | Frontend reconnect wipes state (manual) | manual-only | Visual verification | N/A |

### Sampling Rate
- **Per task commit:** `cargo test --features browser`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src/commands/browser/events.rs` tests -- WsEvent serialization, snapshot building
- [ ] `src/commands/browser/polling.rs` tests -- agents_changed(), messages_changed() delta detection
- [ ] Frontend WS handling -- manual verification (connect, receive snapshot, see updates, reconnect)

## Sources

### Primary (HIGH confidence)
- Codebase inspection: `src/commands/browser.rs`, `src/db/agents.rs`, `src/db/messages.rs`, `src/db/mod.rs`, `src/commands/helpers.rs`, `src/tmux.rs`
- axum examples/chat: https://github.com/tokio-rs/axum/discussions/1335 -- broadcast pattern confirmed
- axum WebSocket API: Already used in Phase 26 echo handler -- `WebSocketUpgrade`, `WebSocket`, `Message` types confirmed working

### Secondary (MEDIUM confidence)
- https://websocket.org/guides/languages/rust/ -- Rust WebSocket patterns overview
- https://medium.com/@mikecode/axum-websocket-468736a5e1c7 -- axum WS broadcast pattern (Dec 2025)

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in Cargo.toml, no new deps
- Architecture: HIGH -- broadcast pattern is canonical axum, verified via official examples
- Pitfalls: HIGH -- identified from codebase inspection (read-only pool, race condition, lagged receivers)
- Frontend integration: MEDIUM -- React WS handling is standard but ConnectionStatus refactoring needs careful state lifting

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable domain, no fast-moving deps)
