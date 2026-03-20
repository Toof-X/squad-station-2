use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db};

pub async fn run_freeze(json: bool) -> anyhow::Result<()> {
    let config_path = std::path::Path::new("squad.yml");
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    let agents = db::agents::list_agents(&pool).await?;
    let mut frozen_count = 0u32;

    for agent in &agents {
        // Freeze all non-dead agents (idle, busy)
        if agent.status != "dead" && agent.status != "frozen" {
            db::agents::update_agent_status(&pool, &agent.name, "frozen").await?;
            frozen_count += 1;
        }
    }

    if json {
        let out = serde_json::json!({
            "frozen": frozen_count,
            "total": agents.len(),
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if std::io::stdout().is_terminal() {
        println!(
            "{} Frozen {} agent(s) — orchestrator cannot send tasks until unfreeze",
            "❄".blue(),
            frozen_count
        );
        println!("  User can now interact with agents directly via tmux.");
        println!("  Run `squad-station unfreeze` when ready to hand back to orchestrator.");
    } else {
        println!("Frozen {} agent(s)", frozen_count);
    }

    Ok(())
}

pub async fn run_unfreeze(json: bool) -> anyhow::Result<()> {
    let config_path = std::path::Path::new("squad.yml");
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    let agents = db::agents::list_agents(&pool).await?;
    let mut unfrozen_count = 0u32;

    for agent in &agents {
        if agent.status == "frozen" {
            db::agents::update_agent_status(&pool, &agent.name, "idle").await?;
            unfrozen_count += 1;
        }
    }

    if json {
        let out = serde_json::json!({
            "unfrozen": unfrozen_count,
            "total": agents.len(),
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if std::io::stdout().is_terminal() {
        println!(
            "{} Unfrozen {} agent(s) — orchestrator can now send tasks",
            "✓".green(),
            unfrozen_count
        );
    } else {
        println!("Unfrozen {} agent(s)", unfrozen_count);
    }

    Ok(())
}
