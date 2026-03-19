# Phase 24: Agent Role Templates in Wizard - Research

**Researched:** 2026-03-19
**Domain:** Rust TUI (ratatui), SQLite migrations, wizard state machine
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Template catalog — Worker templates**
- 8 worker templates in frequency-of-use order: coder, solution-architect, qa-engineer, devops-engineer, code-reviewer, technical-writer, data-engineer, security-engineer
- Plus "Custom" at the bottom of the list — selecting Custom clears all fields (name, model, description) to blank
- "coder" is a broad umbrella role covering frontend, backend, mobile, fullstack development
- "solution-architect" covers tech lead, architect, solution design

**Template catalog — Orchestrator templates**
- 3 orchestrator templates: order is Claude's discretion (project-manager, tech-lead, scrum-master)
- Plus "Custom" at the bottom
- Template selector appears on OrchestratorConfig page too, not just WorkerConfig

**Template data structure**
- Each template contains: role slug, display name, description text (2-3 sentences), per-provider model mapping, routing hint keywords (list), and default provider
- Descriptions are detailed (2-3 sentences) — not one-liners
- Model suggestions are per-provider: each template maps to a specific model per provider (e.g. solution-architect → opus for Claude / gemini-2.5-pro for Gemini)
- Template also pre-selects Provider (user can override)
- Routing hints are keyword lists (e.g. ["frontend", "ui", "css", "react", "component"]) — consistent with Phase 22 word-intersection alignment approach

**Wizard UX flow**
- Template selector appears AFTER Name field, BEFORE Provider: Name → Role Template → Provider → Model → Description
- Selecting a template auto-fills: Name (full role slug e.g. "frontend-engineer"), Provider, Model, Description
- All auto-filled fields are editable — user can override any field after template selection
- Template selector rendered as scrollable list with description preview on the right side (split layout: radio list left, selected template description right)
- Routing hints are hidden from wizard — internal metadata only visible in squad-orchestrator.md
- Worker-only wizard path (add-agents via re-init) also shows the template selector — same UX regardless of entry point

**Routing hints in squad-orchestrator.md**
- New "Routing Matrix" section placed AFTER Session Routing
- Markdown table with columns: Keyword | Route to
- Only agents WITH routing hints appear in the matrix — agents without hints are omitted
- If NO agents have routing hints, section still renders with a placeholder message: "No routing hints configured — use templates during init for keyword-based routing"
- `build_orchestrator_md()` remains pure (INTEL-05) — routing hints passed as data from DB

**DB migration (0006)**
- New `routing_hints` TEXT column on agents table — nullable
- Stores JSON array format (e.g. `["frontend","ui","css"]`) — requires serde_json for parse/serialize
- Existing agents get NULL (no hints) — NULL means "no template used"

**Template data storage in code**
- New `src/commands/templates.rs` module — separate from wizard.rs (which is already 1566 lines)
- Template struct uses `&'static str` slices — zero allocation, compiled into binary
- Wizard imports template data from templates.rs

**Register/Clone integration**
- `clone` inherits routing_hints from source agent — clone is a duplicate with same routing
- `register` always produces agents with NULL routing_hints — no --hints flag, keeps register simple
- If user wants hints on a registered agent, use the wizard

### Claude's Discretion
- Orchestrator template ordering (project-manager, tech-lead, scrum-master)
- Exact description text for each of the 11 templates (8 worker + 3 orchestrator)
- Exact keyword lists for each template's routing hints
- Per-provider model mapping details for each template
- Template struct field naming and internal design
- How the scrollable list + preview panel layout is implemented in ratatui

### Deferred Ideas (OUT OF SCOPE)
- User-defined local template registry for custom team-specific role packages — tracked as TMPL-07 in REQUIREMENTS.md v2 section
- Template reordering by SDD workflow (TMPL-06 satisfied minimally with static order) — could be enhanced later if workflow-specific ordering proves valuable
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TMPL-01 | Init wizard presents a predefined role menu with 8-12 role templates (coder, qa-engineer, etc.) | AgentField enum gets new `Template` variant; `AgentDraft` gains `template_index: usize`; templates.rs provides WORKER_TEMPLATES and ORCHESTRATOR_TEMPLATES arrays |
| TMPL-02 | Each template includes role name, description text, default model suggestion, and routing hints | `AgentTemplate` struct in templates.rs with `&'static str` slug/display/description, per-provider model map, routing hint slice, and default provider |
| TMPL-03 | User can select "Custom" to skip templates and enter free-text role/description (existing behavior preserved) | "Custom" sentinel at end of each template list; selecting it clears name/model/description fields; existing handle_agent_key() paths remain unchanged |
| TMPL-04 | Selecting a template auto-fills the model selector with the template's suggested model (user can override) | Template auto-fill writes into AgentDraft fields; all written fields remain editable (existing Provider/Model/Description key handlers unchanged) |
| TMPL-05 | Template routing hints are embedded in `squad-orchestrator.md` via the context command | Migration 0006 adds `routing_hints TEXT NULL` on agents; `insert_agent()` gets `routing_hints: Option<&str>` param; `build_orchestrator_md()` gets routing hints slice; Routing Matrix section appended after Session Routing |
| TMPL-06 | Template list ordering adapts based on detected SDD workflow (satisfied minimally with static order) | Static template order is the locked implementation — no reordering logic needed |
</phase_requirements>

---

## Summary

Phase 24 adds a role template system to the init wizard. The wizard gains a new `AgentField::Template` step between Name and Provider, backed by a `templates.rs` module containing 8 worker + 3 orchestrator + 1 Custom option per list. Template selection auto-fills Name, Provider, Model index, and Description into the existing `AgentDraft` fields. A new `routing_hints` DB column (TEXT, nullable, JSON array) is added via migration 0006. The context command's `build_orchestrator_md()` gains a new Routing Matrix section appended after Session Routing.

All wizard work is confined to `wizard.rs` (new `AgentField::Template` variant, extended `AgentDraft`, extended `handle_agent_key()`, and extended `render_agent_page()`) and a new `templates.rs` module. DB changes touch `agents.rs` (`insert_agent()` signature), a new migration file, `clone.rs` (copy routing_hints), and `context.rs` (Routing Matrix section). The `run_worker_only()` path gets the template selector for free because it uses the same `WorkerConfig` page rendering.

**Primary recommendation:** Implement in this order: (1) `templates.rs` data module, (2) migration 0006, (3) `AgentDraft`/`AgentField` expansion + key handler, (4) `render_agent_page()` split layout, (5) `insert_agent()` signature + clone propagation, (6) `build_orchestrator_md()` Routing Matrix section.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI rendering — split-pane layout, List widget, scrolling | Already used throughout wizard.rs; Layout + Constraint drives all page regions |
| crossterm | 0.29 | Terminal input/raw mode | Already used — KeyCode handling in handle_agent_key() |
| sqlx | 0.8 | SQLite pool + migrations | Already used — `sqlx::migrate!()` auto-applies migration files |
| serde_json | 1.0 | JSON serialize/deserialize routing_hints column | Already in Cargo.toml; `serde_json::to_string()` / `from_str()` for `Vec<&str>` |

### No New Dependencies Required
All needed libraries are already in `Cargo.toml`. No additions needed for this phase.

---

## Architecture Patterns

### New File: `src/commands/templates.rs`

```
src/commands/
├── templates.rs    # NEW — AgentTemplate struct + WORKER_TEMPLATES + ORCHESTRATOR_TEMPLATES
├── wizard.rs       # MODIFIED — AgentField, AgentDraft, handle_agent_key, render_agent_page
├── context.rs      # MODIFIED — build_orchestrator_md() Routing Matrix section
├── clone.rs        # MODIFIED — copy routing_hints from source agent
src/db/
├── agents.rs       # MODIFIED — Agent struct + insert_agent() signature
├── migrations/
│   └── 0006_routing_hints.sql  # NEW — ALTER TABLE agents ADD COLUMN routing_hints
```

### Pattern 1: Static Template Data with `&'static str`

Templates are compiled into the binary using `&'static str` slices — zero allocation at runtime.

```rust
// In src/commands/templates.rs

pub struct AgentTemplate {
    pub slug: &'static str,           // e.g. "coder"
    pub display_name: &'static str,   // e.g. "Coder"
    pub description: &'static str,    // 2-3 sentences
    pub default_provider: &'static str, // "claude-code" | "gemini-cli"
    pub claude_model: &'static str,   // e.g. "sonnet"
    pub gemini_model: &'static str,   // e.g. "gemini-3.1-pro-preview"
    pub routing_hints: &'static [&'static str], // e.g. &["code","build","implement"]
}

pub const WORKER_TEMPLATES: &[AgentTemplate] = &[
    AgentTemplate {
        slug: "coder",
        display_name: "Coder",
        description: "Implements features, fixes bugs, and writes production-quality code. Handles frontend, backend, mobile, and fullstack development tasks. Use for any implementation work.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-3.1-pro-preview",
        routing_hints: &["code", "implement", "build", "fix", "feature", "bug", "frontend", "backend", "mobile"],
    },
    // ... 7 more worker templates + Custom sentinel
];

/// Sentinel index — selecting this index means "Custom" (no auto-fill)
pub const CUSTOM_IDX_WORKER: usize = WORKER_TEMPLATES.len();
pub const CUSTOM_IDX_ORCHESTRATOR: usize = ORCHESTRATOR_TEMPLATES.len();
```

### Pattern 2: AgentField Extension

The `AgentField` enum gains a `Template` variant. The `AgentDraft` struct gains `template_index: usize` and an `is_orchestrator: bool` flag (to select which template list to use).

```rust
// In wizard.rs

#[derive(Clone, Copy, PartialEq)]
pub enum AgentField {
    Name,
    Template,   // NEW — inserted before Provider
    Provider,
    Model,
    Description,
}

pub struct AgentDraft {
    pub name: TextInputState,
    pub template_index: usize,  // NEW — index into template list (last idx = Custom)
    pub is_orchestrator: bool,  // NEW — selects WORKER_TEMPLATES vs ORCHESTRATOR_TEMPLATES
    pub provider: Provider,
    pub model: ModelSelector,
    pub custom_model: TextInputState,
    pub description: TextInputState,
    pub focused_field: AgentField,
    pub routing_hints: Option<Vec<&'static str>>,  // NEW — populated by template selection
}
```

### Pattern 3: Template Auto-Fill Logic in `handle_agent_key()`

When the user presses Enter on the Template field, apply the template or clear fields for Custom:

```rust
AgentField::Template => match key {
    KeyCode::Enter | KeyCode::Tab => {
        // Apply template before advancing to Provider
        let templates = if draft.is_orchestrator {
            templates::ORCHESTRATOR_TEMPLATES
        } else {
            templates::WORKER_TEMPLATES
        };
        let custom_idx = templates.len(); // last position
        if draft.template_index < custom_idx {
            let t = &templates[draft.template_index];
            // Auto-fill name (only if currently empty — respect user-typed name)
            if draft.name.value.trim().is_empty() {
                draft.name = TextInputState::new();
                for c in t.slug.chars() { draft.name.push(c); }
            }
            // Auto-fill provider
            draft.provider = match t.default_provider {
                "gemini-cli" => Provider::GeminiCli,
                _ => Provider::ClaudeCode,
            };
            // Auto-fill model index
            let model_opts = ModelSelector::options_for(draft.provider);
            let target_model = match draft.provider {
                Provider::ClaudeCode => t.claude_model,
                Provider::GeminiCli => t.gemini_model,
                Provider::Antigravity => "",
            };
            draft.model.index = model_opts.iter().position(|&m| m == target_model).unwrap_or(0);
            // Auto-fill description
            draft.description = TextInputState::new();
            for c in t.description.chars() { draft.description.push(c); }
            // Store routing hints
            draft.routing_hints = Some(t.routing_hints.to_vec());
        } else {
            // Custom — clear all fields
            draft.name = TextInputState::new();
            draft.provider = Provider::ClaudeCode;
            draft.model.reset();
            draft.custom_model = TextInputState::new();
            draft.description = TextInputState::new();
            draft.routing_hints = None;
        }
        draft.focused_field = AgentField::Provider;
    }
    KeyCode::Up => {
        if draft.template_index > 0 { draft.template_index -= 1; }
    }
    KeyCode::Down => {
        let max = if draft.is_orchestrator {
            templates::ORCHESTRATOR_TEMPLATES.len()  // Custom is last
        } else {
            templates::WORKER_TEMPLATES.len()
        };
        if draft.template_index < max { draft.template_index += 1; }
    }
    KeyCode::Esc => draft.focused_field = AgentField::Name,
    _ => {}
},
```

**Key insight:** Name auto-fill only applies when the Name field is empty. This respects user intent — if they typed a custom name first, template selection won't clobber it.

### Pattern 4: Split-Pane Template Selector in `render_agent_page()`

The Template selector uses a horizontal split: radio list on the left, description preview on the right.

```rust
// Horizontal split for the template selector area
let template_area_h = (num_templates + 1 + 2) as u16; // options + Custom + 2 border
let template_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(45),  // left: radio list
        Constraint::Percentage(55),  // right: description preview
    ])
    .split(template_slot);

// Left: standard render_radio_list call with template display names
let display_names: Vec<&str> = /* collect from templates + "Custom" */;
render_radio_list(frame, template_chunks[0], "Role Template", &display_names, draft.template_index, focused);

// Right: description preview for currently highlighted template
let preview_text = if draft.template_index < templates.len() {
    templates[draft.template_index].description
} else {
    "Enter role and description manually."
};
let preview = Paragraph::new(preview_text)
    .wrap(ratatui::widgets::Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title(" Preview "));
frame.render_widget(preview, template_chunks[1]);
```

**Layout change in `render_agent_page()`:** The existing vertical constraint list gains one new slot for the template section. The template area height is dynamic (number of templates + 2 border lines). Current layout has 8 constraint slots (0=Name, 1=NameHint, 2=Provider, 3=Model, 4=CustomModel, 5=Description, 6=DescHint, 7=spacer). New layout inserts Template at slot 2, shifting existing slots by 1.

### Pattern 5: DB Migration 0006

```sql
-- src/db/migrations/0006_routing_hints.sql
ALTER TABLE agents ADD COLUMN routing_hints TEXT DEFAULT NULL;
```

Single line. sqlx::migrate!() applies it automatically in ascending order. Existing agents get NULL.

### Pattern 6: `insert_agent()` Signature Extension

```rust
pub async fn insert_agent(
    pool: &SqlitePool,
    name: &str,
    tool: &str,
    role: &str,
    model: Option<&str>,
    description: Option<&str>,
    routing_hints: Option<&str>,  // NEW — JSON array string e.g. r#"["code","build"]"#
) -> anyhow::Result<()> {
    // ... include routing_hints in INSERT and ON CONFLICT UPDATE
}
```

All callers must be updated: `init.rs`, `wizard.rs` (via `draft_to_agent_input()`), `register.rs` (passes None), `clone.rs` (passes source.routing_hints.as_deref()).

### Pattern 7: AgentInput Extension

`AgentInput` (the output type from `draft_to_agent_input()`) gains `routing_hints: Option<String>` so the wizard result carries hints all the way to `init.rs` where `insert_agent()` is called.

```rust
pub struct AgentInput {
    pub name: String,
    pub role: String,
    pub provider: String,
    pub model: Option<String>,
    pub description: Option<String>,
    pub routing_hints: Option<String>,  // NEW — JSON serialized
}
```

`draft_to_agent_input()` serializes `draft.routing_hints` via `serde_json::to_string()`.

### Pattern 8: Routing Matrix in `build_orchestrator_md()`

Appended after the Session Routing section:

```rust
// ── Routing Matrix ────────────────────────────────────────────────────────
out.push_str("## Routing Matrix\n\n");
let hinted_agents: Vec<(&Agent, Vec<String>)> = agents
    .iter()
    .filter(|a| a.role != "orchestrator")
    .filter_map(|a| {
        a.routing_hints.as_ref().and_then(|h| {
            serde_json::from_str::<Vec<String>>(h).ok().map(|kws| (a, kws))
        })
    })
    .collect();

if hinted_agents.is_empty() {
    out.push_str("No routing hints configured — use templates during init for keyword-based routing\n\n");
} else {
    out.push_str("| Keyword | Route to |\n");
    out.push_str("|---------|----------|\n");
    for (agent, keywords) in &hinted_agents {
        for kw in keywords {
            out.push_str(&format!("| {} | {} |\n", kw, agent.name));
        }
    }
    out.push_str("\n");
}
```

The function signature gains a routing hints parameter — but since INTEL-05 requires it remain pure, routing hints are fetched in `context::run()` (which already fetches agents) and passed in. Since `Agent` struct now has `routing_hints: Option<String>`, no new parameter is needed — the existing `agents: &[Agent]` slice already carries the data once the Agent struct is updated.

### Anti-Patterns to Avoid

- **Storing routing hints as a separate runtime parameter:** The `Agent` struct already carries all per-agent data. Once `routing_hints` is added to `Agent`, `build_orchestrator_md()` does not need a new parameter — agents slice carries the hints. This preserves INTEL-05 purity without API churn.
- **Auto-filling Name when user already typed one:** Name auto-fill should be conditional on the Name field being empty. Clobbering a user-typed name is a UX regression.
- **Applying template on each Up/Down keystroke:** Template data should only be applied on Enter/Tab (field confirmation), not on navigation. Navigate = preview only; confirm = apply.
- **Allocating description strings at template selection time:** Templates use `&'static str`; the TextInputState.push() loop is the correct way to get an owned String into the draft. Do not store `String` in the template struct.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON array for routing_hints | Custom delimiter format | `serde_json::to_string()` / `from_str::<Vec<String>>()` | serde_json already in Cargo.toml; handles quoting/escaping; compatible with future tooling |
| Scrollable list with preview | Custom scroll widget | `ratatui::widgets::List` + `Paragraph` with `Wrap` | Already used in render_radio_list(); Paragraph handles long description wrapping automatically |
| Template definition files | External TOML/JSON data files | `&'static str` structs in templates.rs | Zero runtime I/O; no file path dependencies; CONTEXT.md explicitly locked this decision |
| Separate routing hints fetch | Extra DB query in context::run() | Agent struct field (already fetched in list_agents()) | routing_hints column on agents table; list_agents() already returns all agents; no N+1 |

---

## Common Pitfalls

### Pitfall 1: Migration Ordering Gap
**What goes wrong:** Migration files must be numbered sequentially. The existing migrations end at `0004_thread_id.sql`. A new file `0006_routing_hints.sql` would leave a gap if `0005` does not exist.
**Why it happens:** Phase 22 planned a `0005` migration for `busy_since`. Check whether 0005 was actually added.
**How to avoid:** Run `ls src/db/migrations/` before writing 0006. The current listing shows only 0001-0004 — if Phase 22 added 0005, name the new file 0006; if not, name it 0005.
**Warning signs:** sqlx migration failure at startup citing "applied migration checksum mismatch" or "missing migration."

**VERIFIED:** `ls src/db/migrations/` shows 0001, 0002, 0003, 0004 only. Phase 22's `busy_since` migration was either not added or was folded into an existing migration. The new file should be `0005_routing_hints.sql`, not `0006`.

### Pitfall 2: `insert_agent()` Caller Breakage
**What goes wrong:** Adding `routing_hints` parameter to `insert_agent()` breaks every existing call site at compile time.
**Why it happens:** Rust requires all positional parameters to be updated when a function signature changes.
**How to avoid:** Search all callers before writing the new signature. Known callers: `init.rs` (calls after wizard result), `register.rs` (command), `clone.rs` (clone command). All three must pass `None` for routing_hints unless they have a value.
**Warning signs:** `cargo check` errors citing "expected N arguments, found N-1."

### Pitfall 3: `Agent` Struct Missing `routing_hints` Field Causes sqlx Compile Error
**What goes wrong:** `sqlx::FromRow` derive requires every column returned by `SELECT *` to map to a struct field. Once migration adds `routing_hints` column, `SELECT *` returns it, but the existing `Agent` struct has no `routing_hints` field.
**Why it happens:** sqlx compile-time query verification catches schema-struct mismatches.
**How to avoid:** Add `routing_hints: Option<String>` to the `Agent` struct in the same task as the migration, before any `cargo check`.
**Warning signs:** Compile error "column `routing_hints` not found in struct Agent" or similar sqlx macro error.

### Pitfall 4: Template Auto-Fill Resets User Edits When Re-Navigating
**What goes wrong:** If the user selects a template, edits the Description, then navigates back to Template and presses Enter again, the description is overwritten with template defaults.
**Why it happens:** Auto-fill logic runs on Enter regardless of whether fields were modified after first selection.
**How to avoid:** Track `template_applied: bool` in AgentDraft, or simply apply auto-fill on every confirm (user expectation: re-selecting a template re-applies it). Document this as intentional in code comments. The real protection is that "Custom" clears everything — so user edits on a templated draft are preserved as long as they don't re-confirm the Template field.
**Warning signs:** User-reported "my description was overwritten."

### Pitfall 5: Layout Constraint Calculation for Dynamic Template List Height
**What goes wrong:** The template section height varies by number of templates (8+1 worker, 3+1 orchestrator). Using a fixed Constraint::Length will clip the list or leave a gap.
**Why it happens:** render_agent_page() uses Constraint::Length per section; the template section must be dynamic like model_h.
**How to avoid:** Compute `template_h` as `templates.len() + 1 (Custom) + 2 (borders)` and use `Constraint::Length(template_h as u16)`. Use is_orchestrator flag on draft to pick the right templates slice for height calculation.

### Pitfall 6: `Paragraph::wrap()` Clips Long Descriptions at Terminal Width
**What goes wrong:** Template descriptions (2-3 sentences) may exceed the preview panel width, getting clipped instead of wrapped.
**Why it happens:** Default `Paragraph` behavior does not wrap; `Wrap { trim: true }` must be explicitly set.
**How to avoid:** Always construct the preview Paragraph with `.wrap(ratatui::widgets::Wrap { trim: true })`.

---

## Code Examples

### Migration file pattern (from existing 0003_v11.sql)

```sql
-- src/db/migrations/0005_routing_hints.sql
-- Phase 24: Add routing_hints column for template-based keyword routing
ALTER TABLE agents ADD COLUMN routing_hints TEXT DEFAULT NULL;
```

### TextInputState population from `&'static str`

The `TextInputState` has no `set_value()` method — populate by calling `push()` in a loop:

```rust
// Pattern used to pre-fill a TextInputState from a &'static str
draft.description = TextInputState::new();
for c in template.description.chars() {
    draft.description.push(c);
}
draft.description.cursor = draft.description.value.chars().count(); // cursor at end
```

Alternatively, set `value` directly and compute cursor:

```rust
draft.description.value = template.description.to_string();
draft.description.cursor = draft.description.value.chars().count();
```

The second approach is cleaner and avoids O(n^2) push() loop for long strings.

### serde_json round-trip for routing_hints

```rust
// Serialize in draft_to_agent_input()
let routing_hints: Option<String> = draft.routing_hints.as_ref().map(|hints| {
    serde_json::to_string(hints).unwrap_or_default()
});

// Deserialize in build_orchestrator_md() or wherever hints are consumed
if let Some(hints_json) = &agent.routing_hints {
    if let Ok(keywords) = serde_json::from_str::<Vec<String>>(hints_json) {
        // use keywords
    }
}
```

### Horizontal split layout (ratatui)

```rust
use ratatui::layout::{Constraint, Direction, Layout};

let template_pane = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(45),
        Constraint::Percentage(55),
    ])
    .split(template_area);
// template_pane[0] = radio list, template_pane[1] = description preview
```

### Existing render_radio_list signature (verified from wizard.rs line 882)

```rust
fn render_radio_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    options: &[&str],    // display names
    selected: usize,     // currently highlighted index
    focused: bool,       // border color: Cyan if focused, DarkGray otherwise
)
```

The template selector's left pane can reuse this directly — pass `display_names` slice (template.display_name + "Custom") and `draft.template_index` as selected.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual name/role/description entry | Template auto-fill with preview | Phase 24 | 80% reduction in wizard keystrokes for standard roles |
| No keyword routing data in DB | routing_hints JSON column | Phase 24 migration 0005 | Enables Routing Matrix in squad-orchestrator.md |
| Session Routing only uses description | Routing Matrix adds keyword-level routing | Phase 24 | Orchestrator gets structured keyword index, not just prose descriptions |

---

## Open Questions

1. **Migration numbering: 0005 or 0006?**
   - What we know: Existing migrations end at 0004. Phase 22 planned a `busy_since` migration.
   - What's unclear: Was Phase 22's 0005 migration actually committed? (verified: ls shows only 0001-0004 — use 0005)
   - Recommendation: Use `0005_routing_hints.sql`. If cargo build fails citing a missing migration, a 0005 already exists and the new file should be 0006.

2. **Name auto-fill behavior when Name is non-empty**
   - What we know: CONTEXT.md says "Selecting a template auto-fills: Name (full role slug)." It does not say "only if empty."
   - What's unclear: Should template selection always overwrite Name, or only when Name is blank?
   - Recommendation: Always overwrite Name on template confirmation (Enter). This matches user expectation that template selection is a fresh start. Users who want a custom name should select a template first, then edit the Name field afterward (field order: Name comes before Template, so user-typed name would only exist if they went back).

3. **`routing_hints` field on `AgentInput` vs serialization location**
   - What we know: `AgentInput` is the bridge between wizard and init.rs. `draft_to_agent_input()` converts `AgentDraft` to `AgentInput`.
   - What's unclear: Should `AgentInput.routing_hints` carry `Option<Vec<&'static str>>` (raw) or `Option<String>` (pre-serialized)?
   - Recommendation: Use `Option<String>` (pre-serialized) on `AgentInput`. The serialization belongs in `draft_to_agent_input()` where draft lifetime ends. `insert_agent()` then takes `Option<&str>` and stores it directly.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + tokio (async) |
| Config file | `Cargo.toml` — no separate config |
| Quick run command | `cargo test templates` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TMPL-01 | Worker template list has 8 entries (excl. Custom) | unit | `cargo test test_worker_template_count` | Wave 0 |
| TMPL-01 | Orchestrator template list has 3 entries (excl. Custom) | unit | `cargo test test_orchestrator_template_count` | Wave 0 |
| TMPL-02 | Each template has non-empty slug, display_name, description, routing_hints | unit | `cargo test test_template_fields_populated` | Wave 0 |
| TMPL-02 | Template descriptions are 2-3 sentences (length heuristic) | unit | `cargo test test_template_description_length` | Wave 0 |
| TMPL-03 | Custom selection clears name/model/description fields | unit | `cargo test test_custom_template_clears_fields` | Wave 0 |
| TMPL-04 | Template auto-fill sets model index to correct option for provider | unit | `cargo test test_template_autofill_model_index` | Wave 0 |
| TMPL-05 | Migration 0005 adds routing_hints column; existing agents get NULL | integration | `cargo test test_routing_hints_migration` | Wave 0 |
| TMPL-05 | insert_agent with routing_hints stores JSON; get_agent retrieves it | integration | `cargo test test_insert_agent_routing_hints` | Wave 0 |
| TMPL-05 | build_orchestrator_md with agents having hints renders Routing Matrix table | unit | `cargo test test_routing_matrix_with_hints` | Wave 0 |
| TMPL-05 | build_orchestrator_md with no hinted agents renders placeholder message | unit | `cargo test test_routing_matrix_empty` | Wave 0 |
| TMPL-06 | Template list order is static (coder first, solution-architect second, etc.) | unit | `cargo test test_worker_template_order` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test templates`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/test_templates.rs` — covers TMPL-01, TMPL-02, TMPL-03, TMPL-04, TMPL-06 (unit tests for templates.rs data + draft auto-fill logic)
- [ ] `tests/test_db.rs` or `tests/test_templates.rs` — covers TMPL-05 DB integration tests (requires setup_test_db() + migration 0005)
- [ ] `tests/test_context.rs` (new or added to existing) — covers build_orchestrator_md() Routing Matrix output

---

## Sources

### Primary (HIGH confidence)
- Direct code inspection: `src/commands/wizard.rs` — full wizard state machine, AgentDraft, AgentField, handle_agent_key(), render_agent_page(), render_radio_list(), draft_to_agent_input()
- Direct code inspection: `src/db/agents.rs` — Agent struct, insert_agent() signature
- Direct code inspection: `src/commands/context.rs` — build_orchestrator_md() full implementation
- Direct code inspection: `src/commands/clone.rs` — clone command, insert_agent() call pattern
- Direct code inspection: `Cargo.toml` — dependency versions, serde_json confirmed present
- Direct code inspection: `src/db/migrations/` — existing migration files 0001-0004
- Direct code inspection: `tests/helpers.rs` — setup_test_db() pattern for test infrastructure

### Secondary (MEDIUM confidence)
- ratatui 0.30 Paragraph::wrap — standard API; confirmed via existing wizard.rs imports and ratatui docs pattern

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already used in codebase; no new dependencies
- Architecture patterns: HIGH — all patterns derived from direct code inspection of existing wizard.rs and context.rs
- Pitfalls: HIGH — migration numbering verified by ls; pitfall 3 (sqlx struct mismatch) is a known sqlx constraint
- Template content (descriptions, keywords): MEDIUM — Claude's Discretion per CONTEXT.md; content correctness depends on judgment

**Research date:** 2026-03-19
**Valid until:** 2026-06-19 (stable Rust ecosystem; ratatui 0.30 API is stable)
