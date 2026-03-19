# Project Research Summary

**Project:** Squad Station v1.8 — Smart Agent Management
**Domain:** Rust CLI — stateless binary with embedded SQLite, ratatui TUI, tmux integration
**Researched:** 2026-03-19
**Confidence:** HIGH

## Executive Summary

Squad Station v1.8 adds three capabilities to an already-solid v1.7 foundation: agent role templates embedded in the init wizard, orchestrator intelligence data surfaced in `squad-orchestrator.md`, and a new `clone` subcommand for dynamic agent duplication. All three features are implementable with zero new Rust crates — the existing dependency set (clap 4.5, sqlx 0.8, chrono 0.4, ratatui 0.30, tokio 1, serde/serde_json 1.0) covers every requirement. The only schema addition is a single `ALTER TABLE ADD COLUMN` migration for `busy_since` to enable accurate busy-time tracking. Architecture research confirms each feature maps cleanly to established patterns in the codebase without requiring new cross-cutting concerns.

The recommended implementation order, grounded in dependency analysis, is: (1) orchestrator intelligence data first — it gives the orchestrator the signal it needs to decide when to clone; (2) dynamic agent cloning second — a clean command-per-file addition that reuses existing DB CRUD and tmux session launch; (3) agent role templates in the wizard last — entirely self-contained to `wizard.rs` and independent of runtime orchestration behavior. This ordering keeps each phase focused, avoids refactoring pressure, and delivers immediate orchestrator value before the UX polish.

The most critical risks are concentrated in the clone command: name collision between DB state and orphaned tmux sessions after re-init, partial success (tmux launched but DB write failed or vice versa), missing context regeneration after a successful clone, and accidental orchestrator cloning breaking the signal routing chain. All are preventable with explicitly specified guards and tests. The orchestrator intelligence feature carries one significant design risk: embedding static metric values in a snapshot file creates stale-data misrouting — the correct pattern is embedding CLI commands for live re-query rather than pre-computed tables.

## Key Findings

### Recommended Stack

No new Rust crates are required for any v1.8 feature. All three capabilities use the existing dependency set. The only infrastructure change is one SQL migration file adding `busy_since TEXT DEFAULT NULL` to the `agents` table, following the established `ALTER TABLE ADD COLUMN` pattern from migrations 0003 and 0004. Role templates are compile-time Rust constants (`&[RoleTemplate]`) — static structs are zero-cost at startup, compile-time validated, and require no file-read code path. Template data must not be stored in TOML or JSON files, as that would add a new dependency and a fallible parse path for data that never changes at runtime.

**Core technologies — no version changes:**
- **clap 4.5**: CLI dispatch — add `Clone` variant to `Commands` enum, same derive pattern as all other subcommands
- **sqlx 0.8**: SQLite queries + migrations — one new migration (`0005_v18_metrics.sql`); all query patterns already established
- **chrono 0.4**: Timestamp arithmetic — `signed_duration_since` + `parse_from_rfc3339` for busy-time duration; already imported
- **ratatui 0.30**: TUI rendering — template selector reuses existing `List` widget + `ListState` pattern from Provider/Model selectors; no new widget types
- **serde_json 1.0**: JSON output — `serde_json::json!` already used in every command; clone output follows same pattern
- **Rust stdlib**: Clone name auto-increment — `str::rfind`, `str::parse::<u32>()`, `format!`; no regex crate needed

### Expected Features

Research confirmed the feature set against CrewAI, AutoGen, LangGraph, MetaGPT, and Microsoft multi-agent pattern documentation. All five PROJECT.md active requirements are validated as correct scope for v1.8.

**Must have (table stakes):**
- Predefined role menu in wizard (8+ templates: orchestrator, frontend-engineer, backend-engineer, fullstack-engineer, qa-engineer, devops-engineer, architect, code-reviewer, custom) — every multi-agent framework ships these; absence makes setup feel unfinished
- Custom role escape hatch in wizard — templating systems without a custom option exclude legitimate use cases
- Model pre-fill from template selection — implied by role; requiring manual model selection after template pick is unnecessary friction
- Routing hints from templates embedded in `squad-orchestrator.md` — CrewAI and MetaGPT both encode role goals into orchestration context; without this the orchestrator has no specialization signal
- `squad-station clone <agent>` command — dynamic scaling is a core expectation in workload-aware multi-agent systems; without it the orchestrator cannot expand the fleet
- Pending message count per agent in orchestrator context — fundamental overload detection signal; orchestrator cannot reason about queue depth without it
- Busy-time tracking in orchestrator context — detects stuck agents; `status_updated_at` already exists in the schema

**Should have (differentiators):**
- Task-role alignment hints in `squad-orchestrator.md` — lightweight keyword overlap between recent task bodies and role/description; unique signal no competing tool provides in a file-based context model
- Orchestrator-controlled cloning (not auto-scaling) — deliberately keeps scaling decisions with the AI; CLI provides mechanism, AI decides when; correct abstraction for semantic workload reasoning
- Auto-incremented clone naming with project prefix — deterministic, human-readable, parseable by the orchestrator without additional metadata (unlike UUIDs)
- SDD-aware template ordering in wizard — reorders (not filters) template list based on detected workflow; proactive guidance competitors lack

**Defer to v2+:**
- `clone --n 3 <agent>` batch clone shorthand — add after single clone is validated
- Clone count limit guardrail — warn when more than N agents with same role exist; add when teams hit terminal real-estate limits
- Task-role alignment scoring with ML embeddings — binary size and inference latency are prohibitive at team scale
- Template versioning and user-defined local template registry — relevant only at larger user scale
- Metrics history (snapshots over time) — requires an `agent_events` table; defer until orchestrators need trend data

### Architecture Approach

The v1.8 features integrate as additive changes to the existing layered architecture: CLI dispatch → command handlers → DB layer → SQLite (WAL). No new layers, no new cross-cutting infrastructure. Each feature follows a named pattern already in use in the codebase: `clone.rs` follows command-per-file; `agent_metrics()` follows DB-as-thin-CRUD; `busy_duration_seconds()` follows pure-fn for testability; template data follows compile-time constants. The connect-per-refresh pattern in `ui.rs` means cloned agents appear in the TUI dashboard within one 3-second poll cycle with zero TUI code changes.

**Major components changed:**
1. `src/commands/wizard.rs` — add `RoleTemplate` struct, `ROLE_TEMPLATES` const, `TemplatePickerPageState` in `WizardState`, pre-fill logic, new render branch, hint text
2. `src/db/messages.rs` + `src/db/agents.rs` — add `AgentMetrics` struct, `agent_metrics()` async query, `busy_duration_seconds()` pure fn
3. `src/commands/context.rs` — update `build_orchestrator_md()` signature, add Fleet Status section
4. `src/commands/clone.rs` (NEW) + `src/cli.rs` + `src/commands/mod.rs` + `src/main.rs` — full clone subcommand
5. `src/db/migrations/0005_v18_metrics.sql` (NEW) — single `ALTER TABLE ADD COLUMN` for `busy_since`

**Critical architectural constraints to preserve:**
- `build_orchestrator_md()` must remain a pure `fn` — fetch `Vec<AgentMetrics>` in `context::run()` and pass as a parameter; never make DB calls inside the function
- Clones are DB-only entries; never write clones to `squad.yml` (same as `register` behavior)
- Routing hints live in compile-time `ROLE_TEMPLATES` constants, not in a DB column

### Critical Pitfalls

1. **Clone name collision (DB vs. tmux reality)** — After re-init, orphaned tmux sessions exist that the DB no longer knows about. Auto-increment scanning only the DB produces a name that tmux rejects with "session already exists." Prevention: `generate_clone_name()` must check both `get_agent(pool, candidate)` AND `tmux::session_exists(candidate)` before committing to a name.

2. **Partial clone success — tmux launched before DB write** — If the tmux session is launched before DB registration, a DB write failure leaves an orphaned session invisible to the signal chain. Prevention: always DB-first; if `tmux::launch_agent()` fails after DB write, immediately call compensating `delete_agent_by_name()`.

3. **Context not regenerated after clone** — The orchestrator never learns about the clone because `squad-orchestrator.md` is only updated when `context` is explicitly invoked. Prevention: `clone::run()` must call `build_orchestrator_md()` directly as its final step and print explicit user instructions to reload `/squad-orchestrator` in the orchestrator session.

4. **Stale metrics in orchestrator playbook** — Embedding pre-computed metric values creates stale-data misrouting. Prevention: embed CLI commands for live re-query (`squad-station status --json`, `squad-station agents`) rather than static tables; any static snapshot must include a generated-at timestamp.

5. **Cloning the orchestrator creates a routing loop** — Two `role == "orchestrator"` agents break the signal routing chain silently (`get_orchestrator` returns the wrong record). Prevention: reject `role == "orchestrator"` with a clear error as the first guard in `clone::run()`, before any DB writes.

6. **Template model strings drift from validation allowlist** — Templates compiled into the binary reference model aliases independently of `valid_models_for()` in `config.rs`. Drift causes validation errors on wizard-offered selections. Prevention: templates omit model strings entirely, or a CI test validates each template's generated config against `validate_agent_config()`.

## Implications for Roadmap

Based on the combined research, four phases are recommended. The first three are required for v1.8; the fourth is optional polish.

### Phase 1: Orchestrator Intelligence Data

**Rationale:** Pure additive DB + context changes with no side effects. Landing this first gives the orchestrator the workload signal it needs to make informed clone decisions — making the clone command immediately useful upon delivery. The `build_orchestrator_md()` signature change requires updating existing tests (pass `&[]` for metrics), which is a mechanical fix best completed before any other feature builds on top of context generation.

**Delivers:** Live fleet metrics in `squad-orchestrator.md` — pending message count per agent, busy-time duration, and routing guidance with clone hints embedded as CLI commands (not static values).

**Addresses:** "Message-per-agent count in orchestrator context" (P1) and "Busy-time in orchestrator context" (P1) from FEATURES.md.

**Avoids:** Stale metrics pitfall — embed CLI commands for live re-query, not pre-computed tables. Document `busy_time` limitations (resets on re-init; represents "time in current state," not "total runtime").

**Files:** `src/db/messages.rs`, `src/db/agents.rs`, `src/commands/context.rs`, `src/db/migrations/0005_v18_metrics.sql`

### Phase 2: Dynamic Agent Cloning

**Rationale:** The orchestrator now has workload data from Phase 1 and can act on it immediately via `clone`. This is the highest-value runtime feature. Building it second keeps the new command file isolated and avoids any entanglement with wizard changes.

**Delivers:** `squad-station clone <agent>` command — reads source config, auto-increments name (checking both DB and tmux), registers in DB, launches tmux session, regenerates `squad-orchestrator.md` as a final step.

**Addresses:** "Dynamic agent cloning" (P1) and "Orchestrator-controlled cloning" (differentiator) from FEATURES.md.

**Avoids:** All clone-specific pitfalls — name collision (check DB + tmux), partial success (DB-first with compensating rollback), missing context regeneration (auto-call in clone handler), orchestrator cloning (reject `role == "orchestrator"` first), session name sanitization (apply `config::sanitize_session_name()` to derived names).

**Files:** `src/commands/clone.rs` (NEW), `src/cli.rs`, `src/commands/mod.rs`, `src/main.rs`

### Phase 3: Agent Role Templates in Wizard

**Rationale:** Entirely self-contained to `wizard.rs`. Does not affect clone, metrics, or DB at all. Improves onboarding for the v1.8 release but does not block any orchestrator runtime functionality. Building last keeps wizard changes separate and avoids merge conflicts with Phases 1 and 2.

**Delivers:** Role template selector page in the init wizard — 8+ predefined templates with role, description, model pre-fill, and routing hints. Custom option preserves existing free-text behavior. SDD-aware template ordering when workflow is detected.

**Addresses:** "Predefined role menu in wizard" (P1), "Template includes model suggestion" (P1), "Template includes routing hints" (P2), "SDD-aware template ordering" (P2) from FEATURES.md.

**Avoids:** Template model drift pitfall — CI test validates each template's generated config against `validate_agent_config()`. Model field in templates either omitted or references allowlist aliases only.

**Files:** `src/commands/wizard.rs`

### Phase 4: TUI Live Update Polish (Optional)

**Rationale:** The connect-per-refresh pattern already picks up cloned agents within one 3-second poll cycle — this phase is cosmetic only. Include if time permits before v1.8 release; skip if not without affecting any v1.8 functional requirement.

**Delivers:** Visual highlight for newly-appeared agents in the TUI agent list for one refresh cycle. Optional identification of clone agents by naming convention suffix in the display.

**Files:** `src/commands/ui.rs` (optional `seen_agents: HashSet<String>` addition)

### Phase Ordering Rationale

- Phase 1 before Phase 2: The context file is the coordination mechanism. Cloning without updated orchestrator context produces agents the orchestrator never routes to (Pitfall 3). Phase 1 also establishes the `build_orchestrator_md()` signature change that Phase 2 must call.
- Phase 2 before Phase 3: Runtime orchestration features (metrics + cloning) deliver higher impact than onboarding UX. Phase 3 is independent and carries no dependencies on Phases 1 or 2.
- Phase 4 last: Optional and purely cosmetic. The live update already works; the phase only adds optional highlighting.
- All three features are confirmed independent by the FEATURES.md dependency graph — parallel development is feasible if resources allow, but the sequential order above minimizes integration risk.

### Research Flags

Phases requiring deeper attention during planning:
- **Phase 2 (Clone command):** The five critical pitfalls each require explicit acceptance criteria before coding begins. The implementation plan must enumerate: double-check for name collision (DB + tmux), DB-first ordering with compensating rollback, auto-context-regeneration, orchestrator rejection guard, and session name sanitization. None of these are standard patterns — each requires a specific test.
- **Phase 1 (Playbook text design):** The exact wording of the Fleet Status section and routing guidance in `squad-orchestrator.md` is a UX design decision with correctness implications. Draft and review the generated markdown template before writing any query code.

Phases with standard patterns (skip research-phase):
- **Phase 3 (Templates):** Wizard integration follows the established `List` widget + `ListState` radio-selector pattern visible in the current `wizard.rs` for Provider and Model selectors. No research needed; the implementation pattern is directly observable in the existing code.
- **Phase 4 (TUI polish):** Adding a `HashSet<String>` to `App` state is a trivial additive change. No research needed.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified directly against `Cargo.toml` and all relevant source files. Zero new crates confirmed by inspecting every feature requirement against existing imports. |
| Features | HIGH | PROJECT.md requirements cross-validated against CrewAI docs, Microsoft multi-agent patterns, DRTAG research, and IBM orchestration guidance. Ecosystem patterns confirmed for role templates (8-12 roles standard), dynamic cloning (master-clone architecture), and workload metrics (pending count + busy time). |
| Architecture | HIGH | All findings from direct source inspection of the v1.7 codebase. Integration points verified against actual function signatures in `wizard.rs`, `context.rs`, `db/agents.rs`, `db/messages.rs`, `tmux.rs`, `cli.rs`, `main.rs`. No inferred behavior. |
| Pitfalls | HIGH | All pitfalls grounded in direct codebase inspection (`signal.rs` GUARDs, `config.rs` allowlist, `db/agents.rs` upsert semantics) and the existing CONCERNS.md audit. Recovery strategies verified against available CLI commands. |

**Overall confidence:** HIGH

### Gaps to Address

- **Exact model aliases for templates:** Templates need model suggestions referencing valid aliases from `valid_models_for()` in `config.rs`. Read the allowlist before writing template constants to avoid the validation drift pitfall. Alternatively, omit model from templates entirely and rely on the wizard radio selector — the safer default.
- **`busy_since` vs. `status_updated_at` discrepancy:** STACK.md recommends a new `busy_since` column (migration 0005) for accurate busy-time tracking, while ARCHITECTURE.md and FEATURES.md reference the existing `status_updated_at` column. Both approaches work; `busy_since` is more reliable (not overwritten on every status change). The implementation plan must pick one approach and specify it explicitly before Phase 1 work begins.
- **Playbook text for Fleet Status section:** The exact format of the generated `squad-orchestrator.md` Fleet Status section — column layout, routing guidance wording, CLI command embed vs. static table decision — should be drafted and agreed before `build_orchestrator_md()` is modified. The wording has correctness implications for orchestrator behavior.

## Sources

### Primary (HIGH confidence)

- `Cargo.toml` (local codebase) — confirmed locked versions; zero new dependencies required
- `src/commands/wizard.rs` (local codebase) — confirmed `List` widget + `ListState` radio-selector pattern; `AgentInput` struct fields; `WizardState` structure
- `src/commands/context.rs` (local codebase) — confirmed `build_orchestrator_md()` as `pub fn`; string-building pattern; stateless snapshot design
- `src/db/agents.rs` (local codebase) — confirmed `insert_agent`, `update_agent_status`, `get_agent`, `get_orchestrator` signatures; `status_updated_at` behavior; `delete_all_agents` re-init path
- `src/db/messages.rs` (local codebase) — confirmed existing `status` and `to_agent` columns; `list_messages()` query patterns
- `src/commands/signal.rs` (local codebase) — confirmed GUARD 3 (missing agent = silent exit) and GUARD 4 (orchestrator self-signal) behavior
- `src/config.rs` (local codebase) — confirmed `VALID_PROVIDERS`, `valid_models_for()`, `sanitize_session_name()` APIs
- `src/tmux.rs` (local codebase) — confirmed `launch_agent()`, `session_exists()`, `list_live_session_names()` availability
- `src/db/migrations/` (local codebase) — confirmed `ALTER TABLE ADD COLUMN` pattern from migrations 0003/0004
- `.planning/codebase/CONCERNS.md` — pre-existing audit covering tmux/DB sync risks, reconciliation duplication, single-writer pool limits

### Secondary (MEDIUM confidence)

- [CrewAI Agents Documentation](https://docs.crewai.com/en/concepts/agents) — role, goal, backstory field patterns; software team role set
- [Microsoft ISE Blog: Patterns for Building a Scalable Multi-Agent System](https://devblogs.microsoft.com/ise/multi-agent-systems-at-scale/) — dynamic agent spawning; master-clone architecture
- [Frontiers in AI: Auto-scaling LLM-based multi-agent systems](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1638227/full) — DRTAG pattern; clone-inherits-config pattern
- [IBM: AI Agent Orchestration](https://www.ibm.com/think/topics/ai-agent-orchestration) — orchestrator metrics for workload balancing; pending task queue depth as primary signal
- [Microsoft Azure: AI Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns) — multi-agent coordination patterns

### Tertiary (LOW confidence)

- [Agentic AI Systems Guide: Scaling Multi-Agent AI Systems](https://agenticaiguide.ai/ch_8/sec_8-3.html) — elastic scaling; stateless cloning patterns (single source, not independently verified)

---
*Research completed: 2026-03-19*
*Ready for roadmap: yes*
