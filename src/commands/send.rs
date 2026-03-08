use anyhow::bail;
use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{cli, config, db, tmux};

pub async fn run(agent: String, task: String, priority: cli::Priority, json: bool) -> anyhow::Result<()> {
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

    // 4. Check tmux session alive
    if !tmux::session_exists(&agent) {
        bail!(
            "Agent tmux session not running: {}. Run 'squad-station init' to launch.",
            agent
        );
    }

    // 5. Write message to DB with priority
    let priority_str = priority.to_string();
    let msg_id = db::messages::insert_message(&pool, "orchestrator", &agent, "task_request", &task, &priority_str).await?;

    // 5b. Mark agent as busy now that a task has been sent
    db::agents::update_agent_status(&pool, &agent, "busy").await?;

    // 6. Inject task into agent tmux session (literal send-keys, SAFE-02)
    tmux::send_keys_literal(&agent, &task)?;

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
        println!("Sent task to {} (id={}, priority={})", agent, msg_id, priority_str);
    }

    Ok(())
}
