---
phase: 10-centralized-hooks
plan: 02
subsystem: infra
tags: [hooks, bash, deprecation, migration]

# Dependency graph
requires: []
provides:
  - "Deprecation notices in hooks/claude-code.sh and hooks/gemini-cli.sh"
  - "Inline command alternative documented in both hook file headers"
affects: [documentation, onboarding, provider-hook-registration]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Deprecation notices in shell script headers with inline replacement command shown"]

key-files:
  created: []
  modified:
    - hooks/claude-code.sh
    - hooks/gemini-cli.sh

key-decisions:
  - "Deprecation block inserted after shebang (line 1), before existing description header — preserves script identity comment while making notice immediately visible"
  - "All executable logic left completely unchanged — backward compatibility maintained for users on older setups"

patterns-established:
  - "Deprecation pattern: shebang / DEPRECATED block / original header / executable body"

requirements-completed: [HOOK-02]

# Metrics
duration: 1min
completed: 2026-03-09
---

# Phase 10 Plan 02: Centralized Hooks — Deprecation Notices Summary

**DEPRECATED notices added to both provider hook scripts pointing users to the inline `squad-station signal $TMUX_PANE` replacement command**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-09T05:08:10Z
- **Completed:** 2026-03-09T05:08:49Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Added 7-line deprecation block to `hooks/claude-code.sh` after the shebang line, before the existing header comment
- Added 7-line deprecation block to `hooks/gemini-cli.sh` after the shebang line, before the existing header comment
- Both scripts remain syntactically valid bash (`bash -n` passes) with all executable logic unchanged

## Task Commits

Each task was committed atomically:

1. **Task 1: Add deprecation headers to both hook scripts** - `0db7ad7` (feat)

**Plan metadata:** (to be committed with SUMMARY.md)

## Files Created/Modified

- `hooks/claude-code.sh` - Deprecation header prepended; executable body unchanged
- `hooks/gemini-cli.sh` - Deprecation header prepended; executable body unchanged

## Decisions Made

- Deprecation block placed after shebang, before the existing description comment — ensures the notice is the very first thing a user reads after the shebang, while the file's identity (script name + purpose) still follows immediately below.
- No executable logic modified — backward compatibility is preserved by design (HOOK-02 requirement).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Both hook files now clearly communicate the v1.3 migration path to users
- Phase 10 complete: signal pane-ID detection (plan 01) and hook deprecation notices (plan 02) both done
- Ready for Phase 11 (antigravity provider) or any follow-on phases

---
*Phase: 10-centralized-hooks*
*Completed: 2026-03-09*

## Self-Check: PASSED

- hooks/claude-code.sh: FOUND
- hooks/gemini-cli.sh: FOUND
- 10-02-SUMMARY.md: FOUND
- commit 0db7ad7: FOUND
