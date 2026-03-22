---
phase: 26-axum-server-and-cli-command
plan: 02
subsystem: ui
tags: [tailwind, tailwindcss-v4, react, websocket, vite, spa, dark-theme, status-bar]

# Dependency graph
requires:
  - phase: 26-axum-server-and-cli-command-plan-01
    provides: axum server with /ws WebSocket echo, /api/status JSON endpoint, and SPA serving via rust-embed
provides:
  - Tailwind CSS v4 integrated into web/ via @tailwindcss/vite plugin
  - ConnectionStatus.tsx: WebSocket indicator component connecting to /ws with auto-reconnect
  - StatusBar.tsx: REST status bar fetching /api/status every 10s, showing project/agents/uptime/version
  - Updated App.tsx with dark bg-gray-900 layout, top bar with StatusBar + ConnectionStatus, React Flow main area
affects: [27-websocket-streaming, 28-browser-ui-polish]

# Tech tracking
tech-stack:
  added:
    - tailwindcss 4.x (via @tailwindcss/vite Vite plugin, NO postcss.config.js or tailwind.config.js)
    - "@tailwindcss/vite" Vite plugin
  patterns:
    - Tailwind v4 setup: @import "tailwindcss" in index.css + tailwindcss() in vite.config.ts — no config files
    - WebSocket auto-reconnect: ws.onclose triggers setTimeout(connect, 3000); cleanup nulls onclose before ws.close()
    - Status polling: useEffect with fetch + setInterval(10000) + clearInterval cleanup

key-files:
  created:
    - web/src/components/ConnectionStatus.tsx — WebSocket to /ws, colored dot status indicator (connecting/connected/disconnected)
    - web/src/components/StatusBar.tsx — fetches /api/status, displays project name, agent count, uptime, version
  modified:
    - web/vite.config.ts — added tailwindcss() Vite plugin import and plugins array
    - web/src/index.css — replaced comment with @import "tailwindcss"
    - web/src/App.tsx — dark theme layout, top bar with StatusBar + ConnectionStatus, flex-col with flex-1 React Flow
    - web/package.json — added tailwindcss + @tailwindcss/vite dependencies

key-decisions:
  - "Tailwind v4 requires NO postcss.config.js or tailwind.config.js — only @tailwindcss/vite plugin and @import directive"
  - "WebSocket reconnect nulls ws.onclose before calling ws.close() in cleanup to prevent reconnect loop on unmount"
  - "StatusBar silently ignores fetch errors — server may not be ready yet on initial mount"

patterns-established:
  - "Dark dashboard theme: bg-gray-900 base, bg-gray-800 top bar, gray-700 borders, text-gray-100/400/500 hierarchy"
  - "Component auto-cleanup: useEffect returns cleanup function clearing timers and closing WS connections"

requirements-completed: [SRV-01, UI-01]

# Metrics
duration: 10min
completed: 2026-03-22
---

# Phase 26 Plan 02: Tailwind CSS v4 Design System and SPA Data Flow Summary

**Tailwind CSS v4 integrated into React SPA with live WebSocket status indicator and /api/status REST display in a dark-themed dashboard layout**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-03-22T09:49:04Z
- **Completed:** 2026-03-22T09:59:00Z
- **Tasks:** 2 of 2 (Task 2: human-verify checkpoint — approved by user)
- **Files modified:** 6

## Accomplishments
- Tailwind CSS v4 added with zero config files — just Vite plugin + one CSS import line
- ConnectionStatus.tsx connects to /ws on mount, shows green/yellow/red dot with auto-reconnect every 3s
- StatusBar.tsx polls /api/status every 10s, renders project name, agent count, formatted uptime, version
- App.tsx redesigned as dark bg-gray-900 dashboard with flex-col layout: top bar + full-height React Flow graph
- npm run build and cargo build --features browser both pass; cargo build --release --features browser also passes

## Task Commits

Each task was committed atomically:

1. **Task 1: Set up Tailwind CSS v4 and enhance SPA with WS status and API display** - `7a4c661` (feat)
2. **Task 2: Visual verification of complete browser flow** - human-approved (checkpoint)

## Files Created/Modified
- `web/src/components/ConnectionStatus.tsx` — WebSocket connection indicator with colored dot + auto-reconnect
- `web/src/components/StatusBar.tsx` — Periodic /api/status fetch with formatted display
- `web/vite.config.ts` — Added tailwindcss() Vite plugin
- `web/src/index.css` — Tailwind v4 @import directive
- `web/src/App.tsx` — Dark theme layout with top bar and React Flow main area
- `web/package.json` — tailwindcss + @tailwindcss/vite dependencies

## Decisions Made
- Tailwind v4 needs NO postcss.config.js and NO tailwind.config.js — the @tailwindcss/vite plugin handles everything. Just add @import "tailwindcss" to index.css.
- WebSocket cleanup nulls `ws.onclose` before calling `ws.close()` to prevent the reconnect timer from firing during component unmount.
- StatusBar silently catches fetch errors so the component does not crash if /api/status is briefly unavailable.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Release binary ready at target/release/squad-station — run `squad-station browser` from a project with squad.yml
- Human verified (Task 2 checkpoint): React Flow graph renders, green WS indicator shows, status bar displays project/agents/uptime/version, Ctrl+C stops cleanly, --port and --no-open flags work, non-feature build prints "not enabled" message
- Phase 27 can begin: server /ws endpoint is a placeholder echo — replace with real event-driven streaming of agent/message state changes

---
*Phase: 26-axum-server-and-cli-command*
*Completed: 2026-03-22*
