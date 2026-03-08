use crate::{config, db, tmux};
use owo_colors::OwoColorize;
use owo_colors::Stream;

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
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch agents
    let agents = db::agents::list_agents(&pool).await?;

    if agents.is_empty() {
        println!("No agents registered.");
        return Ok(());
    }

    // 3. Reconcile status against tmux
    for agent in &agents {
        let session_alive = tmux::session_exists(&agent.name);
        if !session_alive && agent.status != "dead" {
            db::agents::update_agent_status(&pool, &agent.name, "dead").await?;
        } else if session_alive && agent.status == "dead" {
            db::agents::update_agent_status(&pool, &agent.name, "idle").await?;
        }
    }

    // 4. Re-fetch after reconciliation
    let agents = db::agents::list_agents(&pool).await?;

    // 5. Count pending messages per agent
    let mut summaries: Vec<AgentStatusSummary> = Vec::new();
    for agent in &agents {
        let pending =
            db::messages::list_messages(&pool, Some(&agent.name), Some("processing"), 9999)
                .await?
                .len();
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
            "  {}: {}  |  {} pending",
            a.name, status_cell, a.pending_messages
        );
    }

    Ok(())
}

fn format_status_with_duration(status: &str, status_updated_at: &str) -> String {
    let since = chrono::DateTime::parse_from_rfc3339(status_updated_at)
        .ok()
        .map(|t| {
            let dur = chrono::Utc::now().signed_duration_since(t);
            let mins = dur.num_minutes();
            if mins < 60 {
                format!("{}m", mins)
            } else {
                format!("{}h{}m", mins / 60, mins % 60)
            }
        })
        .unwrap_or_else(|| "?".to_string());
    format!("{} {}", status, since)
}

fn colorize_agent_status(status: &str) -> String {
    match status {
        "idle" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.green())
        ),
        "busy" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.yellow())
        ),
        "dead" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.red())),
        _ => status.to_string(),
    }
}

fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = width.saturating_sub(raw_len);
    format!("{}{}", colored, " ".repeat(padding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_status_with_duration_minutes() {
        let now = chrono::Utc::now().to_rfc3339();
        let result = format_status_with_duration("idle", &now);
        assert!(result.starts_with("idle 0m") || result.starts_with("idle 1m"));
    }

    #[test]
    fn test_format_status_with_duration_hours_format() {
        let ts = (chrono::Utc::now() - chrono::Duration::minutes(125)).to_rfc3339();
        let result = format_status_with_duration("busy", &ts);
        assert!(result.contains("2h5m"), "got: {}", result);
    }

    #[test]
    fn test_format_status_with_duration_bad_input() {
        assert_eq!(format_status_with_duration("dead", "garbage"), "dead ?");
    }

    #[test]
    fn test_colorize_agent_status_returns_string_with_status() {
        for status in &["idle", "busy", "dead", "other"] {
            let result = colorize_agent_status(status);
            assert!(result.contains(status));
        }
    }

    #[test]
    fn test_pad_colored_status() {
        let result = pad_colored("busy 10m", "busy 10m", 20);
        assert_eq!(result.len(), 20);
    }
}
