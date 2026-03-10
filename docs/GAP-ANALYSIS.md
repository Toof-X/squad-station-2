# Squad Station — Gap Analysis

> Comparison of Source of Truth docs (from Obsidian) vs current codebase.
> Goal: identify what needs to change in the codebase to comply with the original design.
> Updated with: gaps from `04. Upgrade Design — Antigravity & Hooks Optimization`.

---

## Summary

| Severity | Total | Done ✅ | Remaining | Description |
|----------|-------|---------|-----------|-------------|
| 🔴 CRITICAL | 3 | 3 | 0 | ~~Config, Messages schema, Agents schema~~ — ALL DONE |
| 🟡 HIGH | 9 | 9 | 0 | ~~All original + upgrade #04~~ — ALL DONE |
| 🟢 MEDIUM | 6 | 5 | 1 | ~~All original + upgrade #04~~ done; upgrade #05 in progress |

---

## 🔴 CRITICAL — Must fix for design compliance

### GAP-01: Config `squad.yml` — wrong structure ✅ DONE

> **Verified 2026-03-09** — `config.rs` now has `project: String`, `model: Option<String>`, `description: Option<String>`.
> `command` field removed per CONF-03. `db_path` uses env var `SQUAD_STATION_DB`.

**Required changes:**
- [x] `ProjectConfig` → changed to `pub project: String` ✅
- [x] `AgentConfig` → added `model: Option<String>`, `description: Option<String>` ✅
- [x] Field `command` → removed (CONF-03: tool infers launch command) ✅
- [x] Sample `squad.yml` updated in PLAYBOOK.md ✅

### GAP-02: DB Schema `messages` — missing 2-directional and processing status ✅ DONE

> **Verified 2026-03-09** — Migration `0003_v11.sql` added all required columns.
> `messages.rs` struct now has `from_agent`, `to_agent`, `msg_type`, `completed_at`.
> INSERT uses `status = 'processing'` (not 'pending').

**Required changes:**
- [x] Add `from_agent`, `to_agent` — migration MSGS-01 ✅
- [x] Add `type` column (task_request, task_completed, notify) — migration MSGS-02 ✅
- [x] Change status `pending` → `processing` — handled in Rust INSERT ✅
- [x] Add `completed_at` — migration MSGS-04 ✅
- [x] New migration file — `0003_v11.sql` ✅

### GAP-03: DB Schema `agents` — missing critical fields ✅ DONE

> **Verified 2026-03-09** — Migration `0003_v11.sql` added `model`, `description`, `current_task`.
> Column renamed `provider` → `tool` (AGNT-03). `agents.rs` struct reflects all fields.
>
> ⚠️ **NOTE:** Decision #5 said "keep `provider`" but code already renamed to `tool` in migration.
> This rename is already deployed — reverting would require another migration. Keeping `tool` as-is.

**Required changes:**
- [x] Add `model` column — AGNT-01 ✅
- [x] Add `description` column — AGNT-01 ✅
- [x] Add `current_task` FK → messages.id — AGNT-02 ✅
- [x] Rename `provider` → `tool` — AGNT-03 ✅ (already deployed, keeping as-is)
- [x] New migration file — `0003_v11.sql` ✅

---

## 🟡 HIGH — Missing features

### GAP-04: Notification hook not implemented ✅ DONE

> **Verified 2026-03-09** — Both notify scripts exist in `hooks/` folder:
> `claude-code-notify.sh`, `gemini-cli-notify.sh`, plus `test-notify-hooks.sh`.

**Required:**
- [x] Create `hooks/claude-code-notify.sh` for Notification event ✅
- [x] Create `hooks/gemini-cli-notify.sh` ✅
- [x] Document how to register notification hook ✅ (covered in PLAYBOOK.md §4 "Notification Hooks")

### GAP-05: CLI `send` — uses positional arg instead of `--body` flag ✅ DONE

> **Verified 2026-03-09** — `cli.rs` Send variant: `#[arg(long)] body: String`.
> `send.rs` accepts `body: String` as named param.

**Decision:** `--body` flag confirmed. ✅ Already implemented.

### GAP-06: Agent naming convention not enforced ✅ DONE

> **Verified 2026-03-09** — `init.rs` auto-prefixes:
> `format!("{}-{}-{}", config.project, agent.tool, role_suffix)`

**Required:**
- [x] `init` command auto-prefixes agent name with `<project>-<tool>-<role>` ✅
- [x] Convention enforced at init time (not config validation) ✅

### GAP-07: `context` command lacks `description` and `model` ✅ DONE

> **Verified 2026-03-09** — `context.rs` outputs model in heading `## agent (model)`
> and description as body text. Uses `agent.model` and `agent.description` from DB.

**Required:** ~~Fix after GAP-03~~ — GAP-03 done, context now shows model + description ✅

---

## 🟢 MEDIUM — Docs / Naming

### GAP-08: `.planning/research/ARCHITECTURE.md` stale ✅ SUPERSEDED

> **Verified 2026-03-09** — `.planning/` directory no longer exists in the repo.
> Architecture documentation has been consolidated into `docs/SOLUTION-DESIGN.md` and `docs/TECH-STACK.md`.

**Status:** Superseded — no action needed.

### GAP-09: `PLAYBOOK.md` has many incorrect details ✅ DONE

> **Verified 2026-03-09** — PLAYBOOK.md fully rewritten (546 lines). Covers inline hooks (`squad-station signal $TMUX_PANE`),
> Antigravity IDE orchestrator mode, notification hook registration, all `tool:` references updated.

**Required:** ~~Rewrite PLAYBOOK.md after fixing above GAPs.~~ — DONE via GSD Phase 13, plan 13-02.

### GAP-10: Signal notification format ✅ DONE

> **Verified 2026-03-09** — `signal.rs` uses: `format!("{} completed {}", agent, task_id_str)`
> Matches the design spec exactly.

**Decision:** `"<agent> completed <id>"` confirmed. ✅ Already implemented.

---

## Implementation Priority

```
Phase 1 (DB + Config refactor): ✅ ALL DONE
  GAP-01 ✅ Config format
  GAP-02 ✅ Messages schema
  GAP-03 ✅ Agents schema

Phase 2 (Feature completion): ✅ ALL DONE
  GAP-04 ✅ Notification hooks (scripts created; docs deferred)
  GAP-05 ✅ CLI send syntax
  GAP-06 ✅ Naming convention
  GAP-07 ✅ Context with model/description

Phase 3 (Documentation): ✅ ALL DONE
  GAP-08 ✅ .planning/ superseded (consolidated into docs/)
  GAP-09 ✅ PLAYBOOK.md rewritten (v1.3)
  GAP-10 ✅ Signal format finalized

Phase 4 (Antigravity & Hooks Optimization — upgrade #04): ✅ ALL DONE
  GAP-15 ✅ Centralized hooks — signal accepts $TMUX_PANE, scripts deprecated
  GAP-16 ✅ Safe settings.json merge — init auto-merges with .bak backup
  GAP-11 ✅ Antigravity provider in config — is_db_only() helper
  GAP-12 ✅ signal.rs conditional skip notify — DB-only for antigravity
  GAP-13 ✅ init.rs skip tmux for Antigravity — DB-only registration
  GAP-14 ✅ .agent/workflows/ context generation — 3 workflow files
  GAP-17 ✅ Safe tmux multiline injection — load-buffer/paste-buffer

Phase 5 (Unified Playbook — upgrade #05): ⏳ IN PROGRESS
  GAP-18 ⏳ Unified orchestrator playbook — replace 3 fragmented files with single squad-orchestrator.md
```

---

## 🟡 HIGH — Upgrade #04: Antigravity & Hooks (GAP-11 → GAP-15)

### GAP-11: Config `squad.yml` — missing `antigravity` provider ✅ DONE

> **Verified 2026-03-09** — `config.rs` now has `AgentConfig.is_db_only()` method.
> `tool: antigravity` parsed correctly. 3 integration tests cover config loading.

**Required changes:**
- [x] Add `AgentConfig.is_db_only()` helper in `config.rs` ✅
- [x] Validate `tool: antigravity` in config parsing ✅
- [x] Add integration tests for Antigravity config loading ✅

### GAP-12: `signal.rs` — missing conditional skip-notify for IDE Orchestrator ✅ DONE

> **Verified 2026-03-09** — `signal.rs` checks `orch.tool == "antigravity"` at runtime.
> DB-only orchestrators get status update but no tmux notification. 2 integration tests.

**Required changes:**
- [x] Add runtime provider check in `signal.rs` after updating task status ✅
- [x] If IDE provider → skip notification, return Ok ✅
- [x] If CLI provider → existing `tmux send-keys` behavior ✅

### GAP-13: `init.rs` — skip tmux session for Antigravity Orchestrator ✅ DONE

> **Verified 2026-03-09** — `init.rs` calls `config.is_db_only()` before tmux session creation.
> Antigravity orchestrator registered in DB only, logs skip message. 2 integration tests.

**Required changes:**
- [x] Add provider check before creating orchestrator tmux session ✅
- [x] If `antigravity` → skip session creation, register in DB only ✅
- [x] Log clearly: "Skipping tmux session for IDE orchestrator" ✅

### GAP-14: Context generation — missing `.agent/workflows/` for IDE Orchestrators ✅ DONE

> **Verified 2026-03-09** — `context.rs` generates 3 workflow files in `.agent/workflows/`.
> Delegation, monitoring, and roster files created. 8 integration tests cover content + idempotency.
>
> ⚠️ **NOTE:** GAP-18 will supersede this gap's workflow file approach. The 3 fragmented files
> will be replaced by a single unified playbook. Current implementation remains active until GAP-18 is completed.

**Required changes:**
- [x] Add conditional in `context.rs`: detect orchestrator provider ✅
- [x] If IDE provider → generate workflow files: ✅
  - `.agent/workflows/squad-delegate.md` — delegation instructions ✅
  - `.agent/workflows/squad-monitor.md` — monitoring/polling instructions ✅
  - `.agent/workflows/squad-roster.md` — agent list with models/descriptions ✅
- [x] Include behavioral rules (anti-context-decay) in workflow files ✅

### GAP-15: Hook system — not centralized ✅ DONE

> **Verified 2026-03-09** — `signal` accepts `$TMUX_PANE` and pane IDs (`%3`). `session_name_from_pane()` in `tmux.rs`.
> Hook scripts deprecated with notice. PLAYBOOK.md documents inline hook config.

**Required changes:**
- [x] `signal` command: accept `$TMUX_PANE` env var to auto-detect agent session name ✅
- [x] Deprecate `hooks/claude-code.sh` and `hooks/gemini-cli.sh` (keep as reference) ✅
- [x] Update documentation to show inline hook config in `settings.json` ✅

---

## 🟢 MEDIUM — Upgrade #04: Setup & Safety (GAP-16 → GAP-17)

### GAP-16: `init` / `setup-hooks` — no safe settings.json merge ✅ DONE

> **Verified 2026-03-09** — `init.rs` has `merge_hook_entry()` that parses existing settings.json,
> creates `.bak` backup, merges Stop/AfterAgent hooks idempotently. Prints instructions if no file exists.
> 5 integration tests.

**Required changes:**
- [x] Add JSON parse + merge logic in `init` command ✅
- [x] Implement backup before modification ✅
- [x] Print human-readable instructions as fallback ✅

### GAP-17: `tmux::adapter` — no safe multiline injection ✅ DONE

> **Verified 2026-03-09** — `tmux.rs` has `inject_body()` using `load-buffer`/`paste-buffer` pattern.
> Writes to UUID temp file, loads into tmux buffer, pastes to target, cleans up. `send.rs` uses it for all body delivery.
> 4 unit tests.

**Required changes:**
- [x] Implement in `tmux.rs`: write body to temp file → `tmux load-buffer` → `tmux paste-buffer` ✅
- [x] Cleanup temp file after injection ✅
- [x] `send` command should automatically use safe adapter for body content ✅

---

## 🟢 MEDIUM — Upgrade #05: Unified Orchestrator Playbook (GAP-18)

### GAP-18: Eliminate fragmented context files in favor of a cohesive Playbook

> **Identified 2026-03-10** — Current `squad-station context` generates 3 fragmented technical files (`delegate`, `monitor`, `roster`). This approach fails to provide a cohesive Persona and execution discipline as demonstrated by the `withClaudeCodeTmux.vi.toml` command. The fragmented concept (`squad-delegate.md`, `squad-monitor.md`, `squad-roster.md`) must be entirely eliminated.
>
> ⚠️ **NOTE:** Implementation was attempted in commit `0ce6f83` but reverted in commit `7add4ba` which restored the fragmented 3-file approach.

**Required changes:**
- [ ] Eliminate the generation of `squad-delegate`, `squad-monitor`, and `squad-roster`.
- [ ] Adjust the wording from the `withClaudeCodeTmux.vi.toml` command to serve as a base template.
- [ ] Read `squad.yml` to dynamically extract the necessary Agents and inject them into this new playbook.
- [ ] Generate a single, unified playbook file (`squad-orchestrator.md` or `.toml` depending on the orchestrator's provider).
- [ ] Update `squad-station init` console output ("Get Started") to point to this single unified workflow file instead of the deleted `squad-delegate.md`.

---

### All Decisions — Resolved ✅

| # | Question | **Decision** | Rationale |
|---|---------|-------------|------------|
| 1 | `project` config | **String**: `project: myapp` | Đơn giản; `db_path` dùng env var `SQUAD_STATION_DB` thay vì config |
| 2 | Field `command` | **Removed** (CONF-03: tool infers) | Code đã remove, `init` dùng `tool` field trực tiếp làm launch command |
| 3 | CLI `send` syntax | **`--body "..."`** flag ✅ | Đã implement trong `cli.rs` |
| 4 | Signal format | **`"<agent> completed <id>"`** ✅ | Đã implement trong `signal.rs` |
| 5 | `provider` vs `tool` | **`tool`** (code đã rename, giữ nguyên) | ⚠️ Migration 0003 đã rename `provider→tool`. Quyết định gốc nói "keep provider" nhưng code đã đổi — giữ `tool` |
| 6 | Hook setup | **Centralized CLI**: `squad-station signal $TMUX_PANE` | Single inline command, no shell scripts |
| 7 | Context file for IDE orchestrator | **`.agent/workflows/`** | Antigravity already supports this format natively |
| 8 | Settings.json management | **Flexible**: auto-merge if exists, print instructions if not | Non-destructive, preserves user config |
| 9 | Multiline tmux injection | **Rust native** `tmux::adapter` | `load-buffer` / `paste-buffer` pattern |
| 10 | Skip notify logic location | **In `signal.rs`** (runtime DB check) | Single source of truth, hook stays simple |

---
*Generated: 2026-03-08*
*Updated: 2026-03-10 — 17/18 GAPs RESOLVED. GAP-18 (unified playbook) remains. All 10 decisions resolved.*
*Implemented via GSD framework: 4 phases (10-13), 8 plans, 159 tests passing, audit score 15/15.*
*Based on: docs/VISION.md, docs/SOLUTION-DESIGN.md, docs/TECH-STACK.md vs codebase*
