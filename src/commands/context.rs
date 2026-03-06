use crate::{config, db, tmux};

pub async fn run() -> anyhow::Result<()> {
    // 1. Connect to DB
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch all agents
    let agents = db::agents::list_agents(&pool).await?;

    // 3. Reconcile status against tmux for each agent
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

    // 5. Output Markdown
    println!("# Squad Station -- Agent Roster");
    println!();
    println!("## Available Agents");
    println!();
    println!("| Agent | Role | Status | Send Command |");
    println!("|-------|------|--------|--------------|");

    if agents.is_empty() {
        println!("| (none) | — | — | — |");
        println!();
        println!("No agents currently registered. Run `squad-station init` with a squad.yml to set up the squad.");
    } else {
        for agent in &agents {
            println!(
                "| {} | {} | {} | `squad-station send {} \"<task>\"` |",
                agent.name, agent.role, agent.status, agent.name
            );
        }
    }

    println!();
    println!("## Usage");
    println!();
    println!("Send a task to an agent:");
    println!("```");
    println!("squad-station send <agent> \"<task description>\"");
    println!("```");
    println!();
    println!("Check agent status:");
    println!("```");
    println!("squad-station agents");
    println!("```");
    println!();
    println!("Signal task completion (called by hook scripts, not manually):");
    println!("```");
    println!("squad-station signal <agent>");
    println!("```");
    println!();
    println!("List recent messages:");
    println!("```");
    println!("squad-station list --agent <agent>");
    println!("```");
    println!();
    println!("Peek at next pending task:");
    println!("```");
    println!("squad-station peek <agent>");
    println!("```");
    println!();
    println!("## Notes");
    println!();
    println!("- Agents with status \"dead\" have no active tmux session -- they cannot receive tasks");
    println!("- Send only to agents with status \"idle\" for best results");
    println!("- Signal is handled automatically by provider hook scripts");

    Ok(())
}
