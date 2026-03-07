---
phase: 02-lifecycle-and-hooks
plan: "02"
subsystem: cli
tags: [rust, clap, tmux, owo-colors, shell-hooks, markdown]

# Dependency graph
requires:
  - phase: 02-01
    provides: signal command with lifecycle state updates (send=busy, signal=idle), update_agent_status DB function
  - phase: 01-core-foundation
    provides: agents DB layer (list_agents, update_agent_status), tmux::session_exists, config/db connect pattern
provides:
  - agents subcommand with live tmux reconciliation and colored status+duration table
  - context subcommand generating Markdown orchestrator briefing with agent roster and usage guide
  - hooks/claude-code.sh for Claude Code Stop event integration
  - hooks/gemini-cli.sh for Gemini CLI AfterAgent event integration
affects: [03-packaging-and-distribution, integration-tests]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "tmux reconciliation loop: session_exists per agent, update dead/auto-revive idle on status mismatch"
    - "ANSI-safe table padding: pad_colored(raw, colored, width) computes padding from raw text length"
    - "Hook script design: drain stdin, detect TMUX_PANE, extract session name, delegate all logic to binary, always exit 0"

key-files:
  created:
    - src/commands/agents.rs
    - src/commands/context.rs
    - hooks/claude-code.sh
    - hooks/gemini-cli.sh
  modified:
    - src/cli.rs
    - src/commands/mod.rs
    - src/main.rs

key-decisions:
  - "context command has no --json flag -- always outputs Markdown for AI consumption, not TTY display"
  - "Reconciliation loop is duplicated in agents.rs and context.rs (DRY trade-off) -- coupling two independent command files via shared function adds more complexity than the ~10 line duplication"
  - "Hook scripts use SQUAD_STATION_BIN env var to allow custom binary path override"
  - "Hook scripts use TMUX_PANE (not TMUX) for tmux detection -- TMUX_PANE is the reliable pane identifier"

patterns-established:
  - "pad_colored pattern: always compute padding from raw text length to avoid ANSI escape bytes corrupting column alignment"
  - "Hook exit 0 invariant: every code path in hook scripts must exit 0 to prevent provider interruption"

requirements-completed: [SESS-03, SESS-04, SESS-05, HOOK-02]

# Metrics
duration: 3min
completed: 2026-03-06
---

# Phase 2 Plan 02: Agents Command, Context Command, and Provider Hook Scripts Summary

**agents/context CLI commands with live tmux reconciliation, dead-agent auto-revive, and provider hook scripts that drain stdin and delegate all signal logic to the Rust binary**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-06T06:56:42Z
- **Completed:** 2026-03-06T06:58:52Z
- **Tasks:** 3
- **Files modified:** 7

## Accomplishments

- agents subcommand reconciles each agent's DB status against tmux reality on every invocation, auto-reviving dead agents when sessions reappear
- context subcommand generates a self-contained Markdown document suitable for pasting into orchestrator prompts
- Two provider hook scripts (Claude Code Stop, Gemini CLI AfterAgent) that always exit 0 and delegate all guard logic to the binary

## Task Commits

Each task was committed atomically:

1. **Task 1: agents command with tmux reconciliation and CLI wiring** - `3a0bb64` (feat)
2. **Task 2: context command for orchestrator Markdown output** - `1aee0f3` (feat)
3. **Task 3: provider hook scripts for Claude Code and Gemini CLI** - `61259cf` (feat)

**Plan metadata:** (docs commit pending)

## Files Created/Modified

- `src/commands/agents.rs` - agents command: tmux reconciliation loop, colored status+duration table output
- `src/commands/context.rs` - context command: Markdown roster with agent table and usage guide
- `hooks/claude-code.sh` - Claude Code Stop event hook, drains stdin, delegates to squad-station signal
- `hooks/gemini-cli.sh` - Gemini CLI AfterAgent hook, drains stdin, delegates to squad-station signal
- `src/cli.rs` - Added Agents and Context variants to Commands enum
- `src/commands/mod.rs` - Added agents and context module declarations
- `src/main.rs` - Added routing for Agents and Context match arms

## Decisions Made

- context command has no --json flag — always outputs plain Markdown for AI consumption
- Reconciliation loop duplicated in agents.rs and context.rs rather than shared to avoid coupling two independent command files
- SQUAD_STATION_BIN env var allows custom binary path in hook scripts
- Hook scripts use $TMUX_PANE (not $TMUX) — $TMUX_PANE is the reliable pane identifier for session name resolution

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - `cargo build` succeeded on first attempt after creating context.rs (build failed before context.rs existed, as expected since mod.rs declared the module).

## User Setup Required

To wire Claude Code: add `hooks/claude-code.sh` to `.claude/settings.json` under the `Stop` hook event.
To wire Gemini CLI: add `hooks/gemini-cli.sh` to `.gemini/settings.json` under the `AfterAgent` hook event.

No automatic configuration — manual hook registration by user.

## Next Phase Readiness

- Phase 2 complete: all lifecycle commands (signal, agents, context) and hook scripts implemented
- Ready for Phase 3 (packaging and distribution)
- Hook scripts are ready for users to configure in Claude Code and Gemini CLI settings

---
*Phase: 02-lifecycle-and-hooks*
*Completed: 2026-03-06*
