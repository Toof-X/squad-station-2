use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, providers, tmux::{self, TmuxLayer}};

pub async fn run(dry_run: bool, json: bool) -> anyhow::Result<()> {
    let config_path = std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE);
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    let results = reconcile_agents(&pool, dry_run).await?;

    if json {
        let out = serde_json::json!({
            "reconciled": results.iter().filter(|r| r.action != "skip").count(),
            "agents": results,
            "dry_run": dry_run,
        });
        println!("{}", serde_json::to_string(&out)?);
    } else {
        let reconciled: Vec<&ReconcileResult> =
            results.iter().filter(|r| r.action != "skip").collect();
        if reconciled.is_empty() {
            if std::io::stdout().is_terminal() {
                println!("{} All agents in sync", "✓".green());
            } else {
                println!("All agents in sync");
            }
        } else {
            for r in &reconciled {
                if dry_run {
                    println!("[DRY RUN] {} → {}: {}", r.agent, r.action, r.reason);
                } else if std::io::stdout().is_terminal() {
                    println!("{} {} → {}: {}", "✓".green(), r.agent, r.action, r.reason);
                } else {
                    println!("{} → {}: {}", r.agent, r.action, r.reason);
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct ReconcileResult {
    pub agent: String,
    pub action: String,
    pub reason: String,
}

/// Reconcile all busy agents. Returns a list of actions taken.
/// This is also called by status and send commands for embedded reconciliation.
pub async fn reconcile_agents(
    pool: &sqlx::SqlitePool,
    dry_run: bool,
) -> anyhow::Result<Vec<ReconcileResult>> {
    reconcile_agents_with(&tmux::RealTmux, pool, dry_run).await
}

/// Reconcile with an injectable TmuxLayer — used by watchdog tests.
pub async fn reconcile_agents_with(
    tmux_layer: &impl TmuxLayer,
    pool: &sqlx::SqlitePool,
    dry_run: bool,
) -> anyhow::Result<Vec<ReconcileResult>> {
    let agents = db::agents::list_agents(pool).await?;
    let mut results = Vec::new();

    for agent in &agents {
        if agent.status != "busy" {
            continue;
        }

        // Skip if agent became busy less than 2 minutes ago (probably still working)
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&agent.status_updated_at) {
            let elapsed = chrono::Utc::now().signed_duration_since(ts);
            if elapsed.num_seconds() < 120 {
                results.push(ReconcileResult {
                    agent: agent.name.clone(),
                    action: "skip".to_string(),
                    reason: format!("busy for only {}s (< 2m threshold)", elapsed.num_seconds()),
                });
                continue;
            }
        }

        if !tmux_layer.session_exists(&agent.name).await {
            // Session is dead
            if !dry_run {
                db::agents::update_agent_status(pool, &agent.name, "dead").await?;
            }
            results.push(ReconcileResult {
                agent: agent.name.clone(),
                action: "mark_dead".to_string(),
                reason: "no tmux session".to_string(),
            });
            continue;
        }

        if pane_looks_idle_with(tmux_layer, &agent.name, &agent.tool).await {
            // Agent is idle in tmux but busy in DB — signal was lost
            if !dry_run {
                // Complete all processing messages
                let mut completed_count = 0u32;
                loop {
                    let rows = db::messages::update_status(pool, &agent.name).await?;
                    if rows == 0 {
                        break;
                    }
                    completed_count += 1;
                }
                // Clear current_task and set idle
                db::agents::clear_current_task(pool, &agent.name).await?;
                db::agents::update_agent_status(pool, &agent.name, "idle").await?;

                // Notify orchestrator
                if let Ok(Some(orch)) = db::agents::get_orchestrator(pool).await {
                    if orch.tool != "antigravity"
                        && tmux_layer.session_exists(&orch.name).await
                    {
                        let notification = format!(
                            "[SQUAD RECONCILE] Agent '{}' completed {} task(s) (signal was lost). Run: squad-station status",
                            agent.name, completed_count
                        );
                        let _ = tmux_layer.send_keys_literal(&orch.name, &notification).await;
                    }
                }

                results.push(ReconcileResult {
                    agent: agent.name.clone(),
                    action: format!("reconciled ({})", completed_count),
                    reason: "idle pane + busy DB (signal lost)".to_string(),
                });
            } else {
                results.push(ReconcileResult {
                    agent: agent.name.clone(),
                    action: "would_reconcile".to_string(),
                    reason: "idle pane + busy DB (signal lost)".to_string(),
                });
            }
        } else {
            results.push(ReconcileResult {
                agent: agent.name.clone(),
                action: "skip".to_string(),
                reason: "pane shows active output".to_string(),
            });
        }
    }

    Ok(results)
}

/// Detect if an agent's tmux pane shows an idle prompt (injectable for testing).
async fn pane_looks_idle_with(
    tmux_layer: &impl TmuxLayer,
    session_name: &str,
    provider: &str,
) -> bool {
    let last_line = tmux_layer.capture_pane_last_line(session_name).await;
    let last_line = last_line.as_deref().unwrap_or("");

    if let Some(patterns) = providers::idle_patterns(provider) {
        patterns.iter().any(|p| last_line.contains(p))
    } else {
        false // Unknown provider: cannot detect idle (safe default — skip reconcile)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_idle_claude_code() {
        // Test the pattern matching logic (without real tmux)
        let patterns = providers::idle_patterns("claude-code").unwrap();
        let line = "❯ ";
        assert!(patterns.iter().any(|p| line.contains(p)));
    }

    #[test]
    fn test_pane_idle_gemini_cli() {
        let patterns = providers::idle_patterns("gemini-cli").unwrap();
        let line = "> Type your message";
        assert!(patterns.iter().any(|p| line.contains(p)));
    }

    #[test]
    fn test_pane_idle_rejects_bare_gt() {
        // A bare ">" should NOT match for claude-code
        let patterns = providers::idle_patterns("claude-code").unwrap();
        let line = ">";
        assert!(!patterns.iter().any(|p| line.contains(p)));
    }

    #[test]
    fn test_pane_idle_unknown_provider() {
        assert!(providers::idle_patterns("unknown-tool").is_none());
    }
}
