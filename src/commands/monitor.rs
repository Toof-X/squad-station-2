use crossterm::{
    event::{self, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{config, db, tmux};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct MonitorApp {
    agents: Vec<AgentPane>,
    list_state: ListState,
    quit: bool,
}

struct AgentPane {
    name: String,
    #[allow(dead_code)]
    role: String,
    status: String,
    output: String, // captured tmux pane content
    alive: bool,
}

impl MonitorApp {
    fn new() -> Self {
        Self {
            agents: Vec::new(),
            list_state: ListState::default(),
            quit: false,
        }
    }

    fn select_next(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let len = self.agents.len();
        let next = match self.list_state.selected() {
            None => 0,
            Some(i) => (i + 1) % len,
        };
        self.list_state.select(Some(next));
    }

    fn select_prev(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let len = self.agents.len();
        let prev = match self.list_state.selected() {
            None => len.saturating_sub(1),
            Some(0) => len - 1,
            Some(i) => i - 1,
        };
        self.list_state.select(Some(prev));
    }

    fn selected_agent(&self) -> Option<&AgentPane> {
        self.list_state.selected().and_then(|i| self.agents.get(i))
    }
}

// ---------------------------------------------------------------------------
// tmux capture
// ---------------------------------------------------------------------------

fn capture_pane(session_name: &str, lines: u16) -> String {
    let output = std::process::Command::new("tmux")
        .args([
            "capture-pane",
            "-t",
            session_name,
            "-p",       // print to stdout
            "-S",
            &format!("-{}", lines), // last N lines
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).to_string()
        }
        _ => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Data refresh
// ---------------------------------------------------------------------------

async fn refresh_agents(app: &mut MonitorApp, pane_lines: u16) -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    let agents = db::agents::list_agents(&pool).await?;
    let live_sessions = tmux::list_live_session_names().await;

    app.agents = agents
        .iter()
        .filter(|a| a.role != "orchestrator")
        .map(|a| {
            let alive = live_sessions.contains(&a.name);
            let output = if alive {
                capture_pane(&a.name, pane_lines)
            } else {
                "(session not running)".to_string()
            };
            AgentPane {
                name: a.name.clone(),
                role: a.role.clone(),
                status: a.status.clone(),
                output,
                alive,
            }
        })
        .collect();

    // Preserve selection
    if let Some(sel) = app.list_state.selected() {
        if sel >= app.agents.len() {
            app.list_state
                .select(Some(app.agents.len().saturating_sub(1)));
        }
    } else if !app.agents.is_empty() {
        app.list_state.select(Some(0));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(frame: &mut Frame, app: &mut MonitorApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(40)])
        .split(frame.area());

    // Left: agent list
    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|a| {
            let status_color = match a.status.as_str() {
                "busy" => Color::Yellow,
                "idle" => Color::Green,
                "dead" => Color::Red,
                "frozen" => Color::Blue,
                _ => Color::DarkGray,
            };
            let indicator = if a.alive { "●" } else { "○" };
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", indicator),
                    Style::default().fg(status_color),
                ),
                Span::raw(&a.name),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Agents ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // Right: selected agent pane output
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(chunks[1]);

    if let Some(agent) = app.selected_agent() {
        // Header bar
        let status_str = if agent.alive {
            format!(" {} │ {} ", agent.name, agent.status)
        } else {
            format!(" {} │ dead (no session) ", agent.name)
        };
        let header = Paragraph::new(status_str).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Agent "),
        );
        frame.render_widget(header, right_chunks[0]);

        // Output pane — show last lines that fit the viewport
        let pane_height = right_chunks[1].height.saturating_sub(2) as usize; // minus border
        let lines: Vec<&str> = agent.output.lines().collect();
        let visible_start = lines.len().saturating_sub(pane_height);
        let visible: String = lines[visible_start..].join("\n");

        let output = Paragraph::new(visible)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Output "),
            );
        frame.render_widget(output, right_chunks[1]);
    } else {
        let empty = Paragraph::new("  No agent selected").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Output "),
        );
        frame.render_widget(empty, chunks[1]);
    }

    // Footer hint (bottom line overlay)
    let footer_area = ratatui::layout::Rect {
        x: frame.area().x,
        y: frame.area().y + frame.area().height.saturating_sub(1),
        width: frame.area().width,
        height: 1,
    };
    let footer = Paragraph::new(Span::styled(
        " ↑↓: select agent   r: refresh   q/Esc: quit ",
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(footer, footer_area);
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

pub async fn run() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = MonitorApp::new();

    // Initial load
    let term_height = terminal.size()?.height;
    let pane_lines = term_height.saturating_sub(6); // approximate visible lines
    refresh_agents(&mut app, pane_lines).await?;

    let refresh_interval = std::time::Duration::from_secs(3);
    let mut last_refresh = std::time::Instant::now();

    loop {
        terminal.draw(|f| render(f, &mut app))?;

        // Auto-refresh every 3 seconds
        if last_refresh.elapsed() >= refresh_interval {
            let term_height = terminal.size()?.height;
            let pane_lines = term_height.saturating_sub(6);
            let _ = refresh_agents(&mut app, pane_lines).await;
            last_refresh = std::time::Instant::now();
        }

        // Poll for input with timeout
        if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.quit = true;
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Char('r') => {
                        let term_height = terminal.size()?.height;
                        let pane_lines = term_height.saturating_sub(6);
                        let _ = refresh_agents(&mut app, pane_lines).await;
                        last_refresh = std::time::Instant::now();
                    }
                    _ => {}
                }
            }
        }

        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
