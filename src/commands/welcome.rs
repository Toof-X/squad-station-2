use owo_colors::OwoColorize;
use owo_colors::Stream;

const ASCII_ART: &str = r#" ____   ___  _   _    _    ____       ____ _____  _  _____ ___ ___  _   _
/ ___| / _ \| | | |  / \  |  _ \     / ___|_   _|/ \|_   _|_ _/ _ \| \ | |
\___ \| | | | | | | / _ \ | | | |   \___ \ | | / _ \ | |  | | | | |  \| |
 ___) | |_| | |_| |/ ___ \| |_| |    ___) || |/ ___ \| |  | | |_| | |\  |
|____/ \__\_\\___//_/   \_\____/    |____/ |_/_/   \_\_| |___\___/|_| \_|"#;

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
}
