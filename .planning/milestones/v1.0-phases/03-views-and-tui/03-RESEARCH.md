# Phase 3: Views and TUI - Research

**Researched:** 2026-03-06
**Domain:** Rust TUI (ratatui/crossterm), tmux pane management, SQLite read-only polling
**Confidence:** HIGH

## Summary

Phase 3 delivers three new CLI subcommands (`status`, `ui`, `view`) on top of the existing Phase 1-2 infrastructure. All primitives are already in place: DB connection patterns, reconciliation logic, tmux helpers, coloring utilities. The research focus is on ratatui APIs for the interactive dashboard, the SQLite connection strategy to avoid WAL checkpoint starvation during polling, and tmux pane management for the `view` command.

The ratatui 0.26 API (already in `Cargo.toml`) is stable and well-documented. The critical architectural constraint for `ui` is: **never hold a persistent pool across the polling interval** — open a fresh read-only pool per refresh tick, close it fully before the next tick. This prevents the TUI from blocking WAL checkpoints. The `view` command uses `tmux new-window` + `tmux split-window` + `tmux select-layout tiled`, which maps cleanly onto the existing `std::process::Command` tmux pattern.

**Primary recommendation:** Implement `status.rs` first (pure DB query + text output), then `view.rs` (tmux orchestration, stateless), then `ui.rs` (ratatui, most complex). All three modules follow the existing stateless command pattern in `commands/`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Status command (VIEW-01)**
- Shows summary header: project name, total agents, counts by status (e.g., "Agents: 5 — 3 idle, 1 busy, 1 dead"), DB path
- Compact inline list per agent: one line each like "frontend: idle 5m | backend: busy 2m"
- Per-agent pending message counts shown alongside status
- Always reconciles tmux state (like `agents` command does) — accuracy over speed
- Supports `--json` flag for machine-readable output (consistent with all other commands)

**TUI dashboard (VIEW-03)**
- Two-panel split layout: left panel = agent list, right panel = messages for selected agent
- Auto-refresh on interval (polling DB periodically)
- Read-only monitoring — no send/signal actions from within TUI
- Keyboard navigation: up/down to select, tab to switch panels, q to quit
- Colored status indicators consistent with `agents` command (green/yellow/red)

**Tmux pane layout (VIEW-04)**
- Auto grid layout adapting to agent count (like `tmux select-layout tiled`)
- Include all sessions including orchestrator — full fleet visibility
- Skip dead agents (no tmux session) — only show live sessions
- Clean up on re-run: idempotent behavior if view already exists

**VIEW-02 (agents command)**
- Already complete from Phase 2 — no changes needed

### Claude's Discretion
- DB connection strategy for TUI (connect-per-refresh vs read-only persistent connection) — must satisfy ROADMAP success criteria about WAL checkpoint starvation
- Tmux pane creation approach (new window with linked panes vs rearrange existing sessions)
- TUI auto-refresh interval (2-5 seconds range)
- Exact status command layout and formatting
- TUI keybindings beyond the basics (q, arrows, tab)

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| VIEW-01 | User can see squad overview via `squad-station status` (text output) | DB query patterns from `agents.rs`; reconciliation pattern from `context.rs`; pending message count via `list_messages` filter; `--json` via existing `serde_json` pattern |
| VIEW-02 | User can list agents and their status via `squad-station agents` | Already complete in Phase 2 — no work needed |
| VIEW-03 | User can view interactive TUI dashboard via `squad-station ui` (ratatui) | ratatui 0.26 Layout/List/Paragraph APIs; crossterm alternate screen + raw mode; connect-per-refresh DB strategy; tokio::time::interval for polling |
| VIEW-04 | User can view split tmux pane layout of all agents via `squad-station view` | tmux list-sessions -F, new-window, split-window, select-layout tiled; idempotency via has-session check |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.26 | TUI rendering: layout, widgets, frame | Industry standard Rust TUI, pinned in Cargo.toml |
| crossterm | 0.27 | Terminal raw mode, alternate screen, events | Default ratatui backend, already declared |
| sqlx | 0.8 | Async SQLite queries | Already in use for all DB operations |
| tokio | 1.37 | Async runtime, interval timers | Already in use for all async commands |
| owo-colors | 3 | Colored terminal output | Already in use for `agents` command colorization |
| chrono | 0.4 | Duration formatting (e.g., "5m", "2h3m") | Already in use for status timestamps |
| anyhow | 1.0 | Error propagation | Already in use project-wide |

### No New Dependencies Required
All libraries needed for Phase 3 are already declared in `Cargo.toml`. No `cargo add` commands needed.

### Alternatives Considered
| Standard Choice | Alternative | Why Not |
|-----------------|-------------|---------|
| connect-per-refresh DB strategy | Persistent read-only pool | Persistent pool prevents WAL checkpoint even read-only (checkpoint blocked until readers release) |
| `tmux select-layout tiled` | Manual pane size calculation | tiled is built-in, handles any N automatically |
| `tokio::time::interval` for refresh | `std::thread::sleep` | Project uses tokio async — stay consistent, avoid blocking thread |

## Architecture Patterns

### Recommended Module Structure
```
src/commands/
├── status.rs       # VIEW-01: reconcile + query + text output
├── ui.rs           # VIEW-03: ratatui event loop + polling
├── view.rs         # VIEW-04: tmux new-window + split + tiled layout
```

Integration points (as specified in CONTEXT.md):
- `cli.rs`: Add `Status`, `Ui`, `View` to `Commands` enum
- `main.rs`: Add dispatch arms for three new commands
- `commands/mod.rs`: Add `pub mod status; pub mod ui; pub mod view;`
- `tmux.rs`: Add `list_live_sessions()` helper for `view` command

### Pattern 1: Status Command (VIEW-01)
**What:** Stateless command — connect DB, reconcile tmux state, count pending messages, print summary, exit.
**When to use:** Same pattern as `agents` command; extends it with per-agent pending count and summary header.

```rust
// Source: pattern from src/commands/agents.rs + src/commands/context.rs

pub async fn run(json: bool) -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // Reconcile (same as agents.rs)
    let agents = db::agents::list_agents(&pool).await?;
    for agent in &agents {
        let alive = tmux::session_exists(&agent.name);
        if !alive && agent.status != "dead" {
            db::agents::update_agent_status(&pool, &agent.name, "dead").await?;
        } else if alive && agent.status == "dead" {
            db::agents::update_agent_status(&pool, &agent.name, "idle").await?;
        }
    }
    let agents = db::agents::list_agents(&pool).await?;

    // Count pending messages per agent
    // Reuse db::messages::list_messages(&pool, Some(&agent.name), Some("pending"), 9999)
    // and take .len() — or add a count_pending() DB fn

    // Summary header
    // "Project: {name} | Agents: {total} — {idle} idle, {busy} busy, {dead} dead"
    // "DB: {db_path}"
    // Per-agent lines: "  frontend: idle 5m  |  2 pending"

    // JSON branch: serde_json::to_string_pretty(&output_struct)
    Ok(())
}
```

### Pattern 2: TUI Dashboard (VIEW-03) — Connect-Per-Refresh Strategy

**What:** Ratatui event loop with tokio interval timer. On each tick: open a new read-only pool, fetch agents + messages, close pool, re-render.
**When to use:** Any TUI polling SQLite — never hold pool between ticks.

**CRITICAL — WAL checkpoint starvation prevention:**
A persistent SQLite reader (even read-only) blocks WAL checkpoints. SQLite's WAL checkpoint can only reset the WAL file when no connections are open. The project's single-writer pool (`max_connections(1)`) writes to WAL — if the TUI holds an open reader, the WAL grows unboundedly and checkpoints starve.

**Solution: connect-per-refresh.** Open pool → fetch → close pool. Each refresh cycle is isolated. Pool drop releases the reader lock.

```rust
// Source: verified pattern from sqlx docs + SQLite WAL docs
// https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html
// https://sqlite.org/wal.html

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

async fn fetch_snapshot(db_path: &std::path::Path)
    -> anyhow::Result<AppSnapshot>
{
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)  // prevents accidental writes
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    let agents = db::agents::list_agents(&pool).await?;
    // pool drops here — WAL reader lock released
    Ok(AppSnapshot { agents })
}
```

The `read_only(true)` flag on `SqliteConnectOptions` prevents accidental writes and communicates intent. Importantly, `journal_mode` on a read-only connection does NOT switch the WAL mode (WAL is a database-level setting persisted on disk) — it is safe to specify here for documentation clarity.

**Refresh interval recommendation (Claude's discretion):** 3 seconds. Fast enough for meaningful monitoring, slow enough to avoid DB overhead. 2 seconds is acceptable for high-activity squads; 5 seconds is max reasonable latency for status visibility.

### Pattern 3: Ratatui Two-Panel Layout (VIEW-03)

```rust
// Source: https://ratatui.rs/concepts/layout/
// Source: https://docs.rs/ratatui/latest/ratatui/widgets/struct.List.html

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

fn ui(frame: &mut Frame, app: &App) {
    // Split terminal into left 35% + right 65%
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .split(frame.area());

    // Left panel: agent list
    let items: Vec<ListItem> = app.agents.iter().map(|a| {
        ListItem::new(format!("{}: {}", a.name, a.status))
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Agents"))
        .highlight_style(Style::default().reversed())
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, chunks[0], &mut app.list_state.clone());

    // Right panel: messages for selected agent
    let msg_text = app.selected_messages_text();
    let paragraph = Paragraph::new(msg_text)
        .block(Block::default().borders(Borders::ALL).title("Messages"));
    frame.render_widget(paragraph, chunks[1]);
}
```

### Pattern 4: Terminal Setup/Teardown (VIEW-03)

```rust
// Source: https://ratatui.rs/templates/component/tui-rs/

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn enter_tui() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    Ok(Terminal::new(backend)?)
}

fn exit_tui(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
```

**Important:** Always call `exit_tui()` even on error — use a guard or `Drop` to ensure the terminal is restored. Failure to restore leaves the user's terminal in raw mode (no echo, no cursor).

### Pattern 5: Event Loop with Polling (VIEW-03)

```rust
// Source: https://ratatui.rs/tutorials/counter-async-app/async-event-stream/

use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind};
use std::time::Duration;
use tokio::time::Instant;

pub async fn run_tui(db_path: &std::path::Path) -> anyhow::Result<()> {
    let mut terminal = enter_tui()?;
    let mut app = App::default();
    let refresh_interval = Duration::from_secs(3);
    let mut last_refresh = Instant::now() - refresh_interval; // trigger immediate refresh

    loop {
        // Refresh on interval
        if last_refresh.elapsed() >= refresh_interval {
            app.snapshot = fetch_snapshot(db_path).await?;
            last_refresh = Instant::now();
        }

        terminal.draw(|f| ui(f, &app))?;

        // Poll for key events (non-blocking, short timeout to keep refresh working)
        if event::poll(Duration::from_millis(250))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                        KeyCode::Tab => app.toggle_focus(),
                        _ => {}
                    }
                }
            }
        }
    }

    exit_tui(&mut terminal)?;
    Ok(())
}
```

Note: `event::poll(Duration::from_millis(250))` is the synchronous polling API from crossterm. It blocks for at most 250ms, then returns `false` if no event arrived. This keeps the main loop responsive while still honouring the refresh interval. This approach avoids the complexity of `EventStream` (requires `event-stream` feature) and tokio async channels — appropriate for this single-threaded TUI.

### Pattern 6: Tmux Pane Layout (VIEW-04)

**Approach:** Create a new tmux window named `squad-view`, split panes for each live session, apply `tiled` layout. Idempotency: check if `squad-view` window exists, destroy and recreate.

```rust
// Source: tmux man page + verified via tmux documentation

fn build_view_window(sessions: &[String]) -> anyhow::Result<()> {
    // Idempotency: destroy existing squad-view window if present
    Command::new("tmux")
        .args(["kill-window", "-t", "squad-view"])
        .status()
        .ok(); // ignore error if window doesn't exist

    if sessions.is_empty() {
        println!("No live sessions to display.");
        return Ok(());
    }

    // Create new window with first session linked
    Command::new("tmux")
        .args(["new-window", "-n", "squad-view", "-t", &sessions[0]])
        .status()?;

    // Split panes for remaining sessions
    for session in &sessions[1..] {
        Command::new("tmux")
            .args(["split-window", "-t", "squad-view", "-d"])
            .status()?;
        // Link pane to session (see note below)
    }

    // Apply tiled layout
    Command::new("tmux")
        .args(["select-layout", "-t", "squad-view", "tiled"])
        .status()?;

    Ok(())
}
```

**Tmux pane linking approach (Claude's discretion):**

Option A — `link-window`: Link each agent's existing window into the squad-view window as a pane. This shows live content but `link-window` creates windows, not panes.

Option B — `split-window` + display agent name as pane title: Each pane runs `tmux attach-session -t {agent}` within the pane, showing the agent's live terminal. The pane title can be set via `select-pane -t squad-view:{idx} -T {agent_name}`.

**Recommended (Option B — `split-window` each showing `tmux attach`):**
```bash
# In the new window's first pane, run: tmux attach-session -t <first_agent>
# For each additional agent: split-window running: tmux attach-session -t <agent>
# Then: select-layout tiled
```

This is how tools like `tmux-cssh` and multi-pane monitoring setups work. It gives live view into each agent's session without disrupting the session itself.

**Getting live sessions:** Use `tmux list-sessions -F "#{session_name}"` to enumerate all live sessions, then filter against DB agent list (excluding those with status "dead"):

```rust
fn list_live_sessions() -> Vec<String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .unwrap_or_default();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
```

### Pattern 7: Pending Message Count for Status Command

```rust
// Reuse existing db::messages::list_messages for per-agent pending count
// No new DB function needed

async fn count_pending(pool: &SqlitePool, agent_name: &str) -> usize {
    db::messages::list_messages(pool, Some(agent_name), Some("pending"), 9999)
        .await
        .map(|msgs| msgs.len())
        .unwrap_or(0)
}
```

Alternatively, add `count_pending_messages(pool, agent_name) -> u32` to `db/messages.rs` using `SELECT COUNT(*)` for efficiency. Given small agent counts this is optional but cleaner.

### Anti-Patterns to Avoid
- **Persistent DB pool in TUI loop:** Holding `pool` across refresh ticks blocks WAL checkpoints. Always drop pool between fetches.
- **Blocking on tmux in TUI loop:** Calling `tmux::session_exists()` during TUI rendering blocks the render thread. Pre-fetch session state before entering the draw loop, refresh on same interval as DB.
- **Not restoring terminal on panic:** Raw mode + alternate screen must be disabled even on panic. Use a cleanup wrapper or `std::panic::set_hook`.
- **Using `crossterm::event::read()` without poll:** `event::read()` blocks forever — always gate it with `event::poll(timeout)`.
- **Forgetting `KeyEventKind::Press` filter:** Windows emits both Press and Release events. Filter `key.kind == KeyEventKind::Press` to avoid double-processing.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal layout/panels | Custom ANSI escape positioning | `ratatui::Layout` with `Constraint::Percentage` | Handles resize, clipping, all terminal sizes automatically |
| Keyboard event capture | Direct libc termios manipulation | `crossterm::event::poll` + `event::read()` | Cross-platform, handles escape sequences, already in Cargo.toml |
| Alternate screen management | Manual ANSI `\x1b[?1049h` sequences | `crossterm::execute!(EnterAlternateScreen)` | Correct restore on exit including panics |
| Selectable list UI | Manual cursor tracking + ANSI colors | `ratatui::widgets::List` + `ListState` | Selection, scrolling, highlight all handled |
| Tmux grid layout | Manual pane size calculation | `tmux select-layout tiled` | Built-in to tmux, adapts to any N |
| Session enumeration | Parsing `tmux ls` default output | `tmux list-sessions -F "#{session_name}"` | Format flag gives clean parseable output |

**Key insight:** ratatui handles all layout math and rendering — never compute terminal coordinates or ANSI escape sequences manually.

## Common Pitfalls

### Pitfall 1: Terminal Left in Raw Mode on Error
**What goes wrong:** If `ui.rs` panics or returns an error before calling `disable_raw_mode()`, the user's terminal is left in raw mode — no visible typed input, no cursor, must `reset` from a blind prompt.
**Why it happens:** Rust doesn't have `defer`; cleanup requires explicit call or Drop impl.
**How to avoid:** Wrap TUI entry/exit in a function that uses `?` consistently AND add a `std::panic::set_hook` that calls `disable_raw_mode()` before printing panic info.
**Warning signs:** User reports terminal acting weird after using `squad-station ui`.

### Pitfall 2: WAL Checkpoint Starvation
**What goes wrong:** TUI holds a persistent SQLite pool → WAL file grows unboundedly → write performance degrades → other commands become slow.
**Why it happens:** SQLite WAL checkpoint requires no open readers. Even a `read_only` pool with idle connections blocks checkpoints.
**How to avoid:** Connect-per-refresh strategy. Open pool → fetch → let pool drop. Never store pool in `App` struct across ticks.
**Warning signs:** DB file grows unexpectedly; `squad-station send` becomes slow while TUI is running.

### Pitfall 3: Tmux `view` Command Targeting Errors
**What goes wrong:** `tmux split-window -t squad-view` fails if `squad-view` window doesn't exist yet, or targets wrong session.
**Why it happens:** tmux targets are `session:window.pane` — need correct scoping.
**How to avoid:** Always create the window first (`new-window -n squad-view`), then target `squad-view` for subsequent splits. Verify success via `status()?.success()`.
**Warning signs:** Some agent panes missing in the view layout.

### Pitfall 4: `view` Not Idempotent
**What goes wrong:** Running `squad-station view` twice creates a second `squad-view` window alongside the first.
**Why it happens:** `new-window` always creates a new window.
**How to avoid:** Check for existing window (`tmux has-session -t squad-view` or `list-windows -F "#{window_name}" | grep squad-view`) and kill it before recreating. The `kill-window -t squad-view` command is idempotent (returns non-zero if not found, which we ignore with `.ok()`).
**Warning signs:** Multiple `squad-view` windows accumulating.

### Pitfall 5: Reconciliation During TUI Refresh Modifies DB
**What goes wrong:** TUI refresh calls reconciliation (like `agents` command does), which writes to DB, which conflicts with the `read_only` pool strategy.
**Why it happens:** Reconciliation requires write access to update status.
**How to avoid:** TUI should NOT reconcile. TUI is read-only monitoring. Display whatever is in DB as-is. Reconciliation happens on `status` and `agents` commands (stateless, short-lived). This is explicitly specified in CONTEXT.md: TUI is read-only monitoring.
**Warning signs:** TUI fails with "attempt to write a readonly database" if pool is read_only and reconciliation is called.

### Pitfall 6: `crossterm::event::EventStream` Feature Not Enabled
**What goes wrong:** Using `EventStream::new()` (async approach) fails to compile — requires `event-stream` feature flag.
**Why it happens:** `crossterm = "0.27"` in Cargo.toml has no extra features declared.
**How to avoid:** Use synchronous `event::poll()` + `event::read()` instead. This is simpler, avoids feature flag changes, and is correct for this non-async polling model.
**Warning signs:** Compile error referencing `EventStream` not found.

## Code Examples

Verified patterns from official sources:

### Ratatui Terminal Init + Cleanup
```rust
// Source: https://ratatui.rs/templates/component/tui-rs/
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).map_err(Into::into)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
```

### Two-Panel Layout Split
```rust
// Source: https://ratatui.rs/concepts/layout/
use ratatui::prelude::{Constraint, Direction, Layout};

let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
    .split(frame.area());
// chunks[0] = left panel (agent list)
// chunks[1] = right panel (messages)
```

### Stateful List with Selection
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/widgets/struct.List.html
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::style::{Style, Modifier};

let items: Vec<ListItem> = agents.iter().map(|a| {
    ListItem::new(format!("{}: {}", a.name, a.status))
}).collect();

let list = List::new(items)
    .block(Block::default().borders(Borders::ALL).title("Agents"))
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

frame.render_stateful_widget(list, area, &mut state);
```

### Navigation Logic
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/widgets/struct.ListState.html
fn select_next(state: &mut ListState, len: usize) {
    let next = match state.selected() {
        Some(i) => if i >= len - 1 { 0 } else { i + 1 },
        None => 0,
    };
    state.select(Some(next));
}

fn select_previous(state: &mut ListState, len: usize) {
    let prev = match state.selected() {
        Some(i) => if i == 0 { len - 1 } else { i - 1 },
        None => 0,
    };
    state.select(Some(prev));
}
```

### Read-Only DB Connection (per-refresh)
```rust
// Source: https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

async fn open_readonly_pool(db_path: &std::path::Path) -> anyhow::Result<sqlx::SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;
    Ok(pool)
    // caller drops pool after use — WAL reader lock released
}
```

### Tmux Live Session Enumeration
```rust
// Source: tmux man page — list-sessions -F format flag
use std::process::Command;

pub fn list_live_session_names() -> Vec<String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output()
        .unwrap_or_default();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}
```

### Tmux Squad-View Window (idempotent)
```rust
// Source: tmux man page + verified patterns
use std::process::Command;

pub fn create_squad_view(live_agents: &[String]) -> anyhow::Result<()> {
    // Idempotent teardown
    Command::new("tmux").args(["kill-window", "-t", "squad-view"]).status().ok();

    if live_agents.is_empty() {
        println!("No live agent sessions to display.");
        return Ok(());
    }

    // Create window, first pane shows first agent's session
    Command::new("tmux")
        .args(["new-window", "-n", "squad-view",
               "tmux", "attach-session", "-t", &live_agents[0]])
        .status()?;

    // Add remaining agents as panes
    for agent in &live_agents[1..] {
        Command::new("tmux")
            .args(["split-window", "-t", "squad-view",
                   "tmux", "attach-session", "-t", agent])
            .status()?;
    }

    // Apply tiled grid layout
    Command::new("tmux")
        .args(["select-layout", "-t", "squad-view", "tiled"])
        .status()?;

    Ok(())
}
```

### Status Command JSON Output Shape
```rust
// Consistent with existing --json pattern across all commands
#[derive(serde::Serialize)]
struct StatusOutput {
    project: String,
    db_path: String,
    agents: Vec<AgentStatusSummary>,
}

#[derive(serde::Serialize)]
struct AgentStatusSummary {
    name: String,
    role: String,
    status: String,
    status_updated_at: String,
    pending_messages: usize,
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tui` crate | `ratatui` | 2023 (ratatui is the maintained fork) | ratatui is the active community fork; `tui` crate is archived |
| `StatefulWidget` hand-impl | `List::new().highlight_*()` | ratatui 0.22+ | Built-in highlight API, no manual StatefulWidget impl needed |
| `crossterm::event::read()` only | `event::poll(timeout)` + `read()` | Always | poll() prevents blocking; required for refresh loops |
| Persistent DB connection in TUI | Connect-per-refresh | Best practice | WAL checkpoint starvation prevention |

**Deprecated/outdated:**
- `tui` crate: Archived — do not use. `ratatui` is the maintained fork. Already correctly using `ratatui` in Cargo.toml.
- `StatefulList` wrapper struct (old examples): ratatui 0.26 `ListState` has built-in `select_next()`/`select_previous()` methods — verify if available in 0.26 or implement manually with the `match` pattern above.

## Open Questions

1. **Tmux `new-window` + `attach-session` reliability**
   - What we know: `tmux new-window -n squad-view "tmux attach-session -t agent-name"` creates a pane that shows another session. Works in practice with tmux.
   - What's unclear: Behavior when the target session no longer exists between enumeration and pane creation — race condition.
   - Recommendation: Check `tmux::session_exists()` for each agent just before creating the pane. Skip if gone.

2. **`ListState::select_next()` / `select_previous()` in ratatui 0.26**
   - What we know: ratatui 0.28+ added convenience methods on `ListState`. ratatui 0.26 may not have them.
   - What's unclear: Exact 0.26 `ListState` API surface.
   - Recommendation: Implement navigation manually with the `match` pattern shown above — works for all ratatui versions and is trivial code.

3. **`squad-view` window targeting — current session vs. named target**
   - What we know: `kill-window -t squad-view` requires tmux to find the window by name across sessions.
   - What's unclear: If user has multiple sessions, `-t squad-view` might be ambiguous.
   - Recommendation: Use fully-qualified target: determine current session name from `$TMUX_PANE` env var and scope: `kill-window -t {current_session}:squad-view`. Fallback: `kill-window -t :squad-view` (searches all sessions).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + tokio-test 0.4 |
| Config file | No separate config — Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test` |
| Full suite command | `cargo test` |

Current test count: 47 tests passing (pre-Phase 3).

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIEW-01 | `status` command exits 0, prints project name + agent summary | integration (subprocess) | `cargo test test_status_` | ❌ Wave 0 |
| VIEW-01 | `status --json` outputs valid JSON with `agents` array | integration (subprocess) | `cargo test test_status_json` | ❌ Wave 0 |
| VIEW-01 | `status` shows per-agent pending message count | integration (subprocess) | `cargo test test_status_pending_count` | ❌ Wave 0 |
| VIEW-02 | `agents` command (already tested) | existing | `cargo test test_agents_command` | ✅ tests/test_lifecycle.rs |
| VIEW-03 | `ui` command: read-only pool does not hold write lock | unit | `cargo test test_ui_readonly_pool` | ❌ Wave 0 |
| VIEW-03 | TUI keybindings: `q` sets quit flag | unit | `cargo test test_ui_quit_key` | ❌ Wave 0 |
| VIEW-03 | TUI navigation: up/down changes selected index | unit | `cargo test test_ui_navigation` | ❌ Wave 0 |
| VIEW-04 | `view` command: no live agents prints informative message | integration (subprocess) | `cargo test test_view_no_agents` | ❌ Wave 0 |
| VIEW-04 | `view`: live session list filtered to agents not marked dead | unit | `cargo test test_view_filters_dead` | ❌ Wave 0 |

Note: VIEW-03 TUI event loop and tmux-dependent VIEW-04 tests are partially manual (require live tmux session). Unit tests cover the logic layer (app state transitions, DB pool behavior). Integration tests use subprocess invocation pattern from `test_lifecycle.rs`.

### Sampling Rate
- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green + 47 existing + new VIEW tests before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/test_views.rs` — covers VIEW-01 status command (subprocess integration tests + DB query unit tests)
- [ ] `src/commands/status.rs` — VIEW-01 implementation
- [ ] `src/commands/ui.rs` — VIEW-03 implementation (unit-testable App state separate from TUI rendering)
- [ ] `src/commands/view.rs` — VIEW-04 implementation (tmux orchestration functions testable via arg builders pattern from tmux.rs)
- [ ] `src/commands/mod.rs` — add `pub mod status; pub mod ui; pub mod view;`
- [ ] `cli.rs` — add `Status`, `Ui`, `View` variants to `Commands` enum
- [ ] `main.rs` — add dispatch arms for new commands

## Sources

### Primary (HIGH confidence)
- [ratatui docs.rs 0.26](https://docs.rs/ratatui/latest/ratatui/) — Layout, List, ListState, Paragraph, Frame APIs
- [ratatui layout concepts](https://ratatui.rs/concepts/layout/) — Direction, Constraint types, split() usage
- [ratatui component template](https://ratatui.rs/templates/component/tui-rs/) — Terminal enter/exit, raw mode, event loop architecture
- [sqlx SqliteConnectOptions](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html) — read_only(), journal_mode() method signatures
- [SQLite WAL documentation](https://sqlite.org/wal.html) — checkpoint starvation: WAL resets only when no readers
- [tmux man page](https://www.man7.org/linux/man-pages/man1/tmux.1.html) — list-sessions -F, select-layout tiled, split-window
- Project source: `src/commands/agents.rs` — reconciliation pattern to reuse
- Project source: `src/db/mod.rs` — existing connection pattern (`max_connections(1)`, WAL)
- Project source: `src/tmux.rs` — arg-builder pattern for tmux command testability

### Secondary (MEDIUM confidence)
- [SQLite connection pool article](https://emschwartz.me/psa-your-sqlite-connection-pool-might-be-ruining-your-write-performance/) — dual-pool / connect-per-refresh recommendation, verified against SQLite WAL docs
- [ratatui async event stream tutorial](https://ratatui.rs/tutorials/counter-async-app/async-event-stream/) — event loop patterns (used synchronous variant for simplicity)
- [tmux split-window guide](https://gist.github.com/sdondley/b01cc5bb1169c8c83401e438a652b84e) — split-window flags and chaining

### Tertiary (LOW confidence)
- WebSearch result on `tmux attach-session` in new pane pattern — community practice, not formally documented; should be verified empirically

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml, APIs verified via official docs
- Architecture: HIGH — patterns derived from existing codebase + official ratatui docs
- Pitfalls: HIGH — WAL checkpoint starvation verified via SQLite docs + sqlx docs; terminal restore is well-known ratatui gotcha
- DB polling strategy: HIGH — verified via SQLite WAL docs + sqlx SqliteConnectOptions docs
- Tmux pane strategy: MEDIUM — attach-session approach is community practice; exact command syntax needs empirical test

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable ecosystem — ratatui 0.26 API won't change; SQLite WAL behavior is permanent)
