use owo_colors::OwoColorize;
use std::io::IsTerminal;
use tokio::process::Command;

use crate::{config, db, providers, tmux};

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

        if !tmux::session_exists(&agent.name).await {
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

        if pane_looks_idle(&agent.name, &agent.tool).await {
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
                    if orch.tool != "antigravity" && tmux::session_exists(&orch.name).await {
                        let notification = format!(
                            "[SQUAD RECONCILE] Agent '{}' completed {} task(s) (signal was lost). Run: squad-station status",
                            agent.name, completed_count
                        );
                        let _ = tmux::send_keys_literal(&orch.name, &notification).await;
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

/// Detect if an agent's tmux pane shows an idle prompt.
/// Provider-aware: each provider has different prompt patterns and terminal modes.
async fn pane_looks_idle(session_name: &str, provider: &str) -> bool {
    let text = capture_pane(session_name).await;

    // If capture is empty, try alternate screen buffer (Gemini CLI uses full-screen TUI)
    let text = if text.trim().is_empty() && providers::uses_alternate_buffer(provider) {
        capture_pane_alternate(session_name).await
    } else {
        text
    };

    let last_line = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");

    if let Some(patterns) = providers::idle_patterns(provider) {
        patterns.iter().any(|p| last_line.contains(p))
    } else {
        false // Unknown provider: cannot detect idle (safe default — skip reconcile)
    }
}

async fn capture_pane(session: &str) -> String {
    Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p", "-l", "5"])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Capture from alternate screen buffer (for full-screen TUI apps like Gemini CLI)
async fn capture_pane_alternate(session: &str) -> String {
    Command::new("tmux")
        .args(["capture-pane", "-t", session, "-p", "-a", "-l", "5"])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
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
