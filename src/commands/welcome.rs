use owo_colors::OwoColorize;
use owo_colors::Stream;

use std::time::{Duration, Instant};
use crossterm::{
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
}

// ---------------------------------------------------------------------------
// Pure helper functions (unit-testable without a terminal)
// ---------------------------------------------------------------------------

pub fn hint_bar_text(has_config: bool, remaining_secs: u64) -> String {
    if has_config {
        format!("Enter: Open dashboard  Q: Quit  auto-exit {}s", remaining_secs)
    } else {
        format!("Enter: Set up  Q: Quit  auto-exit {}s", remaining_secs)
    }
}

fn commands_list() -> String {
    let mut out = String::new();
    out.push_str("  Commands:\n");
    out.push_str("    init        Initialize squad from config\n");
    out.push_str("    send        Send a task to an agent\n");
    out.push_str("    signal      Signal agent completion\n");
    out.push_str("    peek        Peek at next pending task\n");
    out.push_str("    list        List messages\n");
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

fn draw_welcome(frame: &mut Frame, remaining_secs: u64, has_config: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // BigText title (HalfHeight)
            Constraint::Length(1), // version
            Constraint::Length(1), // spacer
            Constraint::Length(1), // tagline
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // commands table
            Constraint::Length(1), // hint bar
        ])
        .split(frame.area());

    // Chunk 0: BigText pixel-font title
    let title = BigText::builder()
        .pixel_size(PixelSize::HalfHeight)
        .style(Style::default().fg(Color::Red))
        .lines(vec![Line::from("SQUAD-STATION")])
        .centered()
        .build();
    frame.render_widget(title, chunks[0]);

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

    // Chunk 6: hint bar
    let hint = Paragraph::new(hint_bar_text(has_config, remaining_secs))
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(hint, chunks[6]);
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

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut action: Option<WelcomeAction> = None;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break; // timeout = silent exit
        }
        let remaining_secs = remaining.as_secs().max(1); // show at least "1s"
        terminal.draw(|f| draw_welcome(f, remaining_secs, has_config))?;

        if event::poll(remaining.min(Duration::from_secs(1)))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Enter => {
                            action = if has_config {
                                Some(WelcomeAction::LaunchDashboard)
                            } else {
                                Some(WelcomeAction::LaunchInit)
                            };
                            break;
                        }
                        KeyCode::Char('q') | KeyCode::Esc => {
                            action = Some(WelcomeAction::Quit);
                            break;
                        }
                        _ => {}
                    }
                }
            }
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
            "init", "send", "signal", "peek", "list", "ui", "view", "status", "agents",
            "context", "register",
        ];
        for cmd in &subcommands {
            assert!(
                content.contains(cmd),
                "Expected subcommand '{}' in welcome content",
                cmd
            );
        }
    }

    // --- New tests for TUI pure functions ---

    #[test]
    fn test_hint_bar_text_no_config() {
        assert_eq!(
            hint_bar_text(false, 5),
            "Enter: Set up  Q: Quit  auto-exit 5s"
        );
    }

    #[test]
    fn test_hint_bar_text_with_config() {
        assert_eq!(
            hint_bar_text(true, 3),
            "Enter: Open dashboard  Q: Quit  auto-exit 3s"
        );
    }

    #[test]
    fn test_hint_bar_text_one_second() {
        assert_eq!(
            hint_bar_text(false, 1),
            "Enter: Set up  Q: Quit  auto-exit 1s"
        );
    }

    #[test]
    fn test_commands_list_has_all_subcommands() {
        let list = commands_list();
        let subcommands = [
            "init", "send", "signal", "peek", "list", "ui", "view", "status", "agents",
            "context", "register",
        ];
        for cmd in &subcommands {
            assert!(
                list.contains(cmd),
                "Expected subcommand '{}' in commands_list()",
                cmd
            );
        }
    }
}
