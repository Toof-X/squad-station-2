# Changelog

All notable changes to Squad Station are documented in this file.

## v0.6.0 — Signal Reliability (2026-03-20)

Three-layer defense against lost agent completion signals. Zero-config hook setup, project-scoped logging, and a self-healing watchdog daemon.

**233 tests passing** (84 lib + 149 integration). E2E validated on kindle-ai-export with 3 running Claude Code agents.

### Added

- **`squad-station reconcile` command** — reconcile agent statuses against live tmux sessions; supports `--dry-run` and `--json` output
- **`squad-station watch` daemon** — background watchdog with 3-pass detection: individual agent reconcile, global stall detection with orchestrator nudge, and prolonged busy warnings; auto-starts on `init`
- **`clean --all` flag** — deletes logs in addition to DB and sessions
- **`providers.rs` module** — centralized provider metadata (idle patterns, hook events, settings paths, fire-and-forget prefixes, alternate buffer detection)
- **`$SQUAD_AGENT_NAME` environment variable** — set in each tmux session at launch for reliable hook identification; eliminates fragile `tmux display-message` in subprocess contexts
- **Project-scoped signal logging** — all signal events logged to `.squad/log/signal.log` with RFC3339 timestamps, level (OK/WARN/GUARD), agent name, and structured context
- **Watchdog logging** — daemon activity logged to `.squad/log/watch.log` with nudge tracking and stall detection
- **Log rotation** — signal.log auto-truncates to 500 lines when exceeding 1MB
- **Signal uses `current_task` FK** — targeted completion of the exact task being worked on, with FIFO fallback safety net when `current_task` is NULL
- **DB layer functions** — `set_current_task`, `clear_current_task`, `complete_by_id`, `last_completed_id`, `complete_message_by_id`, `count_processing_all`, `total_count`, `last_activity_timestamp`
- **Hook templates upgraded** — Claude Code: Stop + Notification + PostToolUse with `$SQUAD_AGENT_NAME` and stderr-to-log redirection; Gemini CLI: AfterAgent + Notification with JSON stdout compliance and 30s timeout

### Changed

- **Signal flow rewritten** — primary path uses `current_task` FK for targeted completion instead of FIFO queue; FIFO retained as fallback with WARN-level logging
- **Fire-and-forget commands** (`/clear`) no longer set `current_task` — prevents corruption when `/clear` overlaps an in-flight real task
- **Hook resolution** — switched from `tmux display-message -p '#S'` to `$SQUAD_AGENT_NAME` env var with `tmux list-panes` fallback
- **Init command** — now creates `.squad/log/` directory, auto-starts watchdog, installs hooks for all providers in the squad (not just orchestrator)
- **Clean command** — stops watchdog daemon before deleting DB to prevent crash loops

### Fixed

- **current_task corruption** when `/clear` overlaps in-flight task — current_task now correctly reverts to the real task (v0.5.8)
- **Signal race condition** — `/clear` followed by a real task no longer leaves the real task stuck at `processing` (v0.5.7)
- **Shell injection in session names** — `sanitize_session_name` now strips all shell metacharacters (`' ` `` ` `` `$ ; () | & <> \ /` space newline null), not just `.` `:` `"` (PR #2)
- **Unquoted model value in launch commands** — model values validated against `[a-zA-Z0-9._-:]` before shell interpolation (PR #2)
- **Clean command misses sessions** — `compute_session_names` now calls `sanitize_session_name` to match init naming (PR #2)
- **Signal exit-0 violation** — `get_agent` error no longer propagates as non-zero exit; uses soft guard matching the rest of the function (PR #2)
- **Antigravity agents marked dead** — `reconcile_agent_statuses` now skips `tool="antigravity"` agents that never have tmux sessions (PR #2)
- **inject_body corrupts task text** — `&&` splitting now only triggers when ALL parts are slash commands; plain text like "check if A && B" is sent as-is (PR #2)
- **Orphan WAL/SHM files** — `delete_db_file` now removes `station.db-wal` and `station.db-shm` alongside the main DB (PR #2)

### Security

- **sanitize_session_name hardened** — prevents shell injection via crafted session names in `sh -c` view commands
- **Model value validation** — blocks injection via malicious `model` field in `squad.yml`

### Removed

- Raw SQL queries outside `src/db/` — all moved to db layer functions

## v0.5.8 - 2026-03-20

### 🐛 Bug Fixes

- Fixed `current_task` corruption when `/clear` is sent while another task is already processing — `current_task` now reverts to the real task instead of staying pointed at the completed `/clear` message

### 🧪 Tests

- Added `test_fire_and_forget_clear_while_task_processing` integration test — verifies `current_task` and agent status are correct when fire-and-forget overlaps with an in-flight task

## v0.5.7 - 2026-03-20

### 🐛 Bug Fixes

- Fixed signal race condition: `/clear` followed by a real task no longer leaves the real task stuck at `processing` forever
- Root cause: `/clear` in Claude Code produces no response turn, so the Stop hook never fires — its DB message blocked the FIFO signal queue, causing subsequent signals to complete the wrong task
- Fire-and-forget commands (e.g. `/clear`, `/clear hard`) are now auto-completed at send time in `send.rs`
- Agent status correctly resets to idle after fire-and-forget if no other tasks are queued

### 🧪 Tests

- Added `is_fire_and_forget` unit tests (positive + negative cases)
- Added `test_fire_and_forget_clear_auto_completed` integration test — reproduces the exact race condition and verifies signal targets the correct task

## v0.5.6 - 2026-03-20

### 🌟 Highlights

- `/clear` context management upgraded from vague guidance to **hard rules** — weaker models (Haiku) no longer ignore `/clear` decisions

### 🎁 Features

- Mandatory `/clear` triggers: topic shift, 3-task threshold, agent hint detection
- Pre-send checklist added to orchestrator playbook — run before every `squad-station send`
- Explicit `How to /clear` section with code example
- QA Gate step 5 now says "Run the `/clear` checklist" instead of "Decide if `/clear` is needed"

### 🔧 Maintenance

- Version bumped to 0.5.6 across Cargo.toml, npm-package/package.json, and bin/run.js binary download
- npm binary download version aligned to 0.5.6 (was stuck at 0.5.3)

## v0.5.5 - 2026-03-19

### 🌟 Highlights

- Orchestrator context can now be **auto-injected** on session start, resume, or compact — no more forgetting to run `/squad-orchestrator`
- CLI simplified: `close` removed, `clean` now does everything (kill sessions + delete DB)
- New orchestrator guidance for managing agent context with `/clear`

### 🎁 Features

- `squad-station context --inject` outputs orchestrator content to stdout for SessionStart hook consumption
- Orchestrator-only guard: detects tmux session name and silently skips injection for worker agents
- Provider-aware output format: raw markdown for Claude Code, JSON `hookSpecificOutput.additionalContext` for Gemini CLI
- Opt-in SessionStart hook during `squad-station init` with interactive prompt (default: No)
- New "Context Management — /clear" section in orchestrator playbook
- QA Gate now includes step 5: "Decide if `/clear` is needed before the next task"

### 💥 Breaking Changes

- `squad-station close` command removed — use `squad-station clean` instead
- `squad-station clean` now kills all tmux sessions AND deletes the database (previously only deleted the database)

### 🔧 Maintenance

- Version aligned to 0.5.5 across Cargo.toml and npm-package/package.json
- Updated SDD playbooks in npm-package
- 171 tests passing

## v0.5.3 - 2026-03-16

### 🌟 Highlights

- New PostToolUse hook catches agent questions (AskUserQuestion) and forwards them to the orchestrator
- Elicitation dialog support for permission-like prompts

### 🎁 Features

- PostToolUse hook: `AskUserQuestion` matcher notifies orchestrator when an agent asks a question
- Notification hook: added `elicitation_dialog` matcher alongside `permission_prompt`
- Orchestrator resolution fix for multi-agent squads

### 📚 Documentation

- Added README to npm-package

### 🔧 Maintenance

- `cargo fmt` formatting pass across source and tests
- 164 tests passing

## v0.5.1 - 2026-03-16

### 🌟 Highlights

- First public release as an npm package (`npx squad-station install`)
- Provider-agnostic hook system with auto-installation
- Colored, informative init output

### 🎁 Features

- `npx squad-station install` — npm package with postinstall binary download for macOS and Linux
- Colored init output with squad setup summary, hook status, and get-started instructions
- Gemini CLI hooks: AfterAgent (signal) and Notification (notify) auto-installed to `.gemini/settings.json`
- Claude Code hooks: Stop (signal) and Notification (permission_prompt) auto-installed to `.claude/settings.json`
- Gemini CLI slash command generated in TOML format (`.gemini/commands/squad-orchestrator.toml`)
- Provider-specific orchestrator context file paths resolved dynamically
- Freeze/unfreeze commands to block or allow orchestrator task dispatch
- Monitor session: tiled tmux view of all agent panes created during init
- Context command: generates unified `squad-orchestrator.md` with agent roster, routing rules, and playbook references
- Signal command: auto-detects agent from tmux pane ID, idempotent completion handling
- Full messaging pipeline: send, peek, list, signal with priority ordering (urgent > high > normal)
- SQLite WAL mode with single-writer pool and 5s busy timeout
- Literal-mode `send-keys` to prevent shell injection via tmux
- Antigravity provider support (DB-only orchestrator, no tmux session)
- SDD workflow orchestration: playbook-driven task delegation to agents
- Interactive TUI dashboard (ratatui) for monitoring agent status and messages

### 🔧 Maintenance

- Rust CLI with clap argument parsing, async tokio runtime, sqlx migrations
- 160+ tests (unit + integration)
- CI workflow for tests, clippy, and fmt
- curl-pipe-sh installer script
- MIT license
