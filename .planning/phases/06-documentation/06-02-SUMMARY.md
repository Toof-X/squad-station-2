---
phase: 06-documentation
plan: 02
subsystem: documentation
tags: [cli, squad-station, playbook, v1.1, send, tool, register, context, signal]

requires:
  - phase: 05-feature-completion
    provides: v1.1 CLI changes (--body flag, tool rename, context Markdown format, signal format)
  - phase: 06-documentation
    provides: 06-RESEARCH.md with complete gap inventory for PLAYBOOK.md

provides:
  - Corrected PLAYBOOK.md reflecting all v1.1 CLI syntax, config format, and naming conventions
  - Accurate send --body flag documentation (CLI-01)
  - Accurate tool field documentation (CONF-04)
  - Accurate agent naming convention (<project>-<tool>-<role_suffix>) (CLI-02)
  - Accurate context output format (Markdown sections per agent) (CLI-03)
  - Accurate signal format (<agent> completed <msg-id>) (SIG-01)

affects: [users following playbook to orchestrate squads]

tech-stack:
  added: []
  patterns:
    - "PLAYBOOK.md as living reference: update atomically whenever CLI/config changes land"

key-files:
  created: []
  modified:
    - docs/PLAYBOOK.md

key-decisions:
  - "Corrected all squad.yml examples to use tool field (not provider), no command field, project as plain string"
  - "All send examples updated to --body named flag syntax with full auto-prefixed agent names"
  - "register command example updated to --tool flag; legacy --command flag removed from example"
  - "Context output section replaced table with Markdown section-per-agent format matching CLI-03"
  - "Signal format section documents <agent> completed <msg-id> matching SIG-01"
  - "Agent naming convention explanation added: name field is role suffix, full name is auto-prefixed"

patterns-established:
  - "Doc accuracy: zero occurrences of stale field names (provider) as a measurable correctness criterion"

requirements-completed:
  - DOCS-02

duration: 2min
completed: 2026-03-08
---

# Phase 6 Plan 02: PLAYBOOK.md v1.1 Update Summary

**PLAYBOOK.md fully rewritten for v1.1: --body flag on send, tool field throughout, auto-prefixed agent naming, Markdown context output, and correct signal format**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-08T12:28:22Z
- **Completed:** 2026-03-08T12:29:47Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- All send command examples updated from positional argument to `--body` named flag (CLI-01)
- All `provider` occurrences removed and replaced with `tool` — zero remaining (CONF-04)
- squad.yml example updated: flat `project: my-app` string, `tool` field, no `command` field (CONF-01, CONF-03, CONF-04)
- Agent naming convention documented: `<project>-<tool>-<role_suffix>` auto-prefix pattern (CLI-02)
- Section 9 context output replaced table with Markdown section-per-agent format (CLI-03)
- Signal format updated to `<agent> completed <msg-id>` in both Section 7 and workflow diagram (SIG-01)
- `register` command example updated to `--tool` flag (removed stale `--provider` and `--command` flags)
- Command reference table updated with `--body` and `--tool` flags

## Task Commits

1. **Task 1: Update PLAYBOOK.md with post-v1.1 CLI syntax and config format** - `f2a1802` (feat)

**Plan metadata:** (docs commit — follows below)

## Files Created/Modified

- `docs/PLAYBOOK.md` — Complete rewrite for v1.1: --body flag, tool field, agent naming, Markdown context, signal format

## Decisions Made

- Kept all section headings in place (Section 1 through Troubleshooting) — updated content in-place per plan instruction
- Updated agent name examples throughout to use full auto-prefixed names (e.g., `my-app-claude-code-frontend`) for consistency with v1.1 naming convention
- Hook script filenames updated to match Phase 5 deliverables: `claude-code-notify.sh` and `gemini-cli-notify.sh`

## Deviations from Plan

None — plan executed exactly as written. All gap corrections from RESEARCH.md applied in a single task.

## Issues Encountered

None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

Phase 6 documentation complete (DOCS-01 and DOCS-02 both done). No blockers. Both ARCHITECTURE.md and PLAYBOOK.md now accurately describe the v1.1 implementation.

---
*Phase: 06-documentation*
*Completed: 2026-03-08*
