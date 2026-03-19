# Feature Research

**Domain:** Rust CLI — AI agent fleet management (squad-station v1.8 Smart Agent Management)
**Researched:** 2026-03-19
**Confidence:** HIGH (milestone features are explicitly defined in PROJECT.md; ecosystem patterns verified against CrewAI, AutoGen, LangGraph, Microsoft multi-agent patterns, and the existing squad-station codebase)

---

## Context: What v1.8 Smart Agent Management Adds

This milestone adds three capabilities on top of the existing v1.7 + v1.8-pre foundation (which already ships: init wizard with `--tui` flag, TUI dashboard, welcome TUI, npm + curl install, agent lifecycle detection, hook-driven completion signals, orchestrator context generation).

The three v1.8 Smart Agent Management features:

1. **Agent role templates in init wizard** — pre-built packages (role string, model suggestion, description, routing hints) selectable from a list during worker configuration; includes a custom option and a mechanism for system-suggested roles.
2. **Orchestrator intelligence data** — CLI provides task-role alignment metrics, messages-per-agent counts, and busy-time tracking surfaced in `squad-orchestrator.md` so the orchestrator AI can detect overload and misrouting without external tooling.
3. **Dynamic agent cloning** — `squad-station clone <agent-name>` creates a duplicate agent (same role/model/description, auto-incremented name, new tmux session); orchestrator decides when and how many to spawn; cloned agents appear immediately in the TUI dashboard.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in agent management tooling. Missing these makes the product feel unfinished relative to comparable systems (CrewAI, AutoGen, LangGraph multi-agent setups).

| Feature | Why Expected | Complexity | Dependencies on Existing |
|---------|--------------|------------|--------------------------|
| Predefined role menu in wizard | Every multi-agent framework (CrewAI, MetaGPT, AutoGen) provides role templates. Users setting up a software team expect to select "frontend engineer" or "QA" rather than type free-form strings. Typing a role from scratch for every agent is friction that degrades the wizard experience. | LOW | `wizard.rs` WorkerPage already has a radio-selector component (used for Provider and Model). Role templates are a new data structure + a new radio/list input on the same page. No new crate. |
| Custom role option in wizard | Any templating system must offer escape hatch. Users with unusual team structures (data engineer, security auditor, technical writer) need to define their own role. Forcing templates removes legitimate use cases. | LOW | Custom option is an additional list item that activates a free-text input field — pattern already exists in wizard.rs for model input. |
| Template includes model suggestion | When a user selects "backend engineer," they expect a sensible default model pre-filled (e.g., claude-code/sonnet). Having to separately select a model that is already implied by the role is unnecessary friction. | LOW | Template data structure carries `default_model` field; wizard pre-fills the model radio selector when template is chosen. User can override. |
| Template includes routing hints | Orchestrator needs to know which agent to route tasks to. Without routing hints embedded in `squad-orchestrator.md`, the orchestrator has no signal about specialization beyond a free-text description. CrewAI and MetaGPT both embed role goals/descriptions into their orchestration context. | LOW | `context.rs` `build_orchestrator_md()` already writes a "Session Routing" section iterating agents. Templates add structured `routing_hints` to the description field written there. |
| `squad-station clone <agent>` command | Dynamic scaling is a core expectation in any workload-aware multi-agent system. Microsoft's multi-agent patterns and the "master-clone" architecture both identify runtime agent duplication as a first-class operation. Without a CLI command for it, the orchestrator cannot scale the team. | MEDIUM | Requires: new `Commands::Clone` in `cli.rs`, new `src/commands/clone.rs`, existing `tmux.rs` session launch, existing `db::agents::insert_agent()` with auto-incremented name, existing TUI refresh loop (picks up new agents automatically via DB poll). No schema changes needed. |
| Cloned agent appears in TUI dashboard immediately | The TUI polls DB for agents on every refresh (existing behavior). A newly cloned agent registered in DB is visible on the next poll cycle with no additional work. Users expect the monitoring view to reflect fleet state without manual refresh. | LOW | Zero new code: existing `ui.rs` polling loop already does `list_agents()` on every interval. Clone command writes to DB; TUI reads on next poll. |
| Message-per-agent count in orchestrator context | The orchestrator needs to know how many tasks each agent has received to detect overload. Standard observability practice: track request count per service. Without this, the orchestrator must guess at agent load. | LOW | `messages.rs` already has `list_messages()` with agent filter. New aggregate query: `SELECT to_agent, COUNT(*) FROM messages WHERE status = 'processing' GROUP BY to_agent`. Appended to `squad-orchestrator.md` in a new "Fleet Metrics" section. |
| Busy-time tracking | When an agent has been in "busy" status for an unusually long time, the orchestrator should know. Without a `status_updated_at` field, this is impossible. The field already exists in the `agents` DB schema (`status_updated_at` column). | LOW | `status_updated_at` is already in `agents` table and set on every `update_agent_status()` call. No schema migration needed. Context command reads it and derives busy duration. |

### Differentiators (Competitive Advantage)

Features that distinguish squad-station from generic multi-agent frameworks. Competitors handle these poorly or not at all in a CLI-native, tmux-based model.

| Feature | Value Proposition | Complexity | Dependencies on Existing |
|---------|-------------------|------------|--------------------------|
| Task-role alignment hints in orchestrator context | Most orchestration frameworks provide routing only at setup time. Squad-station can surface "agent X has received 8 tasks, 3 of which appear misrouted based on role description mismatch." This is qualitative intelligence the orchestrator AI can act on — not just counters. Implementing even a lightweight version (role keyword vs. task body keyword overlap) gives the orchestrator a signal no competing tool provides in a file-based context. | MEDIUM | Requires lightweight text matching in context.rs: compare recent task bodies against agent role/description keywords. Pure Rust string ops, no NLP crate. Output as bullet list in `squad-orchestrator.md`. |
| Orchestrator-controlled cloning (not auto-scaling) | Unlike Kubernetes-style auto-scaling based on CPU metrics, squad-station deliberately keeps the scaling decision with the orchestrator AI. The CLI provides the mechanism (`clone`); the AI decides when. This is the correct abstraction: AI orchestrators reason about task semantics, not resource metrics. No competing tool surfaces this distinction cleanly. | LOW | Design decision enforced by API surface: `clone` takes an agent name and returns the new name. No threshold config, no auto-trigger. The orchestrator calls `clone` when it decides to, based on workload data from `squad-orchestrator.md`. |
| Auto-incremented clone naming with project prefix | `<project>-<tool>-<role>-2`, `-3`, etc. Names are deterministic, unique per project, and follow the existing `<project>-<tool>-<role>` convention. The orchestrator can parse clone names without additional metadata. Most frameworks use UUIDs or timestamps, which are opaque to the AI. | LOW | Name generation: query DB for agents matching `<project>-<tool>-<role>-*`, find max suffix, increment. Pure string/integer logic. Existing naming convention from v1.1 already in init.rs. |
| System-suggested roles based on project context | When the wizard detects an SDD workflow (bmad, gsd, superpower) from the first wizard page, it can suggest role templates appropriate to that workflow. This is proactive guidance that competitors do not offer in a setup wizard. | MEDIUM | SDD workflow value (`WizardResult.sdd`) is already set on page 1 of the wizard. Role template list shown on the WorkerPage can filter or reorder based on `sdd` value. No new state needed. |
| Fleet metrics without daemon | Other tools require a running daemon to collect metrics. Squad-station derives metrics on-demand from SQLite at `context` generation time. Stateless, zero overhead, instant for any project size at team scale (tens of agents). | LOW | Pure DB aggregate queries in a single `context` command invocation. No background process, no metric store, no time-series DB. Correct for the stateless CLI design constraint. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Auto-scaling: clone agents automatically when queue depth exceeds threshold | Sounds powerful — fewer manual decisions. Some orchestration platforms (Kubernetes, Ray) do this. | Squad-station is a stateless CLI. Auto-scaling requires a persistent observer process polling queue depth and firing `clone` commands. That is a daemon — explicitly out of scope. Additionally, task queue depth is a poor proxy for whether cloning is the right action (the orchestrator may be intentionally serializing work). | Surface queue depth in `squad-orchestrator.md`. Let the orchestrator AI decide. This keeps decision-making with the entity that understands task semantics. |
| Agent-to-agent communication routing through the CLI | Users want workers to communicate directly: agent A sends a message to agent B. Makes sense in theory. | Squad-station's design has all communication routed through the orchestrator. Direct agent-to-agent messaging creates untracked state, breaks the orchestrator's situational awareness, and requires a message routing layer the CLI does not have. This is explicitly called out as out-of-scope in PROJECT.md. | Orchestrator receives signal from agent A, evaluates output, forwards relevant context to agent B in the next task. This keeps routing centralized and auditable. |
| Role-based access control (which agent can receive which task type) | Users want to enforce that QA agents can only receive test tasks. Sounds like guardrails. | RBAC enforcement at the CLI layer adds complexity, breaks the send command's simplicity, and moves task-semantics decisions from the AI to the CLI tool. The AI is better positioned to enforce routing via its own reasoning. | Role templates + routing hints in `squad-orchestrator.md` guide the orchestrator AI to route correctly. The AI can self-enforce. CLI does not police content. |
| Template marketplace / community role registry | Users want to download community-curated role templates. Seems like a feature win. | Requires a network call, a registry service, versioning, and trust verification — all for what amounts to a few strings (role name, description, model suggestion). Network dependency in a stateless CLI that currently has zero runtime dependencies is a regression. | Embed a curated set of 8–12 role templates directly in the binary (compile-time constants). Small teams cover 90% of use cases. Custom option covers the rest. |
| Cloning with modified configuration (different model or description) | User wants `clone --model opus` to clone but upgrade the model. More power, more control. | Creates divergence from the source agent. The orchestrator's mental model of "this agent is a clone of that agent" breaks. Fleet coordination relies on clones being identical workers. Divergent clones must be treated as distinct agents — better served by `init --tui` add-agents flow or `register`. | Clone = identical copy. For a different configuration, use `squad-station register` (existing command) to create a fully new agent. |
| Task-role alignment scoring with ML embeddings | For richer misrouting detection, vector similarity between task body and role description. | Binary size would increase dramatically with embedding models. Adds inference latency to the `context` command. Correctness depends on embedding quality. Overkill for team-scale multi-agent coordination where the orchestrator AI already has full semantic understanding. | Keyword overlap heuristic (pure Rust string ops): extract nouns from task body, check against role/description keywords. Sufficient signal for orchestrator guidance. Flag tasks where no keyword overlap exists. |

---

## Feature Dependencies

```
[Role templates in wizard]
    └──requires──> [template data structure: role, description, default_model, routing_hints]
    └──feeds──> [wizard.rs WorkerPage: radio/list template selector]
    └──feeds──> [wizard.rs WorkerPage: model pre-fill from template.default_model]
    └──feeds──> [init.rs generate_squad_yml(): description field from template]
    └──feeds──> [context.rs build_orchestrator_md(): routing hints in Session Routing section]
    └──optional: SDD-aware template filtering]
        └──depends on──> [WizardResult.sdd from page 1 of wizard] (already exists)

[Orchestrator intelligence data in squad-orchestrator.md]
    └──requires──> [DB aggregate query: pending message count per agent]
        └──depends on──> [messages table with status='processing' and to_agent column] (already exists)
    └──requires──> [busy-time derivation from status_updated_at]
        └──depends on──> [agents table, status + status_updated_at columns] (already exists)
    └──optional: task-role alignment keyword check]
        └──depends on──> [recent completed messages per agent] (already in messages table)
        └──depends on──> [agent description field] (already in agents table)
    └──feeds──> [context.rs build_orchestrator_md(): new "Fleet Metrics" section]
    └──requires no DB schema changes

[Dynamic agent cloning: squad-station clone <agent-name>]
    └──requires──> [new Commands::Clone { agent: String } in cli.rs]
    └──requires──> [new src/commands/clone.rs]
        └──reads──> [db::agents::get_agent(name)] (already exists)
        └──writes──> [db::agents::insert_agent(new_name, ...)] (already exists)
        └──calls──> [tmux::launch_session(new_name)] (existing tmux session launch)
        └──derives──> [auto-incremented name: query DB for existing clones, max suffix + 1]
    └──feeds──> [TUI dashboard: picks up new agent on next poll cycle automatically]
        └──depends on──> [ui.rs existing list_agents() poll loop] (already exists)

[Role templates] ──independent of──> [orchestrator intelligence data]
[Role templates] ──independent of──> [dynamic cloning]
[Orchestrator intelligence data] ──independent of──> [dynamic cloning]

[Cloned agent] ──appears in──> [TUI dashboard] (zero new code: existing poll loop)
[Orchestrator intelligence data] ──informs──> [orchestrator decision to clone]
[Clone command] ──called by──> [orchestrator AI based on fleet metrics]
```

### Dependency Notes

- **Role templates require no new DB columns:** Template selection in the wizard sets the existing `role`, `description`, and `model` fields. The template data structure lives only in Rust source (compile-time constants). Zero schema migration.
- **Orchestrator intelligence data requires no DB schema migration:** `status_updated_at` and `to_agent` already exist. The only addition is aggregate SELECT queries in `context.rs` and a new section appended to the generated markdown.
- **Clone command requires one new subcommand file:** `src/commands/clone.rs` is the only new file. It reuses `get_agent`, `insert_agent`, and tmux session launch — all existing functions. Name auto-increment logic is a DB query + integer arithmetic, no new crate.
- **TUI live update for cloned agents is free:** The existing `ui.rs` poll loop calls `list_agents()` on every refresh interval. A clone registered in DB appears on the next cycle. No TUI changes needed for the "agents appear immediately" requirement.
- **All three features are independent:** No feature blocks another. They can be developed in parallel or in any sequential order.

---

## MVP Definition

### This Milestone (v1.8 Smart Agent Management)

Minimum set to ship v1.8 as a coherent release. All five items from PROJECT.md active requirements.

- [ ] Role template data structure — Rust const array of `RoleTemplate { role, description, default_model, routing_hints }` structs, compiled into binary. Minimum 8 templates covering: orchestrator, frontend-engineer, backend-engineer, fullstack-engineer, qa-engineer, devops-engineer, architect, code-reviewer. Plus `custom` option.
- [ ] Template selector in wizard WorkerPage — radio/list UI, populates role + description + model fields on selection. Custom option activates free-text role input.
- [ ] SDD-aware template ordering — when SDD workflow is known, show most-relevant templates first (not strict filter — all templates remain accessible). Ordering only.
- [ ] Fleet metrics in `squad-orchestrator.md` — new "Fleet Metrics" section: pending message count per agent, agent busy-time duration. Generated by `context` command on each invocation. No daemon, no schema change.
- [ ] `squad-station clone <agent-name>` command — reads source agent config, generates auto-incremented name, registers in DB, launches tmux session with same tool/role/model/description. Prints new agent name to stdout. Exits non-zero if source agent not found.

### Add After Validation (post-v1.8)

- [ ] Task-role alignment hint in `squad-orchestrator.md` — lightweight keyword overlap check between recent task bodies and role/description keywords. Add when orchestrators report misrouting confusion in practice. Trigger: user feedback on misrouting.
- [ ] Clone count limit guardrail — `clone` warns (does not error) when more than N agents with same role exist. N configurable in squad.yml or hardcoded default of 5. Add when teams hit resource/terminal real estate limits.
- [ ] `squad-station clone --n 3 <agent-name>` — batch clone shorthand. Add after single clone is validated. Reduces manual invocation for scale-up scenarios.

### Future Consideration (v2+)

- [ ] Template versioning — as squad-station evolves, embedded templates will need updates without breaking existing squad.yml files. Only relevant at larger user scale.
- [ ] User-defined template registry — local file (e.g., `.squad/templates.toml`) that augments built-in templates. Addresses power users with recurring custom roles.
- [ ] Metrics history — persist fleet metrics snapshots in SQLite for trend analysis over a session. Only relevant if orchestrators need to see degradation over time, not just current state.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Role templates (data structure + compile-time constants) | HIGH — eliminates free-form role entry for 90% of users; reduces wizard friction immediately | LOW — Rust const array, no DB change, no new crate | P1 |
| Template selector UI in wizard WorkerPage | HIGH — required for templates to be usable; without UI, templates are dead code | LOW — extends existing radio-selector pattern in wizard.rs | P1 |
| Model pre-fill from template | HIGH — removes a manual step for common role/model pairings | LOW — set `model_input` default when template selected | P1 |
| Pending message count per agent in `squad-orchestrator.md` | HIGH — fundamental signal for overload detection; orchestrator cannot reason about queue depth without it | LOW — one aggregate SQL query, one new section in build_orchestrator_md() | P1 |
| Busy-time in `squad-orchestrator.md` | HIGH — detects stuck agents; `status_updated_at` already exists | LOW — timestamp diff, string formatting, appended to fleet metrics section | P1 |
| `squad-station clone <agent-name>` | HIGH — enables dynamic scale-up; orchestrator has no mechanism to expand fleet otherwise | MEDIUM — new subcommand file, name auto-increment query, tmux session launch | P1 |
| SDD-aware template ordering in wizard | MEDIUM — quality-of-life for SDD workflow users; all templates remain accessible regardless | LOW — sort/reorder template list based on WizardResult.sdd, no new state | P2 |
| Routing hints from templates in `squad-orchestrator.md` | MEDIUM — richer orchestrator guidance; depends on templates being selected in wizard | LOW — template routing_hints field appended to agent description in context generation | P2 |
| Task-role alignment hint | MEDIUM — high value when misrouting occurs; low value before misrouting is observed | MEDIUM — keyword extraction, overlap check across messages + agent descriptions | P3 |

---

## Ecosystem Patterns Observed

### Role Templates in Multi-Agent Frameworks (HIGH confidence — CrewAI docs + MetaGPT patterns)

CrewAI defines agents with `role`, `goal`, and `backstory` fields. Common software team roles: Engineering Lead, Senior Software Engineer, QA Engineer, Backend Engineer, Frontend Engineer, Test Engineer. MetaGPT encodes roles like Product Manager, Architect, Engineer, QA. The consistent pattern across frameworks: 6–12 predefined roles covering a standard software development team, plus a mechanism to override with custom definitions.

For squad-station, the equivalent is a `RoleTemplate` struct with `role` (stored in DB), `description` (stored in DB as agent description), `default_model` (pre-fills wizard model selector), and `routing_hints` (appended to agent description for orchestrator context). This maps cleanly to existing DB fields — no schema change.

### Dynamic Agent Cloning (MEDIUM confidence — Microsoft multi-agent patterns, frontiersin.org DRTAG research)

The "master-clone" architecture in Microsoft's multi-agent patterns describes a single orchestrator spinning off copies of a worker agent for parallel subtasks. DRTAG (Dynamic Real-Time Agent Generation) research confirms this as a viable pattern for scaling without human intervention. The consistent behavior: clone inherits the source agent's full configuration (role, model, context), gets a unique name, operates identically to the source.

For squad-station, the natural implementation: `clone` reads source agent record from DB, generates name `<source-name>-2` (or `-N` for the next available suffix), registers in DB, launches a new tmux session. The orchestrator AI receives the new agent name via stdout and can route tasks to it immediately.

### Workload Metrics for Orchestrator Intelligence (MEDIUM confidence — multi-agent observability papers, IBM agent orchestration docs)

Multi-agent observability research identifies these key metrics for orchestrator decision-making: (1) pending task queue depth per agent, (2) agent utilization (busy vs. idle ratio over time), (3) task completion time. Squad-station can surface (1) directly from the messages table and (2) from `status_updated_at` duration. Task completion time requires completed_at minus created_at — also available in the existing schema.

The correct delivery mechanism for squad-station: append to `squad-orchestrator.md` at each `context` command invocation. The orchestrator AI reads this file as part of its pre-flight and has current metrics without polling. No daemon, no push notification — consistent with the stateless CLI design.

### Auto-Increment Naming (HIGH confidence — existing squad-station convention)

The `<project>-<tool>-<role>` naming convention is already established (v1.1). For clones, the natural extension is `<project>-<tool>-<role>-2`, `...-3`, etc. The original agent has no numeric suffix (not `-1`). This mirrors standard replica naming in systems like Kubernetes (pod-xxxx suffixes) but uses sequential integers for human readability. The orchestrator AI can parse this pattern to understand the team structure.

---

## Sources

- [CrewAI Agents Documentation](https://docs.crewai.com/en/concepts/agents) — role, goal, backstory field patterns; software team role examples (HIGH confidence)
- [Microsoft ISE Blog: Patterns for Building a Scalable Multi-Agent System](https://devblogs.microsoft.com/ise/multi-agent-systems-at-scale/) — dynamic agent spawning patterns, orchestrator coordination (MEDIUM confidence)
- [Frontiers in AI: Auto-scaling LLM-based multi-agent systems through dynamic integration](https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1638227/full) — DRTAG pattern, dynamic agent generation (MEDIUM confidence)
- [IBM: AI Agent Orchestration](https://www.ibm.com/think/topics/ai-agent-orchestration) — orchestrator metrics, workload balancing (MEDIUM confidence)
- [Microsoft Azure: AI Agent Design Patterns](https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns) — multi-agent architecture patterns (MEDIUM confidence)
- [Agentic AI Systems Guide: Scaling Multi-Agent AI Systems](https://agenticaiguide.ai/ch_8/sec_8-3.html) — elastic scaling, stateless cloning patterns (MEDIUM confidence)
- Codebase (verified directly): `src/commands/wizard.rs`, `src/commands/context.rs`, `src/db/agents.rs`, `src/db/messages.rs`, `src/cli.rs`, `src/tmux.rs`, `src/commands/register.rs` — existing structure for all dependency claims (HIGH confidence)

---

*Feature research for: squad-station v1.8 Smart Agent Management — Role templates, orchestrator intelligence data, dynamic agent cloning*
*Researched: 2026-03-19*
