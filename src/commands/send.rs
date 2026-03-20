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

    // 5b. Fire-and-forget commands (e.g. /clear) are auto-completed immediately.
    // They never trigger a completion hook, so we must NOT set current_task to them
    // (that would cause the next signal to noop since current_task points to a completed message).
    if is_fire_and_forget(&body) {
        // Auto-complete the message without touching current_task
        db::messages::complete_by_id(&pool, &msg_id).await?;

        // If no other tasks are processing, agent goes idle
        let remaining = db::messages::count_processing(&pool, &agent).await?;
        if remaining == 0 && agent_record.status != "busy" {
            // Agent was idle before /clear, stay idle
        } else if remaining == 0 {
            db::agents::update_agent_status(&pool, &agent, "idle").await?;
        }
        // If remaining > 0, current_task already points to the real task — don't touch it
    } else {
        // Real task: set current_task and mark agent as busy
        db::agents::set_current_task(&pool, &agent, &msg_id).await?;
        db::agents::update_agent_status(&pool, &agent, "busy").await?;
    }

    // 6. Inject task into agent tmux session via load-buffer/paste-buffer (TMUX-01, TMUX-02)
    tmux::inject_body(&agent, &body)?;

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
