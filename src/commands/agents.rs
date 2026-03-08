use crate::{config, db, tmux};
use owo_colors::OwoColorize;
use owo_colors::Stream;

pub async fn run(json: bool) -> anyhow::Result<()> {
    // 1. Connect to DB
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch all agents
    let agents = db::agents::list_agents(&pool).await?;

    if agents.is_empty() {
        println!("No agents registered.");
        return Ok(());
    }

    // 3. Reconcile status against tmux for each agent
    for agent in &agents {
        let session_alive = tmux::session_exists(&agent.name);
        if !session_alive && agent.status != "dead" {
            // Session gone but agent not marked dead -- mark dead
            db::agents::update_agent_status(&pool, &agent.name, "dead").await?;
        } else if session_alive && agent.status == "dead" {
            // Session reappeared -- auto-revive to idle
            db::agents::update_agent_status(&pool, &agent.name, "idle").await?;
        }
        // If session alive and status is "idle" or "busy": leave unchanged
    }

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

/// Format status with human-readable duration since last status change.
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

/// Colorize the status word (not the full status+duration string).
fn colorize_agent_status(status: &str) -> String {
    match status {
        "idle" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.green())),
        "busy" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.yellow())),
        "dead" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.red())),
        _ => status.to_string(),
    }
}

/// Build a padded cell where visible width is based on `raw` length but output uses `colored`.
fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = if raw_len < width { width - raw_len } else { 0 };
    format!("{}{}", colored, " ".repeat(padding))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_status_with_duration_valid_timestamp() {
        // Recent timestamp should produce a duration like "Xm"
        let now = chrono::Utc::now().to_rfc3339();
        let result = format_status_with_duration("idle", &now);
        assert!(result.starts_with("idle "));
        assert!(result.contains("0m") || result.contains("1m"));
    }

    #[test]
    fn test_format_status_with_duration_hours() {
        // 90 minutes ago should produce "1h30m"
        let ts = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
        let result = format_status_with_duration("busy", &ts);
        assert!(result.starts_with("busy "));
        assert!(result.contains("1h30m"), "got: {}", result);
    }

    #[test]
    fn test_format_status_with_duration_invalid_timestamp() {
        let result = format_status_with_duration("dead", "not-a-timestamp");
        assert_eq!(result, "dead ?");
    }

    #[test]
    fn test_colorize_agent_status_idle() {
        let result = colorize_agent_status("idle");
        assert!(result.contains("idle"));
    }

    #[test]
    fn test_colorize_agent_status_busy() {
        let result = colorize_agent_status("busy");
        assert!(result.contains("busy"));
    }

    #[test]
    fn test_colorize_agent_status_dead() {
        let result = colorize_agent_status("dead");
        assert!(result.contains("dead"));
    }

    #[test]
    fn test_colorize_agent_status_unknown() {
        let result = colorize_agent_status("custom");
        assert_eq!(result, "custom");
    }

    #[test]
    fn test_pad_colored_agents() {
        let result = pad_colored("idle 5m", "idle 5m", 20);
        assert_eq!(result.len(), 20);
        assert!(result.starts_with("idle 5m"));
    }
}
