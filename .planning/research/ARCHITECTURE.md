# Architecture Research

**Domain:** Rust CLI — stateless binary with embedded SQLite, ratatui TUI, tmux integration (v1.8)
**Researched:** 2026-03-18
**Confidence:** HIGH — all findings derived from direct source inspection of the v1.7 codebase.

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
│                         CLI Dispatch (src/cli.rs)                    │
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
└──────┬─────────────────────────────┬───────────────────────────────┘
       │                             │
┌──────▼──────────┐        ┌─────────▼─────────────────────────────────┐
│  src/tmux.rs    │        │  src/db/  (SQLite via sqlx)                │
│  send_keys      │        │  mod.rs → connect() → pool setup           │
│  inject_body    │        │  agents.rs → insert/get/list/update        │
│  session_exists │        │  messages.rs → insert/list/update          │
│  launch_agent   │        │  migrations/ → auto-applied on connect     │
│  (no capture)   │        └────────────────────────────────────────────┘
└─────────────────┘
```

### Component Responsibilities

| Component | Responsibility | v1.8 Status |
|-----------|----------------|-------------|
| `src/main.rs` | Entry point, SIGPIPE handler, async runtime, None/Some dispatch | Minor: add Install match arm |
| `src/cli.rs` | clap `Commands` enum; one variant per subcommand | Modify: add `Install { tui: bool }` |
| `src/config.rs` | YAML parsing, DB path resolution, session name sanitization | No change |
| `src/tmux.rs` | All tmux shell-outs (arg builders + public API) | Modify: add `capture_pane()` |
| `src/db/agents.rs` | Agent CRUD, status updates | No schema change; `processing` is a new string value in free-form TEXT column |
| `src/db/messages.rs` | Message CRUD, priority ordering | No change |
| `src/db/migrations/` | SQL schema migrations, auto-applied by sqlx | No new migration needed |
| `src/commands/welcome.rs` | Bare-invocation TUI, WelcomeAction routing | No change |
| `src/commands/wizard.rs` | Multi-page ratatui form; project name input | Modify: pre-populate project field with folder name |
| `src/commands/init.rs` | Wizard guard, squad.yml generation, agent registration | No change |
| `src/commands/ui.rs` | `App` state, connect-per-refresh loop, ratatui draw | Modify: add `processing` color, project title, capture-pane poll |
| `src/commands/helpers.rs` | `reconcile_agent_statuses`, colorize, format utilities | Modify: add `processing` to colorize |
| `src/commands/diagram.rs` | ASCII agent fleet diagram | No change |
| `npm-package/bin/run.js` | `npx squad-station install` handler + binary proxy | Modify (optional): delegate scaffolding to Rust binary |
| `install.sh` | curl install path; TTY check + exec handoff | Modify (optional): change exec target |

---

## v1.8 Feature Integration Analysis

### Feature 1: `squad-station install [--tui]`

**Current state:** `npx squad-station install` is handled entirely in `npm-package/bin/run.js` (Node.js). There is no `install` subcommand in the Rust `Commands` enum. The curl `install.sh` auto-launches the bare binary after install.

**Integration points:**

New Rust file: `src/commands/install.rs`

```rust
// src/commands/install.rs
pub async fn run(tui: bool) -> anyhow::Result<()> {
    // 1. Scaffold .squad/ project files (sdd playbooks + example configs)
    // 2. If --tui: route to init wizard (no squad.yml) or dashboard (squad.yml exists)
    //    - same logic as bare invocation None arm in main.rs
    // 3. If no --tui: print next-steps text
}
```

`src/cli.rs` — add variant:
```rust
Install {
    #[arg(long)]
    tui: bool,
}
```

`src/main.rs` — add match arm:
```rust
Install { tui } => commands::install::run(tui).await,
```

`src/commands/mod.rs` — add:
```rust
pub mod install;
```

**npm-package/bin/run.js scope:** The existing `install()` function already works end-to-end. The architecture improvement is optional: move the `scaffoldProject()` logic into the Rust binary so that JS only handles binary download, then calls `spawnSync(destPath, ['install'])`. Doing so gives a single source of truth for what files are scaffolded. If deferred, the JS and Rust scaffold code must be kept in sync manually.

**install.sh scope:** Currently ends with `exec "${INSTALL_DIR}/squad-station"` (bare invocation). Changing to `exec "${INSTALL_DIR}/squad-station" install --tui` makes intent explicit and consistent with the new subcommand. The bare invocation already works due to None-arm routing, so this is a polish change.

**Files changed (Rust):**
- `src/cli.rs` — add `Install` variant
- `src/commands/install.rs` — NEW file
- `src/commands/mod.rs` — `pub mod install;`
- `src/main.rs` — new match arm

**Files changed (distribution, optional):**
- `npm-package/bin/run.js` — delegate scaffolding to binary
- `install.sh` — change exec target

---

### Feature 2: Folder Name as Project Name Default

**Current state:** `WizardState.project_input` is `TextInputState::new()` with empty `value`. The `WizardState::into_result()` trims it and writes to `WizardResult.project`. No folder detection exists.

**Integration points:**

`src/commands/wizard.rs` — in `WizardState::new()` or at `run()` call site:

```rust
// Pre-populate project input with CWD folder name
let folder_default = std::env::current_dir()
    .ok()
    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    .unwrap_or_default();

// In WizardState::new():
let mut input = TextInputState::new();
input.value = folder_default;
input.cursor = input.value.chars().count();
```

`src/commands/ui.rs` — title bar in `draw_ui()`:
- Currently hardcoded `" SQUAD-STATION "`.
- Change: add `project: String` field to `App` struct, populated from `config.project` in `run()` (config is already loaded at the top of `ui::run()`).
- `draw_ui()` renders `format!(" {} ", app.project)` in the title bar Paragraph.

`src/commands/init.rs` — `generate_squad_yml()`:
- No change. It already uses `result.project` verbatim. The value arrives pre-populated from wizard.

**Data flow:**

```
std::env::current_dir()
    → PathBuf::file_name()
    → String (e.g., "my-project")
        ↓
WizardState.project_input.value = "my-project"  (cursor at end)
        ↓
User edits or accepts on wizard Project page
        ↓
WizardState::into_result() → WizardResult.project = "my-project"
        ↓
generate_squad_yml() → "project: my-project\n"
        ↓
squad.yml written → config loaded → ui.rs App.project = "my-project"
        ↓
draw_ui() title bar shows " my-project "
```

**Files changed:**
- `src/commands/wizard.rs` — pre-populate `project_input` in `WizardState::new()` or `run()`
- `src/commands/ui.rs` — add `project: String` to `App`; render in title bar

---

### Feature 3: Orchestrator "Processing" State Detection

**Current state:** Agent status values in use: `idle`, `busy`, `dead`, `frozen`. `tmux.rs` has no `capture_pane` function — it exists only as documentation text in `context.rs` instructing the orchestrator AI how to use it manually. The TUI `status_color()` function has an `idle`/`busy`/catch-all pattern. `helpers.rs::colorize_agent_status()` handles `idle`, `busy`, `dead`, `frozen`.

**What "processing" means:** The orchestrator's tmux pane is actively showing output — tools running, model generating — distinct from `busy` (task assigned, may be idle in pane) and `idle` (waiting for next task). Detected by polling `tmux capture-pane` output for activity markers in the TUI refresh loop.

**Integration points:**

`src/tmux.rs` — new public function following existing arg-builder pattern:

```rust
fn capture_pane_args(session_name: &str) -> Vec<String> {
    vec!["capture-pane".into(), "-t".into(), session_name.into(), "-p".into()]
}

pub fn capture_pane(session_name: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(capture_pane_args(session_name))
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}
```

`src/commands/ui.rs` — inside the 3-second refresh loop:
- After `fetch_snapshot()` returns agents, find the orchestrator agent
- Call `tmux::capture_pane(&orch.name)` — synchronous, fits naturally in the async loop
- If output contains activity markers: open a short-lived writable pool, call `update_agent_status("processing")`, drop pool
- Add `"processing"` arm to `status_color()`: suggested color `Color::Cyan` or `Color::Magenta`

`src/commands/helpers.rs`:
- Add `"processing"` arm to `colorize_agent_status()` — keeps all colorize logic in one place

**Key architectural decision — write pool in TUI:**

The existing `fetch_snapshot()` uses `read_only(true)`. Introducing a write for status updates requires a separate writable pool. The pattern follows the existing convention: open writable `db::connect(&db_path)`, write, drop immediately. This matches what all non-TUI commands do and avoids holding a write lock across multiple refreshes.

```
TUI refresh tick (every 3s)
    │
    ├── fetch_snapshot(&db_path, selected_agent)   [read-only pool, drop on return]
    │       Returns: Vec<Agent>, Vec<Message>
    │
    ├── for orchestrator in agents:
    │       tmux::capture_pane(&orch.name) → Option<String>
    │       classify_pane_output(output) → bool
    │       if is_processing && orch.status != "processing":
    │           pool = db::connect(&db_path).await   [writable pool]
    │           update_agent_status(&pool, &orch.name, "processing").await
    │           drop(pool)
    │
    └── terminal.draw(|f| draw_ui(f, &mut app))
            status_color("processing") → Color::Cyan
```

**Activity marker classification:** Extract into a pure function for testability:

```rust
// Pure, no I/O — unit-testable without tmux
pub fn classify_pane_output(output: &str) -> bool {
    // Returns true if pane looks active (non-empty last lines, known markers)
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return false; }
    // Provider-specific: Claude shows "Thinking...", tool use lines, etc.
    // Gemini shows model output incrementally
    // Simple heuristic: non-empty + contains activity keywords
    let last = lines.last().unwrap_or(&"");
    last.contains("Thinking") || last.contains("Running") || last.contains("⣿")
        || output.contains("tool_use") || output.contains("ToolUse")
}
```

**Status value:** `"processing"` is a new string value in the existing free-form `TEXT` status column. No schema migration is needed. The column was always a free-form string — `status_updated_at` is updated alongside it by `update_agent_status()`.

**Files changed:**
- `src/tmux.rs` — add `capture_pane_args()` private fn + `capture_pane()` public fn
- `src/commands/ui.rs` — add `processing` color + poll in refresh loop + writable pool for status write
- `src/commands/helpers.rs` — add `"processing"` arm to `colorize_agent_status()`

---

## Complete File Change Matrix

| File | Change Type | Feature | What Changes |
|------|-------------|---------|-------------|
| `src/cli.rs` | Modify | install | Add `Install { tui: bool }` variant to `Commands` enum |
| `src/commands/mod.rs` | Modify | install | Add `pub mod install;` |
| `src/commands/install.rs` | New | install | `run(tui: bool)` — scaffold `.squad/` files, optional TUI launch |
| `src/main.rs` | Modify | install | Add `Install { tui }` match arm in `run()` |
| `src/commands/wizard.rs` | Modify | folder default | Pre-populate `project_input.value` with `current_dir()` folder name |
| `src/commands/ui.rs` | Modify | folder default + processing | Add `project: String` to `App`; title bar; add `processing` color; capture-pane poll + write |
| `src/tmux.rs` | Modify | processing | Add `capture_pane_args()` + `capture_pane()` |
| `src/commands/helpers.rs` | Modify | processing | Add `"processing"` arm to `colorize_agent_status()` |
| `npm-package/bin/run.js` | Modify (optional) | install | Delegate scaffolding to `squad-station install`; keep binary download in JS |
| `install.sh` | Modify (optional) | install | Change `exec` target to `squad-station install --tui` |

---

## Suggested Build Order

Dependencies flow: tmux layer → DB layer → commands → CLI → distribution.

### Phase 1: `capture_pane` + processing state (foundation for Feature 3)

**Why first:** Pure addition to `tmux.rs` with no side effects. Adding `processing` to colorize helpers and TUI color is an isolated display change. These touch foundational modules; landing them first reduces rebase risk on later changes.

Work:
- `src/tmux.rs` — `capture_pane_args()` + `capture_pane()` + unit tests for arg builder
- `src/commands/helpers.rs` — `"processing"` arm in `colorize_agent_status()`
- `src/commands/ui.rs` — `status_color("processing")` arm

No DB write yet — just display infrastructure.

### Phase 2: Processing detection in TUI refresh loop

**Why second:** Requires Phase 1 `capture_pane()` to exist. Adds the actual polling logic and the writable-pool-per-status-change pattern to the TUI.

Work:
- `src/commands/ui.rs` — capture-pane call in refresh loop + `classify_pane_output()` pure fn + writable pool write

### Phase 3: Folder name default (zero-risk UX improvement)

**Why third:** Entirely independent of Phases 1 and 2. Touches `wizard.rs` (one-liner) and `ui.rs` (title bar). Batching the `ui.rs` title bar change alongside Phase 2 `ui.rs` work is possible — either order is fine, but keeping them in separate phases reduces cognitive load in review.

Work:
- `src/commands/wizard.rs` — pre-populate in `WizardState::new()`
- `src/commands/ui.rs` — `project: String` field + title bar render

### Phase 4: `install` subcommand (most surface area)

**Why last:** Adds a new file and modifies the CLI enum. Coordinates with distribution files. Self-contained and independent of Phases 1–3, but has the most review surface and distribution touchpoints. Landing it last keeps the core binary changes stable first.

Work:
- `src/commands/install.rs` — NEW
- `src/cli.rs`, `src/commands/mod.rs`, `src/main.rs` — plumbing
- `npm-package/bin/run.js`, `install.sh` — distribution updates

---

## Architectural Patterns in Use

### Pattern 1: Command-per-file

**What:** Each subcommand lives in `src/commands/<name>.rs` with a `pub async fn run(...)` entry point. `mod.rs` re-exports via `pub mod` declarations. `main.rs` calls each `run()` directly.

**New file follows the same pattern:**
```rust
// src/commands/install.rs
pub async fn run(tui: bool) -> anyhow::Result<()> { ... }
```

Reconciliation logic is duplicated ~10 lines per file where needed — this is the established project decision. Do not introduce shared infrastructure for a single new command.

### Pattern 2: Connect-per-refresh in TUI (WAL checkpoint prevention)

**What:** `ui.rs::fetch_snapshot()` creates a read-only `SqlitePool` on every 3-second refresh and explicitly `drop(pool)` at function end. This releases WAL reader locks so the checkpoint can proceed.

**Impact on v1.8:** The processing-state feature introduces a write inside the TUI loop. The write must use a separate short-lived writable pool — same pattern as all non-TUI commands. Never hold a writable pool across refresh cycles.

### Pattern 3: Argument builder functions in tmux.rs

**What:** Each tmux operation has a private `fn _args(...)` returning `Vec<String>` (unit-testable), plus a public function calling `Command::new("tmux").args(...)`. The separation means arg correctness is verified without spawning tmux.

**The new `capture_pane` follows this exactly:**
```rust
fn capture_pane_args(session_name: &str) -> Vec<String> { ... }
pub fn capture_pane(session_name: &str) -> Option<String> { ... }
```

### Pattern 4: Pure rendering and classification functions

**What:** `diagram::render_diagram()` returns `String`; `welcome::routing_action()` returns `Option<WelcomeAction>`. No side effects, directly unit-testable.

**Impact on v1.8:** `classify_pane_output(output: &str) -> bool` must be pure — no I/O, no tmux calls. Keeps the detection logic testable without tmux running.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Persistent writable pool in TUI

**What people do:** Open a writable pool once at TUI startup and keep it for the session duration.
**Why it's wrong:** Holds a WAL write-eligible connection indefinitely. Blocks `signal` hooks from completing writes. Causes `SQLITE_BUSY` under concurrent hook fires.
**Do this instead:** Open a writable `db::connect()` only when a write is needed (status change detected), drop it immediately.

### Anti-Pattern 2: Scaffolding logic duplicated across install paths

**What people do:** Maintain parallel scaffolding code in `npm-package/bin/run.js` (JS) and `src/commands/install.rs` (Rust).
**Why it's wrong:** When SDD playbook files or example configs change, both files must be updated in sync. The JS copy is not compiled, has no tests, and is easy to forget.
**Do this instead:** Keep binary download in JS (inherently JS territory). Delegate `scaffoldProject()` to `spawnSync(destPath, ['install'])`. One source of truth for what files get scaffolded.

### Anti-Pattern 3: Hardcoded processing marker strings in UI code

**What people do:** `output.contains("Thinking...")` scattered through the event loop.
**Why it's wrong:** Provider output formats change between versions. Makes testing fragile. Different providers have different indicators.
**Do this instead:** Isolate in `classify_pane_output(output: &str) -> bool` — single pure function, directly testable, easy to update when providers change.

### Anti-Pattern 4: Overwriting agent status on every refresh

**What people do:** Call `update_agent_status("processing")` on every tick regardless of current status.
**Why it's wrong:** Creates unnecessary DB writes; overrides `idle`/`busy` incorrectly when the pane shows stale output from a completed task.
**Do this instead:** Only write `"processing"` when `orch.status != "processing"` and the pane genuinely shows live activity. Let signal hooks (which call `update_agent_status("idle")`) remain the authoritative completion signal.

---

## Integration Boundaries

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `commands/` ↔ `tmux.rs` | Direct function calls | All tmux ops go through `tmux.rs` — no inline `Command::new("tmux")` in command files |
| `commands/` ↔ `db/` | Async function calls with `&SqlitePool` | Pool lifetime managed by caller; TUI uses connect-per-refresh |
| `commands/install.rs` ↔ `commands/welcome.rs` | Call `run_welcome_tui()` when `--tui` flag set | Same path as bare invocation |
| `commands/wizard.rs` ↔ `std::env` | `current_dir()` call in `WizardState::new()` | Graceful fallback to empty string on failure |
| `commands/ui.rs` ↔ `tmux.rs` | `capture_pane()` per refresh | Returns `Option<String>` — TUI continues on `None`; no crash on missing session |

### External Surfaces

| Surface | v1.8 Change | Notes |
|---------|-------------|-------|
| `npm-package/bin/run.js` `install()` | Optional: delegate scaffolding to Rust binary | Reduces JS/Rust drift risk |
| `install.sh` exec target | Optional: change to `squad-station install --tui` | Explicit subcommand vs bare invocation |
| `squad.yml` `project:` field | Pre-populated with folder name as default | No format change — same YAML structure |
| SQLite `agents.status` column | New string value `"processing"` | No schema migration — free-form `TEXT` column, always was |
| CLI surface | New `install [--tui]` subcommand | Replaces JS-only install path with Rust-native one |

---

## Sources

- Direct source inspection: `src/cli.rs`, `src/main.rs`, `src/commands/ui.rs`, `src/commands/wizard.rs`, `src/commands/init.rs`, `src/commands/helpers.rs`, `src/tmux.rs`, `src/db/agents.rs`, `src/db/migrations/0001_initial.sql`, `src/db/migrations/0002_agent_status.sql`
- Direct source inspection: `npm-package/bin/run.js`, `install.sh`
- Project history in `.planning/PROJECT.md`
- Existing patterns: connect-per-refresh in `ui.rs`; arg-builder in `tmux.rs`; pure-fn in `diagram.rs`/`welcome.rs`

---

*Architecture research for: Squad Station v1.8 — Install subcommand, folder name default, orchestrator processing state*
*Researched: 2026-03-18*
