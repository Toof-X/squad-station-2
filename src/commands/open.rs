use anyhow::bail;

use crate::{config, tmux};

pub async fn run() -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let monitor_session = format!("{}-monitor", config.project);

    if !tmux::session_exists(&monitor_session) {
        bail!(
            "Monitor session '{}' not running. Run 'squad-station init' first.",
            monitor_session
        );
    }

    // Replace current process with tmux attach
    let status = std::process::Command::new("tmux")
        .args(["attach-session", "-t", &monitor_session])
        .status()?;

    if !status.success() {
        bail!("tmux attach failed with exit code: {:?}", status.code());
    }

    Ok(())
}
