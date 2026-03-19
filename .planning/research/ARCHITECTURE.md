# Architecture Research

**Domain:** Rust CLI — stateless binary with embedded SQLite, ratatui TUI, tmux integration (v1.8 Smart Agent Management)
**Researched:** 2026-03-19
**Confidence:** HIGH — all findings derived from direct source inspection of the v1.7 codebase. No external sources required; the question is integration-only, not ecosystem discovery.

---

## Existing Architecture (v1.7 Baseline)

### System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Entry Point                                  │
│   main.rs → SIGPIPE handler → Cli::parse() → run(cli)               │
└──────────────────────────┬──────────────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────────────┐
│                       CLI Dispatch (src/cli.rs)                      │
│   Cli { command: Option<Commands> }                                  │
│   None  →  welcome TUI or print_welcome()                            │
│   Some  →  match to subcommand handler                               │
└──────────┬──────────────────────────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────────────────────────┐
│                   Commands Layer (src/commands/)                     │
│  welcome.rs  wizard.rs  init.rs   ui.rs    send.rs  signal.rs        │
│  agents.rs   context.rs status.rs view.rs  peek.rs  register.rs      │
│  list.rs     notify.rs  close.rs  reset.rs freeze.rs clean.rs        │
│  diagram.rs  helpers.rs                                              │
└──────┬─────────────────────────┬───────────────────────────────────┘
       │                         │
┌──────▼──────────┐    ┌─────────▼─────────────────────────────────┐
│  src/tmux.rs    │    │  src/db/  (SQLite via sqlx)                │
│  send_keys      │    │  mod.rs → connect() → pool setup           │
│  inject_body    │    │  agents.rs → insert/get/list/update        │
│  session_exists │    │  messages.rs → insert/list/update          │
│  launch_agent   │    │  migrations/ → auto-applied on connect     │
│  (arg builders) │    └────────────────────────────────────────────┘
└─────────────────┘
```

### Existing Component Inventory

| Component | Responsibility | v1.8 Impact |
|-----------|----------------|-------------|
| `src/main.rs` | Entry point, SIGPIPE, None/Some dispatch | No change |
| `src/cli.rs` | clap `Commands` enum | Modify: add `Clone` variant |
| `src/config.rs` | YAML parsing, DB path, session name sanitization | No change |
| `src/tmux.rs` | All tmux shell-outs, arg builders | No change for core features |
| `src/db/agents.rs` | Agent CRUD, status updates | Modify: add metrics queries |
| `src/db/messages.rs` | Message CRUD, priority ordering | Modify: add metrics queries |
| `src/db/migrations/` | SQL schema, auto-applied by sqlx | No new migration needed |
| `src/commands/wizard.rs` | Multi-page ratatui form, `WizardState`, `AgentInput` | Modify: add role template selection page |
| `src/commands/init.rs` | Wizard guard, squad.yml generation, registration | No change |
| `src/commands/context.rs` | `build_orchestrator_md()`, context file write | Modify: add metrics section |
| `src/commands/ui.rs` | `App` state, connect-per-refresh loop, ratatui draw | Modify: reflect live clone addition |
| `src/commands/helpers.rs` | Status colorization, reconcile utilities | No change |
| `src/commands/diagram.rs` | ASCII agent fleet diagram | No change |

---

## v1.8 Feature Integration Analysis

### Feature 1: Agent Role Templates in Wizard

**Current state:** The wizard `WorkerPage` has four text/selector inputs: name, role (free-text or radio), provider, model. The `AgentInput` struct that exits the wizard carries `{ name, role, provider, model, description }`. There is no concept of a "template" — users type role and description manually.

**What templates add:** A pre-selection step before the per-worker form. The user picks a template (e.g., "frontend-dev", "architect", "qa-engineer", "custom") from a list. Picking a non-custom template pre-fills `role`, `description`, and a suggested model. The user can still edit any field.

**Integration points:**

New data in `src/commands/wizard.rs` — no new file needed:

```rust
// src/commands/wizard.rs — new section, pure data (no I/O)
pub struct RoleTemplate {
    pub id: &'static str,        // "frontend-dev"
    pub display: &'static str,   // "Frontend Developer"
    pub role: &'static str,      // pre-fills AgentInput.role
    pub description: &'static str,
    pub suggested_model: Option<&'static str>, // pre-fills ModelSelector default
    pub routing_hint: &'static str, // shown in template picker; not stored in DB
}

pub const ROLE_TEMPLATES: &[RoleTemplate] = &[
    RoleTemplate { id: "architect", display: "Architect / Planner", role: "architect",
        description: "Designs system architecture, creates technical specs, reviews PRs",
        suggested_model: Some("opus"), routing_hint: "Reasoning, architecture, planning" },
    RoleTemplate { id: "implementer", display: "Implementer", role: "implementer",
        description: "Writes code, fixes bugs, implements features from specs",
        suggested_model: Some("sonnet"), routing_hint: "Coding, build, fix, deploy" },
    RoleTemplate { id: "qa", display: "QA Engineer", role: "qa",
        description: "Writes tests, validates implementations, catches regressions",
        suggested_model: Some("sonnet"), routing_hint: "Testing, validation, quality" },
    RoleTemplate { id: "custom", display: "Custom (type your own)", role: "",
        description: "", suggested_model: None, routing_hint: "Define your own role" },
];
```

New wizard page state in `WizardState`:

The `WorkerPage` flow becomes two steps:
1. `TemplatePickerPage` — radio list of `ROLE_TEMPLATES`; Enter selects and advances
2. `WorkerConfigPage` (existing) — fields pre-filled from selected template

`WizardState` needs a new field: `worker_template: usize` (selected template index per worker). When user picks a template on step 1, the wizard pre-populates the `WorkerConfigPage` input fields and advances to step 2. "Custom" skips pre-fill (blank inputs, same as today).

`WizardState::into_result()` — no change. Pre-fill is just initial state for text inputs; `AgentInput` is built from whatever the user typed.

**System suggestions ("smart suggestions"):** After the user has configured at least one worker, the wizard can suggest complementary roles. E.g., if user has an architect, suggest adding an implementer. This is cosmetic hint text below the template picker ("Suggested: add an Implementer to pair with your Architect"). It requires no new state — just render logic based on `WizardState.agents.len()` and existing role names.

**Files changed:**
- `src/commands/wizard.rs` — add `RoleTemplate` struct + `ROLE_TEMPLATES` const, `TemplatePickerPageState` field on `WizardState`, pre-fill logic, new render branch, hint text

No other files change. `AgentInput`, `WizardResult`, `generate_squad_yml`, and `init.rs` are untouched.

---

### Feature 2: Orchestrator Intelligence Data in `squad-orchestrator.md`

**Current state:** `context.rs::build_orchestrator_md()` generates a static markdown file. It reads `agents` from DB and `sdd_configs` from `squad.yml`. The "Session Routing" section lists agents with model and description. There are no runtime metrics in the output.

**What intelligence data adds:** The context file gains a "Fleet Status" section with live data: task-role alignment score, message counts per agent, estimated busy time. The orchestrator reads this file before acting and uses the data to decide routing (e.g., "agent-X has 5 pending tasks — clone or use agent-Y instead").

**Metrics needed:**
- Messages per agent: count of `processing` messages per `agent_name`
- Busy duration: time since `status_updated_at` when status = `busy`
- Misrouting hint: comparing `to_agent` role against task category keywords (heuristic, not ML)

**Integration points:**

New DB query functions in `src/db/messages.rs`:

```rust
// src/db/messages.rs
pub struct AgentMetrics {
    pub agent_name: String,
    pub processing_count: i64,
    pub completed_count: i64,
}

pub async fn agent_metrics(pool: &SqlitePool) -> anyhow::Result<Vec<AgentMetrics>> {
    // GROUP BY agent_name with COUNT filtered by status
    // Single query, no schema change — uses existing status column
    sqlx::query_as::<_, AgentMetrics>(
        "SELECT agent_name,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_count,
                COUNT(CASE WHEN status = 'completed'  THEN 1 END) as completed_count
         FROM messages
         GROUP BY agent_name"
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
```

New busy-duration calculation in `src/db/agents.rs`:

```rust
// src/db/agents.rs — pure function, no I/O, unit-testable
pub fn busy_duration_seconds(agent: &Agent, now_rfc3339: &str) -> Option<i64> {
    if agent.status != "busy" { return None; }
    let updated = chrono::DateTime::parse_from_rfc3339(&agent.status_updated_at).ok()?;
    let now = chrono::DateTime::parse_from_rfc3339(now_rfc3339).ok()?;
    Some((now - updated).num_seconds())
}
```

Changes to `src/commands/context.rs::build_orchestrator_md()`:

The function signature changes to accept metrics:

```rust
pub fn build_orchestrator_md(
    agents: &[Agent],
    project_root: &str,
    sdd_configs: &[SddConfig],
    metrics: &[AgentMetrics],   // NEW parameter
) -> String
```

New "Fleet Status" section generated before "Session Routing":

```
## Fleet Status

| Agent | Role | Status | Queue | Busy For |
|-------|------|--------|-------|----------|
| my-project-claude-code-implementer | implementer | busy | 2 pending | 4m 12s |
| my-project-claude-code-architect   | architect   | idle | 0 pending | — |

**Routing guidance:**
- implementer has 2 queued tasks. If parallelizable, consider cloning:
  `squad-station clone my-project-claude-code-implementer`
- architect is idle — ready for new work.
```

Call site in `context::run()` gains two async calls before `build_orchestrator_md`:

```rust
let metrics = db::messages::agent_metrics(&pool).await?;
let now = chrono::Utc::now().to_rfc3339();
// busy_duration_seconds called inside build_orchestrator_md for each agent
let prompt_content = build_orchestrator_md(&agents, &project_root_str, sdd_configs, &metrics);
```

No schema migration needed. All data is derived from existing `messages.status`, `agents.status`, and `agents.status_updated_at` columns.

**Files changed:**
- `src/db/messages.rs` — add `AgentMetrics` struct + `agent_metrics()` async fn
- `src/db/agents.rs` — add `busy_duration_seconds()` pure fn
- `src/commands/context.rs` — update `build_orchestrator_md()` signature + add Fleet Status section + call `agent_metrics()`

Test impact: `build_orchestrator_md` is already a `pub fn` imported directly by integration tests. Tests pass `&[]` for the new `metrics` parameter (empty metrics = no Fleet Status table rendered).

---

### Feature 3: Dynamic Agent Cloning (`squad-station clone <agent>`)

**Current state:** `register.rs` inserts an agent into DB by name. `init.rs` calls `tmux::launch_agent()` after registration. There is no command that combines "copy an existing agent's config + auto-increment name + launch tmux session." Naming convention is `<project>-<tool>-<role>`.

**What cloning adds:** A new subcommand `clone` that:
1. Reads the source agent from DB (name, tool, role, model, description)
2. Generates a new name using auto-increment suffix: `<source-name>-2`, `<source-name>-3`, etc.
3. Inserts the new agent into DB with the same config
4. Launches a new tmux session for the new agent

**Auto-increment logic:** Query DB for all agent names matching `<source-name>-N` pattern, find highest N, use N+1. If source agent has no existing clones, use `-2` (original is implicitly -1/base). Implemented as a pure function for testability.

**Integration points:**

New file `src/commands/clone.rs`:

```rust
// src/commands/clone.rs
pub async fn run(source_name: String, json: bool) -> anyhow::Result<()> {
    // 1. Load config + connect DB
    // 2. get_agent(&pool, &source_name) → bail if not found
    // 3. generate_clone_name(&pool, &source_name).await → new_name
    // 4. insert_agent(&pool, &new_name, tool, role, model, description)
    // 5. tmux::launch_agent(&new_name, tool) → same as init does
    // 6. update_agent_status(&pool, &new_name, "idle")
    // 7. Output: clone created (new_name)
}

async fn generate_clone_name(pool: &SqlitePool, source: &str) -> anyhow::Result<String> {
    // Query: SELECT name FROM agents WHERE name LIKE '<source>-%'
    // Parse suffixes, find max N, return source + "-" + (N+1)
    // If no clones exist: return source + "-2"
}
```

`src/cli.rs` — add variant to `Commands` enum:

```rust
Clone {
    /// Name of the agent to clone
    agent: String,
    #[arg(long)]
    json: bool,
},
```

`src/commands/mod.rs` — add `pub mod clone;`

`src/main.rs` — add match arm:

```rust
Commands::Clone { agent, json } => commands::clone::run(agent, json).await,
```

**tmux session launch:** `tmux::launch_agent()` is called in `init.rs` after registering each agent. Clone uses the same function with the new name and the source agent's tool. No changes to `tmux.rs` needed.

**Squad.yml update:** Cloned agents are NOT written to `squad.yml`. The file is a human-edited config; runtime clones are ephemeral DB-only entries. This is consistent with `register.rs` which also does not update `squad.yml`. The context file (`squad-orchestrator.md`) is updated on next `squad-station context` invocation, which is lightweight and orchestrator-driven.

**Files changed:**
- `src/commands/clone.rs` — NEW file
- `src/cli.rs` — add `Clone` variant
- `src/commands/mod.rs` — add `pub mod clone;`
- `src/main.rs` — add match arm

---

### Feature 4: TUI Live Update for Cloned Agents

**Current state:** `ui.rs::fetch_snapshot()` calls `db::agents::list_agents()` on every 3-second refresh. The `App.agents` vec is replaced completely on each tick. New agents appearing in DB between ticks are naturally picked up at next refresh.

**What live update requires:** No architectural change. The connect-per-refresh pattern already handles this:

```
clone command writes agent to DB
    ↓ (within 3 seconds)
TUI refresh tick → fetch_snapshot() → list_agents() returns new clone
    ↓
App.agents updated → draw_ui() renders new row in agent list
```

The selected-agent cursor may need a guard: if the agent at `selected_index` is replaced by a different agent (list reordered), the selection tracks by index not name. This is the existing behavior for any status change. If the list grows by one entry, the user's selection stays on the same index (which points to the same logical agent unless the clone sorts alphabetically before the selection).

**Optional polish:** Highlight newly-appeared agents in the TUI agent list for one refresh cycle (e.g., with a different background color). This requires storing a `HashSet<String>` of "previously seen agent names" in `App` state and comparing on each fetch. This is additive, isolated to `ui.rs`, and has no DB or schema impact.

**Files changed:**
- `src/commands/ui.rs` — optionally add `seen_agents: HashSet<String>` to `App` for new-agent highlight

---

### Feature 5: Seamless Agent Coordination

**What this means in practice:** Original + clone agents share the same DB and project directory. They receive tasks via `send`, complete them via `signal`, and are listed in the roster. The orchestrator routes to them by name. No special coordination mechanism is needed in the CLI — the existing stateless architecture handles N agents naturally.

**The orchestrator.md update (Feature 2) is the coordination mechanism:** When the context file shows a clone with idle status, the orchestrator knows to use it. When the context file shows an overloaded agent, the orchestrator knows to clone. The CLI provides the data; the AI makes the decision.

**No new CLI components required for this feature.**

---

## Complete File Change Matrix

| File | Change Type | Feature | What Changes |
|------|-------------|---------|--------------|
| `src/commands/wizard.rs` | Modify | Templates | Add `RoleTemplate` struct + `ROLE_TEMPLATES` const; `TemplatePickerPageState` in `WizardState`; pre-fill logic; new render branch; hint text |
| `src/db/messages.rs` | Modify | Intelligence | Add `AgentMetrics` struct + `agent_metrics()` async query |
| `src/db/agents.rs` | Modify | Intelligence | Add `busy_duration_seconds()` pure fn |
| `src/commands/context.rs` | Modify | Intelligence | Update `build_orchestrator_md()` signature; add Fleet Status section; call `agent_metrics()` |
| `src/commands/clone.rs` | New | Cloning | `run()` + `generate_clone_name()` — full clone flow |
| `src/cli.rs` | Modify | Cloning | Add `Clone { agent, json }` variant |
| `src/commands/mod.rs` | Modify | Cloning | Add `pub mod clone;` |
| `src/main.rs` | Modify | Cloning | Add `Clone` match arm |
| `src/commands/ui.rs` | Modify (optional) | Live update | Add `seen_agents: HashSet<String>` to `App` for new-agent highlight |

**No schema migrations needed.** All new DB operations use existing columns.

---

## Data Flow

### Clone Flow

```
User: squad-station clone my-project-claude-code-implementer
    ↓
clone::run()
    ├── config::load_config() + db::connect()
    ├── db::agents::get_agent("my-project-claude-code-implementer")
    │       → Agent { tool, role, model, description, ... }
    ├── generate_clone_name(&pool, "my-project-claude-code-implementer")
    │       → SELECT names matching pattern → "my-project-claude-code-implementer-2"
    ├── db::agents::insert_agent("my-project-claude-code-implementer-2", same config)
    ├── tmux::launch_agent("my-project-claude-code-implementer-2", tool)
    │       → new tmux session created
    └── stdout: "Cloned 'my-project-claude-code-implementer' as 'my-project-claude-code-implementer-2'"
```

### Intelligence Data Flow

```
User: squad-station context
    ↓
context::run()
    ├── config::load_config() + db::connect()
    ├── db::agents::list_agents() → Vec<Agent>
    ├── db::messages::agent_metrics() → Vec<AgentMetrics>   [NEW]
    ├── build_orchestrator_md(agents, root, sdd_configs, metrics)
    │       ├── Fleet Status section: agents × metrics join → table rows
    │       │       busy_duration_seconds(agent, now) → "4m 12s" or "—"
    │       ├── routing guidance: if any agent.processing_count > 1 → clone hint
    │       └── [existing sections unchanged]
    └── write to .claude/commands/squad-orchestrator.md (or .gemini/commands/)
```

### Template Wizard Flow

```
User runs: squad-station init --tui
    ↓
wizard::run()  [existing entry point]
    ↓
ProjectPage → SddPage → OrchestratorConfigPage
    ↓
WorkerCountPage
    ↓ (for each worker)
TemplatePickerPage [NEW]
    │   Radio list: Architect, Implementer, QA Engineer, Custom
    │   Hint text: "Suggested: add Implementer to pair with Architect"
    ↓
WorkerConfigPage [MODIFIED: pre-filled from template]
    │   Name, Role, Provider, Model, Description
    │   (user can override any field)
    ↓ (repeat for each worker)
WizardResult { project, sdd, orchestrator, agents: Vec<AgentInput> }
    ↓
generate_squad_yml() → squad.yml (no change to output format)
    ↓
init::register_agents() → DB insert (no change)
```

---

## Architectural Patterns in Use (and how v1.8 follows them)

### Pattern 1: Command-per-file

Each subcommand lives in `src/commands/<name>.rs` with `pub async fn run(...) -> anyhow::Result<()>`. Clone follows this exactly with `src/commands/clone.rs`.

### Pattern 2: DB layer as thin CRUD + queries

The DB modules (`agents.rs`, `messages.rs`) contain only SQL operations. Business logic (e.g., clone naming, busy duration formatting) lives in the command file or as pure functions in the DB module. New `agent_metrics()` query follows this: raw SQL, returns typed structs.

### Pattern 3: Argument builder functions in tmux.rs

`clone.rs` calls `tmux::launch_agent()` — the existing public API. No new tmux calls. The clone command does not inline `Command::new("tmux")`.

### Pattern 4: Pure rendering and classification functions

`busy_duration_seconds()` is a pure fn (no I/O). `build_orchestrator_md()` remains a pure fn — takes metrics as parameter, no DB calls inside. `generate_clone_name()` is async (DB query) but isolated and directly testable.

### Pattern 5: Connect-per-refresh in TUI

TUI live update for clones requires no new mechanism. The existing connect-per-refresh pattern picks up new DB rows naturally. Any new write (hypothetical status poll) would follow the established short-lived writable pool pattern.

---

## Suggested Build Order

Dependencies flow: DB layer → command layer → CLI → TUI polish.

### Phase 1: Orchestrator Intelligence Data (foundation for orchestrator-guided cloning)

**Why first:** `agent_metrics()` and `busy_duration_seconds()` are pure additions with no side effects. `build_orchestrator_md()` signature change affects integration tests but the fix is mechanical (pass `&[]` for metrics). Landing this first gives the orchestrator the data it needs to decide when to clone — which makes the clone command useful immediately upon delivery.

Work:
- `src/db/messages.rs` — `AgentMetrics` + `agent_metrics()`
- `src/db/agents.rs` — `busy_duration_seconds()`
- `src/commands/context.rs` — updated `build_orchestrator_md()` + Fleet Status section
- Update any existing tests that call `build_orchestrator_md()` directly (add `&[]` arg)

### Phase 2: Dynamic Agent Cloning

**Why second:** Requires DB layer (Phase 1 establishes pattern but is not a hard dependency). Clone is independent of templates. Shipping it before templates means the orchestrator can immediately use it from the updated context file.

Work:
- `src/commands/clone.rs` — NEW
- `src/cli.rs` — `Clone` variant
- `src/commands/mod.rs` — `pub mod clone;`
- `src/main.rs` — match arm
- Tests: `generate_clone_name()` unit tests (pure DB query, testable with `setup_test_db()`)

### Phase 3: Agent Role Templates in Wizard

**Why third:** Wizard changes are entirely self-contained to `wizard.rs`. They do not affect the clone command, metrics, or DB. They improve the onboarding experience for the v1.8 milestone but do not block orchestrator functionality. Landing last keeps wizard changes separate from the core runtime features.

Work:
- `src/commands/wizard.rs` — `RoleTemplate` struct + `ROLE_TEMPLATES` + `TemplatePickerPageState` + render branch + hint text
- Tests: template pre-fill logic (can be tested without ratatui — pre-fill is just `TextInputState.value` assignment)

### Phase 4: TUI Live Update Polish (optional)

**Why last:** The live update already works via connect-per-refresh. This phase adds optional new-agent highlight only if time allows. It is purely cosmetic, isolated to `ui.rs`, and has zero impact on other features.

Work:
- `src/commands/ui.rs` — `seen_agents: HashSet<String>` + per-tick diff + highlight rendering

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Writing clones to squad.yml

**What people do:** After cloning, append the new agent to squad.yml so the team is "persisted."
**Why it's wrong:** Squad.yml is the human-authored source of truth. Clones are runtime scaling decisions. If the user re-inits, clones are discarded — which is correct behavior. Writing them to squad.yml makes re-init destructive and confusing.
**Do this instead:** Clones live in DB only (same as `register` behavior). Orchestrator discovers them via `squad-station agents` or `squad-orchestrator.md`.

### Anti-Pattern 2: Putting metrics calculation in build_orchestrator_md

**What people do:** Pass only `&[Agent]` to `build_orchestrator_md()` and make DB calls inside the function.
**Why it's wrong:** Makes `build_orchestrator_md()` impure (async, I/O). Breaks the existing test pattern where it is imported as a pure `pub fn`. All existing tests call it synchronously.
**Do this instead:** Fetch `Vec<AgentMetrics>` in `context::run()` before calling `build_orchestrator_md()`. Pass as parameter. Function stays pure, stays unit-testable.

### Anti-Pattern 3: Hardcoded clone suffix "-clone"

**What people do:** Name clones `<source>-clone`, `<source>-clone-2`, etc.
**Why it's wrong:** The existing naming convention is `<project>-<tool>-<role>`. A `-clone` suffix breaks pattern matching in `signal.rs`, hook scripts, and orchestrator routing rules (which match on role, not suffix).
**Do this instead:** Auto-increment numeric suffix: `<source>-2`, `<source>-3`. The role and tool remain the same. Orchestrator knows it's a clone by the identical role + model, not by name.

### Anti-Pattern 4: Storing routing hints in the DB

**What people do:** Add a `routing_hint` column to agents table to persist template metadata.
**Why it's wrong:** Routing hints are advisory text for the orchestrator document. They change as the team composition changes. Storing them in DB creates a sync problem with the generated markdown.
**Do this instead:** Routing hints are constants in `ROLE_TEMPLATES` (compile-time data). They are rendered into `squad-orchestrator.md` based on agent role, not stored in DB. The orchestrator file is the integration surface.

### Anti-Pattern 5: Polling agent busy state from the TUI for clone decisions

**What people do:** TUI detects an overloaded agent and auto-clones.
**Why it's wrong:** The CLI is stateless by design. Auto-cloning decisions belong to the orchestrator AI, not to a monitoring TUI. The TUI's job is to display state, not to act on it.
**Do this instead:** TUI displays agent queue depth (visible if metrics are in the DB). Orchestrator reads `squad-orchestrator.md`, sees the queue depth, and invokes `squad-station clone` explicitly.

---

## Integration Boundaries

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `commands/clone.rs` ↔ `db/agents.rs` | `get_agent()` + `insert_agent()` | No new DB functions needed; clone reuses existing CRUD |
| `commands/clone.rs` ↔ `tmux.rs` | `launch_agent()` | Same function init.rs uses; no tmux.rs changes |
| `commands/context.rs` ↔ `db/messages.rs` | `agent_metrics()` [new] | Thin query fn, typed return |
| `commands/context.rs` ↔ `db/agents.rs` | `busy_duration_seconds()` [new pure fn] | Called inside `build_orchestrator_md()` per agent |
| `commands/wizard.rs` ↔ `std` | `ROLE_TEMPLATES` const data | No I/O — compile-time constant slice |

### External Surfaces

| Surface | v1.8 Change | Notes |
|---------|-------------|-------|
| `squad-orchestrator.md` | New "Fleet Status" table + clone hints | Orchestrator reads this on every task; additive change, safe |
| CLI surface | New `clone <agent> [--json]` subcommand | Documented in context file; orchestrator uses it |
| SQLite `agents` table | New rows from clone | Same schema; `INSERT` uses same path as `register` |
| SQLite `messages` table | New `agent_metrics()` read query | Read-only GROUP BY query; no write, no schema change |
| `squad.yml` | No change | Clones are DB-only, not written to config |
| TUI dashboard | Clones appear in agent list within 3 seconds | No protocol change; connect-per-refresh handles it |

---

## Sources

- Direct source inspection: `src/commands/wizard.rs`, `src/commands/context.rs`, `src/commands/register.rs`, `src/commands/send.rs`, `src/commands/ui.rs`, `src/db/agents.rs`, `src/db/messages.rs`, `src/tmux.rs`, `src/cli.rs`, `src/main.rs`
- Direct source inspection: all migration files `src/db/migrations/000{1-4}_*.sql`
- Project history and decisions: `.planning/PROJECT.md`
- Established patterns: connect-per-refresh in `ui.rs`; arg-builder in `tmux.rs`; pure-fn in `diagram.rs`/`welcome.rs`; command-per-file in `commands/`

---

*Architecture research for: Squad Station v1.8 — Agent Role Templates, Orchestrator Intelligence Data, Dynamic Agent Cloning*
*Researched: 2026-03-19*
