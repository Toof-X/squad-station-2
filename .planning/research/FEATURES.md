# Features Research

**Project:** Squad Station
**Domain:** AI agent orchestration CLI / stateless message routing
**Researched:** 2026-03-06
**Overall confidence:** HIGH (corroborated by multiple live projects in the ecosystem)

---

## Table Stakes

Features that users of any multi-agent CLI orchestration system expect. Missing any of these and the tool feels broken or unsafe to use in production.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `send` — inject task into named agent | Core value: orchestrator must be able to dispatch work | Low | `tmux send-keys` under the hood; naming convention = session name |
| `signal` — agent completion notification | Hook-driven; without it orchestrator must poll blindly | Low | Claude Code: `Stop` hook; Gemini CLI: `AfterAgent` hook |
| `status` — query agent lifecycle state (idle/busy/dead) | Orchestrator cannot schedule without knowing capacity | Medium | Pattern-match on `tmux capture-pane` bottom N lines; 300-500ms poll |
| `init` — bootstrap agent registry from `squad.yml` | Zero-friction start; declarative > imperative | Low | YAML config is the ecosystem standard (OpenClaw, ADK, Docker cagent all use YAML) |
| `register` — dynamic agent registration at runtime | Agents can be added without restart; stateless arch requires this | Low | Writes row to SQLite; idempotent on re-register |
| `list` — enumerate registered agents and their state | Debugging and scripting baseline; every comparable tool has this | Low | JSON output mode needed for machine consumption (`--json` flag) |
| Per-project database isolation | Multiple projects cannot share a station; cross-project routing is a footgun | Low | One `~/.agentic-squad/<project>/station.db` per project |
| Orchestrator skip / infinite-loop guard | Hook system fires for ALL sessions including orchestrator; unguarded = recursive dispatch | Low | Check session name or env var before signaling; exit early if orchestrator |
| Idempotent commands | Stateless CLI means any command may be retried; duplicate sends are fatal | Low | Use message-ID deduplication in SQLite before `tmux send-keys` |
| Human-readable error messages with actionable hints | CLI tools are used by developers who debug from terminal output | Low | Exit codes must be machine-parseable; messages must be human-parseable |
| Cross-platform binary (darwin + linux) | Target developers work on macOS, CI runs on linux | Medium | Rust cross-compile via `cross` or GitHub Actions matrix; no runtime dependency |

---

## Differentiators

Features that comparable tools lack or implement poorly. These are competitive advantages for Squad Station.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Provider-agnostic hook adapter | Claude Code, Gemini CLI, Codex, Aider — one tool works for all | Medium | Adapter pattern: detect provider from session name prefix or config flag; different completion signals per provider |
| `squad.yml` agent role naming convention | `<project>-<provider>-<role>` encodes identity in session name — no extra lookup table needed | Low | Self-describing; `tmux display-message -p '#S'` gives identity inside hook |
| Auto-generate orchestrator context file | Orchestrator gets a ready-made file describing all registered agents + their capabilities | Medium | Reduces prompt engineering burden on user; Overstory and Agent Deck lack this |
| TUI dashboard (`ui` subcommand) | Visual fleet monitoring without leaving terminal; shows idle/busy/dead in real-time | High | Comparable: tmuxcc, Agent Deck, NTM all have TUI; Ratatui (Rust) is the obvious choice |
| `view` — split tmux view of all agent panes | One command to lay out all sessions as tiled panes for visual inspection | Medium | NTM has `ntm view`; not universal in minimal tools |
| npm wrapper distribution | `npx squad-station` works for any Node.js developer; no Rust toolchain needed | Medium | Pattern: platform-specific npm packages (`@squad-station/darwin-arm64`) + root package with JS shim; dist tool automates this |
| Zero daemon architecture | Debuggable, CI-friendly, no orphan processes — unlike persistent orchestrator daemons | Low (design) | Explicit design choice; most comparable tools (AgentManager, Ruflo) require running servers |
| Machine-readable JSON output mode | Scripting and programmatic orchestration without parsing human text | Low | `--json` flag on `list`, `status` commands; NTM has `--robot-*` flags for this |

---

## Anti-Features

Things to deliberately NOT build. These are either out of scope, costly footguns, or belong to the orchestrator AI layer.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Task management / workflow logic | Orchestrator AI is the decision-maker; Station is a transport layer. Encoding workflow in Station creates coupling and defeats the provider-agnostic goal | Expose primitives (`send`, `signal`, `status`); let Orchestrator compose them |
| Orchestration decisions / reasoning | Station should not decide which agent to route to — that is the LLM's job | Pass task to the agent the Orchestrator specifies; never pick the agent autonomously |
| File sync / code sharing between agents | Git worktrees + git operations are the correct primitive for that; Station should not duplicate | Document the git worktree pattern in usage guide instead |
| Web UI / browser dashboard | Adds server runtime, auth, TLS surface — contradicts zero-runtime-dependency goal | TUI in terminal covers the visual monitoring need |
| Cost tracking / token counting | Requires provider API access (stream-json parsing, API keys) — Station does not touch agent internals | Each agent's native tool (Claude Code has built-in cost display) handles this |
| Retry/backoff for failed LLM calls | Station injects prompts via tmux; it has no visibility into LLM call outcomes | Agent's own retry behavior handles this; Station only detects if agent session is alive |
| Agent spawning / lifecycle management | Creating and killing tmux sessions is in scope for tools like NTM; Station assumes agents pre-exist | Document that agents must be started manually or via user's own script; keep Station focused on routing |
| Spec-driven methodology integration | V2 feature per PROJECT.md | Defer; adds substantial complexity around planning artifacts |
| Git conflict resolution | Complex domain; conflicts between agents require human or AI judgment | Out of scope; agents work in separate worktrees; conflicts are a git-layer problem |
| Persistent background daemon | Kills the stateless design; complicates deployment, debugging, and distribution | SQLite persistence gives durability without a running process |

---

## Feature Dependencies

Dependencies define the build order. A feature cannot ship before its dependencies.

```
SQLite schema init
  └── register (writes agent rows)
        └── init (reads squad.yml, calls register for each agent)
              └── send (looks up agent, runs tmux send-keys)
                    └── signal (writes completion event, triggers orchestrator notification)
                          └── status (reads agent state from DB + live tmux capture-pane)
                                ├── list (uses status per agent)
                                ├── TUI dashboard (polls status in loop)
                                └── auto-generate orchestrator context file (reads list)

Hook adapters (Claude Code Stop / Gemini AfterAgent)
  └── signal (hook calls signal; orchestrator skip guard lives here)

Cross-compile binary
  └── npm wrapper distribution (needs darwin-arm64, darwin-x64, linux-x64 binaries)
```

**Critical path:** `schema → register → send → signal → status`

All UI and distribution features are built on top of the core five. Build the core first.

---

## Complexity Assessment

| Feature | Effort | Risk | Phase Fit |
|---------|--------|------|-----------|
| SQLite schema + init | Low | Low | Phase 1 (foundation) |
| `register` / `init` (squad.yml) | Low | Low | Phase 1 |
| `send` (tmux send-keys) | Low | Low | Phase 1 |
| `signal` with orchestrator skip guard | Low | Medium | Phase 1 |
| `status` (tmux capture-pane + pattern match) | Medium | Medium | Phase 1-2 |
| `list` with JSON output | Low | Low | Phase 1-2 |
| Provider-agnostic hook adapters | Medium | Medium | Phase 2 |
| Auto-generate orchestrator context file | Medium | Low | Phase 2 |
| Machine-readable output (`--json`) | Low | Low | Phase 2 |
| Per-project DB isolation | Low | Low | Phase 1 (baked into schema design) |
| Idempotency / deduplication | Low | Medium | Phase 1 (easy to miss early, painful to retrofit) |
| `view` (split tmux layout) | Medium | Low | Phase 3 |
| TUI dashboard (Ratatui) | High | Medium | Phase 3 |
| npm wrapper distribution | Medium | Low | Phase 4 (post stable binary) |
| Cross-compile CI pipeline | Medium | Low | Phase 4 |

**Complexity hotspots:**
- `status` detection — Pattern matching against tmux capture-pane output is fragile. Each provider has different idle/busy markers. Agent Deck and tmuxcc both invest significant effort here. Build with testable pattern configs early.
- TUI (Ratatui) — High effort but low technical risk in Rust; Ratatui is mature. This is a scope item to consciously defer until core is solid.
- Hook adapter per provider — Claude Code's `Stop` hook and Gemini CLI's `AfterAgent` hook have different JSON payloads and different exit-code semantics. Needs explicit adapter abstraction.
- Idempotency — Missing deduplication on `send` causes double-dispatch when hooks fire multiple times. SQLite `INSERT OR IGNORE` on message-ID is the standard fix; must be built in Phase 1.

---

## Sources

- [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks) — definitive hook event list (Stop, AfterAgent, TeammateIdle, etc.) — HIGH confidence
- [Overstory — Multi-agent orchestration](https://github.com/jayminwest/overstory) — pluggable runtime, SQLite mail system, typed message protocol — HIGH confidence (live project)
- [Agent Deck — Terminal session manager](https://github.com/asheshgoplani/agent-deck) — smart status detection patterns, TUI features — HIGH confidence (live project)
- [AgentManager — Claude Code orchestrator](https://github.com/simonstaton/AgentManager) — message types, inter-agent bus, cost tracking patterns — HIGH confidence
- [NTM — Named Tmux Manager](https://github.com/Dicklesworthstone/ntm) — agent send, robot JSON output, notification patterns — HIGH confidence
- [IttyBitty — Multi-agent Claude Code](https://adamwulf.me/2026/01/itty-bitty-ai-agent-orchestrator/) — Manager/Worker hierarchy, tmux virtual terminals — MEDIUM confidence
- [Packaging Rust for NPM](https://blog.orhun.dev/packaging-rust-for-npm/) — platform-specific package pattern, dist tool — HIGH confidence
- [Agent Orchestration Anti-Patterns](https://dev.to/onestardao/-ep-6-why-multi-agent-orchestration-collapses-deadlocks-infinite-loops-and-memory-overwrites-1e52) — infinite loops, deadlocks, resource exhaustion patterns — MEDIUM confidence
- [Azure AI Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns) — sequential, concurrent, handoff patterns — HIGH confidence (official docs)
- [OpenAI Multi-Agent docs](https://openai.github.io/openai-agents-python/multi_agent/) — routing, handoffs, message passing — HIGH confidence (official docs)
