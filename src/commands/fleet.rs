use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::commands::context::{build_agent_metrics, AgentMetrics, AlignmentResult};
use crate::commands::helpers::reconcile_agent_statuses;
use crate::{config, db};

pub async fn run(json: bool) -> anyhow::Result<()> {
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // Reconcile agent statuses against tmux
    reconcile_agent_statuses(&pool).await?;

    let agents = db::agents::list_agents(&pool).await?;

    if agents.is_empty() {
        if json {
            println!(r#"{{"agents":[]}}"#);
        } else {
            println!("No agents registered.");
        }
        return Ok(());
    }

    // Build per-agent metrics (skip orchestrator and dead)
    let metrics: Vec<AgentMetrics> = build_agent_metrics(&pool, &agents).await?;

    if json {
        let json_metrics: Vec<serde_json::Value> = metrics
            .iter()
            .map(|m| {
                let alignment_str = match &m.alignment {
                    AlignmentResult::Ok => "ok".to_string(),
                    AlignmentResult::Warning { task_preview, role } => {
                        format!("warning: '{}' → {}", task_preview, role)
                    }
                    AlignmentResult::None => "none".to_string(),
                };
                serde_json::json!({
                    "agent": m.agent_name,
                    "pending": m.pending_count,
                    "busy_for": m.busy_for,
                    "alignment": alignment_str,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "agents": json_metrics }))?
        );
        return Ok(());
    }

    // Text output
    let is_tty = std::io::stdout().is_terminal();

    if is_tty {
        println!();
        println!(
            "{}",
            format!("  Fleet Status — {}", config.project).bold()
        );
        println!();
    } else {
        println!("Fleet Status — {}", config.project);
        println!();
    }

    if metrics.is_empty() {
        println!("  No active workers.");
        return Ok(());
    }

    // Column widths
    let name_w = metrics
        .iter()
        .map(|m| m.agent_name.len())
        .max()
        .unwrap_or(5)
        .max(5);

    // Header
    if is_tty {
        println!(
            "  {:<name_w$}  {:>7}  {:>8}  {}",
            "Agent".bold(),
            "Pending".bold(),
            "Busy For".bold(),
            "Alignment".bold(),
            name_w = name_w
        );
        println!(
            "  {:<name_w$}  {:>7}  {:>8}  {}",
            "─".repeat(name_w),
            "───────",
            "────────",
            "─────────",
            name_w = name_w
        );
    } else {
        println!(
            "  {:<name_w$}  {:>7}  {:>8}  {}",
            "Agent", "Pending", "Busy For", "Alignment",
            name_w = name_w
        );
        println!(
            "  {:<name_w$}  {:>7}  {:>8}  {}",
            "-".repeat(name_w),
            "-------",
            "--------",
            "---------",
            name_w = name_w
        );
    }

    for m in &metrics {
        let pending_str = format!("{}", m.pending_count);
        let alignment_str = match &m.alignment {
            AlignmentResult::Ok => "✓".to_string(),
            AlignmentResult::Warning { task_preview, role } => {
                format!("⚠ '{}' → {}", task_preview, role)
            }
            AlignmentResult::None => "—".to_string(),
        };

        if is_tty {
            let pending_colored = if m.pending_count > 0 {
                format!("{}", pending_str.yellow())
            } else {
                format!("{}", pending_str)
            };
            let busy_colored = if m.busy_for != "idle" {
                format!("{}", m.busy_for.cyan())
            } else {
                m.busy_for.clone()
            };
            let alignment_colored = match &m.alignment {
                AlignmentResult::Ok => format!("{}", "✓".green()),
                AlignmentResult::Warning { task_preview, role } => {
                    format!("{} '{}' → {}", "⚠".yellow(), task_preview, role)
                }
                AlignmentResult::None => "—".to_string(),
            };
            println!(
                "  {:<name_w$}  {:>7}  {:>8}  {}",
                m.agent_name,
                pending_colored,
                busy_colored,
                alignment_colored,
                name_w = name_w
            );
        } else {
            println!(
                "  {:<name_w$}  {:>7}  {:>8}  {}",
                m.agent_name, pending_str, m.busy_for, alignment_str,
                name_w = name_w
            );
        }
    }

    if is_tty {
        println!();
        println!("  {}", "Hints:".dimmed());
        println!("  {} Prefer agents with 0 pending tasks", "•".dimmed());
        println!(
            "  {} ⚠ alignment = task may be misrouted",
            "•".dimmed()
        );
        println!();
    }

    Ok(())
}
