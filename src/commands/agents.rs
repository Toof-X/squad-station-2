use crate::{config, db};

use super::helpers::{
    colorize_agent_status, format_status_with_duration, pad_colored, reconcile_agent_statuses,
};

pub async fn run(json: bool) -> anyhow::Result<()> {
    // 1. Connect to DB
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch all agents
    let agents = db::agents::list_agents(&pool).await?;

    if agents.is_empty() {
        println!("No agents registered.");
        return Ok(());
    }

    // 3. Reconcile status against tmux for each agent
    reconcile_agent_statuses(&pool).await?;

    // 4. Re-fetch after reconciliation for accurate display
    let agents = db::agents::list_agents(&pool).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&agents)?);
        return Ok(());
    }

    // 5. Table mode
    // Columns: NAME (15), ROLE (12), STATUS (20), TOOL (15)
    println!(
        "{:<15}  {:<12}  {:<20}  {:<15}",
        "NAME", "ROLE", "STATUS", "TOOL"
    );
    for agent in &agents {
        let raw_status = format_status_with_duration(&agent.status, &agent.status_updated_at);
        let colored_status_word = colorize_agent_status(&agent.status);
        // Build full colored+duration string: colored word + rest of the raw string (space + duration)
        let duration_part = &raw_status[agent.status.len()..]; // e.g., " 5m"
        let colored_full = format!("{}{}", colored_status_word, duration_part);
        let status_cell = pad_colored(&raw_status, &colored_full, 20);

        println!(
            "{:<15}  {:<12}  {}  {:<15}",
            agent.name, agent.role, status_cell, agent.tool
        );
    }

    Ok(())
}
