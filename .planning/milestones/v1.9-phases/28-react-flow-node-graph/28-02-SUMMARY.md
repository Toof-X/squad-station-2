---
phase: 28-react-flow-node-graph
plan: 02
subsystem: ui
tags: [react, react-flow, typescript, animation, theme, dark-mode, tailwind]

requires:
  - phase: 28-react-flow-node-graph/28-01
    provides: AgentNode, useGraphLayout hook, App.tsx with nodeTypes, edges with animated data

provides:
  - AnimatedEdge: SVG animateMotion crawling dots (3 staggered particles) for in-flight messages
  - EdgeLabelRenderer: task text, priority badge, relative timestamp at edge midpoints
  - useTheme: dark/light hook with localStorage persistence (key 'squad-theme', default dark)
  - ThemeToggle: sun/moon toggle button in header bar
  - Tailwind v4 dark mode via @custom-variant directive

affects:
  - phase 28 visual verification (Task 3 checkpoint)

tech-stack:
  added: []
  patterns:
    - "edgeTypes at module level alongside nodeTypes — same critical React Flow pattern"
    - "useTheme: one useEffect per concern (DOM class sync, localStorage persist)"
    - "Tailwind v4 dark mode via @custom-variant dark (&:where(.dark, .dark *)) in index.css"
    - "React Flow colorMode prop synced with theme state for internal dark/light styling"

key-files:
  created:
    - web/src/components/AnimatedEdge.tsx
    - web/src/hooks/useTheme.ts
    - web/src/components/ThemeToggle.tsx
  modified:
    - web/src/App.tsx
    - web/src/hooks/useGraphLayout.ts
    - web/src/index.css
    - web/src/components/AgentNode.tsx
    - web/src/components/ConnectionStatus.tsx
    - web/src/components/StatusBar.tsx

key-decisions:
  - "3 staggered SVG animateMotion circles (0s/0.66s/1.33s at 2s duration) for GPU-accelerated crawling dots — no JS animation loop"
  - "Edge labels use EdgeLabelRenderer (not foreignObject) for correct z-index and pointer event handling in React Flow"
  - "All edges use type 'animated' (not 'default') — AnimatedEdge handles both static and animated states via data.animated flag"
  - "ThemeToggle placed between StatusBar and ConnectionStatus in header bar"
  - "useTheme reads localStorage synchronously in useState initializer to prevent flash-of-wrong-theme"

patterns-established:
  - "AnimatedEdge pattern: BaseEdge + conditional SVG circles + conditional EdgeLabelRenderer in one component"
  - "Dark mode pattern: @custom-variant dark in index.css + .dark class on documentElement + colorMode on ReactFlow"

requirements-completed: [VIZ-03, VIZ-04, UI-02]

duration: 12min
completed: 2026-03-22
---

# Phase 28 Plan 02: Animated Edges and Theme System Summary

**SVG animateMotion crawling-dots edges for in-flight messages, dark/light theme toggle with localStorage persistence, and full Tailwind dark: variant coverage across all UI components**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-03-22T12:51:15Z
- **Completed:** 2026-03-22T13:03:00Z
- **Tasks:** 2 of 3 complete (Task 3 is human-verify checkpoint)
- **Files modified:** 9

## Accomplishments

- `AnimatedEdge` component: `<BaseEdge>` + 3 `<animateMotion>` SVG circles staggered at 0s/0.66s/1.33s with 2s duration — GPU-accelerated crawling blue dots on in-flight edges
- `EdgeLabelRenderer` label: task text truncated to 30 chars, priority badge (urgent=red, high=orange, normal=gray), relative timestamp (Xm/Xh ago)
- `useTheme` hook: reads localStorage on init (no theme flash), syncs `.dark` class to `documentElement`, persists on change — default dark
- `ThemeToggle` button: unicode sun/moon icons, accessible `aria-label`, placed in header between StatusBar and ConnectionStatus
- `@custom-variant dark` in index.css enables Tailwind v4 dark mode; `colorMode={theme}` syncs React Flow internals
- All components updated with `dark:` Tailwind variants: AgentNode, ConnectionStatus, StatusBar, AnimatedEdge labels

## Task Commits

Each task was committed atomically:

1. **Task 1: Create AnimatedEdge component with crawling dots and edge labels** - `35be19c` (feat)
2. **Task 2: Implement dark/light theme system with toggle and component updates** - `d8ffdda` (feat)
3. **Task 3: Visual verification of complete Phase 28 feature set** - awaiting human checkpoint

## Files Created/Modified

- `web/src/components/AnimatedEdge.tsx` - Custom React Flow edge: SVG crawling dots + EdgeLabelRenderer with task/priority/timestamp
- `web/src/hooks/useTheme.ts` - Theme state hook: localStorage persistence, DOM class sync, default dark
- `web/src/components/ThemeToggle.tsx` - Sun/moon toggle button with accessible aria-label
- `web/src/App.tsx` - Added AnimatedEdge, ThemeToggle, useTheme; edgeTypes at module level; colorMode prop; theme-aware root div
- `web/src/hooks/useGraphLayout.ts` - Changed edge type from 'default' to 'animated'
- `web/src/index.css` - Added @custom-variant dark + React Flow dark mode background override
- `web/src/components/AgentNode.tsx` - Added dark: variants for card, text, tooltip
- `web/src/components/ConnectionStatus.tsx` - Added dark: text variant
- `web/src/components/StatusBar.tsx` - Added dark: bg and text variants

## Decisions Made

- Used 3 `<animateMotion>` SVG circles with staggered `begin` instead of requestAnimationFrame — SVG animation is GPU-accelerated and simpler
- `EdgeLabelRenderer` preferred over `foreignObject` — correct z-index, pointer events work as expected in React Flow
- All edges use `type: 'animated'` so custom edge renders consistently for both animated/static states (component handles both via `data.animated`)
- `localStorage.getItem` in `useState` initializer (not `useEffect`) prevents initial render with wrong theme

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None beyond the checkpoint awaiting human verification.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All automated work for Phase 28 is complete — build succeeds, all components have theme support
- Task 3 (human checkpoint) requires visual verification of: animated edges during in-flight messages, edge labels, theme toggle, theme persistence
- Upon approval, Phase 28 (and milestone v1.9) is complete

---
*Phase: 28-react-flow-node-graph*
*Completed: 2026-03-22*
