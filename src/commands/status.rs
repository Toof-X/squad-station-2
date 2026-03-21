use crate::{config, db};

use super::helpers::{
    colorize_agent_status, format_status_with_duration, pad_colored, reconcile_agent_statuses,
};

#[derive(serde::Serialize)]
struct StatusOutput {
    project: String,
    db_path: String,
    agents: Vec<AgentStatusSummary>,
}

#[derive(serde::Serialize)]
struct AgentStatusSummary {
    name: String,
    role: String,
    status: String,
    status_updated_at: String,
    pending_messages: usize,
}

pub async fn run(json: bool) -> anyhow::Result<()> {
    // 1. Load config + connect
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch agents
    let agents = db::agents::list_agents(&pool).await?;

    if agents.is_empty() {
        println!("No agents registered.");
        return Ok(());
    }

    // 3. Reconcile status against tmux
    reconcile_agent_statuses(&pool).await?;

    // 4. Re-fetch after reconciliation
    let agents = db::agents::list_agents(&pool).await?;

    // 5. Count pending messages per agent
    let mut summaries: Vec<AgentStatusSummary> = Vec::new();
    for agent in &agents {
        let pending = db::messages::count_processing(&pool, &agent.name).await? as usize;
        summaries.push(AgentStatusSummary {
            name: agent.name.clone(),
            role: agent.role.clone(),
            status: agent.status.clone(),
            status_updated_at: agent.status_updated_at.clone(),
            pending_messages: pending,
        });
    }

    if json {
        let output = StatusOutput {
            project: config.project.clone(),
            db_path: db_path.to_string_lossy().to_string(),
            agents: summaries,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // 6. Text output
    let total = summaries.len();
    let idle_count = summaries.iter().filter(|a| a.status == "idle").count();
    let busy_count = summaries.iter().filter(|a| a.status == "busy").count();
    let dead_count = summaries.iter().filter(|a| a.status == "dead").count();

    println!("Project: {}", config.project);
    println!("DB: {}", db_path.display());
    println!(
        "Agents: {} -- {} idle, {} busy, {} dead",
        total, idle_count, busy_count, dead_count
    );
    println!();

    for a in &summaries {
        let raw_status = format_status_with_duration(&a.status, &a.status_updated_at);
        let colored_status_word = colorize_agent_status(&a.status);
        let duration_part = &raw_status[a.status.len()..];
        let colored_full = format!("{}{}", colored_status_word, duration_part);
        let status_cell = pad_colored(&raw_status, &colored_full, 20);
        println!(
            "  {}: {}  |  {} processing",
            a.name, status_cell, a.pending_messages
        );
    }

    Ok(())
}
