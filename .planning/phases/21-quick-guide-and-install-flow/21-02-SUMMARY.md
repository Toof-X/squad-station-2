---
phase: 21-quick-guide-and-install-flow
plan: "02"
subsystem: install
tags: [npm, shell, tty, auto-launch, install-script]

# Dependency graph
requires:
  - phase: 20-tty-safe-welcome-tui-core
    provides: squad-station binary with welcome TUI that is auto-launched after install
provides:
  - TTY-guarded auto-launch in npm install path (run.js spawnSync)
  - TTY-guarded auto-launch in curl install path (install.sh exec)
  - installBinary() returns destPath for use by install()
affects: [first-run-onboarding, install-paths]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - TTY-guard pattern using process.stdout.isTTY in Node.js
    - TTY-guard pattern using [ -t 1 ] in POSIX shell
    - exec handoff (replaces shell process cleanly) in curl installer

key-files:
  created: []
  modified:
    - npm-package/bin/run.js
    - install.sh

key-decisions:
  - "installBinary() returns destPath to avoid PATH uncertainty in auto-launch — uses full absolute path"
  - "TTY check only (no CI env var guards) — process.stdout.isTTY / [ -t 1 ] is sufficient per locked decision"
  - "npm path uses spawnSync to block until TUI exits; curl path uses exec for clean process replacement"
  - "TTY guard placement: bottom of install() after scaffoldProject() and 'Next steps' output"

patterns-established:
  - "TTY guard before auto-launch: if (process.stdout.isTTY) in Node.js, if [ -t 1 ] in shell"
  - "Pass destPath directly from installBinary() to avoid PATH resolution ambiguity"

requirements-completed:
  - INSTALL-01
  - INSTALL-02
  - INSTALL-03

# Metrics
duration: 8min
completed: 2026-03-18
---

# Phase 21 Plan 02: Quick Guide and Install Flow - Auto-Launch Summary

**TTY-guarded auto-launch of squad-station welcome TUI added to both npm (spawnSync via destPath) and curl (exec via INSTALL_DIR) install paths, with silent degradation in non-interactive environments.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-18T02:30:00Z
- **Completed:** 2026-03-18T02:38:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- `installBinary()` in run.js now returns `destPath` (both early-return and end-of-function paths)
- `install()` captures `var destPath = installBinary()` and uses it in the TTY guard
- npm install path: `if (process.stdout.isTTY) { spawnSync(destPath, [], { stdio: 'inherit' }); }` at bottom of `install()`
- curl install path: `if [ -t 1 ]; then exec "${INSTALL_DIR}/squad-station"; fi` at end of `install.sh`
- Non-interactive environments (CI, pipes, sudo, etc.) skip the launch block silently

## Task Commits

Each task was committed atomically:

1. **Task 1: Add TTY-guarded auto-launch to npm install and curl installer** - `99b3c12` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `npm-package/bin/run.js` - installBinary() returns destPath; install() captures it; TTY guard at bottom
- `install.sh` - TTY-guarded exec block added after final echo/FALLBACK check

## Decisions Made
- Used full `destPath` returned from `installBinary()` rather than relying on PATH resolution — avoids edge case where ~/.local/bin is not yet in PATH at launch time
- TTY check only (`process.stdout.isTTY` / `[ -t 1 ]`) per locked decision — no CI env var detection
- `exec` (not subshell call) in curl installer for clean process handoff; trap cleanup fires on exec but TMPFILE already moved to final location, so rm -f is a no-op
- `spawnSync` in npm path blocks until the TUI exits, which is the correct behavior (install command should not return until the user quits the TUI)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Both install paths now auto-launch the welcome TUI in interactive terminals (INSTALL-01, INSTALL-02, INSTALL-03 satisfied)
- Phase 21 is complete — both plans (quick guide TUI page and auto-launch install flow) are done
- Ready for v1.7 milestone release

---
*Phase: 21-quick-guide-and-install-flow*
*Completed: 2026-03-18*

## Self-Check: PASSED

- npm-package/bin/run.js: FOUND
- install.sh: FOUND
- 21-02-SUMMARY.md: FOUND
- Commit 99b3c12: FOUND
