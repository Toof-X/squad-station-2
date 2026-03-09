use anyhow::{bail, Result};
use std::process::Command;

// --- Argument builders (testable without invoking tmux) ---

fn send_keys_args(target: &str, text: &str) -> Vec<String> {
    vec![
        "send-keys".to_string(),
        "-t".to_string(),
        target.to_string(),
        "-l".to_string(),
        text.to_string(),
    ]
}

fn enter_args(target: &str) -> Vec<String> {
    vec![
        "send-keys".to_string(),
        "-t".to_string(),
        target.to_string(),
        "Enter".to_string(),
    ]
}

fn launch_args(session_name: &str, command: &str) -> Vec<String> {
    vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        session_name.to_string(),
        command.to_string(),
    ]
}

fn list_sessions_args() -> Vec<String> {
    vec![
        "list-sessions".into(),
        "-F".into(),
        "#{session_name}".into(),
    ]
}

fn list_panes_args(pane_id: &str) -> Vec<String> {
    vec![
        "list-panes".into(),
        "-t".into(),
        pane_id.to_string(),
        "-F".into(),
        "#S".into(),
    ]
}

fn kill_window_args(window_name: &str) -> Vec<String> {
    vec!["kill-window".into(), "-t".into(), window_name.to_string()]
}

fn new_window_args(window_name: &str, command: &str) -> Vec<String> {
    vec![
        "new-window".into(),
        "-n".into(),
        window_name.to_string(),
        command.to_string(),
    ]
}

fn split_window_args(target: &str, command: &str) -> Vec<String> {
    vec![
        "split-window".into(),
        "-t".into(),
        target.to_string(),
        command.to_string(),
    ]
}

fn select_layout_args(target: &str, layout: &str) -> Vec<String> {
    vec![
        "select-layout".into(),
        "-t".into(),
        target.to_string(),
        layout.to_string(),
    ]
}

// --- Public API ---

/// Send text literally to a tmux target, followed by Enter (SAFE-02)
///
/// Always uses `-l` flag to prevent special character injection.
/// Sends Enter as a separate call so it is interpreted as a key, not literal text.
pub fn send_keys_literal(target: &str, text: &str) -> Result<()> {
    // Step 1: Send text as literal (no key name interpretation)
    let args = send_keys_args(target, text);
    let status = Command::new("tmux").args(&args).status()?;
    if !status.success() {
        bail!("tmux send-keys failed for target: {}", target);
    }

    // Step 2: Send Enter as separate key (NOT -l, so Enter key is recognized)
    let enter = enter_args(target);
    let status = Command::new("tmux").args(&enter).status()?;
    if !status.success() {
        bail!("tmux send-keys Enter failed for target: {}", target);
    }

    Ok(())
}

/// Check whether a tmux session exists
pub fn session_exists(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// List all live tmux session names.
pub fn list_live_session_names() -> Vec<String> {
    let output = match Command::new("tmux").args(list_sessions_args()).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Kill a tmux window by name (idempotent — ignores errors if window does not exist).
pub fn kill_window(window_name: &str) -> Result<()> {
    let _ = Command::new("tmux")
        .args(kill_window_args(window_name))
        .status();
    Ok(())
}

/// Create a view window with tiled panes, one per session, each attaching to that session.
pub fn create_view_window(window_name: &str, sessions: &[String]) -> Result<()> {
    if sessions.is_empty() {
        return Ok(());
    }

    // First pane: new-window with first session
    let first_cmd = format!("tmux attach-session -t {}", sessions[0]);
    let status = Command::new("tmux")
        .args(new_window_args(window_name, &first_cmd))
        .status()?;
    if !status.success() {
        bail!("tmux new-window failed for window: {}", window_name);
    }

    // Remaining panes: split-window
    for session in sessions.iter().skip(1) {
        let cmd = format!("tmux attach-session -t {}", session);
        Command::new("tmux")
            .args(split_window_args(window_name, &cmd))
            .status()?;
    }

    // Apply tiled layout
    Command::new("tmux")
        .args(select_layout_args(window_name, "tiled"))
        .status()?;

    Ok(())
}

/// Resolve tmux session name from a pane ID (e.g. "%3" → "my-agent").
/// Returns None if tmux is not running or pane ID is invalid.
pub fn session_name_from_pane(pane_id: &str) -> Option<String> {
    let args = list_panes_args(pane_id);
    let output = Command::new("tmux").args(&args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Launch an agent in a new detached tmux session (SAFE-03)
///
/// Passes the command directly to `new-session` to avoid shell readiness race conditions.
pub fn launch_agent(session_name: &str, command: &str) -> Result<()> {
    let args = launch_args(session_name, command);
    let status = Command::new("tmux").args(&args).status()?;
    if !status.success() {
        bail!("Failed to create tmux session: {}", session_name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_keys_args_have_literal_flag() {
        let args = send_keys_args("my-session", "hello world");
        assert_eq!(args[0], "send-keys");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "my-session");
        assert_eq!(
            args[3], "-l",
            "SAFE-02: -l flag must be present to prevent key interpretation"
        );
        assert_eq!(args[4], "hello world");
    }

    #[test]
    fn test_enter_args_no_literal_flag() {
        let args = enter_args("my-session");
        assert_eq!(args[0], "send-keys");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "my-session");
        assert_eq!(
            args[3], "Enter",
            "Enter must be sent without -l so it is interpreted as a key"
        );
        assert!(args.len() == 4, "No -l flag in Enter call");
        assert!(
            !args.contains(&"-l".to_string()),
            "Enter call must NOT have -l flag"
        );
    }

    #[test]
    fn test_launch_args_use_direct_command() {
        let args = launch_args(
            "agent-session",
            "claude-code --dangerously-skip-permissions",
        );
        assert_eq!(args[0], "new-session");
        assert_eq!(args[1], "-d");
        assert_eq!(args[2], "-s");
        assert_eq!(args[3], "agent-session");
        assert_eq!(
            args[4], "claude-code --dangerously-skip-permissions",
            "SAFE-03: command must be passed directly to new-session"
        );
    }

    #[test]
    fn test_list_sessions_args() {
        let args = list_sessions_args();
        assert_eq!(args[0], "list-sessions");
        assert_eq!(args[1], "-F");
        assert_eq!(args[2], "#{session_name}");
    }

    #[test]
    fn test_kill_window_args() {
        let args = kill_window_args("squad-view");
        assert_eq!(args[0], "kill-window");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "squad-view");
    }

    #[test]
    fn test_new_window_args() {
        let args = new_window_args("squad-view", "tmux attach-session -t alice");
        assert_eq!(args[0], "new-window");
        assert_eq!(args[1], "-n");
        assert_eq!(args[2], "squad-view");
        assert_eq!(args[3], "tmux attach-session -t alice");
    }

    #[test]
    fn test_split_window_args() {
        let args = split_window_args("squad-view", "tmux attach-session -t bob");
        assert_eq!(args[0], "split-window");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "squad-view");
        assert_eq!(args[3], "tmux attach-session -t bob");
    }

    #[test]
    fn test_select_layout_args() {
        let args = select_layout_args("squad-view", "tiled");
        assert_eq!(args[0], "select-layout");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "squad-view");
        assert_eq!(args[3], "tiled");
    }

    #[test]
    fn test_list_panes_args() {
        let args = list_panes_args("%3");
        assert_eq!(args[0], "list-panes");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "%3");
        assert_eq!(args[3], "-F");
        assert_eq!(args[4], "#S", "#S must not be escaped — plain tmux format string");
    }

    #[test]
    fn test_list_panes_args_arbitrary_pane_id() {
        let args = list_panes_args("%42");
        assert_eq!(args[2], "%42");
        assert_eq!(args[4], "#S");
    }

    #[test]
    fn test_send_keys_args_with_special_chars() {
        // Verify -l flag is always present even with special characters
        let special = "task: [urgent] fix the API\nDo it now";
        let args = send_keys_args("target", special);
        assert_eq!(
            args[3], "-l",
            "SAFE-02: -l flag required even with special chars like [, newlines"
        );
        assert_eq!(args[4], special);
    }
}
