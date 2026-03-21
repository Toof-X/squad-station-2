use crate::{db, tmux};
use owo_colors::OwoColorize;
use owo_colors::Stream;
use sqlx::SqlitePool;

/// Reconcile agent statuses against live tmux sessions.
/// Marks agents as "dead" if their session is gone, or revives to "idle" if session reappears.
/// Skips db-only agents (e.g. antigravity) that never have tmux sessions.
/// Session existence checks run in parallel for faster reconciliation with many agents.
pub async fn reconcile_agent_statuses(pool: &SqlitePool) -> anyhow::Result<()> {
    let agents = db::agents::list_agents(pool).await?;

    // Collect agents that need tmux session checks (skip frozen and db-only)
    let checkable: Vec<&db::agents::Agent> = agents
        .iter()
        .filter(|a| a.status != "frozen" && a.tool != "antigravity")
        .collect();

    // Check all tmux sessions in parallel
    let futures: Vec<_> = checkable
        .iter()
        .map(|a| tmux::session_exists(&a.name))
        .collect();
    let alive_results = futures::future::join_all(futures).await;

    // Apply status updates sequentially (single-writer DB)
    for (agent, session_alive) in checkable.iter().zip(alive_results) {
        if !session_alive && agent.status != "dead" {
            db::agents::update_agent_status(pool, &agent.name, "dead").await?;
        } else if session_alive && agent.status == "dead" {
            db::agents::update_agent_status(pool, &agent.name, "idle").await?;
        }
    }
    Ok(())
}

/// Format status with human-readable duration since last status change.
pub fn format_status_with_duration(status: &str, status_updated_at: &str) -> String {
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
pub fn colorize_agent_status(status: &str) -> String {
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
        "frozen" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.blue())),
        _ => status.to_string(),
    }
}

/// Build a padded cell where visible width is based on `raw` length but output uses `colored`.
pub fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
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
        assert!(result.starts_with("idle "));
        assert!(result.contains("0m") || result.contains("1m"));
    }

    #[test]
    fn test_format_status_with_duration_hours() {
        let ts = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
        let result = format_status_with_duration("busy", &ts);
        assert!(result.starts_with("busy "));
        assert!(result.contains("1h30m"), "got: {}", result);
    }

    #[test]
    fn test_format_status_with_duration_hours_format_125m() {
        let ts = (chrono::Utc::now() - chrono::Duration::minutes(125)).to_rfc3339();
        let result = format_status_with_duration("busy", &ts);
        assert!(result.contains("2h5m"), "got: {}", result);
    }

    #[test]
    fn test_format_status_with_duration_invalid_timestamp() {
        let result = format_status_with_duration("dead", "not-a-timestamp");
        assert_eq!(result, "dead ?");
    }

    #[test]
    fn test_colorize_agent_status_all_variants() {
        for status in &["idle", "busy", "dead", "custom"] {
            let result = colorize_agent_status(status);
            assert!(result.contains(status));
        }
    }

    #[test]
    fn test_pad_colored_width() {
        let result = pad_colored("idle 5m", "idle 5m", 20);
        assert_eq!(result.len(), 20);
        assert!(result.starts_with("idle 5m"));
    }
}
