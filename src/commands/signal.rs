use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, tmux};

pub async fn run(agent: String, json: bool) -> anyhow::Result<()> {
    // GUARD 1: Not in tmux -- silent exit 0 (HOOK-03)
    // Cheapest check first: if TMUX_PANE is not set, we are not inside a tmux session.
    // This catches invocations outside tmux (e.g. unit tests, CI, raw shell).
    if std::env::var("TMUX_PANE").is_err() {
        return Ok(());
    }

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

    // GUARD 3: Agent not registered -- silent exit 0 (HOOK-03)
    // Unregistered agents must be a silent exit 0 (not an error) in hook context.
    let agent_record = match db::agents::get_agent(&pool, &agent).await? {
        Some(r) => r,
        None => return Ok(()),
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
    let orchestrator_notified = if rows > 0 {
        let orchestrator = db::agents::get_orchestrator(&pool).await?;
        if let Some(orch) = orchestrator {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            let notification = format!(
                "[SIGNAL] agent={} status=completed task_id={}",
                agent, task_id_str
            );
            // Only notify if orchestrator tmux session is running.
            // If session is down, signal is persisted in DB — not an error (per user decision).
            if tmux::session_exists(&orch.name) {
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

    // After successful signal, set agent status back to idle and clear current_task FK.
    if rows > 0 {
        // AGNT-02: clear current_task FK on the signaling agent
        sqlx::query("UPDATE agents SET current_task = NULL WHERE name = ?")
            .bind(&agent)
            .execute(&pool).await?;
        db::agents::update_agent_status(&pool, &agent, "idle").await?;
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
