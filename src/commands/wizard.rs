// wizard.rs — TUI wizard for interactive squad.yml configuration
// Multi-page ratatui form that collects project name, agent count, and per-agent
// configuration (role, tool, model, description) with inline validation.

use crossterm::{
    event::{self, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

// ----------------------------------------------------------------------------
// Public API types
// ----------------------------------------------------------------------------

pub struct WizardResult {
    pub project: String,
    pub agents: Vec<AgentInput>,
}

pub struct AgentInput {
    pub role: String,
    pub tool: String, // "claude-code" | "gemini-cli" | "antigravity"
    pub model: Option<String>,
    pub description: Option<String>,
}

// ----------------------------------------------------------------------------
// Tool enum
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    ClaudeCode,
    GeminiCli,
    Antigravity,
}

impl Tool {
    pub fn cycle_next(self) -> Self {
        match self {
            Tool::ClaudeCode => Tool::GeminiCli,
            Tool::GeminiCli => Tool::Antigravity,
            Tool::Antigravity => Tool::ClaudeCode,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Tool::ClaudeCode => "claude-code",
            Tool::GeminiCli => "gemini-cli",
            Tool::Antigravity => "antigravity",
        }
    }
}

// ----------------------------------------------------------------------------
// TextInputState
// ----------------------------------------------------------------------------

pub struct TextInputState {
    pub value: String,
    pub error: Option<String>,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            error: None,
        }
    }

    pub fn push(&mut self, c: char) {
        self.value.push(c);
        self.error = None;
    }

    pub fn pop(&mut self) {
        self.value.pop();
        self.error = None;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// AgentDraft
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum AgentField {
    Role,
    Tool,
    Model,
    Description,
}

pub struct AgentDraft {
    pub role: TextInputState,
    pub tool: Tool,
    pub model: TextInputState,
    pub description: TextInputState,
    pub focused_field: AgentField,
}

impl AgentDraft {
    pub fn new() -> Self {
        Self {
            role: TextInputState::new(),
            tool: Tool::ClaudeCode,
            model: TextInputState::new(),
            description: TextInputState::new(),
            focused_field: AgentField::Role,
        }
    }
}

impl Default for AgentDraft {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// WizardPage and WizardState
// ----------------------------------------------------------------------------

enum WizardPage {
    ProjectName,
    AgentCount,
    AgentConfig { index: usize },
    Summary,
}

struct WizardState {
    page: WizardPage,
    project_input: TextInputState,
    count_input: TextInputState,
    agent_count: usize,
    agents: Vec<AgentDraft>,
}

impl WizardState {
    fn new() -> Self {
        Self {
            page: WizardPage::ProjectName,
            project_input: TextInputState::new(),
            count_input: TextInputState::new(),
            agent_count: 0,
            agents: Vec::new(),
        }
    }

    fn into_result(self) -> WizardResult {
        WizardResult {
            project: self.project_input.value.trim().to_string(),
            agents: self
                .agents
                .into_iter()
                .map(|d| AgentInput {
                    role: d.role.value.trim().to_string(),
                    tool: d.tool.as_str().to_string(),
                    model: if d.model.value.trim().is_empty() {
                        None
                    } else {
                        Some(d.model.value.trim().to_string())
                    },
                    description: if d.description.value.trim().is_empty() {
                        None
                    } else {
                        Some(d.description.value.trim().to_string())
                    },
                })
                .collect(),
        }
    }

    /// Current step number (1-based) for the progress header
    fn current_step(&self) -> usize {
        match &self.page {
            WizardPage::ProjectName => 1,
            WizardPage::AgentCount => 2,
            WizardPage::AgentConfig { index } => 3 + index,
            WizardPage::Summary => 3 + self.agent_count,
        }
    }

    /// Total number of steps: ProjectName + AgentCount + agent_count + Summary
    fn total_steps(&self) -> usize {
        // Before agent count is known, assume at least 1 agent
        let count = if self.agent_count > 0 { self.agent_count } else { 1 };
        2 + count + 1
    }

    fn page_title(&self) -> String {
        match &self.page {
            WizardPage::ProjectName => "Project Name".to_string(),
            WizardPage::AgentCount => "Agent Count".to_string(),
            WizardPage::AgentConfig { index } => format!("Agent {} Configuration", index + 1),
            WizardPage::Summary => "Review".to_string(),
        }
    }
}

// ----------------------------------------------------------------------------
// Validation functions
// ----------------------------------------------------------------------------

fn validate_project_name(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Project name is required".to_string())
    } else {
        Ok(())
    }
}

fn validate_count(input: &str) -> Result<usize, String> {
    input
        .trim()
        .parse::<usize>()
        .map_err(|_| "Please enter a whole number (e.g. 2)".to_string())
        .and_then(|n| {
            if n >= 1 {
                Ok(n)
            } else {
                Err("Agent count must be at least 1".to_string())
            }
        })
}

fn validate_role(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Role is required".to_string())
    } else {
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// Terminal setup / teardown
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Key action
// ----------------------------------------------------------------------------

enum KeyAction {
    Continue,
    Complete,
}

// ----------------------------------------------------------------------------
// Event handler
// ----------------------------------------------------------------------------

fn handle_key(state: &mut WizardState, key: KeyCode) -> KeyAction {
    match &state.page {
        WizardPage::ProjectName => match key {
            KeyCode::Enter => {
                let val = state.project_input.value.clone();
                match validate_project_name(&val) {
                    Ok(()) => state.page = WizardPage::AgentCount,
                    Err(msg) => state.project_input.error = Some(msg),
                }
            }
            KeyCode::Esc => {} // first page — no-op
            KeyCode::Backspace => state.project_input.pop(),
            KeyCode::Char(c) => state.project_input.push(c),
            _ => {}
        },
        WizardPage::AgentCount => match key {
            KeyCode::Enter => {
                let val = state.count_input.value.clone();
                match validate_count(&val) {
                    Ok(n) => {
                        state.agent_count = n;
                        // Pre-allocate agent drafts (preserving any existing drafts)
                        while state.agents.len() < n {
                            state.agents.push(AgentDraft::new());
                        }
                        state.page = WizardPage::AgentConfig { index: 0 };
                    }
                    Err(msg) => state.count_input.error = Some(msg),
                }
            }
            KeyCode::Esc => state.page = WizardPage::ProjectName,
            KeyCode::Backspace => state.count_input.pop(),
            KeyCode::Char(c) => state.count_input.push(c),
            _ => {}
        },
        WizardPage::AgentConfig { index } => {
            let index = *index;
            let focused = state.agents[index].focused_field;
            match focused {
                AgentField::Role => match key {
                    KeyCode::Enter => {
                        let val = state.agents[index].role.value.clone();
                        match validate_role(&val) {
                            Ok(()) => state.agents[index].focused_field = AgentField::Tool,
                            Err(msg) => state.agents[index].role.error = Some(msg),
                        }
                    }
                    KeyCode::Backspace => state.agents[index].role.pop(),
                    KeyCode::Char(c) => state.agents[index].role.push(c),
                    KeyCode::Esc => {
                        if index == 0 {
                            state.page = WizardPage::AgentCount;
                        } else {
                            state.agents[index - 1].focused_field = AgentField::Description;
                            state.page = WizardPage::AgentConfig { index: index - 1 };
                        }
                    }
                    _ => {}
                },
                AgentField::Tool => match key {
                    KeyCode::Enter => {
                        state.agents[index].focused_field = AgentField::Model;
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                        state.agents[index].tool = state.agents[index].tool.cycle_next();
                    }
                    KeyCode::Esc => {
                        state.agents[index].focused_field = AgentField::Role;
                    }
                    _ => {}
                },
                AgentField::Model => match key {
                    KeyCode::Enter => {
                        state.agents[index].focused_field = AgentField::Description;
                    }
                    KeyCode::Backspace => state.agents[index].model.pop(),
                    KeyCode::Char(c) => state.agents[index].model.push(c),
                    KeyCode::Esc => {
                        state.agents[index].focused_field = AgentField::Tool;
                    }
                    _ => {}
                },
                AgentField::Description => match key {
                    KeyCode::Enter => {
                        if index == state.agent_count - 1 {
                            state.page = WizardPage::Summary;
                        } else {
                            state.agents[index + 1].focused_field = AgentField::Role;
                            state.page = WizardPage::AgentConfig { index: index + 1 };
                        }
                    }
                    KeyCode::Backspace => state.agents[index].description.pop(),
                    KeyCode::Char(c) => state.agents[index].description.push(c),
                    KeyCode::Esc => {
                        state.agents[index].focused_field = AgentField::Model;
                    }
                    _ => {}
                },
            }
        }
        WizardPage::Summary => match key {
            KeyCode::Enter => return KeyAction::Complete,
            KeyCode::Esc => {
                let last = state.agent_count - 1;
                state.agents[last].focused_field = AgentField::Description;
                state.page = WizardPage::AgentConfig { index: last };
            }
            _ => {}
        },
    }
    KeyAction::Continue
}

// ----------------------------------------------------------------------------
// Rendering
// ----------------------------------------------------------------------------

fn render_page(frame: &mut Frame, state: &WizardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(10),   // content
            Constraint::Length(2), // footer
        ])
        .split(frame.size());

    // Header
    let step = state.current_step();
    let total = state.total_steps();
    let title_text = format!("Step {} of {} -- {}", step, total, state.page_title());
    let header = Paragraph::new(Line::from(vec![Span::styled(
        title_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Content — varies by page
    match &state.page {
        WizardPage::ProjectName => render_text_field(
            frame,
            chunks[1],
            " Project Name ",
            &state.project_input,
            true,
        ),
        WizardPage::AgentCount => render_text_field(
            frame,
            chunks[1],
            " How Many Agents? ",
            &state.count_input,
            true,
        ),
        WizardPage::AgentConfig { index } => {
            render_agent_page(frame, chunks[1], &state.agents[*index]);
        }
        WizardPage::Summary => {
            render_summary_page(frame, chunks[1], state);
        }
    }

    // Footer
    let footer_text = match &state.page {
        WizardPage::ProjectName => "Enter: next   Ctrl+C: cancel",
        WizardPage::AgentCount => "Enter: next   Esc: back   Ctrl+C: cancel",
        WizardPage::AgentConfig { index } => {
            let focused = state.agents[*index].focused_field;
            if focused == AgentField::Tool {
                "Left/Right/Tab: cycle tool   Enter: next   Esc: back   Ctrl+C: cancel"
            } else {
                "Enter: next   Esc: back   Ctrl+C: cancel"
            }
        }
        WizardPage::Summary => "Enter: confirm   Esc: back   Ctrl+C: cancel",
    };
    let footer = Paragraph::new(footer_text);
    frame.render_widget(footer, chunks[2]);
}

/// Render a single text input field with optional error line below.
fn render_text_field(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    input: &TextInputState,
    focused: bool,
) {
    let field_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // field
            Constraint::Length(1), // error
            Constraint::Min(0),    // spacer
        ])
        .split(area);

    let border_color = if input.error.is_some() {
        Color::Red
    } else if focused {
        Color::Cyan
    } else {
        Color::Reset
    };

    let display_value = format!("{}|", input.value);
    let field_widget = Paragraph::new(display_value).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title),
    );
    frame.render_widget(field_widget, field_chunks[0]);

    if let Some(err) = &input.error {
        let error_widget = Paragraph::new(format!("  {}", err))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error_widget, field_chunks[1]);
    }
}

/// Render the 4-field agent configuration page.
fn render_agent_page(frame: &mut Frame, area: ratatui::layout::Rect, draft: &AgentDraft) {
    // 4 fields, each followed by an error/hint line: [field(3), hint(1)] x4 + spacer
    let agent_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Role field
            Constraint::Length(1), // Role error
            Constraint::Length(3), // Tool field
            Constraint::Length(1), // Tool hint
            Constraint::Length(3), // Model field
            Constraint::Length(1), // Model hint
            Constraint::Length(3), // Description field
            Constraint::Length(1), // Description hint
            Constraint::Min(0),    // spacer
        ])
        .split(area);

    // --- Role field ---
    let role_focused = draft.focused_field == AgentField::Role;
    let role_color = if draft.role.error.is_some() {
        Color::Red
    } else if role_focused {
        Color::Cyan
    } else {
        Color::Reset
    };
    let role_value = if role_focused {
        format!("{}|", draft.role.value)
    } else {
        draft.role.value.clone()
    };
    let role_widget = Paragraph::new(role_value).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(role_color))
            .title(" Role "),
    );
    frame.render_widget(role_widget, agent_chunks[0]);

    if let Some(err) = &draft.role.error {
        let error_widget = Paragraph::new(format!("  {}", err))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error_widget, agent_chunks[1]);
    }

    // --- Tool selector ---
    let tool_focused = draft.focused_field == AgentField::Tool;
    let tool_color = if tool_focused { Color::Cyan } else { Color::Reset };
    let tool_display = format!("[ {} ]", draft.tool.as_str());
    let tool_widget = Paragraph::new(Line::from(vec![Span::raw(tool_display)])).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(tool_color))
            .title(" Tool "),
    );
    frame.render_widget(tool_widget, agent_chunks[2]);
    // No hint/error for tool row (always valid)

    // --- Model field ---
    let model_focused = draft.focused_field == AgentField::Model;
    let model_color = if model_focused { Color::Cyan } else { Color::Reset };
    let model_value = if model_focused {
        format!("{}|", draft.model.value)
    } else {
        draft.model.value.clone()
    };
    let model_widget = Paragraph::new(model_value).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(model_color))
            .title(" Model "),
    );
    frame.render_widget(model_widget, agent_chunks[4]);

    let model_hint = Paragraph::new("optional -- press Enter to skip")
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(model_hint, agent_chunks[5]);

    // --- Description field ---
    let desc_focused = draft.focused_field == AgentField::Description;
    let desc_color = if desc_focused { Color::Cyan } else { Color::Reset };
    let desc_value = if desc_focused {
        format!("{}|", draft.description.value)
    } else {
        draft.description.value.clone()
    };
    let desc_widget = Paragraph::new(desc_value).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(desc_color))
            .title(" Description "),
    );
    frame.render_widget(desc_widget, agent_chunks[6]);

    let desc_hint = Paragraph::new("optional -- press Enter to skip")
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(desc_hint, agent_chunks[7]);
}

/// Render the summary page.
fn render_summary_page(frame: &mut Frame, area: ratatui::layout::Rect, state: &WizardState) {
    let summary_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // agent list block
            Constraint::Length(1), // confirm hint
        ])
        .split(area);

    let project_line = format!("Project: {}", state.project_input.value.trim());
    let items: Vec<ListItem> = std::iter::once(ListItem::new(project_line))
        .chain(state.agents.iter().enumerate().map(|(i, d)| {
            let model_str = if d.model.value.trim().is_empty() {
                "-".to_string()
            } else {
                d.model.value.trim().to_string()
            };
            let desc_str = if d.description.value.trim().is_empty() {
                "-".to_string()
            } else {
                d.description.value.trim().to_string()
            };
            ListItem::new(format!(
                "Agent {}: role={}, tool={}, model={}, desc={}",
                i + 1,
                d.role.value.trim(),
                d.tool.as_str(),
                model_str,
                desc_str
            ))
        }))
        .collect();

    let list_widget = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Review "),
    );
    frame.render_widget(list_widget, summary_chunks[0]);

    let confirm_hint = Paragraph::new("Press Enter to confirm, Esc to go back")
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(confirm_hint, summary_chunks[1]);
}

// ----------------------------------------------------------------------------
// Public run function
// ----------------------------------------------------------------------------

pub async fn run() -> anyhow::Result<Option<WizardResult>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let mut state = WizardState::new();

    loop {
        terminal.draw(|frame| render_page(frame, &state))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Ctrl+C: cancel wizard
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        restore_terminal(&mut terminal)?;
                        return Ok(None);
                    }

                    match handle_key(&mut state, key.code) {
                        KeyAction::Continue => {}
                        KeyAction::Complete => {
                            restore_terminal(&mut terminal)?;
                            return Ok(Some(state.into_result()));
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Unit tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_push() {
        let mut s = TextInputState::new();
        assert_eq!(s.value, "");
        s.push('a');
        s.push('b');
        assert_eq!(s.value, "ab");
    }

    #[test]
    fn test_text_input_pop() {
        let mut s = TextInputState::new();
        s.push('a');
        s.push('b');
        s.pop();
        assert_eq!(s.value, "a");
        s.pop();
        assert_eq!(s.value, "");
        // pop on empty should not panic
        s.pop();
        assert_eq!(s.value, "");
    }

    #[test]
    fn test_text_input_error() {
        let mut s = TextInputState::new();
        s.error = Some("err".to_string());
        s.clear_error();
        assert_eq!(s.error, None);
    }

    #[test]
    fn test_validate_count_valid() {
        assert_eq!(validate_count("3"), Ok(3));
        assert_eq!(validate_count("1"), Ok(1));
    }

    #[test]
    fn test_validate_count_invalid() {
        let err = validate_count("0").unwrap_err();
        assert!(
            err.contains("at least 1"),
            "Expected 'at least 1' in: {}",
            err
        );

        let err = validate_count("abc").unwrap_err();
        assert!(
            err.contains("whole number"),
            "Expected 'whole number' in: {}",
            err
        );

        let err = validate_count("").unwrap_err();
        assert!(
            err.contains("whole number"),
            "Expected 'whole number' in: {}",
            err
        );
    }

    #[test]
    fn test_validate_role_valid() {
        assert!(validate_role("backend").is_ok());
        assert!(validate_role("  lead  ").is_ok());
    }

    #[test]
    fn test_validate_role_empty() {
        let err = validate_role("").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );

        let err = validate_role("   ").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );
    }

    #[test]
    fn test_tool_cycle() {
        assert_eq!(Tool::ClaudeCode.cycle_next(), Tool::GeminiCli);
        assert_eq!(Tool::GeminiCli.cycle_next(), Tool::Antigravity);
        assert_eq!(Tool::Antigravity.cycle_next(), Tool::ClaudeCode);
    }

    #[test]
    fn test_tool_as_str() {
        assert_eq!(Tool::ClaudeCode.as_str(), "claude-code");
        assert_eq!(Tool::GeminiCli.as_str(), "gemini-cli");
        assert_eq!(Tool::Antigravity.as_str(), "antigravity");
    }

    #[test]
    fn test_tool_display_matches_valid_providers() {
        let valid_providers = &["antigravity", "claude-code", "gemini-cli"];
        assert!(
            valid_providers.contains(&Tool::ClaudeCode.as_str()),
            "ClaudeCode.as_str() not in VALID_PROVIDERS"
        );
        assert!(
            valid_providers.contains(&Tool::GeminiCli.as_str()),
            "GeminiCli.as_str() not in VALID_PROVIDERS"
        );
        assert!(
            valid_providers.contains(&Tool::Antigravity.as_str()),
            "Antigravity.as_str() not in VALID_PROVIDERS"
        );
    }

    #[test]
    fn test_validate_project_name() {
        let err = validate_project_name("").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );

        assert!(validate_project_name("my-project").is_ok());
    }
}
