# Stack Research: v1.8 Smart Agent Management

**Domain:** Rust CLI — agent role templates, orchestrator intelligence metrics, dynamic agent cloning
**Researched:** 2026-03-19
**Confidence:** HIGH

> **Scope:** This document covers only what is NEW or CHANGED for v1.8 Smart Agent Management.
> The existing validated stack (ratatui 0.30, crossterm 0.29, tui-big-text 0.8, clap 4.5,
> tokio 1.37, sqlx 0.8, serde/serde_json 1.0, serde-saphyr, owo-colors 3, uuid 1.8, chrono 0.4,
> anyhow 1.0, libc 0.2) is NOT re-researched here. All findings below assume this baseline.

---

## Executive Summary

Zero new Rust crates are required for v1.8 Smart Agent Management. All three feature areas —
role templates in the wizard, orchestrator intelligence metrics, and dynamic agent cloning —
are implementable with the existing dependency set plus Rust stdlib. The only schema addition
is one new column (`busy_since`) on the `agents` table to enable accurate busy-time tracking;
this is a lightweight `ALTER TABLE ADD COLUMN` migration consistent with the existing pattern.

The npm package and curl installer require no changes for these features.

---

## Recommended Stack

### Core Technologies — No Version Changes

| Technology | Locked Version | Purpose | v1.8 Smart Agent Mgmt Status |
|------------|---------------|---------|------------------------------|
| clap 4 | 4.5.x | CLI subcommand dispatch | Add `Clone` variant to `Commands` enum — no version change |
| sqlx 0.8 | 0.8.x | SQLite queries + migrations | One new migration (busy_since column); all query patterns already established |
| chrono 0.4 | 0.4.x | Timestamp arithmetic | `Utc::now() - DateTime::parse_from_rfc3339(busy_since)` for busy-time metrics — already imported |
| ratatui 0.30 | 0.30.x | TUI rendering | Cloned agents appear in existing `ui.rs` agent list on next poll tick — no change |
| tokio 1 | 1.37 | Async runtime | No new async primitives needed |
| serde + serde_json | 1.0 | Serialization | Metrics output via `serde_json::json!` in JSON mode — already used in every command |
| uuid 1.8 | 1.8 | ID generation | `Uuid::new_v4()` already used in `insert_agent` — no change |
| std (stdlib) | stable | String ops, env, path | Role template matching, auto-increment naming, all via stdlib string ops |

### Supporting Libraries — No Additions Required

| Library | Version | Purpose | v1.8 Relevance |
|---------|---------|---------|----------------|
| anyhow 1.0 | 1.0 | Error propagation | `clone.rs` follows same `anyhow::Result<()>` pattern as all other commands |
| owo-colors 3 | 3.x | Colored output | Clone confirmation banner reuses existing color helpers |

---

## Feature-by-Feature Stack Analysis

### Feature 1: Agent Role Templates in Wizard

**What it needs:**
- A static list of `RoleTemplate` structs embedded in `wizard.rs` (or a new `templates.rs` module)
- A new wizard page: role-selector (radio list) that pre-fills tool, model, and description fields
- A "Custom" option that leaves all fields blank (existing behavior)
- Optional "Suggest for me" path that picks a template based on prior selections

**Implementation uses only stdlib + existing crates:**

```rust
// src/templates.rs — new file, zero additional deps
pub struct RoleTemplate {
    pub name: &'static str,        // "Backend Engineer"
    pub role: &'static str,        // "backend"
    pub model_hint: &'static str,  // "sonnet"
    pub description: &'static str, // pre-filled description shown in wizard
    pub routing_hint: &'static str,// appended to orchestrator routing section
}

pub const ROLE_TEMPLATES: &[RoleTemplate] = &[
    RoleTemplate {
        name: "Backend Engineer",
        role: "backend",
        model_hint: "sonnet",
        description: "Implements APIs, services, and database logic.",
        routing_hint: "API design, database, server-side logic",
    },
    // ... more templates
    RoleTemplate {
        name: "Custom",
        role: "",
        model_hint: "",
        description: "",
        routing_hint: "",
    },
];
```

**Why not TOML/JSON for templates:** `toml` crate would add a dependency and a file-read code
path for data that never changes at runtime. Static Rust structs compile to read-only memory,
are zero-cost at startup, and are validated at compile time. `include_str!` with a data file
adds complexity without benefit for a fixed 8-12 template list.

**Wizard integration:** The existing `ratatui` `List` widget (already used for Provider and
Model radio selectors in `wizard.rs`) handles the template selector without any new widget type.
The existing `KeyCode::Up/Down` navigation pattern applies directly.

**`context.rs` integration:** Templates have a `routing_hint` field. `build_orchestrator_md`
reads `agents[].description` from the DB (already stored). Templates write the description
at init time — no schema change, no new DB field. Routing hints can be embedded in
the description string itself or added as a separate optional DB column if finer control is needed.

---

### Feature 2: Orchestrator Intelligence Data in `squad-orchestrator.md`

**What it needs:**
- A new DB query in `db/agents.rs` or `db/messages.rs` for per-agent metrics
- A new migration adding `busy_since TEXT DEFAULT NULL` to `agents`
- `chrono` arithmetic for busy-time duration (already imported)
- Extended `build_orchestrator_md` in `context.rs` to emit a metrics section

**Schema addition — one new column (migration 0005):**

```sql
-- 0005_v18_metrics.sql
ALTER TABLE agents ADD COLUMN busy_since TEXT DEFAULT NULL;
```

`busy_since` is set to `Utc::now().to_rfc3339()` when an agent transitions to `busy`
status, and cleared to `NULL` on `idle`/`dead`. This avoids computing busy time from
`status_updated_at`, which is overwritten on every status change.

`UPDATE agents SET busy_since = ? WHERE name = ?` is added alongside the existing
`update_agent_status` call in `signal.rs` and `send.rs` — both already touch agent status.

**Metrics query — pure SQL, no new crate:**

```sql
-- Per-agent stats
SELECT
    a.name,
    a.role,
    a.status,
    a.busy_since,
    COUNT(m.id) AS total_messages,
    SUM(CASE WHEN m.status = 'processing' THEN 1 ELSE 0 END) AS pending_count,
    SUM(CASE WHEN m.status = 'completed' THEN 1 ELSE 0 END) AS completed_count
FROM agents a
LEFT JOIN messages m ON m.to_agent = a.name
WHERE a.role != 'orchestrator'
GROUP BY a.name
```

**chrono for busy duration — already in scope:**

```rust
// Already available: chrono::Utc, chrono::DateTime, chrono::Duration
let busy_duration = agent.busy_since.as_deref()
    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
    .map(|start| chrono::Utc::now().signed_duration_since(start));
```

**Output in `squad-orchestrator.md`:**
The metrics section is appended by `build_orchestrator_md` as a markdown table. No new
serialization format — plain string building already used throughout `context.rs`.

**What NOT to add:** Do not add `prometheus`, `metrics`, or any observability crate. The
orchestrator reads a markdown file, not a metrics endpoint. String-formatted markdown is
the correct output format.

---

### Feature 3: Dynamic Agent Cloning (`squad-station clone <agent>`)

**What it needs:**
- A new `Commands::Clone { name: String }` variant in `cli.rs`
- A new `src/commands/clone.rs` handler
- Auto-increment naming logic using stdlib string ops
- `insert_agent` (existing in `db/agents.rs`)
- tmux session launch (existing `launch_session` in `tmux.rs`)

**Auto-increment naming — pure stdlib:**

```rust
// Given agent name "myproj-claude-backend", produce "myproj-claude-backend-2"
// If "myproj-claude-backend-2" exists, produce "myproj-claude-backend-3", etc.
fn next_clone_name(base: &str, existing_names: &[String]) -> String {
    // Strip existing "-N" suffix if present to find the canonical base
    let root = if let Some(pos) = base.rfind('-') {
        let suffix = &base[pos+1..];
        if suffix.chars().all(|c| c.is_ascii_digit()) {
            &base[..pos]
        } else {
            base
        }
    } else {
        base
    };
    // Find highest existing clone number
    let max = existing_names.iter()
        .filter_map(|n| {
            n.strip_prefix(root)
                .and_then(|s| s.strip_prefix('-'))
                .and_then(|s| s.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(1);
    format!("{}-{}", root, max + 1)
}
```

**tmux session launch:** `tmux.rs` already has `launch_session(name, command)`. The clone
command reads the source agent's `tool` and `model` from DB, derives the session command
using the same logic as `init.rs`, and calls `launch_session`. No new tmux primitives needed.

**TUI live update:** `ui.rs` already re-fetches all agents from DB on each refresh tick
(connect-per-refresh pattern from v1.4 decision). A newly cloned agent registered in DB
appears on the dashboard within one tick interval. No changes to `ui.rs` needed.

---

## Schema Changes Summary

| Migration | File | Change | Reason |
|-----------|------|--------|--------|
| 0005 | `0005_v18_metrics.sql` | `ALTER TABLE agents ADD COLUMN busy_since TEXT DEFAULT NULL` | Enables accurate busy-time duration without rewriting status_updated_at |

No other schema changes. The `messages` table already contains enough data for
task-count and completion-rate metrics via the existing `status` and `to_agent` columns.

---

## Cargo.toml Changes

**None.** All three features use the existing dependency set.

The `version` field bump (e.g., `0.5.4` → `0.6.0`) is a project management step, not a
stack change.

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| Static `&[RoleTemplate]` Rust array | TOML file + `toml` crate | New dep, file-read code path, no compile-time validation — all overhead for data that never changes at runtime |
| Static `&[RoleTemplate]` Rust array | JSON via `include_str!` + `serde_json` | `serde_json` already present but `from_str` adds fallible parse at startup; static structs have zero startup cost |
| `busy_since` column for metrics | Derive from `status_updated_at` | `status_updated_at` is overwritten on every status change; not reliable for busy-time duration |
| SQL aggregation for message counts | Rust-side counting after `list_agents` + `list_messages` | SQL aggregation is one query vs N+1 fetches; more efficient and idiomatic with sqlx |
| stdlib string ops for clone naming | `regex` crate | Suffix detection (`rfind('-')` + `parse::<u32>()`) is 6 lines; regex would be massive overkill |
| Extend existing `tmux::launch_session` | New tmux wrapper | `launch_session` already handles new session creation cleanly; clone just passes different args |
| `ratatui` existing `List` widget for template selector | New custom widget | List with radio-style rendering is already used for Provider and Model selectors — identical pattern |

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `toml` crate | Role templates are static compile-time data, not user-editable config | Static Rust `const` structs in `src/templates.rs` |
| `prometheus` / `metrics` crates | Orchestrator reads markdown, not a metrics endpoint | Plain string formatting in `build_orchestrator_md` |
| `regex` crate | Clone name auto-increment is simple suffix arithmetic | `str::rfind`, `str::parse::<u32>()`, `format!` |
| `reqwest` | No HTTP calls needed for any v1.8 feature | N/A — not applicable |
| New tmux primitives | `launch_session` in `tmux.rs` handles all session creation | Pass cloned agent config to existing function |
| Daemon / background process | Stateless CLI constraint; clone command exits after registering | TUI polls DB on its own tick interval |

---

## Integration Points

| New Code | Integrates With | Integration Notes |
|----------|----------------|-------------------|
| `src/templates.rs` | `src/commands/wizard.rs` | Import `ROLE_TEMPLATES`; new wizard page pre-fills `AgentInput` fields |
| `src/templates.rs` | `src/commands/context.rs` | `routing_hint` from template description written at init time; `build_orchestrator_md` reads `agent.description` unchanged |
| `0005_v18_metrics.sql` | `src/db/agents.rs` | `update_agent_status` gains `busy_since` update; metrics query added as `get_agent_metrics()` |
| `src/commands/clone.rs` | `src/cli.rs` | `Commands::Clone { name }` variant; `match` arm in `main.rs` |
| `src/commands/clone.rs` | `src/db/agents.rs` | Read source agent → compute clone name → `insert_agent` |
| `src/commands/clone.rs` | `src/tmux.rs` | Call `launch_session` with clone agent's name and tool command |
| `src/commands/context.rs` | `src/db/agents.rs` | New `get_agent_metrics()` query feeds metrics section in `build_orchestrator_md` |

---

## Version Compatibility

All existing crate versions are fully compatible. No version bumps needed.

| Package | Locked Version | Compatibility Note |
|---------|---------------|-------------------|
| sqlx 0.8 | 0.8.x | `ALTER TABLE ADD COLUMN` migration follows existing 0003/0004 patterns exactly |
| chrono 0.4 | 0.4.x | `signed_duration_since` and `parse_from_rfc3339` are stable chrono API, used elsewhere in codebase |
| clap 4.5 | 4.5.x | New `Clone` subcommand is a trivial derive addition — same pattern as all other subcommands |
| ratatui 0.30 | 0.30.x | Template selector uses existing `List` widget + `ListState` — no new widget API |

---

## Sources

- `Cargo.toml` (local) — confirmed locked versions; zero deps to add
- `src/db/agents.rs` — confirmed `update_agent_status` signature; `busy_since` update slots in alongside existing `status` update
- `src/db/migrations/0003_v11.sql` — confirmed `ALTER TABLE ADD COLUMN` pattern for schema extension
- `src/commands/wizard.rs` — confirmed `List` widget + `KeyCode::Up/Down` radio selector pattern used for Provider and Model pages
- `src/commands/context.rs` — confirmed `build_orchestrator_md` string-building pattern; metrics section slots in as additional `push_str` block
- `src/tmux.rs` — confirmed `launch_session` function signature; clone command reuses directly
- `src/commands/register.rs` — confirmed `insert_agent` call pattern for new agent registration
- Rust stdlib (stable) — `str::rfind`, `str::parse::<u32>()`, `format!` — all needed for clone naming logic
- chrono 0.4 docs — `signed_duration_since`, `parse_from_rfc3339` confirmed stable API

---

*Stack research for: squad-station v1.8 Smart Agent Management — role templates, orchestrator metrics, dynamic cloning*
*Researched: 2026-03-19*
