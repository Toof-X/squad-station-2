---
phase: 05-feature-completion
plan: 01
subsystem: hooks
tags: [bash, tmux, claude-code, gemini-cli, notification-hooks, permission-prompt]

# Dependency graph
requires:
  - phase: 02-lifecycle-and-hooks
    provides: "Hook script structural model (claude-code.sh, gemini-cli.sh) and SQUAD_BIN guard pattern"
provides:
  - "hooks/claude-code-notify.sh: Claude Code Notification hook forwarding permission prompts to orchestrator"
  - "hooks/gemini-cli-notify.sh: Gemini CLI Notification hook forwarding permission prompts to orchestrator"
affects: [user-documentation, onboarding, orchestrator-workflow]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Notification hook guard pattern: read-stdin, TMUX_PANE check, AGENT_NAME lookup, SQUAD_BIN guard, JSON parse, orchestrator lookup, tmux send-keys"
    - "TDD for shell scripts: test script with PASS/FAIL counters, RED/GREEN phases using file existence, syntax, exit-code, and content checks"

key-files:
  created:
    - hooks/claude-code-notify.sh
    - hooks/gemini-cli-notify.sh
    - hooks/test-notify-hooks.sh
  modified: []

key-decisions:
  - "Both notify hooks use identical implementation body — only header comments differ (provider-specific registration instructions)"
  - "python3 used for JSON parsing (consistent with project, no additional dependencies)"
  - "orchestrator lookup via 'squad-station agents --json' keeps hooks stateless — no hardcoded session names"
  - "Message format '[NOTIFY] <agent> needs permission: <message>' — prefix enables orchestrator to pattern-match notification messages"

patterns-established:
  - "Notification hook pattern: identical guard chain to Stop hooks, extended with JSON parse + orchestrator discovery"
  - "Shell script TDD: dedicated test script with explicit RED/GREEN phases committed separately"

requirements-completed: [HOOK-01, HOOK-02]

# Metrics
duration: 8min
completed: 2026-03-08
---

# Phase 5 Plan 01: Notification Hooks Summary

**Two bash notification hooks that forward blocked-permission prompts from Claude Code and Gemini CLI agents to the orchestrator's tmux session via `tmux send-keys`**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-08T11:56:34Z
- **Completed:** 2026-03-08T12:04:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `hooks/claude-code-notify.sh`: registers under Claude Code Notification event (permission_prompt matcher), reads JSON from stdin, extracts message, discovers orchestrator via `squad-station agents --json`, forwards via `tmux send-keys -l`
- `hooks/gemini-cli-notify.sh`: identical implementation with Gemini CLI-specific header comments and registration instructions for `.gemini/settings.json`
- Both scripts always exit 0, are no-ops outside tmux, guard against missing binary or unreachable orchestrator session
- TDD test script `hooks/test-notify-hooks.sh` covers 12 cases (existence, executability, syntax, exit-0 behavior, tmux send-keys presence) per hook

## Task Commits

Each task was committed atomically:

1. **RED: Failing tests for both hooks** - `6e9a119` (test)
2. **Task 1 + 2: Both notification hooks** - `122fb4b` (feat)

## Files Created/Modified

- `hooks/claude-code-notify.sh` - Claude Code Notification hook for forwarding permission prompts
- `hooks/gemini-cli-notify.sh` - Gemini CLI Notification hook (identical logic, provider-specific comments)
- `hooks/test-notify-hooks.sh` - TDD test script, 12 tests, all GREEN after implementation

## Decisions Made

- Both notify hooks share identical implementation — only header comments differ (provider-specific registration instructions). This keeps the scripts maintainable and behaviorally consistent.
- `python3` used for JSON parsing inline — consistent with existing project hooks and avoids introducing `jq` as a new dependency.
- Orchestrator session discovered at runtime via `squad-station agents --json` — keeps hooks stateless and works regardless of how the orchestrator was named.
- Message format `[NOTIFY] <agent> needs permission: <message>` uses a prefix the orchestrator can pattern-match to distinguish notification messages from task output.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

Users must register these hooks in their provider settings:

**Claude Code** — add to `.claude/settings.json`:
```json
"hooks": { "Notification": [{ "matcher": "permission_prompt", "hooks": [{ "type": "command", "command": "/path/to/hooks/claude-code-notify.sh" }] }] }
```

**Gemini CLI** — add to `.gemini/settings.json`:
```json
"hooks": { "Notification": [{ "hooks": [{ "type": "command", "command": "/path/to/hooks/gemini-cli-notify.sh" }] }] }
```

## Next Phase Readiness

- HOOK-01 and HOOK-02 requirements closed
- Ready for 05-02 (next plan in phase 5)
- No blockers

## Self-Check: PASSED

- hooks/claude-code-notify.sh: FOUND
- hooks/gemini-cli-notify.sh: FOUND
- 05-01-SUMMARY.md: FOUND
- Commit 6e9a119 (test RED): FOUND
- Commit 122fb4b (feat GREEN): FOUND

---
*Phase: 05-feature-completion*
*Completed: 2026-03-08*
