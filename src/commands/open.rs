use anyhow::bail;

use crate::{config, tmux};

pub async fn run() -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let monitor_session = format!("{}-monitor", config.project);

    if !tmux::session_exists(&monitor_session).await {
        bail!(
            "Monitor session '{}' not running. Run 'squad-station init' first.",
            monitor_session
        );
    }

    // Open tmux attach in a new terminal window
    if cfg!(target_os = "macos") {
        let script = format!(
            r#"tell application "Terminal"
    activate
    do script "tmux attach-session -t {}"
end tell"#,
            monitor_session
        );
        let status = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()?;
        if !status.success() {
            bail!("Failed to open new Terminal window");
        }
    } else {
        // Fallback: attach in current terminal on non-macOS
        let status = std::process::Command::new("tmux")
            .args(["attach-session", "-t", &monitor_session])
            .status()?;
        if !status.success() {
            bail!("tmux attach failed with exit code: {:?}", status.code());
        }
    }

    Ok(())
}
