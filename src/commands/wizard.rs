// wizard.rs — TUI wizard for interactive squad.yml configuration
// Multi-page ratatui form that collects project name, agent count, and per-agent
// configuration (name, role, provider, model, description) with inline validation.

use crate::commands::templates;
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
    pub install_dir: String,
    pub project: String,
    pub sdd: SddWorkflow,
    pub orchestrator: AgentInput,
    pub agents: Vec<AgentInput>, // workers
}

// ----------------------------------------------------------------------------
// SddWorkflow enum
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SddWorkflow {
    Bmad,
    GetShitDone,
    Superpower,
}

impl SddWorkflow {
    pub fn cycle_next(self) -> Self {
        match self {
            SddWorkflow::Bmad => SddWorkflow::GetShitDone,
            SddWorkflow::GetShitDone => SddWorkflow::Superpower,
            SddWorkflow::Superpower => SddWorkflow::Bmad,
        }
    }

    pub fn cycle_prev(self) -> Self {
        match self {
            SddWorkflow::Bmad => SddWorkflow::Superpower,
            SddWorkflow::GetShitDone => SddWorkflow::Bmad,
            SddWorkflow::Superpower => SddWorkflow::GetShitDone,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SddWorkflow::Bmad => "bmad",
            SddWorkflow::GetShitDone => "gsd",
            SddWorkflow::Superpower => "superpower",
        }
    }

    /// Returns the embedded playbook content for this workflow.
    pub fn playbook_content(self) -> &'static str {
        match self {
            SddWorkflow::Bmad => include_str!("../../npm-package/.squad/sdd/bmad-playbook.md"),
            SddWorkflow::GetShitDone => {
                include_str!("../../npm-package/.squad/sdd/gsd-playbook.md")
            }
            SddWorkflow::Superpower => {
                include_str!("../../npm-package/.squad/sdd/superpowers-playbook.md")
            }
        }
    }

    pub fn index(self) -> usize {
        match self {
            SddWorkflow::Bmad => 0,
            SddWorkflow::GetShitDone => 1,
            SddWorkflow::Superpower => 2,
        }
    }

    pub const ALL: [&'static str; 3] = ["bmad", "get-shit-done", "superpower"];
}

pub struct AgentInput {
    pub name: String,
    pub role: String,     // "orchestrator" | "worker"
    pub provider: String, // "claude-code" | "gemini-cli" | "antigravity"
    pub model: Option<String>,
    pub description: Option<String>,
    pub routing_hints: Option<String>, // Phase 24: JSON-serialized routing keywords
}

// ----------------------------------------------------------------------------
// Provider enum
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Provider {
    ClaudeCode,
    GeminiCli,
    Antigravity,
}

impl Provider {
    pub fn cycle_next(self) -> Self {
        match self {
            Provider::ClaudeCode => Provider::GeminiCli,
            Provider::GeminiCli => Provider::Antigravity,
            Provider::Antigravity => Provider::ClaudeCode,
        }
    }

    pub fn cycle_prev(self) -> Self {
        match self {
            Provider::ClaudeCode => Provider::Antigravity,
            Provider::GeminiCli => Provider::ClaudeCode,
            Provider::Antigravity => Provider::GeminiCli,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::ClaudeCode => "claude-code",
            Provider::GeminiCli => "gemini-cli",
            Provider::Antigravity => "antigravity",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Provider::ClaudeCode => 0,
            Provider::GeminiCli => 1,
            Provider::Antigravity => 2,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            0 => Provider::ClaudeCode,
            1 => Provider::GeminiCli,
            _ => Provider::Antigravity,
        }
    }

    pub const ALL: [&'static str; 3] = ["claude-code", "gemini-cli", "antigravity"];
}

// ----------------------------------------------------------------------------
// Role enum
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    Orchestrator,
    Worker,
}

impl Role {
    pub fn cycle_next(self) -> Self {
        match self {
            Role::Orchestrator => Role::Worker,
            Role::Worker => Role::Orchestrator,
        }
    }

    pub fn cycle_prev(self) -> Self {
        match self {
            Role::Orchestrator => Role::Worker,
            Role::Worker => Role::Orchestrator,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Orchestrator => "orchestrator",
            Role::Worker => "worker",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Role::Orchestrator => 0,
            Role::Worker => 1,
        }
    }

    pub const ALL: [&'static str; 2] = ["orchestrator", "worker"];
}

// ----------------------------------------------------------------------------
// ModelSelector — per-provider enum with Left/Right cycling
// ----------------------------------------------------------------------------

pub struct ModelSelector {
    pub index: usize,
}

impl ModelSelector {
    pub fn new() -> Self {
        Self { index: 0 }
    }

    pub fn options_for(provider: Provider) -> &'static [&'static str] {
        match provider {
            Provider::ClaudeCode => &[
                "sonnet",
                "opus",
                "haiku",
                "other",
            ],
            Provider::GeminiCli => &[
                "gemini-3.1-pro-preview",
                "gemini-3-flash-preview",
                "gemini-2.5-pro",
                "gemini-2.5-flash",
                "gemini-2.5-flash-lite",
                "other",
            ],
            Provider::Antigravity => &[],
        }
    }

    pub fn is_other(&self, provider: Provider) -> bool {
        self.current(provider) == Some("other")
    }

    pub fn current(&self, provider: Provider) -> Option<&'static str> {
        Self::options_for(provider).get(self.index).copied()
    }

    pub fn cycle_next(&mut self, provider: Provider) {
        let opts = Self::options_for(provider);
        if !opts.is_empty() {
            self.index = (self.index + 1) % opts.len();
        }
    }

    pub fn cycle_prev(&mut self, provider: Provider) {
        let opts = Self::options_for(provider);
        if !opts.is_empty() {
            self.index = if self.index == 0 {
                opts.len() - 1
            } else {
                self.index - 1
            };
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// TextInputState — with cursor position support
// ----------------------------------------------------------------------------

pub struct TextInputState {
    pub value: String,
    pub cursor: usize, // char index (not byte)
    pub error: Option<String>,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            error: None,
        }
    }

    pub fn with_value(value: String) -> Self {
        let cursor = value.chars().count();
        Self {
            value,
            cursor,
            error: None,
        }
    }

    fn char_to_byte(&self, char_pos: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_pos)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    /// Insert character at cursor position and advance cursor.
    pub fn push(&mut self, c: char) {
        let byte_pos = self.char_to_byte(self.cursor);
        self.value.insert(byte_pos, c);
        self.cursor += 1;
        self.error = None;
    }

    /// Delete character before cursor (backspace behaviour).
    pub fn pop(&mut self) {
        if self.cursor > 0 {
            let byte_pos = self.char_to_byte(self.cursor - 1);
            self.value.remove(byte_pos);
            self.cursor -= 1;
        }
        self.error = None;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor < self.char_count() {
            self.cursor += 1;
        }
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Render the value with a `|` cursor marker at the current position.
    /// When `active` is false, returns the raw value without cursor.
    pub fn display(&self, active: bool) -> String {
        if !active {
            return self.value.clone();
        }
        let chars: Vec<char> = self.value.chars().collect();
        let mut s = String::with_capacity(self.value.len() + 1);
        for (i, c) in chars.iter().enumerate() {
            if i == self.cursor {
                s.push('|');
            }
            s.push(*c);
        }
        if self.cursor >= chars.len() {
            s.push('|');
        }
        s
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
    Name,
    Template, // NEW — between Name and Provider
    Provider,
    Model,
    Description,
}

pub struct AgentDraft {
    pub name: TextInputState,
    pub template_index: usize,                          // NEW — index into template list (last = Custom)
    pub is_orchestrator: bool,                          // NEW — selects which template list to use
    pub provider: Provider,
    pub model: ModelSelector,
    pub custom_model: TextInputState,                   // used when model is "other"
    pub description: TextInputState,
    pub focused_field: AgentField,
    pub routing_hints: Option<Vec<&'static str>>,       // NEW — set by template selection
}

impl AgentDraft {
    pub fn new() -> Self {
        Self {
            name: TextInputState::new(),
            template_index: 0,
            is_orchestrator: false,
            provider: Provider::ClaudeCode,
            model: ModelSelector::new(),
            custom_model: TextInputState::new(),
            description: TextInputState::new(),
            focused_field: AgentField::Name,
            routing_hints: None,
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

#[derive(Clone, Copy, PartialEq)]
enum ProjectField {
    InstallDir,
    Name,
    Sdd,
}

enum WizardPage {
    Project,
    OrchestratorConfig,
    WorkerCount,
    WorkerConfig { index: usize },
    Summary,
}

struct WizardState {
    page: WizardPage,
    project_field: ProjectField,
    install_dir_input: TextInputState,
    project_input: TextInputState,
    sdd: SddWorkflow,
    orchestrator: AgentDraft,
    worker_count_input: TextInputState,
    worker_count: usize,
    workers: Vec<AgentDraft>,
    worker_only: bool, // true when launched via run_worker_only — skips Project + Orchestrator pages
    // Existing agents passed in during add-agents flow, shown on review page
    existing_orchestrator: Option<String>, // "name (provider)" label
    existing_workers: Vec<String>,         // "name (provider)" labels
}

impl WizardState {
    fn new() -> Self {
        let mut orchestrator = AgentDraft::new();
        orchestrator.is_orchestrator = true; // orchestrator uses ORCHESTRATOR_TEMPLATES

        // Default install_dir to current working directory
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());

        // Default project name to current directory name
        let dir_name = std::path::Path::new(&cwd)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            page: WizardPage::Project,
            project_field: ProjectField::InstallDir,
            install_dir_input: TextInputState::with_value(cwd),
            project_input: TextInputState::with_value(dir_name),
            sdd: SddWorkflow::Bmad,
            orchestrator,
            worker_count_input: TextInputState::new(),
            worker_count: 0,
            workers: Vec::new(),
            worker_only: false,
            existing_orchestrator: None,
            existing_workers: Vec::new(),
        }
    }

    fn into_result(self) -> WizardResult {
        WizardResult {
            install_dir: self.install_dir_input.value.trim().to_string(),
            project: self.project_input.value.trim().to_string(),
            sdd: self.sdd,
            orchestrator: draft_to_agent_input(self.orchestrator, "orchestrator"),
            agents: self
                .workers
                .into_iter()
                .take(self.worker_count)
                .map(|d| draft_to_agent_input(d, "worker"))
                .collect(),
        }
    }

    /// Current step number (1-based) for the progress header
    fn current_step(&self) -> usize {
        match &self.page {
            WizardPage::Project => 1,
            WizardPage::OrchestratorConfig => 2,
            WizardPage::WorkerCount => 3,
            WizardPage::WorkerConfig { index } => 4 + index,
            WizardPage::Summary => 4 + self.worker_count,
        }
    }

    /// Total steps: Project + OrchestratorConfig + WorkerCount + workers + Summary
    fn total_steps(&self) -> usize {
        let count = if self.worker_count > 0 { self.worker_count } else { 1 };
        3 + count + 1
    }

    fn page_title(&self) -> String {
        match &self.page {
            WizardPage::Project => "Project".to_string(),
            WizardPage::OrchestratorConfig => "Orchestrator Configuration".to_string(),
            WizardPage::WorkerCount => "Number of Worker Agents".to_string(),
            WizardPage::WorkerConfig { index } => format!("Worker {} Configuration", index + 1),
            WizardPage::Summary => "Review".to_string(),
        }
    }
}

fn draft_to_agent_input(d: AgentDraft, role: &str) -> AgentInput {
    let model = if d.model.is_other(d.provider) {
        let v = d.custom_model.value.trim().to_string();
        if v.is_empty() { None } else { Some(v) }
    } else {
        d.model.current(d.provider).map(|s| s.to_string())
    };
    let description = if d.description.value.trim().is_empty() {
        None
    } else {
        Some(d.description.value.trim().to_string())
    };
    let routing_hints = d.routing_hints.as_ref().map(|hints| {
        serde_json::to_string(hints).unwrap_or_default()
    });
    AgentInput {
        name: d.name.value.trim().to_string(),
        role: role.to_string(),
        provider: d.provider.as_str().to_string(),
        model,
        description,
        routing_hints,
    }
}

// ----------------------------------------------------------------------------
// Validation functions
// ----------------------------------------------------------------------------


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
    Cancel, // used by worker-only mode when Esc on first page
}

// ----------------------------------------------------------------------------
// Event handler
// ----------------------------------------------------------------------------

fn handle_key(state: &mut WizardState, key: KeyCode) -> KeyAction {
    match &state.page {
        WizardPage::Project => match state.project_field {
            ProjectField::InstallDir => match key {
                KeyCode::Enter | KeyCode::Tab => {
                    state.project_field = ProjectField::Name;
                }
                KeyCode::Esc => {} // first field, first page — no-op
                KeyCode::Backspace => state.install_dir_input.pop(),
                KeyCode::Left => state.install_dir_input.cursor_left(),
                KeyCode::Right => state.install_dir_input.cursor_right(),
                KeyCode::Char(c) => state.install_dir_input.push(c),
                _ => {}
            },
            ProjectField::Name => match key {
                KeyCode::Enter | KeyCode::Tab => {
                    state.project_field = ProjectField::Sdd;
                }
                KeyCode::Esc => state.project_field = ProjectField::InstallDir,
                KeyCode::Backspace => state.project_input.pop(),
                KeyCode::Left => state.project_input.cursor_left(),
                KeyCode::Right => state.project_input.cursor_right(),
                KeyCode::Char(c) => state.project_input.push(c),
                _ => {}
            },
            ProjectField::Sdd => match key {
                KeyCode::Enter => state.page = WizardPage::OrchestratorConfig,
                KeyCode::Esc | KeyCode::Tab => state.project_field = ProjectField::Name,
                KeyCode::Up => state.sdd = state.sdd.cycle_prev(),
                KeyCode::Down => state.sdd = state.sdd.cycle_next(),
                _ => {}
            },
        },
        WizardPage::OrchestratorConfig => {
            handle_agent_key(
                key,
                &mut state.orchestrator,
                /* on_done */ || WizardPage::WorkerCount,
                /* on_back */ || WizardPage::Project,
            )
            .apply(state);
        }
        WizardPage::WorkerCount => match key {
            KeyCode::Enter => {
                let val = state.worker_count_input.value.clone();
                match validate_count(&val) {
                    Ok(n) => {
                        state.worker_count = n;
                        while state.workers.len() < n {
                            state.workers.push(AgentDraft::new());
                        }
                        state.workers.truncate(n);
                        state.page = WizardPage::WorkerConfig { index: 0 };
                    }
                    Err(msg) => state.worker_count_input.error = Some(msg),
                }
            }
            KeyCode::Esc => {
                if state.worker_only {
                    // Worker-only mode: Esc on first page cancels the wizard
                    return KeyAction::Cancel;
                }
                state.orchestrator.focused_field = AgentField::Description;
                state.page = WizardPage::OrchestratorConfig;
            }
            KeyCode::Backspace => state.worker_count_input.pop(),
            KeyCode::Left => state.worker_count_input.cursor_left(),
            KeyCode::Right => state.worker_count_input.cursor_right(),
            KeyCode::Char(c) => state.worker_count_input.push(c),
            _ => {}
        },
        WizardPage::WorkerConfig { index } => {
            let index = *index;
            let worker_count = state.worker_count;
            let on_done = move || {
                if index == worker_count - 1 {
                    WizardPage::Summary
                } else {
                    WizardPage::WorkerConfig { index: index + 1 }
                }
            };
            let on_back = move || {
                if index == 0 {
                    WizardPage::WorkerCount
                } else {
                    WizardPage::WorkerConfig { index: index - 1 }
                }
            };
            handle_agent_key(key, &mut state.workers[index], on_done, on_back)
                .apply(state);
        }
        WizardPage::Summary => match key {
            KeyCode::Enter => return KeyAction::Complete,
            KeyCode::Esc => {
                if state.worker_count > 0 {
                    let last = state.worker_count - 1;
                    state.workers[last].focused_field = AgentField::Description;
                    state.page = WizardPage::WorkerConfig { index: last };
                } else {
                    state.page = WizardPage::WorkerCount;
                }
            }
            _ => {}
        },
    }
    KeyAction::Continue
}

/// Page transition returned by handle_agent_key.
enum PageTransition {
    Stay,
    Go(WizardPage),
}

impl PageTransition {
    fn apply(self, state: &mut WizardState) {
        if let PageTransition::Go(page) = self {
            state.page = page;
        }
    }
}

/// Shared key handler for a single AgentDraft (orchestrator or worker).
/// `on_done` is called when the user advances past Description.
/// `on_back` is called when Esc is pressed on the Name field.
fn handle_agent_key(
    key: KeyCode,
    draft: &mut AgentDraft,
    on_done: impl FnOnce() -> WizardPage,
    on_back: impl FnOnce() -> WizardPage,
) -> PageTransition {
    match draft.focused_field {
        AgentField::Name => match key {
            KeyCode::Enter | KeyCode::Tab => {
                draft.focused_field = AgentField::Template;
            }
            KeyCode::Backspace => draft.name.pop(),
            KeyCode::Left => draft.name.cursor_left(),
            KeyCode::Right => draft.name.cursor_right(),
            KeyCode::Char(c) => draft.name.push(c),
            KeyCode::Esc => return PageTransition::Go(on_back()),
            _ => {}
        },
        AgentField::Template => match key {
            KeyCode::Enter | KeyCode::Tab => {
                let tmpl_list = if draft.is_orchestrator {
                    templates::ORCHESTRATOR_TEMPLATES
                } else {
                    templates::WORKER_TEMPLATES
                };
                let custom_idx = tmpl_list.len();
                if draft.template_index < custom_idx {
                    let t = &tmpl_list[draft.template_index];
                    // Auto-fill name — always overwrite (per UI-SPEC)
                    draft.name.value = t.slug.to_string();
                    draft.name.cursor = draft.name.value.chars().count();
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
                    draft.description.value = t.description.to_string();
                    draft.description.cursor = draft.description.value.chars().count();
                    // Store routing hints
                    draft.routing_hints = Some(t.routing_hints.to_vec());
                } else {
                    // Custom — clear all fields
                    draft.name = TextInputState::new();
                    draft.provider = Provider::ClaudeCode;
                    draft.model = ModelSelector::new();
                    draft.custom_model = TextInputState::new();
                    draft.description = TextInputState::new();
                    draft.routing_hints = None;
                }
                draft.focused_field = AgentField::Provider;
            }
            KeyCode::Up => {
                if draft.template_index > 0 {
                    draft.template_index -= 1;
                }
            }
            KeyCode::Down => {
                let max = if draft.is_orchestrator {
                    templates::ORCHESTRATOR_TEMPLATES.len()
                } else {
                    templates::WORKER_TEMPLATES.len()
                };
                if draft.template_index < max {
                    draft.template_index += 1;
                }
            }
            KeyCode::Esc => {
                draft.focused_field = AgentField::Name;
            }
            _ => {}
        },
        AgentField::Provider => match key {
            KeyCode::Enter | KeyCode::Tab => {
                if draft.provider == Provider::Antigravity {
                    draft.focused_field = AgentField::Description;
                } else {
                    draft.focused_field = AgentField::Model;
                }
            }
            KeyCode::Up => {
                draft.provider = draft.provider.cycle_prev();
                draft.model.reset();
                draft.custom_model = TextInputState::new();
            }
            KeyCode::Down => {
                draft.provider = draft.provider.cycle_next();
                draft.model.reset();
                draft.custom_model = TextInputState::new();
            }
            KeyCode::Esc => draft.focused_field = AgentField::Template,
            _ => {}
        },
        AgentField::Model => match key {
            KeyCode::Enter | KeyCode::Tab => {
                draft.focused_field = AgentField::Description;
            }
            KeyCode::Up => draft.model.cycle_prev(draft.provider),
            KeyCode::Down => draft.model.cycle_next(draft.provider),
            KeyCode::Backspace => {
                if draft.model.is_other(draft.provider) {
                    draft.custom_model.pop();
                }
            }
            KeyCode::Left => {
                if draft.model.is_other(draft.provider) {
                    draft.custom_model.cursor_left();
                }
            }
            KeyCode::Right => {
                if draft.model.is_other(draft.provider) {
                    draft.custom_model.cursor_right();
                }
            }
            KeyCode::Char(c) => {
                if draft.model.is_other(draft.provider) {
                    draft.custom_model.push(c);
                }
            }
            KeyCode::Esc => draft.focused_field = AgentField::Provider,
            _ => {}
        },
        AgentField::Description => match key {
            KeyCode::Enter => return PageTransition::Go(on_done()),
            KeyCode::Backspace => draft.description.pop(),
            KeyCode::Left => draft.description.cursor_left(),
            KeyCode::Right => draft.description.cursor_right(),
            KeyCode::Char(c) => draft.description.push(c),
            KeyCode::Esc => {
                if draft.provider == Provider::Antigravity {
                    draft.focused_field = AgentField::Provider;
                } else {
                    draft.focused_field = AgentField::Model;
                }
            }
            _ => {}
        },
    }
    PageTransition::Stay
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
        .split(frame.area());

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
        WizardPage::Project => {
            render_project_page(frame, chunks[1], state);
        }
        WizardPage::OrchestratorConfig => {
            render_agent_page(frame, chunks[1], &state.orchestrator);
        }
        WizardPage::WorkerCount => render_text_input(
            frame,
            chunks[1],
            " How Many Worker Agents? ",
            &state.worker_count_input,
            true,
        ),
        WizardPage::WorkerConfig { index } => {
            render_agent_page(frame, chunks[1], &state.workers[*index]);
        }
        WizardPage::Summary => {
            render_summary_page(frame, chunks[1], state);
        }
    }

    // Footer
    let agent_footer = |draft: &AgentDraft| match draft.focused_field {
        AgentField::Template => {
            "↑↓: select template   Enter/Tab: apply   Esc: back   Ctrl+C: cancel"
        }
        AgentField::Provider => {
            "↑↓: select provider   Enter/Tab: next   Esc: back   Ctrl+C: cancel"
        }
        AgentField::Model => {
            if draft.model.is_other(draft.provider) {
                "↑↓: change option   type: custom model   Enter/Tab: next   Esc: back   Ctrl+C: cancel"
            } else {
                "↑↓: select model   Enter/Tab: next   Esc: back   Ctrl+C: cancel"
            }
        }
        _ => "Enter: next   Esc: back   Ctrl+C: cancel",
    };
    let footer_text = match &state.page {
        WizardPage::Project => match state.project_field {
            ProjectField::InstallDir => "Enter/Tab: next field   Ctrl+C: cancel",
            ProjectField::Name => "Enter/Tab: next field   Esc: back   Ctrl+C: cancel",
            ProjectField::Sdd => "↑↓: select workflow   Enter: next page   Esc/Tab: back   Ctrl+C: cancel",
        },
        WizardPage::OrchestratorConfig => agent_footer(&state.orchestrator),
        WizardPage::WorkerCount => "Enter: next   Esc: back   Ctrl+C: cancel",
        WizardPage::WorkerConfig { index } => agent_footer(&state.workers[*index]),
        WizardPage::Summary => "Enter: confirm   Esc: back   Ctrl+C: cancel",
    };
    let footer = Paragraph::new(footer_text);
    frame.render_widget(footer, chunks[2]);
}

/// Render a single text input field. Unfocused fields use DarkGray border so titles remain visible.
fn render_text_input(
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
        Color::DarkGray
    };

    let field_widget = Paragraph::new(input.display(focused)).block(
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

/// Render a radio-button list where every option is visible and the selected one is highlighted.
fn render_radio_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    title: &str,
    options: &[&str],
    selected: usize,
    focused: bool,
) {
    let border_color = if focused { Color::Cyan } else { Color::DarkGray };
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(i, opt)| {
            let (marker, style) = if i == selected {
                (
                    "●",
                    if focused {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                )
            } else {
                ("○", Style::default().fg(Color::DarkGray))
            };
            ListItem::new(Span::styled(format!("  {} {}", marker, opt), style))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", title)),
    );
    frame.render_widget(list, area);
}

/// Render the combined Installation Directory + Project Name + SDD Workflow page.
fn render_project_page(frame: &mut Frame, area: ratatui::layout::Rect, state: &WizardState) {
    let sdd_h = SddWorkflow::ALL.len() as u16 + 2; // options + 2 border lines
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),     // Installation Directory text input
            Constraint::Length(3),     // Project Name text input
            Constraint::Length(sdd_h), // SDD radio list
            Constraint::Min(0),        // spacer
        ])
        .split(area);

    let dir_focused = state.project_field == ProjectField::InstallDir;
    let dir_color = if dir_focused { Color::Cyan } else { Color::DarkGray };
    let dir_widget = Paragraph::new(state.install_dir_input.display(dir_focused)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(dir_color))
            .title(" Installation Directory "),
    );
    frame.render_widget(dir_widget, chunks[0]);

    let name_focused = state.project_field == ProjectField::Name;
    let name_color = if name_focused { Color::Cyan } else { Color::DarkGray };
    let name_widget = Paragraph::new(state.project_input.display(name_focused)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(name_color))
            .title(" Project Name "),
    );
    frame.render_widget(name_widget, chunks[1]);

    render_radio_list(
        frame,
        chunks[2],
        "SDD Workflow",
        &SddWorkflow::ALL,
        state.sdd.index(),
        state.project_field == ProjectField::Sdd,
    );
}

/// Render a single agent configuration page (orchestrator or worker).
/// Role is not shown — it is implicit from which page we're on.
fn render_agent_page(frame: &mut Frame, area: ratatui::layout::Rect, draft: &AgentDraft) {
    let model_opts = ModelSelector::options_for(draft.provider);
    let model_h = if model_opts.is_empty() { 3u16 } else { model_opts.len() as u16 + 2 };
    let custom_model_h: u16 = if draft.model.is_other(draft.provider) { 3 } else { 0 };

    // Compute template section height dynamically
    let templates_list = if draft.is_orchestrator {
        templates::ORCHESTRATOR_TEMPLATES
    } else {
        templates::WORKER_TEMPLATES
    };
    let template_h = (templates_list.len() + 1 + 2) as u16; // options + Custom + 2 border lines

    let agent_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),              // 0: Name field
            Constraint::Length(1),              // 1: Name hint
            Constraint::Length(template_h),     // 2: Template selector (NEW — horizontal split)
            Constraint::Length(5),              // 3: Provider radio (3 options + 2 border)
            Constraint::Length(model_h),        // 4: Model radio (dynamic)
            Constraint::Length(custom_model_h), // 5: Custom model text input
            Constraint::Length(3),              // 6: Description field
            Constraint::Length(1),              // 7: Description hint
            Constraint::Min(0),                 // 8: spacer
        ])
        .split(area);

    // --- Name field ---
    let name_focused = draft.focused_field == AgentField::Name;
    let name_color = if name_focused { Color::Cyan } else { Color::DarkGray };
    let name_widget = Paragraph::new(draft.name.display(name_focused)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(name_color))
            .title(" Name "),
    );
    frame.render_widget(name_widget, agent_chunks[0]);
    let name_hint = Paragraph::new("  optional — unique identifier (e.g. backend, frontend)")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(name_hint, agent_chunks[1]);

    // --- Template selector: horizontal split (45% list / 55% preview) ---
    let template_pane = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(agent_chunks[2]);

    // LEFT pane: radio list of template display names + "Custom"
    let mut display_names: Vec<&str> = templates_list.iter().map(|t| t.display_name).collect();
    display_names.push("Custom");
    let template_focused = draft.focused_field == AgentField::Template;
    render_radio_list(
        frame,
        template_pane[0],
        " Role Template ",
        &display_names,
        draft.template_index,
        template_focused,
    );

    // RIGHT pane: description preview
    let preview_text = if draft.template_index < templates_list.len() {
        templates_list[draft.template_index].description
    } else {
        "Enter role and description manually."
    };
    let preview_border_color = if template_focused { Color::Cyan } else { Color::DarkGray };
    let preview = Paragraph::new(preview_text)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Preview ")
                .border_style(Style::default().fg(preview_border_color)),
        );
    frame.render_widget(preview, template_pane[1]);

    // --- Provider radio list ---
    render_radio_list(
        frame,
        agent_chunks[3],
        "Provider",
        &Provider::ALL,
        draft.provider.index(),
        draft.focused_field == AgentField::Provider,
    );

    // --- Model radio list (or "not applicable" for Antigravity) ---
    if model_opts.is_empty() {
        let na = Paragraph::new(Span::styled(
            "  (no model — antigravity uses no provider)",
            Style::default().fg(Color::DarkGray),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Model "),
        );
        frame.render_widget(na, agent_chunks[4]);
    } else {
        render_radio_list(
            frame,
            agent_chunks[4],
            "Model",
            model_opts,
            draft.model.index,
            draft.focused_field == AgentField::Model,
        );
    }

    // --- Custom model text input (shown when "other" is selected) ---
    if draft.model.is_other(draft.provider) {
        let model_focused = draft.focused_field == AgentField::Model;
        let color = if model_focused { Color::Cyan } else { Color::DarkGray };
        let custom_widget = Paragraph::new(draft.custom_model.display(model_focused)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(" Custom Model "),
        );
        frame.render_widget(custom_widget, agent_chunks[5]);
    }

    // --- Description field ---
    let desc_focused = draft.focused_field == AgentField::Description;
    let desc_color = if desc_focused { Color::Cyan } else { Color::DarkGray };
    let desc_widget = Paragraph::new(draft.description.display(desc_focused)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(desc_color))
            .title(" Description "),
    );
    frame.render_widget(desc_widget, agent_chunks[6]);
    let desc_hint = Paragraph::new("  optional — press Enter to continue")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(desc_hint, agent_chunks[7]);
}

fn draft_summary_line(label: &str, d: &AgentDraft) -> String {
    let model_str = if d.model.is_other(d.provider) {
        let v = d.custom_model.value.trim();
        if v.is_empty() { "other (not set)".to_string() } else { v.to_string() }
    } else {
        d.model.current(d.provider).map(|s| s.to_string()).unwrap_or_else(|| "-".to_string())
    };
    let desc_str = if d.description.value.trim().is_empty() {
        "-".to_string()
    } else {
        d.description.value.trim().to_string()
    };
    let name_str = d.name.value.trim();
    let display = if name_str.is_empty() {
        label.to_string()
    } else {
        format!("{} ({})", label, name_str)
    };
    format!("{}: provider={}, model={}, desc={}", display, d.provider.as_str(), model_str, desc_str)
}

/// Render the summary/review page.
fn render_summary_page(frame: &mut Frame, area: ratatui::layout::Rect, state: &WizardState) {
    let summary_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // summary list
            Constraint::Length(1), // confirm hint
        ])
        .split(area);

    // Build agent fleet diagram lines
    // Collect all worker labels: existing (unmarked) + new (tagged [new])
    let orch_label: String = if state.worker_only {
        state.existing_orchestrator.clone().unwrap_or_else(|| "orchestrator".to_string())
    } else {
        let name = state.orchestrator.name.value.trim();
        if name.is_empty() {
            format!("orchestrator ({})", state.orchestrator.provider.as_str())
        } else {
            format!("{} ({})", name, state.orchestrator.provider.as_str())
        }
    };

    let mut all_workers: Vec<(String, bool)> = state.existing_workers
        .iter()
        .map(|s| (s.clone(), false))
        .collect();
    for d in state.workers.iter().take(state.worker_count) {
        let name = d.name.value.trim();
        let label = if name.is_empty() {
            format!("worker ({})", d.provider.as_str())
        } else {
            format!("{} ({})", name, d.provider.as_str())
        };
        all_workers.push((label, true));
    }

    // Build the fleet diagram as text lines.
    // Layout:
    //   ┌────────────────────────┐
    //   │  orchestrator (claude) │
    //   └────────────┬───────────┘
    //                │
    //                ├──▶ worker1 (gemini)
    //                └──▶ worker2 (claude) [new]
    let mut diagram_lines: Vec<Line> = Vec::new();
    let inner_label = format!("  {}  ", orch_label);
    let box_inner = inner_label.len().max(24);
    let top    = format!("  ┌{}┐", "─".repeat(box_inner));
    let mid    = format!("  │{:<width$}│", inner_label, width = box_inner);
    // stem exits from the middle of the bottom border
    let stem_col = 2 + box_inner / 2; // column of the ┬ / │
    let bottom = {
        let left_dashes  = stem_col.saturating_sub(3);   // "  └" is 3 chars
        let right_dashes = box_inner.saturating_sub(left_dashes + 1);
        format!("  └{}┬{}┘", "─".repeat(left_dashes), "─".repeat(right_dashes))
    };
    let indent = " ".repeat(stem_col);

    let cyan = Style::default().fg(Color::Cyan);
    diagram_lines.push(Line::from(Span::styled(top, cyan)));
    diagram_lines.push(Line::from(Span::styled(mid, cyan)));
    diagram_lines.push(Line::from(Span::styled(bottom, cyan)));

    if all_workers.is_empty() {
        diagram_lines.push(Line::from(Span::styled(
            format!("{}│", indent),
            cyan,
        )));
        diagram_lines.push(Line::from(Span::styled(
            format!("{}└──▶ (no workers)", indent),
            cyan,
        )));
    } else {
        diagram_lines.push(Line::from(Span::styled(format!("{}│", indent), cyan)));
        for (i, (label, is_new)) in all_workers.iter().enumerate() {
            let is_last = i == all_workers.len() - 1;
            let branch = if is_last { "└──▶ " } else { "├──▶ " };
            let tag = if *is_new { " [new]" } else { "" };
            let worker_style = if *is_new {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };
            diagram_lines.push(Line::from(vec![
                Span::styled(format!("{}{}", indent, branch), cyan),
                Span::styled(format!("{}{}", label, tag), worker_style),
            ]));
        }
    }

    // Build summary list
    let mut items: Vec<ListItem> = Vec::new();

    if !state.worker_only {
        items.push(ListItem::new(format!("Dir     : {}", state.install_dir_input.value.trim())));
        items.push(ListItem::new(format!("Project : {}", state.project_input.value.trim())));
        items.push(ListItem::new(format!("SDD     : {}", state.sdd.as_str())));
        items.push(ListItem::new(""));
        items.push(ListItem::new(Span::styled(
            "Orchestrator",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        items.push(ListItem::new(format!("  {}", draft_summary_line("orchestrator", &state.orchestrator))));
        items.push(ListItem::new(""));
        items.push(ListItem::new(Span::styled(
            format!("Workers ({})", state.worker_count),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for (i, d) in state.workers.iter().take(state.worker_count).enumerate() {
            items.push(ListItem::new(format!(
                "  {}",
                draft_summary_line(&format!("Worker {}", i + 1), d)
            )));
        }
    } else {
        // Add-agents mode: show existing workers, then new ones
        items.push(ListItem::new(Span::styled(
            format!("Workers — {} existing + {} new", state.existing_workers.len(), state.worker_count),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for label in &state.existing_workers {
            items.push(ListItem::new(format!("  {} (existing)", label)));
        }
        for (i, d) in state.workers.iter().take(state.worker_count).enumerate() {
            items.push(ListItem::new(format!(
                "  {}  [new]",
                draft_summary_line(&format!("Worker {}", state.existing_workers.len() + i + 1), d)
            )));
        }
    }

    // Fleet diagram + summary list share the top chunk
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((diagram_lines.len() + 2) as u16), // diagram
            Constraint::Min(3),                                    // list
        ])
        .split(summary_chunks[0]);

    let diagram_widget = Paragraph::new(diagram_lines)
        .block(Block::default().borders(Borders::ALL).title(" Agent Fleet "));
    frame.render_widget(diagram_widget, inner_chunks[0]);

    let list_widget = List::new(items).block(
        Block::default().borders(Borders::ALL).title(" Review "),
    );
    frame.render_widget(list_widget, inner_chunks[1]);

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
                        KeyAction::Cancel => {
                            restore_terminal(&mut terminal)?;
                            return Ok(None);
                        }
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

/// Run the wizard starting at the WorkerCount page, skipping Project and OrchestratorConfig.
/// Used by the `init --add-agents` path (Plan 02) to add new workers to an existing project.
/// Returns `Some(Vec<AgentInput>)` on completion, `None` if the user cancels.
/// `existing_orchestrator` and `existing_workers` are display labels shown on the Review page.
pub async fn run_worker_only(
    existing_orchestrator: Option<String>,
    existing_workers: Vec<String>,
) -> anyhow::Result<Option<Vec<AgentInput>>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;
    let mut state = WizardState::new();
    state.existing_orchestrator = existing_orchestrator;
    state.existing_workers = existing_workers;
    state.page = WizardPage::WorkerCount; // Start at worker count, skip Project + Orchestrator
    state.worker_only = true;

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
                        KeyAction::Cancel => {
                            restore_terminal(&mut terminal)?;
                            return Ok(None);
                        }
                        KeyAction::Complete => {
                            restore_terminal(&mut terminal)?;
                            // Extract only the workers (no project/orchestrator)
                            let worker_count = state.worker_count;
                            let agents: Vec<AgentInput> = state
                                .workers
                                .into_iter()
                                .take(worker_count)
                                .map(|d| draft_to_agent_input(d, "worker"))
                                .collect();
                            return Ok(Some(agents));
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
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn test_text_input_pop() {
        let mut s = TextInputState::new();
        s.push('a');
        s.push('b');
        s.pop();
        assert_eq!(s.value, "a");
        assert_eq!(s.cursor, 1);
        s.pop();
        assert_eq!(s.value, "");
        assert_eq!(s.cursor, 0);
        // pop on empty should not panic
        s.pop();
        assert_eq!(s.value, "");
        assert_eq!(s.cursor, 0);
    }

    #[test]
    fn test_text_input_cursor_movement() {
        let mut s = TextInputState::new();
        s.push('a');
        s.push('b');
        s.push('c');
        assert_eq!(s.cursor, 3);

        s.cursor_left();
        assert_eq!(s.cursor, 2);

        s.cursor_left();
        s.cursor_left();
        assert_eq!(s.cursor, 0);

        // cannot go left past 0
        s.cursor_left();
        assert_eq!(s.cursor, 0);

        s.cursor_right();
        assert_eq!(s.cursor, 1);

        // insert at cursor inserts in the middle
        s.push('X');
        assert_eq!(s.value, "aXbc");
        assert_eq!(s.cursor, 2);
    }

    #[test]
    fn test_text_input_cursor_right_boundary() {
        let mut s = TextInputState::new();
        s.push('a');
        s.cursor_right(); // already at end
        assert_eq!(s.cursor, 1);
    }

    #[test]
    fn test_text_input_display_cursor() {
        let mut s = TextInputState::new();
        s.push('a');
        s.push('b');
        // cursor at end
        assert_eq!(s.display(true), "ab|");
        // cursor in middle
        s.cursor_left();
        assert_eq!(s.display(true), "a|b");
        // inactive: no cursor
        assert_eq!(s.display(false), "ab");
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
    fn test_role_cycle() {
        assert_eq!(Role::Orchestrator.cycle_next(), Role::Worker);
        assert_eq!(Role::Worker.cycle_next(), Role::Orchestrator);
    }

    #[test]
    fn test_role_as_str() {
        assert_eq!(Role::Orchestrator.as_str(), "orchestrator");
        assert_eq!(Role::Worker.as_str(), "worker");
    }

    #[test]
    fn test_provider_cycle() {
        assert_eq!(Provider::ClaudeCode.cycle_next(), Provider::GeminiCli);
        assert_eq!(Provider::GeminiCli.cycle_next(), Provider::Antigravity);
        assert_eq!(Provider::Antigravity.cycle_next(), Provider::ClaudeCode);
    }

    #[test]
    fn test_provider_cycle_prev() {
        assert_eq!(Provider::ClaudeCode.cycle_prev(), Provider::Antigravity);
        assert_eq!(Provider::GeminiCli.cycle_prev(), Provider::ClaudeCode);
        assert_eq!(Provider::Antigravity.cycle_prev(), Provider::GeminiCli);
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(Provider::ClaudeCode.as_str(), "claude-code");
        assert_eq!(Provider::GeminiCli.as_str(), "gemini-cli");
        assert_eq!(Provider::Antigravity.as_str(), "antigravity");
    }

    #[test]
    fn test_provider_display_matches_valid_providers() {
        let valid_providers = &["antigravity", "claude-code", "gemini-cli"];
        assert!(valid_providers.contains(&Provider::ClaudeCode.as_str()));
        assert!(valid_providers.contains(&Provider::GeminiCli.as_str()));
        assert!(valid_providers.contains(&Provider::Antigravity.as_str()));
    }

    #[test]
    fn test_model_selector_claude() {
        let mut m = ModelSelector::new();
        assert_eq!(m.current(Provider::ClaudeCode), Some("sonnet"));
        m.cycle_next(Provider::ClaudeCode);
        assert_eq!(m.current(Provider::ClaudeCode), Some("opus"));
        m.cycle_next(Provider::ClaudeCode);
        assert_eq!(m.current(Provider::ClaudeCode), Some("haiku"));
        m.cycle_next(Provider::ClaudeCode);
        assert_eq!(m.current(Provider::ClaudeCode), Some("other"));
        // wraps around
        m.cycle_next(Provider::ClaudeCode);
        assert_eq!(m.current(Provider::ClaudeCode), Some("sonnet"));
    }

    #[test]
    fn test_model_selector_prev() {
        let mut m = ModelSelector::new();
        m.cycle_prev(Provider::ClaudeCode);
        assert_eq!(m.current(Provider::ClaudeCode), Some("other"));
    }

    #[test]
    fn test_model_selector_gemini() {
        let mut m = ModelSelector::new();
        assert_eq!(m.current(Provider::GeminiCli), Some("gemini-3.1-pro-preview"));
        m.cycle_next(Provider::GeminiCli);
        assert_eq!(m.current(Provider::GeminiCli), Some("gemini-3-flash-preview"));
        m.cycle_next(Provider::GeminiCli);
        assert_eq!(m.current(Provider::GeminiCli), Some("gemini-2.5-pro"));
        m.cycle_next(Provider::GeminiCli);
        assert_eq!(m.current(Provider::GeminiCli), Some("gemini-2.5-flash"));
        m.cycle_next(Provider::GeminiCli);
        assert_eq!(m.current(Provider::GeminiCli), Some("gemini-2.5-flash-lite"));
        m.cycle_next(Provider::GeminiCli);
        assert_eq!(m.current(Provider::GeminiCli), Some("other"));
    }

    #[test]
    fn test_model_selector_is_other() {
        let mut m = ModelSelector::new();
        assert!(!m.is_other(Provider::ClaudeCode));
        // cycle to last option ("other")
        let opts = ModelSelector::options_for(Provider::ClaudeCode);
        for _ in 0..opts.len() - 1 {
            m.cycle_next(Provider::ClaudeCode);
        }
        assert!(m.is_other(Provider::ClaudeCode));
        // Antigravity has no options — never "other"
        assert!(!m.is_other(Provider::Antigravity));
    }

    #[test]
    fn test_model_selector_antigravity_empty() {
        let m = ModelSelector::new();
        assert_eq!(m.current(Provider::Antigravity), None);
        assert_eq!(ModelSelector::options_for(Provider::Antigravity).len(), 0);
    }

    #[test]
    fn test_model_selector_reset_on_provider_change() {
        let mut m = ModelSelector::new();
        m.cycle_next(Provider::ClaudeCode);
        m.cycle_next(Provider::ClaudeCode);
        assert_eq!(m.index, 2);
        m.reset();
        assert_eq!(m.index, 0);
    }

}
