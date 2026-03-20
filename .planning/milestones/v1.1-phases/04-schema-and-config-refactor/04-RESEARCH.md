# Phase 4: Schema and Config Refactor - Research

**Researched:** 2026-03-08
**Domain:** Rust / SQLite migrations (sqlx) / serde YAML deserialization / schema evolution
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CONF-01 | User can configure project using `project: myapp` string format in squad.yml | serde untagged enum or string deserialization; `ProjectConfig` needs to become `String` (or newtype) |
| CONF-02 | User can specify `model` and `description` for each agent and orchestrator in squad.yml | Add optional `model: Option<String>` and `description: Option<String>` to `AgentConfig` |
| CONF-03 | squad.yml no longer requires `command` field | Remove `command` from `AgentConfig`; tool infers launch command in Phase 5 |
| CONF-04 | squad.yml and DB use `tool` field instead of `provider` | Rename `provider` → `tool` in `AgentConfig` and `SquadConfig`; update all callers |
| MSGS-01 | System tracks message direction with `from_agent` and `to_agent` fields | Migration: add `from_agent TEXT`, `to_agent TEXT`; retain `agent_name` as nullable or drop; update CRUD |
| MSGS-02 | System records message type (task_request / task_completed / notify) | Migration: add `type TEXT NOT NULL DEFAULT 'task_request'`; update `insert_message` signature |
| MSGS-03 | System supports `processing` status alongside completed/failed | Migration: rename default status from `'pending'` → `'processing'`; update all callers including tests |
| MSGS-04 | System records `completed_at` timestamp when message finishes | Migration: add `completed_at TEXT`; populate in `update_status` when status becomes `completed` |
| AGNT-01 | System stores `model` and `description` for each registered agent | Migration: add `model TEXT`, `description TEXT`; update `insert_agent` signature |
| AGNT-02 | System tracks `current_task` FK linking agent to active message | Migration: add `current_task TEXT REFERENCES messages(id)`; update `send` command to write it |
| AGNT-03 | Agent records use `tool` field instead of `provider` | Migration: rename column `provider` → `tool`; update all queries, structs, callers |
</phase_requirements>

---

## Summary

Phase 4 is a pure refactor phase — no new user-visible commands are added, only the config format and DB schema are brought into alignment with `docs/SOLUTION-DESIGN.md`. Three areas of change are tightly coupled: (1) `config.rs` YAML parsing, (2) DB migration files, and (3) the Rust structs and CRUD functions that bridge the two.

The changes are invasive but mechanically straightforward. SQLite's `ALTER TABLE ADD COLUMN` supports adding new nullable/defaulted columns non-destructively. The two new columns that require a rename (`provider` → `tool` in agents, `agent_name` → `to_agent` in messages) cannot use `ALTER TABLE RENAME COLUMN` safely in all SQLite versions; the canonical sqlx approach is to add the new column, backfill, and optionally keep the old column for a transition period — but because backward compatibility is explicitly out of scope per REQUIREMENTS.md, a clean table rebuild via `CREATE TABLE ... AS SELECT` or explicit column copy is acceptable.

All locked decisions from STATE.md are already resolved: `project` becomes a string, `command` is removed, `provider` is renamed to `tool`, and `pending` status becomes `processing`. The planner must treat these as facts, not choices.

**Primary recommendation:** Write migration `0003` that adds all new columns and renames `provider` → `tool` (via new-column-copy-drop or a single-transaction table rebuild). Update Rust structs and CRUD in the same plan wave. Update all callers (commands, tests) as the final wave.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sqlx | 0.8 (already in Cargo.toml) | SQLite pool, compile-time queries, migration runner | Already adopted; `sqlx::migrate!()` auto-applies files in `src/db/migrations/` |
| serde + serde-saphyr | 1.0 / 0.0.17 (already in Cargo.toml) | YAML deserialization of squad.yml | Already adopted; all we change is the shape of the deserialized structs |
| chrono | 0.4 (already in Cargo.toml) | RFC3339 timestamps for `completed_at` | Already used for all other timestamps |

### No new dependencies required for Phase 4.

---

## Architecture Patterns

### Recommended File Touch List

```
src/config.rs                    — CONF-01, CONF-02, CONF-03, CONF-04
src/db/agents.rs                 — AGNT-01, AGNT-02, AGNT-03
src/db/messages.rs               — MSGS-01, MSGS-02, MSGS-03, MSGS-04
src/db/migrations/0003_v11.sql   — All DB schema changes (single migration file)
src/commands/init.rs             — Calls insert_agent; update for new signature + fields
src/commands/register.rs         — Calls insert_agent; update for new signature + tool field
src/commands/send.rs             — Calls insert_message; update for from_agent/to_agent/type; sets current_task
src/commands/signal.rs           — Calls update_status; must set completed_at + clear current_task
src/commands/agents.rs           — Displays agents; update PROVIDER column header → TOOL
src/commands/list.rs             — Displays messages; update AGENT column → FROM/TO columns
tests/test_db.rs                 — Update all test call sites (new function signatures)
squad.yml                        — Sample config: update to new format
```

### Pattern 1: sqlx Migration File Naming

**What:** sqlx applies migration files in lexicographic order. Each file must have a unique integer prefix.
**When to use:** Every schema change gets its own numbered file.
**Example:**
```sql
-- src/db/migrations/0003_v11.sql
-- All Phase 4 schema changes in one file (atomic for this milestone)
```

`sqlx::migrate!("./src/db/migrations")` in `db/mod.rs` auto-discovers and runs all unapplied files on every `db::connect()` call. The migration table `_sqlx_migrations` tracks which files have been applied; re-running is a no-op for already-applied files.

### Pattern 2: SQLite Column Rename via Copy-and-Drop

**What:** SQLite does not support `ALTER TABLE RENAME COLUMN` in older versions (it was added in SQLite 3.25.0, 2018). The sqlx bundled SQLite version is recent enough to support it, but the project targets "existing v1.0 DBs" — since backward compatibility is explicitly out of scope, a direct rename is safe.

**Safe approach for renaming `provider` → `tool` in agents:**
```sql
-- Option A: RENAME COLUMN (SQLite >= 3.25, 2018 — safe for our use case)
ALTER TABLE agents RENAME COLUMN provider TO tool;

-- Option B (fallback if sqlx bundled SQLite version is older than 3.25):
ALTER TABLE agents ADD COLUMN tool TEXT NOT NULL DEFAULT '';
UPDATE agents SET tool = provider;
-- Cannot DROP COLUMN in older SQLite either; column stays but is ignored by new code
```

**Recommendation:** Use `RENAME COLUMN` — it is the cleanest approach. If CI fails, fall back to Option B.

**For `agent_name` → directional routing in messages:**
The design replaces single-direction `agent_name` with `from_agent` + `to_agent`. Existing rows have no `from_agent` value. Since backward compat is out of scope:
```sql
ALTER TABLE messages ADD COLUMN from_agent TEXT NOT NULL DEFAULT '';
ALTER TABLE messages ADD COLUMN to_agent   TEXT NOT NULL DEFAULT '';
UPDATE messages SET to_agent = agent_name WHERE to_agent = '';
-- Keep agent_name for transition; new code writes to from_agent/to_agent
-- After verifying, a follow-up migration can drop agent_name (optional)
```

### Pattern 3: Config `project` as Plain String

**What:** Change `SquadConfig.project` from `ProjectConfig { name, db_path }` to a plain `String`. The DB path is always derived as `~/.agentic-squad/<project>/station.db`.

**Before (current):**
```rust
pub struct SquadConfig {
    pub project: ProjectConfig,
    ...
}
pub struct ProjectConfig {
    pub name: String,
    pub db_path: Option<String>,
}
```

**After (target):**
```rust
pub struct SquadConfig {
    pub project: String,   // CONF-01: "project: myapp"
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}
```

All callers that use `config.project.name` must change to `config.project`. The `db_path` override feature is dropped (not in solution design).

### Pattern 4: `AgentConfig` Field Changes

**Before:**
```rust
pub struct AgentConfig {
    pub name: String,
    pub provider: String,
    pub role: String,
    pub command: String,
}
```

**After (target for CONF-02, CONF-03, CONF-04):**
```rust
pub struct AgentConfig {
    pub name: Option<String>,       // orchestrator has no name in design (name is auto-generated)
    pub tool: String,               // CONF-04: renamed from provider
    pub role: Option<String>,       // defaults to "worker" for agents
    pub model: Option<String>,      // CONF-02: new field
    pub description: Option<String>, // CONF-02: new field
    // command: REMOVED (CONF-03)
}
```

Note: The solution design shows orchestrator without a `name` field — the name is auto-generated as `<project>-<tool>-orchestrator`. For worker agents, `name` in squad.yml is the role suffix (e.g. `implement`), and the full name becomes `<project>-<tool>-implement`. Agent auto-prefixing is in Phase 5 (CLI-02), not Phase 4. For Phase 4, keep `name` as-is in the struct but make it `Option<String>`. Keep generating agent names from the config for now; the auto-prefix enforcement lands in Phase 5.

### Pattern 5: Updated `insert_agent` Signature

```rust
// Current
pub async fn insert_agent(
    pool: &SqlitePool,
    name: &str,
    provider: &str,
    role: &str,
    command: &str,
) -> anyhow::Result<()>

// Target (AGNT-01, AGNT-02, AGNT-03)
pub async fn insert_agent(
    pool: &SqlitePool,
    name: &str,
    tool: &str,         // renamed
    role: &str,
    model: Option<&str>,
    description: Option<&str>,
) -> anyhow::Result<()>
// command parameter removed
// current_task is set separately by send command
```

### Pattern 6: Updated `insert_message` Signature

```rust
// Current
pub async fn insert_message(
    pool: &SqlitePool,
    agent_name: &str,
    task: &str,
    priority: &str,
) -> anyhow::Result<String>

// Target (MSGS-01, MSGS-02, MSGS-03)
pub async fn insert_message(
    pool: &SqlitePool,
    from_agent: &str,   // MSGS-01
    to_agent: &str,     // MSGS-01
    msg_type: &str,     // MSGS-02 ("task_request")
    body: &str,         // renamed from task (matches solution design)
    priority: &str,
) -> anyhow::Result<String>
// status default changes from 'pending' → 'processing' (MSGS-03)
```

### Pattern 7: Updating `update_status` for `completed_at` (MSGS-04)

```rust
// Current UPDATE sets status='completed', updated_at=now
// Target: also set completed_at=now and clear agent's current_task

// In messages.rs update_status: add completed_at to SET clause
"UPDATE messages SET status = 'completed', updated_at = ?, completed_at = ? WHERE id = (...)"

// In signal.rs: after update_status, also clear agents.current_task
sqlx::query("UPDATE agents SET current_task = NULL WHERE name = ?")
    .bind(&agent)
    .execute(pool).await?;
```

### Pattern 8: Setting `current_task` on Send (AGNT-02)

```rust
// In send.rs, after insert_message returns msg_id:
sqlx::query("UPDATE agents SET current_task = ? WHERE name = ?")
    .bind(&msg_id)
    .bind(&agent)
    .execute(&pool).await?;
```

### Anti-Patterns to Avoid

- **Dropping `agent_name` column in migration:** SQLite DROP COLUMN requires SQLite >= 3.35 (2021). Safer to add new columns and keep `agent_name` as a legacy column — new code reads `to_agent`, old column stays nullable.
- **Changing `pending` → `processing` globally without updating tests:** 17 tests in `test_db.rs` hard-code `"pending"` as expected status. All must be updated to `"processing"`.
- **Making `model` and `description` NOT NULL in migration:** These fields are optional in the design and some agents may not have them. Use `TEXT DEFAULT NULL`.
- **Forgetting to update the `Agent` and `Message` Rust structs:** `sqlx::FromRow` derives will fail at compile time if the struct fields don't match the schema — this is actually a safety feature, not a pitfall.
- **Splitting config refactor and DB rename into separate commits without coordinating callers:** The `provider` → `tool` rename must land atomically — migration + struct + CRUD + commands + tests in one cohesive plan.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Migration versioning | Custom version table | `sqlx::migrate!()` with numbered files | Already set up; just add `0003_v11.sql` |
| YAML optional fields | Custom `Deserialize` impl | `Option<T>` fields with `#[serde(default)]` | serde handles missing keys as `None` |
| UUID generation | Custom ID scheme | `uuid::Uuid::new_v4().to_string()` | Already used; keep consistent |
| Timestamp generation | Custom formatter | `chrono::Utc::now().to_rfc3339()` | Already used for all timestamps |

---

## Common Pitfalls

### Pitfall 1: `status = 'pending'` Hardcoded in Many Places
**What goes wrong:** `test_db.rs` has 17+ assertions checking `status == "pending"`. After changing the default to `"processing"`, all tests fail.
**Why it happens:** The status string is duplicated across insert query, test assertions, `list.rs` filter display, `colorize_status`, `peek_message` query, `update_status` subquery, and `signal.rs`.
**How to avoid:** Search globally for `"pending"` before writing the migration. Update every occurrence as part of the same plan.
**Warning signs:** Test compile succeeds but test run fails with assertion mismatches on status field.

### Pitfall 2: `sqlx::FromRow` Compile Failures After Schema Change
**What goes wrong:** After adding columns to the DB, the `Agent` and `Message` structs must include those columns (or use `#[sqlx(rename)]` / `#[allow(dead_code)]`). `SELECT *` queries will return more columns than the struct expects, causing a runtime `ColumnNotFound` error or a compile error with offline mode.
**Why it happens:** sqlx maps `SELECT *` results positionally and by column name.
**How to avoid:** Add new fields to the Rust structs (`model`, `description`, `current_task`, `from_agent`, `to_agent`, `msg_type`, `body`, `completed_at`) at the same time as the migration. Fields can be `Option<String>` for nullable DB columns.
**Warning signs:** `cargo test` compiles but panics at `sqlx::query_as` with a column mismatch message.

### Pitfall 3: `INSERT OR IGNORE` Preserves Old Schema on Duplicate
**What goes wrong:** The idempotent `INSERT OR IGNORE` in `insert_agent` means if an agent was registered with the old schema (no `model`, `description`), re-running `init` after migration does NOT update those fields.
**Why it happens:** `INSERT OR IGNORE` only inserts; it never updates an existing row.
**How to avoid:** This is acceptable for Phase 4 — the migration adds the columns with `DEFAULT NULL`, so existing rows simply have `NULL` for the new fields. Document that users must `register` agents again (or manually `UPDATE`) to populate model/description. This is not a blocker.

### Pitfall 4: Foreign Key Constraint on `current_task`
**What goes wrong:** Adding `current_task TEXT REFERENCES messages(id)` as a nullable FK. SQLite FK enforcement is OFF by default and must be enabled per-connection with `PRAGMA foreign_keys = ON`.
**Why it happens:** SQLite does not enforce FK constraints unless explicitly enabled.
**How to avoid:** For Phase 4, define the FK constraint in the migration (for documentation value) but do not enable FK enforcement in `db/mod.rs` unless required — this matches the existing approach (no FK enforcement currently). The constraint is decorative but communicates intent.

### Pitfall 5: `project: myapp` vs `project: {name: myapp}` Deserialization
**What goes wrong:** serde-saphyr will fail to parse the old `project: {name: ...}` format if the struct changes to `pub project: String`.
**Why it happens:** YAML deserialization is strict about the shape — a mapping where a string is expected is a parse error.
**How to avoid:** The solution design and locked decisions say the new format IS `project: myapp` (string). Old squad.yml files must be updated. The sample `squad.yml` in the repo must be updated as part of this phase. No backward compat needed (per REQUIREMENTS.md Out of Scope table).

### Pitfall 6: `from_agent` Default Value for Existing Messages
**What goes wrong:** After migration adds `from_agent TEXT NOT NULL DEFAULT ''`, existing messages get an empty string for `from_agent`. Code that tries to look up an agent by `from_agent` will find nothing.
**Why it happens:** Migration cannot know who sent historical messages.
**How to avoid:** Make `from_agent` nullable (`TEXT DEFAULT NULL`) rather than `NOT NULL DEFAULT ''`. New messages always populate it. Old messages have `NULL`, which is fine for display purposes.

---

## Code Examples

### Migration 0003 — Full schema change
```sql
-- src/db/migrations/0003_v11.sql

-- agents: add model, description, current_task; rename provider → tool
ALTER TABLE agents RENAME COLUMN provider TO tool;
ALTER TABLE agents ADD COLUMN model       TEXT DEFAULT NULL;
ALTER TABLE agents ADD COLUMN description TEXT DEFAULT NULL;
ALTER TABLE agents ADD COLUMN current_task TEXT DEFAULT NULL
    REFERENCES messages(id);

-- messages: add directional fields, type, processing status, completed_at
-- Keep agent_name as legacy nullable for transition
ALTER TABLE messages ADD COLUMN from_agent  TEXT DEFAULT NULL;
ALTER TABLE messages ADD COLUMN to_agent    TEXT DEFAULT NULL;
ALTER TABLE messages ADD COLUMN type        TEXT NOT NULL DEFAULT 'task_request';
ALTER TABLE messages ADD COLUMN completed_at TEXT DEFAULT NULL;

-- Backfill to_agent from legacy agent_name column
UPDATE messages SET to_agent = agent_name WHERE to_agent IS NULL;

-- Note: changing default status 'pending' → 'processing' cannot be done
-- with ALTER TABLE. New inserts must explicitly pass 'processing'.
-- The INSERT statement in messages.rs must change its hardcoded 'pending'
-- to 'processing'.

-- Update indexes
CREATE INDEX IF NOT EXISTS idx_messages_direction ON messages(from_agent, to_agent);
```

### Config struct after changes (config.rs)
```rust
#[derive(Deserialize, Debug)]
pub struct SquadConfig {
    pub project: String,                 // CONF-01: "project: myapp"
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    pub name: Option<String>,            // optional; auto-generated name in Phase 5
    pub tool: String,                    // CONF-04: renamed from provider
    #[serde(default = "default_role")]
    pub role: String,
    pub model: Option<String>,           // CONF-02
    pub description: Option<String>,     // CONF-02
    // command: REMOVED (CONF-03)
}

fn default_role() -> String { "worker".to_string() }
```

### resolve_db_path after CONF-01 (config.rs)
```rust
pub fn resolve_db_path(config: &SquadConfig) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot determine home directory"))?;
    let db_path = home
        .join(".agentic-squad")
        .join(&config.project)          // config.project is now a String directly
        .join("station.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(db_path)
}
```

### init.rs caller changes (CONF-03, CONF-04)
```rust
// Before:
db::agents::insert_agent(&pool, &config.orchestrator.name,
    &config.orchestrator.provider, "orchestrator",
    &config.orchestrator.command).await?;

// After:
db::agents::insert_agent(&pool, &orch_name,
    &config.orchestrator.tool, "orchestrator",
    config.orchestrator.model.as_deref(),
    config.orchestrator.description.as_deref()).await?;
// where orch_name = format!("{}-{}-orchestrator", config.project, config.orchestrator.tool)
// Note: auto-prefix is Phase 5 (CLI-02); for Phase 4 use config name as-is if present
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `project: {name: ..., db_path: ...}` | `project: myapp` (string) | Phase 4 | All callers of `config.project.name` → `config.project` |
| `provider: claude-code` | `tool: claude-code` | Phase 4 | Rename in struct, migration, all callers |
| `command: "claude"` required | `command` removed | Phase 4 | `init` and `register` callers drop the argument |
| `status: pending` (messages) | `status: processing` | Phase 4 | All INSERT SQL, tests, display code |
| Single `agent_name` column | `from_agent` + `to_agent` | Phase 4 | `insert_message` signature changes |
| No `completed_at` | `completed_at` set on signal | Phase 4 | `update_status` SQL adds SET clause |
| No `model`/`description` on agents | Added as nullable columns | Phase 4 | `insert_agent` gains 2 optional params |
| No `current_task` FK | Added as nullable FK column | Phase 4 | `send` sets it; `signal` clears it |

**Deprecated/outdated after Phase 4:**
- `ProjectConfig` struct: removed entirely, replaced by `pub project: String`
- `AgentConfig.command` field: removed
- `AgentConfig.provider` field: renamed to `tool`
- `agents.provider` DB column: renamed to `tool`
- `messages.agent_name` DB column: kept as legacy nullable, superseded by `to_agent`
- Message status value `"pending"`: replaced by `"processing"`

---

## Open Questions

1. **`agent_name` column drop timing**
   - What we know: `ALTER TABLE DROP COLUMN` requires SQLite 3.35+ (2021). The project uses sqlx's bundled SQLite, which is likely >= 3.35 but unverified.
   - What's unclear: Whether to drop `agent_name` in migration 0003 or leave it as dead weight.
   - Recommendation: Leave `agent_name` in place for Phase 4 (add `to_agent`, backfill, new code uses `to_agent`). If needed, drop in a future migration once confirmed SQLite version supports it.

2. **Orchestrator `name` field in new squad.yml**
   - What we know: Solution design shows orchestrator without `name` field; name is auto-generated as `<project>-<tool>-orchestrator`.
   - What's unclear: Phase 4 CONF-01 says init must succeed with the new format. But Phase 5 CLI-02 handles auto-prefix enforcement. Should Phase 4 handle the orchestrator name derivation or keep requiring it in config?
   - Recommendation: For Phase 4, make `AgentConfig.name: Option<String>`. In `init.rs`, if name is `None`, derive as `<project>-<tool>-<role>`. This makes the new squad.yml format work end-to-end without waiting for Phase 5.

3. **`register` command `--provider` → `--tool` flag rename**
   - What we know: `cli.rs` has `Register { ..., provider: String }` with `--provider` flag.
   - What's unclear: Whether changing `--provider` to `--tool` in CLI is Phase 4 scope (it's a CLI-visible change).
   - Recommendation: Rename `--provider` → `--tool` in Phase 4 since it directly maps to AGNT-03 and CONF-04. All DB and config changes must stay in sync.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in + tokio-test (via `#[tokio::test]`) |
| Config file | `Cargo.toml` `[dev-dependencies]` section |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CONF-01 | `project: myapp` string parses correctly | unit | `cargo test test_config` | ❌ Wave 0 |
| CONF-02 | `model` and `description` fields parse from YAML | unit | `cargo test test_config` | ❌ Wave 0 |
| CONF-03 | Config without `command` field parses without error | unit | `cargo test test_config` | ❌ Wave 0 |
| CONF-04 | `tool` field parses from YAML | unit | `cargo test test_config` | ❌ Wave 0 |
| MSGS-01 | `insert_message` stores `from_agent` and `to_agent` correctly | unit | `cargo test test_db` | ✅ (needs update) |
| MSGS-02 | `insert_message` stores `type` field | unit | `cargo test test_db` | ✅ (needs update) |
| MSGS-03 | New messages have status `processing` not `pending` | unit | `cargo test test_db` | ✅ (needs update) |
| MSGS-04 | `update_status` sets `completed_at` | unit | `cargo test test_db` | ✅ (needs update) |
| AGNT-01 | `insert_agent` stores `model` and `description` | unit | `cargo test test_db` | ✅ (needs update) |
| AGNT-02 | `send` command sets `current_task` FK | unit | `cargo test test_db` | ❌ Wave 0 |
| AGNT-03 | `insert_agent` uses `tool` field, not `provider` | unit | `cargo test test_db` | ✅ (needs update) |

### Sampling Rate
- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `tests/test_config.rs` — covers CONF-01, CONF-02, CONF-03, CONF-04 (config parsing unit tests)
- [ ] Add `test_send_sets_current_task` to `tests/test_db.rs` — covers AGNT-02
- [ ] Update all 17+ existing tests in `tests/test_db.rs` that assert `status == "pending"` to assert `status == "processing"` — covers MSGS-03
- [ ] Update `insert_agent` call sites in `tests/test_db.rs` to match new signature (drop `command` arg, add `model`/`description`, rename `provider` → `tool`)
- [ ] Update `insert_message` call sites in `tests/test_db.rs` to match new signature (`from_agent`, `to_agent`, `type`, `body`)

---

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection: `src/config.rs`, `src/db/agents.rs`, `src/db/messages.rs`, `src/db/mod.rs`, `src/db/migrations/0001_initial.sql`, `src/db/migrations/0002_agent_status.sql`
- `docs/SOLUTION-DESIGN.md` — authoritative target schema and config format
- `docs/GAP-ANALYSIS.md` — precise diff between current code and design
- `src/cli.rs`, all `src/commands/*.rs` — call site inventory
- `tests/test_db.rs`, `tests/helpers.rs` — existing test infrastructure
- `.planning/REQUIREMENTS.md` — locked requirement definitions
- `.planning/STATE.md` — locked design decisions for this phase

### Secondary (MEDIUM confidence)
- sqlx SQLite migration behavior: documented in sqlx README and confirmed by existing `0001`/`0002` migration files working correctly in the project
- SQLite `RENAME COLUMN` availability (3.25+, 2018): well-established SQLite changelog knowledge; current sqlx 0.8 bundles a recent SQLite

### Tertiary (LOW confidence)
- None — all findings are grounded in direct codebase and official project docs.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all tools already in use
- Architecture: HIGH — changes are fully specified by gap analysis and solution design; all touch points identified by code inspection
- Pitfalls: HIGH — identified from direct reading of code (hardcoded strings, struct field names, test assertions)

**Research date:** 2026-03-08
**Valid until:** Stable until code changes; no external library updates needed
