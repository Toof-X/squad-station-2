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
        assert_eq!(args[3], "-l", "SAFE-02: -l flag must be present to prevent key interpretation");
        assert_eq!(args[4], "hello world");
    }

    #[test]
    fn test_enter_args_no_literal_flag() {
        let args = enter_args("my-session");
        assert_eq!(args[0], "send-keys");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "my-session");
        assert_eq!(args[3], "Enter", "Enter must be sent without -l so it is interpreted as a key");
        assert!(args.len() == 4, "No -l flag in Enter call");
        assert!(!args.contains(&"-l".to_string()), "Enter call must NOT have -l flag");
    }

    #[test]
    fn test_launch_args_use_direct_command() {
        let args = launch_args("agent-session", "claude-code --dangerously-skip-permissions");
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
    fn test_send_keys_args_with_special_chars() {
        // Verify -l flag is always present even with special characters
        let special = "task: [urgent] fix the API\nDo it now";
        let args = send_keys_args("target", special);
        assert_eq!(args[3], "-l", "SAFE-02: -l flag required even with special chars like [, newlines");
        assert_eq!(args[4], special);
    }
}
