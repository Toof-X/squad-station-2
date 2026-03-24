use anyhow::{bail, Result};
use tokio::process::Command;

// --- TmuxLayer trait for testability ---

/// Abstraction over tmux interactions, enabling mock implementations for testing.
/// The real implementation (`RealTmux`) delegates to actual tmux commands.
/// Tests use `MockTmux` to avoid requiring a real tmux server.
pub trait TmuxLayer: Send + Sync {
    fn send_keys_literal(
        &self,
        target: &str,
        text: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn session_exists(
        &self,
        session_name: &str,
    ) -> impl std::future::Future<Output = bool> + Send;

    fn capture_pane_last_line(
        &self,
        session_name: &str,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

/// Real tmux implementation — delegates to the module-level functions.
pub struct RealTmux;

impl TmuxLayer for RealTmux {
    async fn send_keys_literal(&self, target: &str, text: &str) -> Result<()> {
        send_keys_literal(target, text).await
    }

    async fn session_exists(&self, session_name: &str) -> bool {
        session_exists(session_name).await
    }

    async fn capture_pane_last_line(&self, session_name: &str) -> Option<String> {
        capture_pane_last_line(session_name).await
    }
}

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

fn launch_args_with_dir(session_name: &str, command: &str, start_dir: &str) -> Vec<String> {
    vec![
        "new-session".to_string(),
        "-d".to_string(),
        "-s".to_string(),
        session_name.to_string(),
        "-c".to_string(),
        start_dir.to_string(),
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

fn kill_session_args(session_name: &str) -> Vec<String> {
    vec!["kill-session".into(), "-t".into(), session_name.to_string()]
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

fn load_buffer_args(path: &str) -> Vec<String> {
    vec!["load-buffer".to_string(), path.to_string()]
}

fn paste_buffer_args(target: &str) -> Vec<String> {
    vec![
        "paste-buffer".to_string(),
        "-t".to_string(),
        target.to_string(),
    ]
}

// --- Public API ---

/// Delay (milliseconds) between pasting text and pressing Enter.
/// Claude Code needs time to render the input before Enter is processed.
/// 300ms is sufficient for load-buffer/paste-buffer (content arrives atomically).
/// send_keys_literal uses a longer delay since text trickles character-by-character.
const PASTE_ENTER_DELAY_MS: u64 = 300;
const SENDKEYS_ENTER_DELAY_MS: u64 = 500;

/// Delay (seconds) between sending consecutive commands (e.g., /clear && /gsd:plan-phase 1).
/// The first command needs time to execute before the next one is sent.
const MULTI_CMD_DELAY_SECS: u64 = 5;

/// Send text literally to a tmux target, followed by Enter (SAFE-02)
///
/// Always uses `-l` flag to prevent special character injection.
/// Sends Enter as a separate call so it is interpreted as a key, not literal text.
/// Includes a delay between text and Enter to ensure the target pane
/// has received and rendered the text before Enter is processed.
pub async fn send_keys_literal(target: &str, text: &str) -> Result<()> {
    // Step 1: Send text as literal (no key name interpretation)
    let args = send_keys_args(target, text);
    let status = Command::new("tmux").args(&args).status().await?;
    if !status.success() {
        bail!("tmux send-keys failed for target: {}", target);
    }

    // Step 2: Wait for the pane to receive and render the text
    tokio::time::sleep(std::time::Duration::from_millis(SENDKEYS_ENTER_DELAY_MS)).await;

    // Step 3: Send Enter as separate key (NOT -l, so Enter key is recognized)
    let enter = enter_args(target);
    let status = Command::new("tmux").args(&enter).status().await?;
    if !status.success() {
        bail!("tmux send-keys Enter failed for target: {}", target);
    }

    Ok(())
}

/// Inject a single piece of content into a tmux target using load-buffer/paste-buffer.
async fn inject_single(target: &str, text: &str) -> Result<()> {
    let temp_path =
        std::env::temp_dir().join(format!("squad-station-msg-{}", uuid::Uuid::new_v4()));
    std::fs::write(&temp_path, text)?;

    let path_str = temp_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("temp path contains invalid UTF-8"))?
        .to_string();

    let load_args = load_buffer_args(&path_str);
    let status = Command::new("tmux").args(&load_args).status().await?;
    if !status.success() {
        let _ = std::fs::remove_file(&temp_path);
        bail!("tmux load-buffer failed for target: {}", target);
    }

    let paste_args = paste_buffer_args(target);
    let status = Command::new("tmux").args(&paste_args).status().await?;
    let _ = std::fs::remove_file(&temp_path);
    if !status.success() {
        bail!("tmux paste-buffer failed for target: {}", target);
    }

    tokio::time::sleep(std::time::Duration::from_millis(PASTE_ENTER_DELAY_MS)).await;

    let enter = enter_args(target);
    let status = Command::new("tmux").args(&enter).status().await?;
    if !status.success() {
        bail!("tmux send-keys Enter failed for target: {}", target);
    }

    Ok(())
}

/// Split a body into compound slash commands if applicable.
/// Returns Some(parts) if the body is a compound slash command (e.g. "/clear && /review"),
/// or None if it's plain text that should be sent as-is.
fn split_compound_commands(body: &str) -> Option<Vec<&str>> {
    let parts: Vec<&str> = body
        .split("&&")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Only split when ALL parts are slash commands — prevents corrupting
    // plain task text that happens to contain &&
    if parts.len() > 1 && parts.iter().all(|p| p.starts_with('/')) {
        Some(parts)
    } else {
        None
    }
}

/// Inject body content into a tmux target.
///
/// If the body contains `&&` and every part is a slash command (starts with `/`),
/// it is split into separate commands sent individually with a delay between them.
/// This handles compound slash commands like `/clear && /gsd:plan-phase 1` that
/// Claude Code's TUI cannot parse as one input.
///
/// Plain task text containing `&&` (e.g. "check if A && B") is sent as-is.
pub async fn inject_body(target: &str, body: &str) -> Result<()> {
    if let Some(parts) = split_compound_commands(body) {
        // Multiple slash commands: send each separately with delay between
        for (i, part) in parts.iter().enumerate() {
            inject_single(target, part).await?;
            if i < parts.len() - 1 {
                // Wait for the command to execute before sending the next
                tokio::time::sleep(std::time::Duration::from_secs(MULTI_CMD_DELAY_SECS)).await;
            }
        }
    } else {
        // Single command or plain task text: send as-is
        inject_single(target, body).await?;
    }

    Ok(())
}

/// Check whether a tmux session exists
pub async fn session_exists(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List all live tmux session names.
pub async fn list_live_session_names() -> Vec<String> {
    let output = match Command::new("tmux")
        .args(list_sessions_args())
        .output()
        .await
    {
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
pub async fn kill_window(window_name: &str) -> Result<()> {
    let _ = Command::new("tmux")
        .args(kill_window_args(window_name))
        .status()
        .await;
    Ok(())
}

/// Kill a tmux session by name (idempotent — ignores errors if session does not exist).
pub async fn kill_session(session_name: &str) -> Result<()> {
    let _ = Command::new("tmux")
        .args(kill_session_args(session_name))
        .stderr(std::process::Stdio::null())
        .status()
        .await;
    Ok(())
}

/// Create a view window with tiled panes, one per session, each attaching to that session.
pub async fn create_view_window(window_name: &str, sessions: &[String]) -> Result<()> {
    if sessions.is_empty() {
        return Ok(());
    }

    // First pane: new-window with first session
    let first_cmd = format!("tmux attach-session -t {}", sessions[0]);
    let status = Command::new("tmux")
        .args(new_window_args(window_name, &first_cmd))
        .status()
        .await?;
    if !status.success() {
        bail!("tmux new-window failed for window: {}", window_name);
    }

    // Remaining panes: split-window
    for session in sessions.iter().skip(1) {
        let cmd = format!("tmux attach-session -t {}", session);
        Command::new("tmux")
            .args(split_window_args(window_name, &cmd))
            .status()
            .await?;
    }

    // Apply tiled layout
    Command::new("tmux")
        .args(select_layout_args(window_name, "tiled"))
        .status()
        .await?;

    Ok(())
}

/// Create a dedicated monitor session with orchestrator on the left and workers on the right.
///
/// Layout: orchestrator takes the full left column (50% width),
/// worker agents stack vertically in the right column.
///
/// Each pane runs `sh -c 'TMUX= tmux attach-session -t <agent>'` to bypass the
/// nested-tmux restriction ($TMUX set in the calling environment).
pub async fn create_view_session(session_name: &str, agent_sessions: &[String]) -> Result<()> {
    if agent_sessions.is_empty() {
        return Ok(());
    }

    // First pane (left): orchestrator — always agent_sessions[0]
    let orch_cmd = format!("sh -c 'TMUX= tmux attach-session -t {}'", agent_sessions[0]);
    let status = Command::new("tmux")
        .args(launch_args(session_name, &orch_cmd))
        .status()
        .await?;
    if !status.success() {
        bail!(
            "tmux new-session failed for monitor session: {}",
            session_name
        );
    }

    if agent_sessions.len() > 1 {
        // Split horizontally: left = orchestrator, right = first worker
        let first_worker_cmd =
            format!("sh -c 'TMUX= tmux attach-session -t {}'", agent_sessions[1]);
        Command::new("tmux")
            .args(["split-window", "-t", session_name, "-h", &first_worker_cmd])
            .status()
            .await?;

        // Remaining workers: split vertically within the right column
        for agent in agent_sessions.iter().skip(2) {
            let cmd = format!("sh -c 'TMUX= tmux attach-session -t {}'", agent);
            // Target the last pane (right column) for vertical split
            Command::new("tmux")
                .args(["split-window", "-t", session_name, "-v", &cmd])
                .status()
                .await?;
        }
    }

    // Focus the first pane (orchestrator) so it's selected when attaching
    let first_pane = format!("{}.0", session_name);
    Command::new("tmux")
        .args(["select-pane", "-t", &first_pane])
        .status()
        .await?;

    Ok(())
}

/// Capture the last non-empty line from a tmux pane.
/// Used by the browser server to detect if an agent is at a prompt (idle) or working (busy).
pub async fn capture_pane_last_line(session_name: &str) -> Option<String> {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", session_name, "-p"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
}

/// Resolve tmux session name from a pane ID (e.g. "%3" → "my-agent").
/// Returns None if tmux is not running or pane ID is invalid.
pub async fn session_name_from_pane(pane_id: &str) -> Option<String> {
    let args = list_panes_args(pane_id);
    let output = Command::new("tmux").args(&args).output().await.ok()?;
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

/// Wrap a command with environment setup so hook subprocesses can reliably
/// identify which agent they belong to and find the squad-station binary.
/// - `SQUAD_AGENT_NAME` — agent identity for signal/hook scripts
/// - `PATH` prepend — ensures `squad-station` is findable inside tmux sessions
///   even when ~/.local/bin or ~/.cargo/bin aren't in the default tmux PATH.
fn wrap_with_agent_env(session_name: &str, command: &str) -> String {
    format!(
        "export SQUAD_AGENT_NAME='{}'; \
         export PATH=\"$HOME/.local/bin:$HOME/.cargo/bin:$PATH\"; {}",
        session_name, command
    )
}

/// Launch an agent in a new detached tmux session (SAFE-03)
///
/// Passes the command directly to `new-session` to avoid shell readiness race conditions.
/// Sets `SQUAD_AGENT_NAME` environment variable for reliable hook identification.
pub async fn launch_agent(session_name: &str, command: &str) -> Result<()> {
    let wrapped = wrap_with_agent_env(session_name, command);
    let args = launch_args(session_name, &wrapped);
    let status = Command::new("tmux").args(&args).status().await?;
    if !status.success() {
        bail!("Failed to create tmux session: {}", session_name);
    }
    Ok(())
}

/// Launch an agent in a new detached tmux session at a specific working directory.
/// Sets `SQUAD_AGENT_NAME` environment variable for reliable hook identification.
pub async fn launch_agent_in_dir(session_name: &str, command: &str, start_dir: &str) -> Result<()> {
    let wrapped = wrap_with_agent_env(session_name, command);
    let args = launch_args_with_dir(session_name, &wrapped, start_dir);
    let status = Command::new("tmux").args(&args).status().await?;
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
        assert_eq!(
            args[4], "#S",
            "#S must not be escaped — plain tmux format string"
        );
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

    #[test]
    fn test_load_buffer_args() {
        let args = load_buffer_args("/tmp/squad-station-msg-abc");
        assert_eq!(args[0], "load-buffer");
        assert_eq!(args[1], "/tmp/squad-station-msg-abc");
        assert_eq!(
            args.len(),
            2,
            "load-buffer takes only the path, no extra flags"
        );
    }

    #[test]
    fn test_load_buffer_args_with_spaces_in_path() {
        let args = load_buffer_args("/tmp/my path/file");
        assert_eq!(
            args[1], "/tmp/my path/file",
            "path with spaces must be preserved as single arg element"
        );
    }

    #[test]
    fn test_paste_buffer_args() {
        let args = paste_buffer_args("my-agent");
        assert_eq!(args[0], "paste-buffer");
        assert_eq!(args[1], "-t");
        assert_eq!(args[2], "my-agent");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_paste_buffer_args_no_p_flag() {
        let args = paste_buffer_args("my-agent");
        assert!(
            !args.contains(&"-p".to_string()),
            "paste-buffer must use -t not -p; -p pastes to current pane ignoring -t target"
        );
    }

    #[test]
    fn test_wrap_with_agent_env() {
        let result =
            wrap_with_agent_env("kindle-implement", "claude --dangerously-skip-permissions");
        assert!(result.contains("SQUAD_AGENT_NAME='kindle-implement'"));
        assert!(result.contains("$HOME/.local/bin"));
        assert!(result.contains("$HOME/.cargo/bin"));
        assert!(result.ends_with("claude --dangerously-skip-permissions"));
    }

    #[test]
    fn test_wrap_with_agent_env_preserves_special_chars() {
        let result = wrap_with_agent_env("my-agent", "gemini -y --model gemini-3.1-pro");
        assert!(result.contains("SQUAD_AGENT_NAME='my-agent'"));
        assert!(result.ends_with("gemini -y --model gemini-3.1-pro"));
    }

    #[test]
    fn test_launch_args_include_agent_env_wrapper() {
        // Verify that launch_agent would produce args with the env wrapper
        let wrapped = wrap_with_agent_env("test-agent", "claude --dangerously-skip-permissions");
        let args = launch_args("test-agent", &wrapped);
        assert!(
            args[4].contains("SQUAD_AGENT_NAME"),
            "Launch command must include SQUAD_AGENT_NAME export"
        );
        assert!(
            args[4].contains("claude --dangerously-skip-permissions"),
            "Original command must be preserved after env export"
        );
    }

    #[test]
    fn test_split_compound_commands_slash_commands() {
        let result = split_compound_commands("/clear && /gsd:plan-phase 1");
        assert_eq!(result, Some(vec!["/clear", "/gsd:plan-phase 1"]));
    }

    #[test]
    fn test_split_compound_commands_plain_text_not_split() {
        assert_eq!(split_compound_commands("check if A && B are set"), None);
        assert_eq!(split_compound_commands("run foo && bar"), None);
    }

    #[test]
    fn test_split_compound_commands_mixed_not_split() {
        assert_eq!(split_compound_commands("/clear && run tests"), None);
    }

    #[test]
    fn test_split_compound_commands_single_command() {
        assert_eq!(split_compound_commands("/clear"), None);
        assert_eq!(split_compound_commands("build the feature"), None);
    }
}
