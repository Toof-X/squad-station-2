use anyhow::bail;

use crate::{config, db};

pub async fn run(agent: String, json: bool) -> anyhow::Result<()> {
    // 1. Resolve DB path
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;

    // 2. Connect to DB
    let pool = db::connect(&db_path).await?;

    // 3. Validate agent exists
    if db::agents::get_agent(&pool, &agent).await?.is_none() {
        bail!("Agent not found: {}", agent);
    }

    // 4. Query the highest-priority pending message for this agent
    let message = db::messages::peek_message(&pool, &agent).await?;

    match message {
        Some(msg) => {
            if json {
                // JSON mode: serialize the full message
                println!("{}", serde_json::to_string_pretty(&msg)?);
            } else {
                // Text mode: display task prominently for agent to act on
                println!("[{}] (priority={}) {}", msg.status, msg.priority, msg.task);
                println!("id: {}", msg.id);
            }
        }
        None => {
            if json {
                // JSON mode: indicate no pending work
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "pending": false,
                        "agent": agent
                    }))?
                );
            } else {
                // Text mode: friendly message — not an error
                println!("No pending tasks for {agent}");
            }
        }
    }

    // Always return Ok — no pending tasks is normal operation, not an error
    Ok(())
}
