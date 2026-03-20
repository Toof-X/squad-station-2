---
phase: 12-ide-context-hook-setup
plan: 02
subsystem: hooks
tags: [rust, serde_json, settings-json, init, hooks, tdd]

# Dependency graph
requires:
  - phase: 10-centralized-hooks
    provides: signal command with TMUX_PANE resolution
  - phase: 11-antigravity-provider-core
    provides: antigravity tool (DB-only orchestrator, no tmux)
provides:
  - Auto-merge of squad-station hook entry into .claude/settings.json on init
  - Auto-merge of squad-station hook entry into .gemini/settings.json on init
  - Backup (.json.bak) before any settings.json modification
  - Idempotent merge (no duplicate entries on re-run)
  - Stdout instructions fallback when settings.json does not exist
affects: [docs, e2e-testing, user-onboarding]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - serde_json Value mutation with graceful fallback on malformed JSON
    - JSON-mode guard in init prevents stdout pollution for machine-parseable output

key-files:
  created: []
  modified:
    - src/commands/init.rs
    - tests/test_integration.rs

key-decisions:
  - "JSON mode skips hook instructions on stdout — preserves machine-parseable output for --json flag"
  - "merge_hook_entry uses path.with_extension('json.bak') for correct .json.bak extension"
  - "Graceful error: merge failure falls through to print_hook_instructions, never aborts init"
  - "Deduplication on 'command' field value — same as provider detection pattern"

patterns-established:
  - "Hook merge: backup then parse with fallback json!({}), ensure nested keys, dedup-append, pretty-write"
  - "Provider settings paths checked sequentially: .claude/settings.json (Stop), .gemini/settings.json (AfterAgent)"

requirements-completed: [HOOK-03, HOOK-04]

# Metrics
duration: 6min
completed: 2026-03-09
---

# Phase 12 Plan 02: Settings.json Hook Merge Summary

**Auto-merges squad-station Stop/AfterAgent hook entries into .claude/settings.json and .gemini/settings.json on `init`, with .bak backup, idempotent dedup, and stdout instructions fallback when no settings file exists.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-09T07:48:07Z
- **Completed:** 2026-03-09T07:54:00Z
- **Tasks:** 2 (RED + GREEN — TDD cycle)
- **Files modified:** 2

## Accomplishments
- `merge_hook_entry` function: backup, parse JSON gracefully, ensure hooks object/event array, dedup-append entry, pretty-write back
- `print_hook_instructions` function: formatted stdout snippet showing the hooks JSON structure when file is absent
- Full TDD cycle: 5 failing tests written first, then implementation made all 5 pass
- Idempotent: re-running `init` on a project that already has the hook entry produces exactly 1 entry in the array
- Graceful: malformed JSON in settings file falls through to instructions branch without aborting init

## Task Commits

Each task was committed atomically:

1. **Task 1: RED — failing hook tests** - `e663434` (test)
2. **Task 2: GREEN — implement merge logic** - `8ae1064` (feat)

_Note: TDD plan — two commits per RED/GREEN cycle._

## Files Created/Modified
- `src/commands/init.rs` — Added `merge_hook_entry`, `print_hook_instructions` helpers and step 9 hook setup block in `run()`
- `tests/test_integration.rs` — Added 5 new hook tests: backup creation, entry addition, idempotency, instructions fallback, Gemini AfterAgent

## Decisions Made
- **JSON mode guard:** When `--json` flag is active, hook instructions are suppressed from stdout to keep the output machine-parseable. This was discovered as a regression fix (Rule 1 — bug) and applied inline.
- **path.with_extension("json.bak"):** Used instead of string format, correctly handling the `.json` extension to produce `.json.bak`.
- **Graceful fallback:** `unwrap_or_else(|_| serde_json::json!({}))` on parse — any malformed settings.json silently gets treated as `{}` and instructions are printed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] JSON mode printed hook instructions to stdout, breaking --json parsing**
- **Found during:** Task 2 (GREEN — after full test suite run)
- **Issue:** `test_init_antigravity_orchestrator_skips_tmux` failed with JSON parse error because `print_hook_instructions` was printing to stdout unconditionally, corrupting the single-line JSON output
- **Fix:** Wrapped step 9 in `if !json { ... }` guard so hook instructions only appear in human-readable mode
- **Files modified:** `src/commands/init.rs`
- **Verification:** Full test suite passes — 46 integration tests green, 0 failures
- **Committed in:** `8ae1064` (part of GREEN task commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug)
**Impact on plan:** Fix essential for correctness — JSON mode contract must not be broken. No scope creep.

## Issues Encountered
- None beyond the JSON mode regression documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- HOOK-03 and HOOK-04 requirements complete
- `squad-station init` now handles full hook setup automatically for both Claude Code and Gemini CLI
- No manual editing of settings.json needed after init
- Phase 12 plan 03 (if any) can proceed

---
*Phase: 12-ide-context-hook-setup*
*Completed: 2026-03-09*

## Self-Check: PASSED

- FOUND: `.planning/phases/12-ide-context-hook-setup/12-02-SUMMARY.md`
- FOUND: commit `e663434` (test RED)
- FOUND: commit `8ae1064` (feat GREEN)
- Full test suite: 46 integration tests passed, 0 failed
