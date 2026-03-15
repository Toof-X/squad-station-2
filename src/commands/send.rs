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
