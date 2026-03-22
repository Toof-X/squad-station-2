use owo_colors::OwoColorize;
use owo_colors::Stream;

use std::time::{Duration, Instant};
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::Paragraph,
    Frame, Terminal,
};
use tui_big_text::{BigText, PixelSize};

const ASCII_ART: &str = r#" ____   ___  _   _    _    ____       ____ _____  _  _____ ___ ___  _   _
/ ___| / _ \| | | |  / \  |  _ \     / ___|_   _|/ \|_   _|_ _/ _ \| \ | |
\___ \| | | | | | | / _ \ | | | |   \___ \ | | / _ \ | |  | | | | |  \| |
 ___) | |_| | |_| |/ ___ \| |_| |    ___) || |/ ___ \| |  | | |_| | |\  |
|____/ \__\_\\___//_/   \_\____/    |____/ |_/_/   \_\_| |___\___/|_| \_|"#;

// ---------------------------------------------------------------------------
// Public action enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum WelcomeAction {
    LaunchInit,
    LaunchDashboard,
    Quit,
    ShowGuide,
    ShowTitle,
}

/// Current page in the welcome TUI state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum WelcomePage {
    Title,
    Guide,
}

/// Determine the action for a given keypress in the welcome TUI (title page).
/// Returns None if the key should be ignored (countdown continues).
pub fn routing_action(key: KeyCode, has_config: bool) -> Option<WelcomeAction> {
    match key {
        KeyCode::Enter | KeyCode::Char('y') => {
            if has_config {
                Some(WelcomeAction::LaunchDashboard)
            } else {
                Some(WelcomeAction::LaunchInit)
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => Some(WelcomeAction::Quit),
        KeyCode::Tab | KeyCode::Right => Some(WelcomeAction::ShowGuide),
        _ => None,
    }
}

/// Determine the action for a given keypress on the guide page.
/// Returns None if the key should be ignored.
pub fn guide_routing_action(key: KeyCode) -> Option<WelcomeAction> {
    match key {
        KeyCode::Tab | KeyCode::Left => Some(WelcomeAction::ShowTitle),
        KeyCode::Char('q') | KeyCode::Esc => Some(WelcomeAction::Quit),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Pure helper functions (unit-testable without a terminal)
// ---------------------------------------------------------------------------

pub fn hint_bar_text(_has_config: bool, remaining_secs: u64) -> String {
    format!("\u{25cf} \u{25cb}  Tab: Guide  Q: Quit  {}s", remaining_secs)
}

pub fn proceed_prompt_text() -> &'static str {
    "  Ok to proceed? (y) "
}

/// Hint bar text for the guide page (dot indicator shows second page active).
pub fn guide_hint_bar_text() -> String {
    "\u{25cb} \u{25cf}  Tab/\u{2190}: Back  Q: Quit".to_string()
}

/// Multi-line content for the guide page: concept summary + 3 numbered steps + footer.
pub fn guide_content() -> String {
    let mut out = String::new();
    out.push_str("One orchestrator AI coordinates N worker agents. Each agent runs in its own tmux session.");
    out.push_str("\n\n");
    out.push_str("  1. Set up your squad\n");
    out.push_str("     Run squad-station init to register your agents.\n");
    out.push('\n');
    out.push_str("  2. Send tasks to agents\n");
    out.push_str("     Use squad-station send to assign work to any agent by name.\n");
    out.push('\n');
    out.push_str("  3. Agents signal completion automatically\n");
    out.push_str("     Hook scripts notify squad-station when a task finishes.\n");
    out.push('\n');
    out.push_str("Run squad-station --help for all commands");
    out
}

fn commands_list() -> String {
    let mut out = String::new();
    out.push_str("  Commands:\n");
    out.push_str("    init        Initialize squad from config\n");
    out.push_str("    send        Send a task to an agent\n");
    out.push_str("    signal      Signal agent completion\n");
    out.push_str("    peek        Peek at next pending task\n");
    out.push_str("    list        List messages\n");
    out.push_str("    monitor     Live agent pane viewer (recommended)\n");
    out.push_str("    fleet       Fleet status overview\n");
    out.push_str("    open        Attach to tmux monitor session\n");
    out.push_str("    ui          Launch TUI dashboard\n");
    out.push_str("    view        Open tmux tiled view\n");
    out.push_str("    status      Show project status\n");
    out.push_str("    agents      List agents\n");
    out.push_str("    context     Generate orchestrator context\n");
    out.push_str("    register    Register an agent\n");
    out
}

// ---------------------------------------------------------------------------
// Terminal setup / teardown (mirrors ui.rs pattern)
// ---------------------------------------------------------------------------

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen, SetCursorStyle::BlinkingBlock)?;
    Terminal::new(CrosstermBackend::new(std::io::stdout())).map_err(Into::into)
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, SetCursorStyle::DefaultUserShape)?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn draw_welcome(frame: &mut Frame, remaining_secs: u64, has_config: bool, cursor_visible: bool) {
    // "SQUAD-STATION" = 13 chars × 8px font
    // HalfHeight: ~104 cols wide, 4 rows tall
    // Quadrant:    ~52 cols wide, 4 rows tall
    // Plain text:   any width,    1 row tall
    let width = frame.area().width;
    let (title_height, pixel_size) = if width >= 105 {
        (4, Some(PixelSize::HalfHeight))
    } else if width >= 55 {
        (4, Some(PixelSize::Quadrant))
    } else {
        (1, None)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height), // title (scalable)
            Constraint::Length(1),  // version
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // tagline
            Constraint::Length(1),  // spacer
            Constraint::Length(15), // commands table (fixed — 15 lines)
            Constraint::Length(1),  // spacer between commands and prompt
            Constraint::Length(1),  // proceed prompt
            Constraint::Min(0),     // flexible spacer
            Constraint::Length(1),  // hint bar (nav)
        ])
        .split(frame.area());

    // Chunk 0: title — scales with terminal width
    if let Some(ps) = pixel_size {
        let title = BigText::builder()
            .pixel_size(ps)
            .style(Style::default().fg(Color::Red))
            .lines(vec![Line::from("SQUAD-STATION")])
            .centered()
            .build();
        frame.render_widget(title, chunks[0]);
    } else {
        let title = Paragraph::new("SQUAD-STATION")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[0]);
    }

    // Chunk 1: version
    let version = Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
        .alignment(Alignment::Center);
    frame.render_widget(version, chunks[1]);

    // Chunk 2: spacer — no widget needed

    // Chunk 3: tagline
    let tagline = Paragraph::new("Multi-agent orchestration for AI coding")
        .alignment(Alignment::Center);
    frame.render_widget(tagline, chunks[3]);

    // Chunk 4: spacer — no widget needed

    // Chunk 5: commands table
    let cmds = Paragraph::new(commands_list())
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(cmds, chunks[5]);

    // Chunk 6: spacer — no widget

    // Chunk 7: proceed prompt with software-blink cursor
    let prompt = Paragraph::new(proceed_prompt_text())
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(prompt, chunks[7]);
    if cursor_visible {
        frame.set_cursor_position((
            chunks[7].x + proceed_prompt_text().len() as u16,
            chunks[7].y,
        ));
    }

    // Chunk 8: flexible spacer — no widget

    // Chunk 9: hint bar (navigation)
    let hint = Paragraph::new(hint_bar_text(has_config, remaining_secs))
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(hint, chunks[9]);
}

fn draw_guide(frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header "Quick Guide"
            Constraint::Length(1), // blank
            Constraint::Min(0),    // guide content (steps + footer)
            Constraint::Length(1), // hint bar
        ])
        .split(frame.area());

    // Chunk 0: centered header
    let header = Paragraph::new("Quick Guide")
        .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Chunk 1: blank — no widget

    // Chunk 2: guide content (concept + steps + footer)
    let content = Paragraph::new(guide_content());
    frame.render_widget(content, chunks[2]);

    // Chunk 3: hint bar
    let hint = Paragraph::new(guide_hint_bar_text())
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(hint, chunks[3]);
}

// ---------------------------------------------------------------------------
// TUI event loop
// ---------------------------------------------------------------------------

pub async fn run_welcome_tui(has_config: bool) -> anyhow::Result<Option<WelcomeAction>> {
    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let mut terminal = setup_terminal()?;

    let mut deadline = Instant::now() + Duration::from_secs(25);
    let mut action: Option<WelcomeAction> = None;
    let mut page = WelcomePage::Title;
    let mut cursor_visible = true;
    let mut blink_deadline = Instant::now() + Duration::from_millis(500);

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break; // timeout = silent exit
        }
        let remaining_secs = remaining.as_secs().max(1); // show at least "1s"
        terminal.draw(|f| match page {
            WelcomePage::Title => draw_welcome(f, remaining_secs, has_config, cursor_visible),
            WelcomePage::Guide => draw_guide(f),
        })?;

        // Poll for the shorter of: time-to-next-blink or 1s countdown tick
        let blink_remaining = blink_deadline.saturating_duration_since(Instant::now());
        let poll_timeout = remaining.min(blink_remaining).min(Duration::from_millis(500));

        if event::poll(poll_timeout)? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let act = match page {
                        WelcomePage::Title => routing_action(key.code, has_config),
                        WelcomePage::Guide => guide_routing_action(key.code),
                    };
                    if let Some(a) = act {
                        match a {
                            WelcomeAction::ShowGuide => {
                                page = WelcomePage::Guide;
                                deadline = Instant::now() + Duration::from_secs(25);
                            }
                            WelcomeAction::ShowTitle => {
                                page = WelcomePage::Title;
                                // Keep remaining time — do not reset on return to title
                            }
                            WelcomeAction::Quit => {
                                action = None;
                                break;
                            }
                            other => {
                                action = Some(other);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Toggle cursor every 500ms (software blink)
        if Instant::now() >= blink_deadline {
            cursor_visible = !cursor_visible;
            blink_deadline = Instant::now() + Duration::from_millis(500);
        }
    }

    restore_terminal(&mut terminal)?;
    // Restore default panic hook
    let _ = std::panic::take_hook();
    Ok(action)
}

// ---------------------------------------------------------------------------
// Build the full welcome screen as a plain string (static fallback content)
// ---------------------------------------------------------------------------

/// Build the full welcome screen as a plain string (no ANSI color codes).
/// Used by unit tests to verify content without terminal color codes.
#[cfg_attr(not(test), allow(dead_code))]
fn welcome_content() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut out = String::new();

    // ASCII art title: SQUAD STATION (figlet-style block)
    out.push_str(ASCII_ART);
    // Plaintext subtitle so callers can assert on "SQUAD" and "STATION"
    out.push_str("\n  SQUAD STATION\n");
    out.push('\n');
    out.push_str(&format!("  v{version}\n"));
    out.push('\n');
    out.push_str("  Get started: squad-station init\n");
    out.push('\n');
    out.push_str("  Commands:\n");
    out.push_str("    init        Initialize squad from config\n");
    out.push_str("    send        Send a task to an agent\n");
    out.push_str("    signal      Signal agent completion\n");
    out.push_str("    peek        Peek at next pending task\n");
    out.push_str("    list        List messages\n");
    out.push_str("    monitor     Live agent pane viewer (recommended)\n");
    out.push_str("    fleet       Fleet status overview\n");
    out.push_str("    open        Attach to tmux monitor session\n");
    out.push_str("    ui          Launch TUI dashboard\n");
    out.push_str("    view        Open tmux tiled view\n");
    out.push_str("    status      Show project status\n");
    out.push_str("    agents      List agents\n");
    out.push_str("    context     Generate orchestrator context\n");
    out.push_str("    register    Register an agent\n");
    out.push('\n');
    out.push_str("  Run squad-station --help for full usage\n");

    out
}

/// Print the branded welcome screen to stdout, with color when supported.
pub fn print_welcome() {
    // Print the ASCII art title in red when color is supported.
    let art = ASCII_ART.if_supports_color(Stream::Stdout, |s| s.red());
    println!("{art}");

    let version = env!("CARGO_PKG_VERSION");
    println!("  SQUAD STATION");
    println!();
    println!("  v{version}");
    println!();
    println!("  Get started: squad-station init");
    println!();
    println!("  Commands:");
    println!("    init        Initialize squad from config");
    println!("    send        Send a task to an agent");
    println!("    signal      Signal agent completion");
    println!("    peek        Peek at next pending task");
    println!("    list        List messages");
    println!("    monitor     Live agent pane viewer (recommended)");
    println!("    fleet       Fleet status overview");
    println!("    open        Attach to tmux monitor session");
    println!("    ui          Launch TUI dashboard");
    println!("    view        Open tmux tiled view");
    println!("    status      Show project status");
    println!("    agents      List agents");
    println!("    context     Generate orchestrator context");
    println!("    register    Register an agent");
    println!();
    println!("  Run squad-station --help for full usage");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    // --- Routing action tests ---

    #[test]
    fn test_routing_action_enter_no_config() {
        assert_eq!(
            routing_action(KeyCode::Enter, false),
            Some(WelcomeAction::LaunchInit)
        );
    }

    #[test]
    fn test_routing_action_enter_with_config() {
        assert_eq!(
            routing_action(KeyCode::Enter, true),
            Some(WelcomeAction::LaunchDashboard)
        );
    }

    #[test]
    fn test_routing_action_quit_q() {
        assert_eq!(
            routing_action(KeyCode::Char('q'), false),
            Some(WelcomeAction::Quit)
        );
    }

    #[test]
    fn test_routing_action_quit_esc() {
        assert_eq!(
            routing_action(KeyCode::Esc, true),
            Some(WelcomeAction::Quit)
        );
    }

    #[test]
    fn test_routing_action_ignored_key() {
        assert_eq!(routing_action(KeyCode::Char('a'), false), None);
    }

    // --- Existing tests (static fallback verification) ---

    #[test]
    fn test_welcome_content_has_ascii_art() {
        let content = welcome_content();
        assert!(content.contains("SQUAD"), "Expected 'SQUAD' in welcome content");
        assert!(content.contains("STATION"), "Expected 'STATION' in welcome content");
    }

    #[test]
    fn test_welcome_content_has_version() {
        let content = welcome_content();
        assert!(
            content.contains(env!("CARGO_PKG_VERSION")),
            "Expected version '{}' in welcome content",
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn test_welcome_content_has_init_hint() {
        let content = welcome_content();
        assert!(
            content.contains("squad-station init"),
            "Expected 'squad-station init' hint in welcome content"
        );
    }

    #[test]
    fn test_welcome_content_has_subcommands() {
        let content = welcome_content();
        let subcommands = [
            "init", "send", "signal", "peek", "list", "monitor", "fleet", "open", "ui", "view",
            "status", "agents", "context", "register",
        ];
        for cmd in &subcommands {
            assert!(
                content.contains(cmd),
                "Expected subcommand '{}' in welcome content",
                cmd
            );
        }
    }

    // --- Updated hint_bar_text tests (now include dot indicator and Tab: Guide) ---

    #[test]
    fn test_hint_bar_text_no_config() {
        let text = hint_bar_text(false, 5);
        assert!(text.contains("Tab: Guide"), "Expected 'Tab: Guide' in hint bar");
        assert!(text.contains("5s"), "Expected '5s' in hint bar");
    }

    #[test]
    fn test_hint_bar_text_with_config() {
        let text = hint_bar_text(true, 3);
        assert!(text.contains("Tab: Guide"), "Expected 'Tab: Guide' in hint bar");
        assert!(text.contains("3s"), "Expected '3s' in hint bar");
    }

    #[test]
    fn test_hint_bar_text_one_second() {
        let text = hint_bar_text(false, 1);
        assert!(text.contains("Tab: Guide"), "Expected 'Tab: Guide' in hint bar");
        assert!(text.contains("1s"), "Expected '1s' in hint bar");
    }

    #[test]
    fn test_proceed_prompt_text() {
        let text = proceed_prompt_text();
        assert!(text.contains("Ok to proceed?"), "Expected 'Ok to proceed?' in prompt");
        assert!(text.contains("y"), "Expected 'y' in prompt");
    }

    #[test]
    fn test_commands_list_has_all_subcommands() {
        let list = commands_list();
        let subcommands = [
            "init", "send", "signal", "peek", "list", "monitor", "fleet", "open", "ui", "view",
            "status", "agents", "context", "register",
        ];
        for cmd in &subcommands {
            assert!(
                list.contains(cmd),
                "Expected subcommand '{}' in commands_list()",
                cmd
            );
        }
    }

    // --- New routing_action tests for ShowGuide ---

    #[test]
    fn test_routing_action_tab_opens_guide() {
        assert_eq!(
            routing_action(KeyCode::Tab, false),
            Some(WelcomeAction::ShowGuide)
        );
    }

    #[test]
    fn test_routing_action_right_opens_guide() {
        assert_eq!(
            routing_action(KeyCode::Right, false),
            Some(WelcomeAction::ShowGuide)
        );
    }

    #[test]
    fn test_routing_action_left_noop() {
        assert_eq!(routing_action(KeyCode::Left, false), None);
    }

    // --- New guide_routing_action tests ---

    #[test]
    fn test_guide_routing_tab_returns_title() {
        assert_eq!(
            guide_routing_action(KeyCode::Tab),
            Some(WelcomeAction::ShowTitle)
        );
    }

    #[test]
    fn test_guide_routing_left_returns_title() {
        assert_eq!(
            guide_routing_action(KeyCode::Left),
            Some(WelcomeAction::ShowTitle)
        );
    }

    #[test]
    fn test_guide_routing_quit() {
        assert_eq!(
            guide_routing_action(KeyCode::Char('q')),
            Some(WelcomeAction::Quit)
        );
    }

    #[test]
    fn test_guide_routing_esc_quit() {
        assert_eq!(
            guide_routing_action(KeyCode::Esc),
            Some(WelcomeAction::Quit)
        );
    }

    #[test]
    fn test_guide_routing_enter_noop() {
        assert_eq!(guide_routing_action(KeyCode::Enter), None);
    }

    // --- guide_hint_bar_text tests ---

    #[test]
    fn test_guide_hint_bar_text() {
        let text = guide_hint_bar_text();
        assert!(text.contains("Tab"), "Expected 'Tab' in guide hint bar");
        assert!(text.contains("Back"), "Expected 'Back' in guide hint bar");
        assert!(text.contains("Q: Quit"), "Expected 'Q: Quit' in guide hint bar");
    }

    // --- guide_content tests ---

    #[test]
    fn test_guide_content() {
        let content = guide_content();
        assert!(content.contains("orchestrator"), "Expected 'orchestrator' in guide content");
        assert!(content.contains("tmux"), "Expected 'tmux' in guide content");
        assert!(content.contains("Set up your squad"), "Expected 'Set up your squad' in guide content");
        assert!(content.contains("Send tasks"), "Expected 'Send tasks' in guide content");
        assert!(content.contains("signal completion"), "Expected 'signal completion' in guide content");
        assert!(content.contains("squad-station --help"), "Expected 'squad-station --help' in guide content");
    }

    // --- hint_bar_text includes Tab: Guide ---

    #[test]
    fn test_hint_bar_text_includes_tab_guide() {
        let text = hint_bar_text(false, 5);
        assert!(text.contains("Tab: Guide"), "Expected 'Tab: Guide' in hint bar text");
    }

    #[test]
    fn test_hint_bar_text_no_proceed_prompt() {
        // proceed prompt is rendered separately — not in hint_bar_text
        let text = hint_bar_text(false, 5);
        assert!(!text.contains("Ok to proceed?"), "Proceed prompt should be in separate widget");
    }

    #[test]
    fn test_routing_action_y_no_config() {
        assert_eq!(
            routing_action(KeyCode::Char('y'), false),
            Some(WelcomeAction::LaunchInit)
        );
    }

    #[test]
    fn test_routing_action_y_with_config() {
        assert_eq!(
            routing_action(KeyCode::Char('y'), true),
            Some(WelcomeAction::LaunchDashboard)
        );
    }
}
