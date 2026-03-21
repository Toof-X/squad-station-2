use anyhow::Result;
use std::path::PathBuf;

use crate::{config, tmux};

/// Compute the expected tmux session names for all agents in a squad config.
/// Mirrors the naming logic in init.rs: `{project}-{provider}-{name_or_role}`.
pub fn compute_session_names(config: &config::SquadConfig) -> Vec<String> {
    let mut names = Vec::new();

    let orch_role = config
        .orchestrator
        .name
        .as_deref()
        .unwrap_or("orchestrator");
    names.push(format!("{}-{}", config.project, orch_role));

    for agent in &config.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        names.push(format!("{}-{}", config.project, role_suffix));
    }

    names
}

pub async fn run(config_path: PathBuf, json: bool) -> Result<()> {
    let config = config::load_config(&config_path)?;
    let session_names = compute_session_names(&config);

    let mut killed = 0u32;
    let mut skipped = 0u32;
    let mut killed_names: Vec<String> = Vec::new();
    let mut skipped_names: Vec<String> = Vec::new();

    for name in &session_names {
        if tmux::session_exists(name).await {
            tmux::kill_session(name).await?;
            killed += 1;
            killed_names.push(name.clone());
        } else {
            skipped += 1;
            skipped_names.push(name.clone());
        }
    }

    // Also kill the monitor session
    let monitor_name = format!("{}-monitor", config.project);
    if tmux::session_exists(&monitor_name).await {
        tmux::kill_session(&monitor_name).await?;
        killed += 1;
        killed_names.push(monitor_name);
    }

    let remaining = tmux::list_live_session_names().await;

    if json {
        let output = serde_json::json!({
            "project": config.project,
            "killed": killed,
            "skipped": skipped,
            "remaining": remaining,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║         Squad Station  •  Close                              ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("  Project : {}", config.project);
        println!();
        for name in &killed_names {
            println!("  [KILLED] {}", name);
        }
        for name in &skipped_names {
            println!("  [SKIP]   {} — not running", name);
        }
        println!();
        println!("  Killed: {}, Skipped: {}", killed, skipped);
        println!();
        if remaining.is_empty() {
            println!("No tmux sessions remaining.");
        } else {
            println!("Remaining tmux sessions:");
            for s in &remaining {
                println!("  {}", s);
            }
        }
        println!();
    }

    Ok(())
}
