use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, tmux};

pub async fn run(agent: Option<String>, json: bool) -> anyhow::Result<()> {
    // GUARD 1: No explicit agent name provided -- silent exit 0 (HOOK-03)
    // The hook command passes the session name explicitly via $(tmux display-message -p '#S').
    // If no name is provided (e.g. outside tmux, in CI), we silently exit.
    let agent: String = match agent {
        Some(name) => name,
        None => return Ok(()),
    };

    // GUARD 2: Config/DB connection -- warning to stderr + exit 0 on failure
    // Per locked decision: real errors go to stderr but NEVER fail the provider (exit 0 always).
    let config_path = std::path::Path::new("squad.yml");
    let config = match config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("squad-station: warning: {e}");
            return Ok(());
        }
    };
    let db_path = match config::resolve_db_path(&config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("squad-station: warning: {e}");
            return Ok(());
        }
    };
    let pool = match db::connect(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("squad-station: warning: DB connection failed: {e}");
            return Ok(());
        }
    };

    // GUARD 3: Agent not registered -- silent exit 0 in hook context (HOOK-03),
    // but print a message when running interactively so manual usage isn't confusing.
    let agent_record = match db::agents::get_agent(&pool, &agent).await? {
        Some(r) => r,
        None => {
            if std::io::stdout().is_terminal() {
                println!("Agent not found: {} (ignored)", agent);
            }
            return Ok(());
        }
    };

    // GUARD 4: Orchestrator self-signal -- silent exit 0 (HOOK-01)
    // Prevents infinite loop where the orchestrator's AfterAgent hook signals itself.
    if agent_record.role == "orchestrator" {
        return Ok(());
    }

    // --- Existing signal flow (from Phase 1) ---

    // Idempotent status update (MSG-03): only updates the most recent pending message.
    // Returns 0 if no pending message exists — this is NOT an error (duplicate signal silently succeeds).
    let rows = db::messages::update_status(&pool, &agent).await?;

    // Retrieve task_id of the message that was just completed (only if state actually changed).
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

    // Find orchestrator and notify (only on actual state change).
    // GUARD 5: Skip notification when agent is frozen (user is in control).
    let orchestrator_notified = if rows > 0 && agent_record.status != "frozen" {
        let orchestrator = db::agents::get_orchestrator(&pool).await?;
        if let Some(orch) = orchestrator {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            let notification = format!(
                "[SQUAD SIGNAL] Agent '{}' completed task {}. Read output: tmux capture-pane -t {} -p | Next: squad-station status",
                agent, task_id_str, agent
            );
            if orch.tool == "antigravity" {
                // DB-only orchestrator: polls DB for completions, no push notification needed.
                false
            } else if tmux::session_exists(&orch.name) {
                // Only notify if orchestrator tmux session is running.
                // If session is down, signal is persisted in DB — not an error (per user decision).
                tmux::send_keys_literal(&orch.name, &notification)?;
                true
            } else {
                false
            }
        } else {
            // No orchestrator registered — signal is persisted in DB only.
            false
        }
    } else {
        false
    };

    // After successful signal, check remaining tasks and update agent status accordingly.
    if rows > 0 {
        let remaining = db::messages::count_processing(&pool, &agent).await?;
        if remaining > 0 {
            // Still has processing tasks — update current_task to next task, stay busy
            let next = db::messages::peek_message(&pool, &agent).await?;
            if let Some(next_msg) = next {
                sqlx::query("UPDATE agents SET current_task = ? WHERE name = ?")
                    .bind(&next_msg.id)
                    .bind(&agent)
                    .execute(&pool)
                    .await?;
            }
            // Agent remains busy — don't change status
        } else {
            // No remaining tasks — clear current_task and set idle
            sqlx::query("UPDATE agents SET current_task = NULL WHERE name = ?")
                .bind(&agent)
                .execute(&pool)
                .await?;
            db::agents::update_agent_status(&pool, &agent, "idle").await?;
        }
    }

    // Output result
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
            println!(
                "Signaled completion for {} (task_id={})",
                agent, task_id_str
            );
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
