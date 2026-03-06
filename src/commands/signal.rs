use anyhow::bail;
use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, tmux};

pub async fn run(agent: String, json: bool) -> anyhow::Result<()> {
    // 1. Resolve DB path from squad.yml in cwd
    let config_path = std::path::Path::new("squad.yml");
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;

    // 2. Connect to DB
    let pool = db::connect(&db_path).await?;

    // 3. Validate agent exists in DB
    let agent_record = db::agents::get_agent(&pool, &agent).await?;
    if agent_record.is_none() {
        bail!("Agent not found: {}", agent);
    }

    // 4. Idempotent status update (MSG-03): only updates the most recent pending message
    // Returns 0 if no pending message exists — this is NOT an error (duplicate signal silently succeeds)
    let rows = db::messages::update_status(&pool, &agent).await?;

    // 5. Retrieve task_id of the message that was just completed (only if state actually changed)
    let task_id: Option<String> = if rows > 0 {
        // Query the most recently completed message for this agent
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM messages WHERE agent_name = ? AND status = 'completed' ORDER BY updated_at DESC LIMIT 1"
        )
        .bind(&agent)
        .fetch_optional(&pool)
        .await?;
        result.map(|(id,)| id)
    } else {
        None
    };

    // 6. Find orchestrator and notify (only on actual state change)
    let orchestrator_notified = if rows > 0 {
        let orchestrator = db::agents::get_orchestrator(&pool).await?;
        if let Some(orch) = orchestrator {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            let notification = format!(
                "[SIGNAL] agent={} status=completed task_id={}",
                agent, task_id_str
            );
            // Only notify if orchestrator tmux session is running
            // If session is down, signal is persisted in DB — not an error (per user decision)
            if tmux::session_exists(&orch.name) {
                tmux::send_keys_literal(&orch.name, &notification)?;
                true
            } else {
                false
            }
        } else {
            // No orchestrator registered — signal is persisted in DB only
            false
        }
    } else {
        false
    };

    // 7. Output result
    if json {
        let out = serde_json::json!({
            "signaled": true,
            "agent": agent,
            "task_id": task_id,
            "orchestrator_notified": orchestrator_notified,
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if rows > 0 {
        let task_id_str = task_id.as_deref().unwrap_or("unknown");
        if std::io::stdout().is_terminal() {
            println!(
                "{} Signaled completion for {} (task_id={})",
                "✓".green(),
                agent,
                task_id_str
            );
        } else {
            println!("Signaled completion for {} (task_id={})", agent, task_id_str);
        }
    } else {
        // rows == 0: duplicate signal — silently succeed (MSG-03)
        if std::io::stdout().is_terminal() {
            println!(
                "{} Signal acknowledged (no pending task for {})",
                "✓".green(),
                agent
            );
        } else {
            println!("Signal acknowledged (no pending task for {})", agent);
        }
    }

    Ok(())
}
