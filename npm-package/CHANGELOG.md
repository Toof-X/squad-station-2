# Changelog

All notable changes to Squad Station are documented in this file.

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
