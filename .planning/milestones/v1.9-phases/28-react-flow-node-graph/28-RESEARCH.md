# Phase 28: React Flow Node Graph - Research

**Researched:** 2026-03-22
**Domain:** React Flow v12 custom nodes/edges, dagre layout, Tailwind v4 theming
**Confidence:** HIGH

## Summary

This phase transforms the existing static placeholder React Flow graph into a live, data-driven node graph visualization. The existing codebase already has `@xyflow/react` v12.10.1, React 19, and Tailwind v4 installed. The `useSquadWebSocket` hook provides real-time `agents[]` and `messages[]` data. The work is entirely frontend: create custom node components (agent cards with status dots), custom animated edge components (crawling dots for in-flight messages), hierarchical auto-layout via dagre, edge labels via `EdgeLabelRenderer`, and dark/light theme toggling with Tailwind v4's `@custom-variant` directive.

React Flow v12 has built-in `colorMode` prop support for dark/light themes, which pairs well with Tailwind's class-based dark mode. The dagre library (`@dagrejs/dagre`) is the standard choice for hierarchical layout with React Flow -- lightweight, fast, and well-documented in official React Flow examples. Custom animated edges using SVG `<animateMotion>` provide the crawling dots effect without performance overhead from stroke-dasharray approaches.

**Primary recommendation:** Use dagre for layout, custom node components with Handle/tooltip patterns, custom edge components with SVG `<animateMotion>` for crawling dots, and React Flow's built-in `colorMode` prop combined with Tailwind v4 `@custom-variant dark` for theming.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Each node shows: icon (visual identifier), agent name, and color-coded status dot/badge
- Status color coding: green = busy (actively working), gray = idle (waiting), red = dead (session gone)
- Detailed info (role, model, description, current task, routing hints) appears in a tooltip on hover -- not cluttering the node face
- Keep nodes compact -- the graph should be scannable at a glance
- Use crawling dots animation style on edges to indicate message flow (in-flight/processing messages)
- Animation runs continuously while a message has `status: "processing"` between two agents; stops when message completes
- For MVP: simple is best -- one animated edge per in-flight message, no stacking/multiplexing of concurrent messages on the same edge
- Edge labels or tooltips show message task text, priority, and timestamp per VIZ-04
- Theme toggle button lives in the top-right corner of the UI (near the existing ConnectionStatus indicator)
- Theme preference persists in localStorage -- survives page refresh and reconnection
- Default theme remains dark (matches current bg-gray-900 baseline from Phase 26)
- Hierarchical layout: orchestrator node at top, worker nodes arranged below
- Layout derived automatically from squad.yml topology (orchestrator role vs worker roles) -- no manual positioning
- Graph is pannable and zoomable (React Flow built-in controls) for scalability with many agents
- fitView on initial load so the entire topology is visible without manual adjustment
- Should remain usable with 10-20+ agents -- workers wrap or space evenly below orchestrator

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

### Deferred Ideas (OUT OF SCOPE)
None captured during discussion.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VIZ-01 | Each agent rendered as a React Flow node showing name, role, model, and live status with color coding | Custom node component with Handle, tooltip on hover, status dot badge -- patterns documented in Architecture section |
| VIZ-02 | Hierarchical auto-layout -- orchestrator at top, workers below -- derived from topology | dagre library with `rankdir: 'TB'` and `getLayoutedElements` helper -- full pattern in Code Examples |
| VIZ-03 | Continuous animated arrows on edges while message is in-flight | Custom edge with SVG `<animateMotion>` crawling dots -- pattern documented in Code Examples |
| VIZ-04 | Edge labels or tooltips showing message task, priority, and timestamp | `EdgeLabelRenderer` component for HTML labels on edges -- pattern in Code Examples |
| UI-02 | Dark and light theme support with toggle | React Flow `colorMode` prop + Tailwind v4 `@custom-variant dark` + localStorage persistence |
| UI-03 | Connection status indicator showing WebSocket state | Already implemented in ConnectionStatus.tsx -- needs minor theme-aware styling updates |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@xyflow/react` | ^12.10.1 | Node graph rendering, custom nodes/edges, pan/zoom | Already installed; official React Flow v12 package |
| `react` | ^19.2.4 | UI framework | Already installed |
| `tailwindcss` | ^4.2.2 | Utility CSS, dark mode variants | Already installed; v4 with `@custom-variant` |
| `@dagrejs/dagre` | ^2.0.4 | Hierarchical graph layout (DAG) | Official React Flow recommended layout library; lightweight, fast |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tailwindcss/vite` | ^4.2.2 | Tailwind Vite integration | Already installed |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| dagre | elkjs | elkjs is more powerful (supports compound graphs, constraints) but heavier (~200KB vs ~30KB); dagre is sufficient for simple orchestrator-worker hierarchy |
| dagre | Manual position calc | Simpler for 2-level tree but breaks when agent count varies; dagre handles spacing automatically |
| Custom tooltip | Floating UI / Radix | Overkill for MVP; native CSS `:hover` + absolute positioning or `title` attr is sufficient |

**Installation:**
```bash
cd web && npm install @dagrejs/dagre
```

Note: `@dagrejs/dagre` ships with TypeScript types included -- no `@types/` package needed for v2.x.

## Architecture Patterns

### Recommended Project Structure
```
web/src/
  App.tsx                    # Main layout: theme provider, ReactFlow wrapper
  main.tsx                   # React entry point (unchanged)
  index.css                  # Tailwind imports + @custom-variant dark + React Flow dark overrides
  hooks/
    useSquadWebSocket.ts     # Existing WS hook (unchanged)
    useTheme.ts              # Theme state hook: dark/light toggle, localStorage persistence
    useGraphLayout.ts        # Dagre layout: agents[] + messages[] -> positioned nodes[] + edges[]
  components/
    ConnectionStatus.tsx     # Existing (minor theme updates)
    StatusBar.tsx            # Existing (minor theme updates)
    ThemeToggle.tsx          # Sun/moon icon button in top-right corner
    AgentNode.tsx            # Custom React Flow node: icon + name + status dot + tooltip
    AnimatedEdge.tsx         # Custom edge: crawling dots via SVG animateMotion
    AgentTooltip.tsx         # Tooltip content for agent hover (role, model, task, etc.)
```

### Pattern 1: Custom Node with Status Badge
**What:** A React Flow custom node component that renders an agent card with icon, name, and color-coded status dot. Tooltip shows full details on hover.
**When to use:** For every agent in the `agents[]` array.
**Key approach:** Register via `nodeTypes` prop on `<ReactFlow>`. Use `Handle` components for top (target) and bottom (source) connections. Status dot color derived from `agent.status` field.

### Pattern 2: Derived Graph State
**What:** A pure function (or `useMemo` hook) that transforms `agents[]` and `messages[]` into React Flow `Node[]` and `Edge[]`, then applies dagre layout.
**When to use:** Every time agents or messages update from WebSocket.
**Key approach:**
1. Map agents to nodes (orchestrator identified by role)
2. Create edges from orchestrator to each worker (structural edges always present)
3. Mark edges as animated when a message with `status === "processing"` exists between those agents
4. Run dagre layout to assign positions
5. Memoize with `useMemo` to avoid re-layout on every render

### Pattern 3: React Flow colorMode + Tailwind Dark Mode
**What:** Sync React Flow's built-in dark mode with Tailwind's dark class.
**When to use:** Theme toggle affects both React Flow internal styling and all Tailwind-styled components.
**Key approach:** React Flow's `colorMode` prop accepts `'dark' | 'light'`. Adding `.dark` class to root element enables Tailwind's `dark:` variants. Both are controlled by the same `useTheme` hook state.

### Anti-Patterns to Avoid
- **Storing positioned nodes in state and re-running dagre on every WS update:** Layout should only re-run when the set of agents changes (add/remove), not on status updates. Use separate memos for layout vs styling.
- **Using `useNodesState`/`useEdgesState` for WS-driven data:** These hooks are for user-interactive graphs. Since our graph is read-only (data comes from WS), derive nodes/edges with `useMemo` and pass directly to `<ReactFlow>`.
- **Animating with stroke-dasharray CSS:** Performance-heavy for multiple edges. SVG `<animateMotion>` is GPU-accelerated and more efficient.
- **Inline `nodeTypes`/`edgeTypes` objects in JSX:** Creates new object reference every render, causing React Flow to re-mount all nodes/edges. Define outside component or use `useMemo`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Graph layout algorithm | Custom position calculation | `@dagrejs/dagre` | Handles variable node counts, spacing, edge routing automatically |
| Pan/zoom/viewport | Custom mouse handlers | React Flow built-in (already works) | Gesture handling, touch support, minimap integration |
| Edge path calculation | Manual SVG path math | `getBezierPath`/`getSmoothStepPath` from `@xyflow/react` | Handles handle positions, curvature, label midpoint calculation |
| Dark mode CSS variables | Manual React Flow restyling | React Flow `colorMode` prop | Automatically adjusts ~30 CSS variables for nodes, edges, controls, background |
| Edge label positioning | Manual coordinate math | `EdgeLabelRenderer` + `labelX`/`labelY` from path functions | Handles viewport transforms, keeps labels in correct coordinate space |

**Key insight:** React Flow v12 has mature theming, layout integration, and edge customization APIs. The main custom work is the node component (visual design) and the data transformation layer (agents/messages to nodes/edges).

## Common Pitfalls

### Pitfall 1: nodeTypes/edgeTypes Object Recreation
**What goes wrong:** Defining `nodeTypes` or `edgeTypes` inline in JSX creates a new object every render, causing React Flow to unmount and remount all nodes/edges, killing performance and animation state.
**Why it happens:** React Flow uses referential equality to check if types changed.
**How to avoid:** Define `nodeTypes` and `edgeTypes` as module-level constants outside the component.
**Warning signs:** Nodes flash/flicker on every state update; animations restart randomly.

### Pitfall 2: Dagre Layout on Every Render
**What goes wrong:** Running dagre layout computation on every WebSocket message update causes jank and unnecessary position recalculations.
**Why it happens:** Messages update frequently (status changes), but node positions only need recalculating when agents are added/removed.
**How to avoid:** Separate layout computation from edge styling. `useMemo` for layout keyed on agent IDs only; separate `useMemo` for edge animation state keyed on messages.
**Warning signs:** Nodes visibly jump positions when messages update; high CPU usage.

### Pitfall 3: React Flow Dark Mode Without colorMode Prop
**What goes wrong:** Only adding Tailwind `dark:` classes without setting React Flow's `colorMode` prop leaves React Flow's internal elements (grid, controls, minimap, edges) in light mode.
**Why it happens:** React Flow manages its own CSS variables independently of Tailwind.
**How to avoid:** Always pass `colorMode` prop to `<ReactFlow>` component, synced with the theme state.
**Warning signs:** Background grid stays white in dark mode; edge colors look wrong.

### Pitfall 4: Missing Handle Components on Custom Nodes
**What goes wrong:** Edges don't connect to custom nodes; they appear but have no visible connection points.
**Why it happens:** Custom nodes must explicitly render `<Handle>` components -- React Flow doesn't add them automatically.
**How to avoid:** Always include `<Handle type="target" position={Position.Top} />` and `<Handle type="source" position={Position.Bottom} />` in custom nodes.
**Warning signs:** Edges don't render or connect to wrong positions.

### Pitfall 5: EdgeLabelRenderer Coordinate Space
**What goes wrong:** Edge labels appear in wrong position or don't move with the viewport.
**Why it happens:** `EdgeLabelRenderer` renders in screen coordinates; you must apply the correct CSS transform using `labelX` and `labelY` from the path calculation function.
**How to avoid:** Always use `transform: translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` and apply `pointerEvents: 'all'` plus `nopan nodrag` classes.
**Warning signs:** Labels stuck in corner or don't move when panning.

## Code Examples

Verified patterns from official React Flow documentation:

### Custom Agent Node Component
```typescript
// Source: https://reactflow.dev/examples/nodes/custom-node + project-specific adaptation
import { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import type { NodeProps, Node } from '@xyflow/react';

type AgentNodeData = {
  name: string;
  role: string;
  status: 'idle' | 'busy' | 'dead';
  model: string | null;
  description: string | null;
  currentTask: string | null;
};

type AgentNode = Node<AgentNodeData, 'agent'>;

const statusColors: Record<string, string> = {
  busy: 'bg-green-500',
  idle: 'bg-gray-400',
  dead: 'bg-red-500',
};

export const AgentNode = memo(({ data }: NodeProps<AgentNode>) => {
  return (
    <div className="px-3 py-2 rounded-lg border border-gray-600 bg-gray-800 dark:bg-gray-800
                    shadow-md min-w-[120px] text-center relative group">
      <Handle type="target" position={Position.Top} className="!bg-gray-500" />

      {/* Status dot */}
      <span className={`absolute top-2 right-2 w-2.5 h-2.5 rounded-full ${statusColors[data.status] ?? 'bg-gray-400'}`} />

      {/* Agent icon + name */}
      <div className="text-sm font-semibold text-gray-100">{data.name}</div>
      <div className="text-xs text-gray-400">{data.role}</div>

      {/* Tooltip on hover */}
      <div className="absolute left-1/2 -translate-x-1/2 top-full mt-2 hidden group-hover:block
                      bg-gray-900 border border-gray-600 rounded-md p-3 text-left text-xs
                      text-gray-300 shadow-xl z-50 min-w-[200px] whitespace-nowrap">
        <div><strong>Model:</strong> {data.model ?? 'N/A'}</div>
        <div><strong>Status:</strong> {data.status}</div>
        {data.currentTask && <div><strong>Task:</strong> {data.currentTask}</div>}
        {data.description && <div><strong>Desc:</strong> {data.description}</div>}
      </div>

      <Handle type="source" position={Position.Bottom} className="!bg-gray-500" />
    </div>
  );
});
```

### Crawling Dots Animated Edge
```typescript
// Source: https://reactflow.dev/examples/edges/animating-edges
import { BaseEdge, EdgeLabelRenderer, getSmoothStepPath } from '@xyflow/react';
import type { EdgeProps, Edge } from '@xyflow/react';

type AnimatedEdgeData = {
  animated: boolean;
  task?: string;
  priority?: string;
  timestamp?: string;
};

type MessageEdge = Edge<AnimatedEdgeData, 'animated'>;

const PARTICLE_COUNT = 3;
const DURATION = 2; // seconds

export function AnimatedEdge({
  id, sourceX, sourceY, targetX, targetY,
  sourcePosition, targetPosition, data,
}: EdgeProps<MessageEdge>) {
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX, sourceY, sourcePosition,
    targetX, targetY, targetPosition,
  });

  return (
    <>
      <BaseEdge id={id} path={edgePath} />

      {/* Crawling dots - only when animated */}
      {data?.animated && Array.from({ length: PARTICLE_COUNT }).map((_, i) => (
        <circle key={i} r="3" fill="#3b82f6">
          <animateMotion
            dur={`${DURATION}s`}
            repeatCount="indefinite"
            path={edgePath}
            begin={`${(i * DURATION) / PARTICLE_COUNT}s`}
          />
        </circle>
      ))}

      {/* Edge label for in-flight messages */}
      {data?.task && (
        <EdgeLabelRenderer>
          <div
            className="nodrag nopan pointer-events-auto bg-gray-800 border border-gray-600
                       rounded px-2 py-1 text-xs text-gray-300 shadow-md"
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
            }}
          >
            <div className="font-medium">{data.task}</div>
            {data.priority && <div className="text-gray-500">{data.priority}</div>}
            {data.timestamp && <div className="text-gray-500">{data.timestamp}</div>}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}
```

### Dagre Layout Helper
```typescript
// Source: https://reactflow.dev/examples/layout/dagre
import dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';

const NODE_WIDTH = 150;
const NODE_HEIGHT = 60;

export function getLayoutedElements(
  nodes: Node[],
  edges: Edge[],
  direction: 'TB' | 'LR' = 'TB',
): { nodes: Node[]; edges: Edge[] } {
  const g = new dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: direction, ranksep: 80, nodesep: 50 });

  nodes.forEach((node) => {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  });

  edges.forEach((edge) => {
    g.setEdge(edge.source, edge.target);
  });

  dagre.layout(g);

  const layoutedNodes = nodes.map((node) => {
    const pos = g.node(node.id);
    return {
      ...node,
      targetPosition: direction === 'TB' ? 'top' : 'left',
      sourcePosition: direction === 'TB' ? 'bottom' : 'right',
      position: {
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
      },
    };
  });

  return { nodes: layoutedNodes, edges };
}
```

### Theme Hook with localStorage Persistence
```typescript
// Source: https://tailwindcss.com/docs/dark-mode + project adaptation
import { useState, useEffect } from 'react';

type Theme = 'dark' | 'light';

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(() => {
    if (typeof window !== 'undefined') {
      return (localStorage.getItem('theme') as Theme) ?? 'dark';
    }
    return 'dark';
  });

  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
    localStorage.setItem('theme', theme);
  }, [theme]);

  const toggleTheme = () => setTheme((t) => (t === 'dark' ? 'light' : 'dark'));

  return { theme, toggleTheme };
}
```

### Tailwind v4 Dark Mode CSS Setup
```css
/* web/src/index.css */
@import "tailwindcss";
@custom-variant dark (&:where(.dark, .dark *));

/* React Flow dark mode overrides (optional, colorMode prop handles most) */
.dark .react-flow {
  --xy-background-color: #111827; /* gray-900 */
}
```

### Registering Custom Types (Module-Level Constants)
```typescript
// IMPORTANT: Define outside component to prevent re-renders
import { AgentNode } from './components/AgentNode';
import { AnimatedEdge } from './components/AnimatedEdge';

const nodeTypes = { agent: AgentNode };
const edgeTypes = { animated: AnimatedEdge };

// In component JSX:
<ReactFlow
  nodes={nodes}
  edges={edges}
  nodeTypes={nodeTypes}
  edgeTypes={edgeTypes}
  colorMode={theme}
  fitView
/>
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `reactflow` package | `@xyflow/react` package | React Flow v12 (2024) | New package name; TypeScript-first; `Node<TData>` generics |
| `useNodesState` for all graphs | Direct props for read-only graphs | React Flow v12 best practice | Simpler code; avoid unnecessary state management for WS-driven data |
| `darkMode: 'class'` in tailwind.config.js | `@custom-variant dark` in CSS | Tailwind v4 (2025) | CSS-first config; no JS config file needed |
| stroke-dasharray edge animation | SVG `<animateMotion>` + custom edge | React Flow edge examples (2024) | Better performance; GPU-accelerated; more visual control |
| `dagre` (original) | `@dagrejs/dagre` v2 | 2024 | Maintained fork with TS types included; same API |

**Deprecated/outdated:**
- `react-flow-renderer`: Old package name, replaced by `@xyflow/react`
- `dagre` (npm): Unmaintained; use `@dagrejs/dagre` instead
- `tailwind.config.js` `darkMode` key: Does not exist in Tailwind v4; use `@custom-variant` in CSS

## Open Questions

1. **Orchestrator identification heuristic**
   - What we know: Agents have a `role` field. CONTEXT.md says "orchestrator identified by role containing 'orchestrator' or by being the agent in `from_agent` fields"
   - What's unclear: Exact role naming conventions in squad.yml -- could be "orchestrator", "Orchestrator", or something else
   - Recommendation: Use case-insensitive match on role containing "orchestrator"; fall back to the agent that appears most as `from_agent` in messages

2. **Edge derivation: structural vs message-based**
   - What we know: CONTEXT.md says one animated edge per in-flight message; edges connect from_agent to to_agent
   - What's unclear: Should edges only appear when there's a message, or should structural edges (orchestrator->worker) always be visible?
   - Recommendation: Always show structural edges from orchestrator to each worker (gray/static); overlay animation when a processing message exists on that edge

3. **dagre TypeScript types in v2**
   - What we know: `@dagrejs/dagre` v2.0.4 claims to include types
   - What's unclear: Whether types are complete or if `@types/dagre` is still needed
   - Recommendation: Install `@dagrejs/dagre` first; add `@types/dagre` only if TS compilation fails

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None currently configured for web/ |
| Config file | None -- see Wave 0 |
| Quick run command | `cd web && npx vitest run --reporter=verbose` |
| Full suite command | `cd web && npx vitest run` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIZ-01 | Agent nodes render with name, role, status dot | manual-only | Visual verification in browser | N/A |
| VIZ-02 | Hierarchical layout: orchestrator top, workers below | unit | `npx vitest run src/hooks/useGraphLayout.test.ts -t "layout"` | No -- Wave 0 |
| VIZ-03 | Animated edges for in-flight messages | manual-only | Visual verification in browser | N/A |
| VIZ-04 | Edge labels show task, priority, timestamp | manual-only | Visual verification in browser | N/A |
| UI-02 | Dark/light theme toggle persists | unit | `npx vitest run src/hooks/useTheme.test.ts -t "theme"` | No -- Wave 0 |
| UI-03 | Connection status indicator | manual-only | Already functional from Phase 27 | N/A |

### Sampling Rate
- **Per task commit:** Visual inspection via `npm run dev`
- **Per wave merge:** `cd web && npm run build` (TypeScript + Vite build succeeds)
- **Phase gate:** Build succeeds + visual verification of all 5 success criteria

### Wave 0 Gaps
- [ ] `web/vitest.config.ts` -- Vitest configuration for web tests (if unit tests are desired)
- [ ] `web/src/hooks/useGraphLayout.test.ts` -- Tests for dagre layout logic (node positioning, orchestrator detection)
- [ ] `web/src/hooks/useTheme.test.ts` -- Tests for theme toggle and localStorage persistence
- [ ] Framework install: `cd web && npm install -D vitest @testing-library/react jsdom` -- if unit tests are desired

Note: Most requirements (VIZ-01, VIZ-03, VIZ-04, UI-03) are visual and best validated through manual browser inspection rather than automated tests. The build check (`npm run build`) serves as the primary automated gate -- TypeScript compilation catches type errors and broken imports.

## Sources

### Primary (HIGH confidence)
- [React Flow Custom Nodes](https://reactflow.dev/examples/nodes/custom-node) - Custom node component patterns, Handle usage, nodeTypes registration
- [React Flow Animating Edges](https://reactflow.dev/examples/edges/animating-edges) - SVG animateMotion crawling dots pattern, BaseEdge usage
- [React Flow AnimatedSVGEdge](https://reactflow.dev/ui/components/animated-svg-edge) - Official AnimatedSVGEdge component API, particle customization
- [React Flow Dagre Layout](https://reactflow.dev/examples/layout/dagre) - Complete dagre integration example, getLayoutedElements helper
- [React Flow Custom Edges](https://reactflow.dev/examples/edges/custom-edges) - EdgeLabelRenderer usage, button edges, label positioning
- [React Flow Theming](https://reactflow.dev/learn/customization/theming) - colorMode prop, CSS variables, dark mode support
- [React Flow NodeProps](https://reactflow.dev/api-reference/types/node-props) - Complete NodeProps type definition
- [Tailwind CSS Dark Mode](https://tailwindcss.com/docs/dark-mode) - v4 @custom-variant directive, class-based toggling, localStorage pattern

### Secondary (MEDIUM confidence)
- [@dagrejs/dagre npm](https://www.npmjs.com/package/@dagrejs/dagre) - Package version (v2.0.4), TypeScript types inclusion

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All core libraries already installed; dagre is React Flow's official recommended layout library
- Architecture: HIGH - Patterns directly from official React Flow v12 docs with verified code examples
- Pitfalls: HIGH - Well-documented issues from React Flow community (nodeTypes recreation, layout performance)

**Research date:** 2026-03-22
**Valid until:** 2026-04-22 (stable libraries, unlikely to change significantly)
