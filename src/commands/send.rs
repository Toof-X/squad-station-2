use anyhow::bail;
use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{cli, config, db, tmux};

pub async fn run(
    agent: String,
    body: String,
    priority: cli::Priority,
    json: bool,
    thread_id: Option<String>,
) -> anyhow::Result<()> {
    // 0. Validate body is not empty
    if body.trim().is_empty() {
        bail!("Task body cannot be empty");
    }

    // 1. Resolve DB path from squad.yml in cwd
    let config_path = std::path::Path::new("squad.yml");
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;

    // 2. Connect to DB
    let pool = db::connect(&db_path).await?;

    // 3. Validate agent exists in DB
    let agent_record = match db::agents::get_agent(&pool, &agent).await? {
        Some(r) => r,
        None => bail!("Agent not found: {}", agent),
    };

    // 3b. Prevent sending tasks to orchestrator-role agents
    if agent_record.role == "orchestrator" {
        bail!("Cannot send tasks to orchestrator agent: {}", agent);
    }

    // 3c. Prevent sending tasks to frozen agents
    if agent_record.status == "frozen" {
        bail!(
            "Agent '{}' is frozen. Run 'squad-station unfreeze' to allow sending tasks.",
            agent
        );
    }

    // 4. Check tmux session alive
    if !tmux::session_exists(&agent) {
        bail!(
            "Agent tmux session not running: {}. Run 'squad-station init' to launch.",
            agent
        );
    }

    // 5. Write message to DB with priority
    let priority_str = priority.to_string();
    let msg_id = db::messages::insert_message(
        &pool,
        "orchestrator",
        &agent,
        "task_request",
        &body,
        &priority_str,
        thread_id.as_deref(),
    )
    .await?;

    // 5b. AGNT-02: set current_task FK on the target agent
    sqlx::query("UPDATE agents SET current_task = ? WHERE name = ?")
        .bind(&msg_id)
        .bind(&agent)
        .execute(&pool)
        .await?;

    // 5c. Mark agent as busy now that a task has been sent
    db::agents::update_agent_status(&pool, &agent, "busy").await?;

    // 6. Inject task into agent tmux session via load-buffer/paste-buffer (TMUX-01, TMUX-02)
    tmux::inject_body(&agent, &body)?;

    // 6b. Auto-complete fire-and-forget commands that never trigger a Stop hook.
    // `/clear` in Claude Code executes instantly without producing a response turn,
    // so the Stop hook never fires and the message would stay `processing` forever,
    // blocking the FIFO queue for subsequent real tasks.
    if is_fire_and_forget(&body) {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE messages SET status = 'completed', updated_at = ?, completed_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(&msg_id)
        .execute(&pool)
        .await?;

        // Restore agent state: point current_task to next real task, or go idle
        let remaining = db::messages::count_processing(&pool, &agent).await?;
        if remaining > 0 {
            // current_task was set to the /clear message in step 5b — fix it
            let next = db::messages::peek_message(&pool, &agent).await?;
            if let Some(next_msg) = next {
                sqlx::query("UPDATE agents SET current_task = ? WHERE name = ?")
                    .bind(&next_msg.id)
                    .bind(&agent)
                    .execute(&pool)
                    .await?;
            }
        } else {
            sqlx::query("UPDATE agents SET current_task = NULL WHERE name = ?")
                .bind(&agent)
                .execute(&pool)
                .await?;
            db::agents::update_agent_status(&pool, &agent, "idle").await?;
        }
    }

    // 7. Output result
    if json {
        let out = serde_json::json!({
            "sent": true,
            "message_id": msg_id,
            "agent": agent,
            "priority": priority_str,
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if std::io::stdout().is_terminal() {
        println!(
            "{} Sent task to {} (id={}, priority={})",
            "✓".green(),
            agent,
            msg_id,
            priority_str
        );
    } else {
        println!(
            "Sent task to {} (id={}, priority={})",
            agent, msg_id, priority_str
        );
    }

    Ok(())
}

/// Returns true for commands that execute instantly without producing a provider response turn.
/// These commands never trigger the Stop hook, so their DB messages must be auto-completed
/// to prevent blocking the FIFO signal queue.
fn is_fire_and_forget(body: &str) -> bool {
    let trimmed = body.trim().to_lowercase();
    // `/clear` (with optional args like `/clear hard`) — Claude Code context reset
    trimmed == "/clear" || trimmed.starts_with("/clear ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fire_and_forget_clear_variants() {
        assert!(is_fire_and_forget("/clear"));
        assert!(is_fire_and_forget("/clear hard"));
        assert!(is_fire_and_forget("  /clear  "));
        assert!(is_fire_and_forget("/CLEAR"));
        assert!(is_fire_and_forget("/Clear Hard"));
    }

    #[test]
    fn test_is_fire_and_forget_normal_tasks() {
        assert!(!is_fire_and_forget("/review PR #2"));
        assert!(!is_fire_and_forget("build the feature"));
        assert!(!is_fire_and_forget("/clearance check"));
        assert!(!is_fire_and_forget("run /clear in the agent"));
    }
}
