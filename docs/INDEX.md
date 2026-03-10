# Squad Station — Documentation Index

> Source of truth for the Squad Station project.
> Read documents in the order listed below.

---

## Reading Order

| # | File | Purpose | Priority |
|---|------|---------|----------|
| 1 | [VISION.md](./VISION.md) | Vision, architecture overview, scope, problems to solve | **Start here** |
| 2 | [SOLUTION-DESIGN.md](./SOLUTION-DESIGN.md) | Detailed design: config format, data model, hooks, CLI, scenarios, naming conventions | Core reference |
| 3 | [TECH-STACK.md](./TECH-STACK.md) | Rust stack decisions, module structure, safety checklist, roadmap | Implementation guide |
| 4 | [GAP-ANALYSIS.md](./GAP-ANALYSIS.md) | 17 gaps between design specs and current codebase (all decisions resolved ✅) | **Action items** |
| 5 | [PLAYBOOK.md](./PLAYBOOK.md) | ⚠️ **STALE** — User-facing guide, needs rewrite after gaps are fixed | Do not trust |
| 6 | [FEEDBACK-04-UPGRADE.md](./FEEDBACK-04-UPGRADE.md) | 🟡 Feedback for Obsidian Doc Writer — 4 fixes needed in source doc #04 | Task tracking |

## Document Hierarchy

```
VISION.md                    ← WHY: Vision, scope, problems
    └── SOLUTION-DESIGN.md   ← WHAT: Design specifications (source of truth)
        └── TECH-STACK.md    ← HOW: Implementation decisions
            └── GAP-ANALYSIS.md  ← DELTA: What needs to change
                └── PLAYBOOK.md  ← GUIDE: User manual (rewrite last)
```

## Source Origin

All documents are derived from the original Obsidian design notes:

| Doc | Obsidian Source |
|-----|-----------------|
| VISION.md | `01. Vision & Scope.md` |
| SOLUTION-DESIGN.md | `02. Solution Design - Squad Station.md` |
| TECH-STACK.md | `03. Tech Stack Decision - Squad Station.md` |
| GAP-ANALYSIS.md | Cross-reference analysis (Obsidian vs codebase) |
| *Upgrade #04* | `04. Upgrade Design - Antigravity & Hooks Optimization.md` |

## Key Decisions (Quick Reference)

| Decision | Value | Source |
|----------|-------|--------|
| Language | **Rust** | TECH-STACK.md |
| DB Library | **sqlx** (async, compile-time SQL) | TECH-STACK.md |
| Architecture | **Stateless CLI** — no daemon | VISION.md |
| DB | **SQLite WAL** — 1 file per project | SOLUTION-DESIGN.md |
| Config | `squad.yml` with `project` (string), `model`, `description`, `command` fields | SOLUTION-DESIGN.md |
| Agent Naming | `<project>-<provider>-<role>` (using `provider`, not `tool`) | SOLUTION-DESIGN.md |
| Hooks | **Centralized CLI**: `squad-station signal $TMUX_PANE` (no shell scripts) | SOLUTION-DESIGN.md §04 |
| CLI Send | **`--body "..."`** named flag (not positional arg) | GAP-ANALYSIS.md |
| Signal Format | **`"<agent> completed <id>"`** (short, AI-parseable) | GAP-ANALYSIS.md |
| Messages Schema | **2-directional**: `from_agent` → `to_agent`, status: processing/completed/failed | SOLUTION-DESIGN.md |
| Orchestrator Modes | **CLI-based** (event-driven) + **IDE-based** (polling, e.g. Antigravity) | VISION.md §04 |
| IDE Context | **`.agent/workflows/`** for IDE orchestrators | SOLUTION-DESIGN.md §04 |
| Tmux Injection | **Rust-native** `tmux::adapter` with `load-buffer`/`paste-buffer` | TECH-STACK.md §04 |
| Skip Notify | **In `signal.rs`** — runtime provider check from DB | SOLUTION-DESIGN.md §04 |

---
*Last updated: 2026-03-10*

## Recent Updates (2026-03-10)

**Added CLI Reference section** to SOLUTION-DESIGN.md documenting all available commands:
- Core commands (init, send, signal, list, agents, status, register, context)
- Cleanup commands (close, reset, clean) — NEW
- UI & monitoring commands (ui, view, peek)

**Cleanup Commands** — New feature for resetting squad state:
- `squad-station close` — Kill tmux sessions
- `squad-station reset` — Full cleanup + relaunch
- `squad-station clean` — Delete DB with confirmation
