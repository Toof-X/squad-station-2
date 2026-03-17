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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::{config, db};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FocusPanel {
    AgentPanel,
    MessagePanel,
}

pub struct App {
    pub agents: Vec<db::agents::Agent>,
    pub messages: Vec<db::messages::Message>,
    pub agent_list_state: ListState,
    pub focus: FocusPanel,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            agents: vec![],
            messages: vec![],
            agent_list_state: ListState::default(),
            focus: FocusPanel::AgentPanel,
            quit: false,
        }
    }

    pub fn select_next(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let len = self.agents.len();
        let next = match self.agent_list_state.selected() {
            None => 0,
            Some(i) => (i + 1) % len,
        };
        self.agent_list_state.select(Some(next));
    }

    pub fn select_previous(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let len = self.agents.len();
        let prev = match self.agent_list_state.selected() {
            None => len.saturating_sub(1),
            Some(0) => len - 1,
            Some(i) => i - 1,
        };
        self.agent_list_state.select(Some(prev));
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPanel::AgentPanel => FocusPanel::MessagePanel,
            FocusPanel::MessagePanel => FocusPanel::AgentPanel,
        };
    }

    pub fn selected_agent_name(&self) -> Option<&str> {
        self.agent_list_state
            .selected()
            .and_then(|i| self.agents.get(i))
            .map(|a| a.name.as_str())
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.select_next(),
            KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Home => {
                if !self.agents.is_empty() {
                    self.agent_list_state.select(Some(0));
                }
            }
            KeyCode::End => {
                if !self.agents.is_empty() {
                    self.agent_list_state.select(Some(self.agents.len() - 1));
                }
            }
            _ => {}
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Data fetching (connect-per-refresh — WAL checkpoint starvation prevention)
// ---------------------------------------------------------------------------

async fn fetch_snapshot(
    db_path: &std::path::Path,
    selected_agent: Option<&str>,
) -> anyhow::Result<(Vec<db::agents::Agent>, Vec<db::messages::Message>)> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await?;

    let agents = db::agents::list_agents(&pool).await?;
    let messages = if let Some(agent) = selected_agent {
        db::messages::list_messages(&pool, Some(agent), None, 50).await?
    } else {
        vec![]
    };
    // pool drops here — WAL reader lock released
    drop(pool);
    Ok((agents, messages))
}

// ---------------------------------------------------------------------------
// Terminal setup / teardown
// ---------------------------------------------------------------------------

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout())).map_err(Into::into)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn status_color(status: &str) -> Color {
    match status {
        "idle" => Color::Green,
        "busy" => Color::Yellow,
        _ => Color::Red, // dead or unknown
    }
}

fn draw_ui(frame: &mut Frame, app: &mut App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(frame.area());

    // --- Title bar ---
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " SQUAD-STATION ",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, outer[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    // --- Left panel: agent list ---
    let agent_panel_focused = app.focus == FocusPanel::AgentPanel;
    let agent_border_style = if agent_panel_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|a| {
            let color = status_color(&a.status);
            let line = Line::from(vec![
                Span::raw(format!("{}: ", a.name)),
                Span::styled(a.status.clone(), Style::default().fg(color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let agent_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(agent_border_style)
                .title(" Agents [Tab] "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    frame.render_stateful_widget(agent_list, chunks[0], &mut app.agent_list_state);

    // --- Right panel: messages ---
    let msg_panel_focused = app.focus == FocusPanel::MessagePanel;
    let msg_border_style = if msg_panel_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let title = match app.selected_agent_name() {
        Some(name) => format!(" Messages: {} ", name),
        None => " Messages — Select an agent ".to_string(),
    };

    let msg_text: String = if app.agents.is_empty() {
        "No agents registered.".to_string()
    } else if app.agent_list_state.selected().is_none() {
        "Select an agent to view messages".to_string()
    } else if app.messages.is_empty() {
        match app.selected_agent_name() {
            Some(name) => format!("No messages for {}", name),
            None => "Select an agent to view messages".to_string(),
        }
    } else {
        app.messages
            .iter()
            .map(|m| {
                format!(
                    "[{}] ({}) {}\n  {}\n",
                    m.status, m.priority, m.task, m.updated_at
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let msg_widget = Paragraph::new(msg_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(msg_border_style)
                .title(title),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(msg_widget, chunks[1]);
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

pub async fn run() -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let refresh_interval = std::time::Duration::from_secs(3);
    let mut last_refresh = tokio::time::Instant::now()
        .checked_sub(refresh_interval)
        .unwrap_or_else(tokio::time::Instant::now);

    loop {
        // Refresh on interval
        if last_refresh.elapsed() >= refresh_interval {
            let selected = app.selected_agent_name().map(String::from);
            match fetch_snapshot(&db_path, selected.as_deref()).await {
                Ok((agents, messages)) => {
                    app.agents = agents;
                    app.messages = messages;
                    // Auto-select first agent if none selected and agents exist
                    if app.agent_list_state.selected().is_none() && !app.agents.is_empty() {
                        app.agent_list_state.select(Some(0));
                    }
                }
                Err(_) => {
                    // Keep stale data on refresh error — TUI continues running
                }
            }
            last_refresh = tokio::time::Instant::now();
        }

        terminal.draw(|f| draw_ui(f, &mut app))?;

        // Poll for key events (250ms timeout for responsive refresh)
        if event::poll(std::time::Duration::from_millis(250))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }

        if app.quit {
            break;
        }
    }

    restore_terminal(&mut terminal)?;
    // Restore default panic hook
    let _ = std::panic::take_hook();
    Ok(())
}
