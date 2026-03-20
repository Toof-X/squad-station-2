use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, tmux};

pub async fn run(body: String, agent: Option<String>, json: bool) -> anyhow::Result<()> {
    // Resolve agent name: explicit arg or auto-detect from tmux session
    let agent = match agent {
        Some(name) => name,
        None => {
            // Try to get session name from tmux
            let output = std::process::Command::new("tmux")
                .args(["display-message", "-p", "#S"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                _ => anyhow::bail!("Cannot detect agent name. Use --agent to specify."),
            }
        }
    };

    // Load config and connect to DB
    let config_path = std::path::Path::new("squad.yml");
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // Verify agent exists
    let agent_record = match db::agents::get_agent(&pool, &agent).await? {
        Some(r) => r,
        None => anyhow::bail!("Agent not found: {}", agent),
    };

    // Skip if orchestrator (prevent self-notification loop)
    if agent_record.role == "orchestrator" {
        if json {
            println!(
                "{}",
                serde_json::json!({"notified": false, "reason": "orchestrator_skip"})
            );
        }
        return Ok(());
    }

    // Find orchestrator and send notification
    let orchestrator = db::agents::get_orchestrator(&pool).await?;
    let notified = if let Some(orch) = orchestrator {
        if orch.tool == "antigravity" {
            false // DB-only orchestrator
        } else if tmux::session_exists(&orch.name) {
            let notification = format!("[SQUAD INPUT NEEDED] Agent '{}': {}", agent, body);
            tmux::send_keys_literal(&orch.name, &notification)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    // Output result (do NOT change task status or agent status)
    if json {
        let out = serde_json::json!({
            "notified": notified,
            "agent": agent,
            "body": body,
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if std::io::stdout().is_terminal() {
        println!(
            "{} Notification sent from {} to orchestrator",
            "✓".green(),
            agent
        );
    } else {
        println!("Notification sent from {} to orchestrator", agent);
    }

    Ok(())
}
