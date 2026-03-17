# Phase 19: Agent Diagram - Research

**Researched:** 2026-03-17
**Domain:** Rust terminal output — ASCII box drawing, owo-colors, post-init printing
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Layout & orientation**
- Vertical stack: orchestrator box centered at top, worker boxes in a horizontal row below
- Workers laid out in a fixed-width row (~80 cols); when workers exceed the row width, wrap to a new row below
- Arrows run from the orchestrator box down to each worker box (▼ per worker column)

**Box content**
- Agent name is also the tmux session name — show it once (no duplication)
- Orchestrator box: first line is bold/uppercase `ORCHESTRATOR` label, then name, then `tool: <tool>  model: <model>`, then `[status]`
- Worker boxes: name on first line, then `tool: <tool>  model: <model>`, then `[status]`
- Fields per box: name, role (implied by position/label), tool, model (if set), status

**Color & visual style**
- Unicode box-drawing characters: `┌─┐`, `│`, `└─┘`, `▼` arrows
- Box borders are neutral (no color)
- Only the `[status]` badge is colored via owo_colors: green=idle, yellow=busy, red=dead
- Consistent with how `colorize_agent_status` works in helpers.rs

**Placement in init output**
- Diagram printed as the final section after the "Get Started:" block
- Section header: `Agent Fleet:` (newline before)
- Suppressed when `--json` flag is active — same guard as hook instructions in init.rs
- Uses `if_supports_color(Stream::Stdout, ...)` for all color output

### Claude's Discretion

None specified — all visual and integration decisions are locked.

### Deferred Ideas (OUT OF SCOPE)

- `squad-station diagram` standalone subcommand — DIAG-F01, explicitly future
- Message queue depth per agent in diagram — DIAG-F02, explicitly future
- Animated/updating diagram — out of scope per REQUIREMENTS.md Out of Scope section
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DIAG-01 | After `squad-station init` completes, print ASCII diagram with all agents as labeled boxes containing name, role, provider, and tmux session name | New `src/commands/diagram.rs` module; `pub fn print_diagram(agents: &[Agent])` called from init.rs |
| DIAG-02 | Diagram shows directional arrows (▼) from orchestrator box to each worker agent box | Arrow row rendered between orchestrator and workers using `│` and `▼` at each worker column midpoint |
| DIAG-03 | Each agent box displays current DB status (idle/busy/dead) colored via existing `colorize_agent_status()` | Call `reconcile_agent_statuses()` before diagram; use `colorize_agent_status()` and `pad_colored()` from helpers.rs |
</phase_requirements>

---

## Summary

Phase 19 adds a static ASCII fleet diagram printed at the end of `squad-station init`. The implementation is self-contained Rust string formatting — no new crates required. All necessary infrastructure (owo-colors, `colorize_agent_status()`, `reconcile_agent_statuses()`, `list_agents()`, `pad_colored()`) already exists in the codebase.

The work is a new file `src/commands/diagram.rs` exposing one public function `print_diagram(agents: &[Agent])`, called from `init.rs` inside the existing `if !json` guard, after the "Get Started:" block. The diagram renders orchestrator as a centered box at top, workers as a horizontal row below, with `▼` arrows connecting them, and colored `[status]` badges.

The critical implementation challenge is correct terminal-width accounting for colored strings: ANSI escape codes inflate `str::len()` but not visible width. The existing `pad_colored()` helper in `helpers.rs` solves this precisely — it takes `raw` (for width measurement) and `colored` (for output) separately.

**Primary recommendation:** New `src/commands/diagram.rs` with pure string-building logic; zero new dependencies; integrate into init.rs after "Get Started:" block inside the existing `if !json` guard.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| owo-colors | already in Cargo.toml | Conditional ANSI color for `[status]` badges | Project standard; `colorize_agent_status()` already uses it |
| std (Rust stdlib) | stable | String building, `format!`, width arithmetic | No external crate needed for box drawing |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `crate::db::agents::Agent` | internal | Data source for diagram | Only source of truth for agent fields |
| `crate::commands::helpers::colorize_agent_status` | internal | Colored `[status]` text | All status badge rendering |
| `crate::commands::helpers::pad_colored` | internal | Width-correct padding when colored text is present | Status column alignment |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled box drawing | `tui-big-text`, `ratatui` widgets | Ratatui is already a dep (for TUI command) but using it for one-shot stdout printing adds unnecessary complexity; hand-rolled is 50 lines and simpler |
| `pad_colored` from helpers | `unicode-width` crate | `pad_colored` already handles the project's exact case — raw vs colored widths |

**Installation:** No new packages. All needed crates are in Cargo.toml.

---

## Architecture Patterns

### Recommended Project Structure

```
src/commands/
├── diagram.rs      # NEW: print_diagram(agents: &[Agent])
├── init.rs         # MODIFIED: call print_diagram after "Get Started:" block
├── helpers.rs      # UNCHANGED: colorize_agent_status, pad_colored used by diagram.rs
└── mod.rs          # MODIFIED: pub mod diagram;
```

### Pattern 1: Single Public Function Module

**What:** One public function in a dedicated module, all helper logic private.
**When to use:** When a feature has one call site and bounded complexity.

```rust
// src/commands/diagram.rs
use crate::db::agents::Agent;

pub fn print_diagram(agents: &[Agent]) {
    let orchestrators: Vec<&Agent> = agents.iter().filter(|a| a.role == "orchestrator").collect();
    let workers: Vec<&Agent> = agents.iter().filter(|a| a.role != "orchestrator").collect();

    // 1. Render orchestrator box (centered)
    // 2. Render arrow row
    // 3. Render worker row (wrap at ~80 cols)
    println!("\nAgent Fleet:");
    // ... box rendering
}
```

### Pattern 2: Visible-Width-Aware Padding (Critical)

**What:** When printing colored text that must align with other columns, measure width from the raw string and apply padding from the colored string. `pad_colored()` in helpers.rs does exactly this.
**When to use:** Any time a `colorize_agent_status()` result appears inside a fixed-width box column.

```rust
// Source: src/commands/helpers.rs (existing)
pub fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = width.saturating_sub(raw_len);
    format!("{}{}", colored, " ".repeat(padding))
}

// Usage in diagram:
let raw_status = format!("[{}]", agent.status);
let colored_status = format!("[{}]", colorize_agent_status(&agent.status));
let cell = pad_colored(&raw_status, &colored_status, STATUS_COL_WIDTH);
```

### Pattern 3: owo-colors Conditional Color (Project Standard)

**What:** Wrap all color calls with `if_supports_color(Stream::Stdout, ...)` so non-TTY output is clean.
**When to use:** All color output in CLI commands.

```rust
// Source: src/commands/helpers.rs, src/commands/welcome.rs (existing)
use owo_colors::OwoColorize;
use owo_colors::Stream;

// For bold ORCHESTRATOR label:
let label = "ORCHESTRATOR".if_supports_color(Stream::Stdout, |s| s.bold());
```

### Pattern 4: JSON Mode Guard (init.rs integration)

**What:** The `if !json { ... }` block in init.rs wraps all human-readable stdout. The diagram call goes inside this block.
**When to use:** Any post-init output that should not appear in machine-parseable JSON mode.

```rust
// Source: src/commands/init.rs (existing pattern, simplified)
if !json {
    // ... existing hook instructions, "Get Started:" block ...

    println!("\nAgent Fleet:");
    // Fetch fresh agents slice — pool is still in scope at this point
    let agents = db::agents::list_agents(&pool).await?;
    crate::commands::diagram::print_diagram(&agents);
}
```

### Pattern 5: Box Drawing Algorithm

**What:** Build each box line by line. Compute box width as `max(content_line_lengths) + 4` (2 for `│ ` border + space, 2 for ` │`).
**When to use:** Building agent boxes.

```
Box structure (width W, content C lines):
┌─────────────────────┐   <- "┌" + "─" * (W-2) + "┐"
│ <content line>      │   <- "│ " + content.pad_right(W-4) + " │"
└─────────────────────┘   <- "└" + "─" * (W-2) + "┘"
```

### Pattern 6: Arrow Row Between Orchestrator and Workers

**What:** Arrow row has one `▼` centered below each worker box, connected to the orchestrator by `│` characters. The orchestrator is full-width; each `▼` is placed at the horizontal midpoint of each worker box column.
**When to use:** Rendering the connector between orchestrator and worker row.

```
        ▼            ▼
```

For a single worker, the `▼` can be placed directly below the orchestrator center. For multiple workers, place `│` at each worker midpoint position in the gap row.

### Anti-Patterns to Avoid

- **Measuring colored string length with `.len()`:** ANSI escape codes add bytes that are invisible. Always measure from the raw (uncolored) string. Use `pad_colored()`.
- **Calling `reconcile_agent_statuses()` inside `print_diagram()`:** Reconciliation is async and touches the DB pool. Call it in init.rs before passing agents, keeping the diagram function sync and pure.
- **Hardcoding box width to a constant:** Different agent names have different lengths. Compute box width from the longest content line in each box.
- **Mixing the diagram output into the JSON path:** The call must be inside `if !json`. Never add diagram output outside that guard.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Colored status text | Custom color codes | `colorize_agent_status()` in helpers.rs | Consistent with rest of codebase; handles all statuses |
| Width-aware padding of colored text | Custom ANSI-stripping logic | `pad_colored()` in helpers.rs | Already implemented, tested, correct |
| DB agent fetch | Another query | `db::agents::list_agents(&pool)` | Returns `Vec<Agent>` with all needed fields |
| Tmux status reconciliation | Custom tmux checks | `reconcile_agent_statuses(&pool)` in helpers.rs | Already handles dead/idle transitions correctly |

**Key insight:** The hardest parts of this feature (color-aware padding, status reconciliation) are already solved in helpers.rs. The diagram function itself is pure string formatting.

---

## Common Pitfalls

### Pitfall 1: ANSI Escape Code Width Inflation

**What goes wrong:** Using `.len()` on a colored string gives a larger value than the terminal-visible width, causing box borders to misalign.
**Why it happens:** ANSI escape codes (e.g., `\x1b[32m...\x1b[0m`) add non-printing bytes that `.len()` counts.
**How to avoid:** Always compute display width from the raw (uncolored) string; use `pad_colored(raw, colored, width)`.
**Warning signs:** Box right borders don't line up when statuses have different lengths.

### Pitfall 2: Async/Sync Boundary

**What goes wrong:** `print_diagram` is a sync fn but `reconcile_agent_statuses` is async. Calling reconciliation inside the diagram function requires making it async and propagating `.await`.
**Why it happens:** Conflating "prepare data" and "render" concerns.
**How to avoid:** Reconcile in init.rs (which is already async), pass the reconciled `Vec<Agent>` slice to the sync `print_diagram` function.

### Pitfall 3: Worker Row Wrapping Off-By-One

**What goes wrong:** Wrapping logic places workers on the wrong row when they exactly fill the row width.
**Why it happens:** Off-by-one in cumulative width check (using `>` vs `>=`).
**How to avoid:** Track cumulative column width including inter-box gaps (2 spaces). Wrap when `current_width + box_width + gap > 80`.

### Pitfall 4: Orchestrator Box Width vs Worker Row Width

**What goes wrong:** Orchestrator box is narrower than the total worker row, making the diagram look asymmetric.
**Why it happens:** Orchestrator box width is computed only from orchestrator content.
**How to avoid:** Per the target visual in CONTEXT.md, the orchestrator box is its own natural width — asymmetry is acceptable and matches the example. Don't force-expand the orchestrator box to match all workers.

### Pitfall 5: Missing `pub mod diagram` in mod.rs

**What goes wrong:** Rust compile error: `crate::commands::diagram` not found.
**Why it happens:** New file added but not declared in `src/commands/mod.rs`.
**How to avoid:** Add `pub mod diagram;` to `src/commands/mod.rs` before integration.

---

## Code Examples

Verified patterns from project source:

### colorize_agent_status (helpers.rs)

```rust
// Source: src/commands/helpers.rs
pub fn colorize_agent_status(status: &str) -> String {
    match status {
        "idle" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.green())),
        "busy" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.yellow())),
        "dead" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.red())),
        "frozen" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.blue())),
        _ => status.to_string(),
    }
}
```

### pad_colored (helpers.rs)

```rust
// Source: src/commands/helpers.rs
pub fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = width.saturating_sub(raw_len);
    format!("{}{}", colored, " ".repeat(padding))
}
```

### if_supports_color pattern (welcome.rs, init.rs)

```rust
// Source: src/commands/welcome.rs
use owo_colors::OwoColorize;
use owo_colors::Stream;

let art = ASCII_ART.if_supports_color(Stream::Stdout, |s| s.red());
// Source: src/commands/init.rs
let bold = |s: &str| {
    s.if_supports_color(Stream::Stdout, |s| s.bold()).to_string()
};
```

### JSON guard pattern (init.rs)

```rust
// Source: src/commands/init.rs lines 304+
if !json {
    let green = |s: &str| { s.if_supports_color(Stream::Stdout, |s| s.green()).to_string() };
    // ... output block
}
```

### Agent struct fields (db/agents.rs)

```rust
// Source: src/db/agents.rs
pub struct Agent {
    pub id: String,
    pub name: String,      // also the tmux session name
    pub tool: String,      // "tool" column (renamed from provider)
    pub role: String,      // "orchestrator" or "worker"
    pub model: Option<String>,
    pub status: String,    // "idle" | "busy" | "dead" | "frozen"
    // ...
}
```

### Target diagram output (from CONTEXT.md)

```
Agent Fleet:
┌───────────────────────────────┐
│ ORCHESTRATOR                  │
│ myproj-claude-code-orch       │
│ tool: claude-code  [idle]     │
└───────────────────────────────┘
        │            │
        ▼            ▼
┌─────────────┐  ┌─────────────┐
│ worker1     │  │ worker1     │
│ tool: cc    │  │ tool: cc    │
│ [idle]      │  │ [busy]      │
└─────────────┘  └─────────────┘
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No post-init diagram | ASCII fleet diagram after init | Phase 19 | Users immediately see the full fleet topology and status |

**Deprecated/outdated:**
- None. This is new functionality.

---

## Open Questions

1. **Model field omission when None**
   - What we know: `Agent.model` is `Option<String>`; some agents have no model
   - What's unclear: Should the `tool: <tool>  model: <model>` line be omitted entirely when model is None, or show `tool: <tool>` only?
   - Recommendation: Show `tool: <tool>` only when model is None (omit `model:` portion). This keeps boxes narrower for agents without models. Consistent with how `generate_squad_yml` omits optional fields.

2. **Single-worker arrow rendering**
   - What we know: With one worker, there is one `▼`
   - What's unclear: Should the arrow be centered under the orchestrator, or left-aligned at worker box midpoint?
   - Recommendation: Center `▼` at the horizontal midpoint of the worker box, regardless of orchestrator box width. This matches the target visual from CONTEXT.md.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`) |
| Config file | none — uses `#[cfg(test)]` inline and `tests/` integration files |
| Quick run command | `cargo test diagram` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DIAG-01 | `print_diagram` renders orchestrator + worker boxes with correct fields | unit | `cargo test diagram` | ❌ Wave 0 |
| DIAG-02 | Arrow row (`▼`) appears between orchestrator and each worker | unit | `cargo test diagram` | ❌ Wave 0 |
| DIAG-03 | Status badges use correct color keywords (idle=green, busy=yellow, dead=red) | unit | `cargo test diagram` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test diagram`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/commands/diagram.rs` — module with `pub fn print_diagram()` and inline `#[cfg(test)]` tests
- [ ] Tests cover: box border characters present, agent name present, status badge present, `▼` present for worker count, `ORCHESTRATOR` label present, no output when agents list is empty

*(No new test infrastructure needed — existing `#[cfg(test)]` pattern in commands modules is the standard. No conftest or framework install required.)*

---

## Sources

### Primary (HIGH confidence)

- `src/commands/helpers.rs` — `colorize_agent_status()`, `pad_colored()`, `reconcile_agent_statuses()` — direct code read
- `src/commands/init.rs` — JSON guard pattern, pool availability, "Get Started:" block placement — direct code read
- `src/commands/welcome.rs` — `if_supports_color(Stream::Stdout, ...)` pattern, `OwoColorize` import — direct code read
- `src/db/agents.rs` — `Agent` struct fields, `list_agents()` — direct code read
- `.planning/phases/19-agent-diagram/19-CONTEXT.md` — all locked decisions — direct read

### Secondary (MEDIUM confidence)

- `.planning/REQUIREMENTS.md` — DIAG-01, DIAG-02, DIAG-03 exact acceptance criteria
- `tests/helpers.rs` — `setup_test_db()` pattern for validation architecture

### Tertiary (LOW confidence)

- None. All findings are verified from project source code.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml; verified from source
- Architecture: HIGH — integration points confirmed by reading init.rs and helpers.rs directly
- Pitfalls: HIGH — ANSI width pitfall confirmed by `pad_colored()` existence in helpers.rs; others confirmed from code reading

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable Rust project; no fast-moving dependencies)
