use std::path::PathBuf;

use crate::{config, db, tmux};

pub async fn run(config_path: PathBuf, json: bool) -> anyhow::Result<()> {
    // 1. Parse squad.yml
    let config = config::load_config(&config_path)?;

    // 2. Resolve DB path
    let db_path = config::resolve_db_path(&config)?;

    // 3. Connect to DB (creates file + runs migrations)
    let pool = db::connect(&db_path).await?;

    // 4. Register orchestrator with hardcoded role="orchestrator"
    // TODO: Plan 03 will complete this — orchestrator name auto-derived from project+tool+role
    let orch_name = config
        .orchestrator
        .name
        .clone()
        .unwrap_or_else(|| format!("{}-{}-orchestrator", config.project, config.orchestrator.tool));
    db::agents::insert_agent(
        &pool,
        &orch_name,
        &config.orchestrator.tool,
        "orchestrator",
        "", // TODO: Plan 03 — command derived from tool
    )
    .await?;

    // 5. Launch orchestrator tmux session (if not already running)
    let orch_launched = if tmux::session_exists(&orch_name) {
        false
    } else {
        tmux::launch_agent(&orch_name, &config.orchestrator.tool)?; // TODO: Plan 03 — real command
        true
    };
    let orch_skipped = !orch_launched;

    // 6. Register and launch each worker agent — continue on partial failure
    let mut failed: Vec<(String, String)> = vec![];
    let mut skipped_names: Vec<String> = vec![];
    let mut launched: u32 = if orch_launched { 1 } else { 0 };
    let mut skipped: u32 = if orch_skipped { 1 } else { 0 };

    if orch_skipped {
        skipped_names.push(orch_name.clone());
    }

    for agent in &config.agents {
        // TODO: Plan 03 will complete this — full agent name + command derivation
        let agent_name = agent
            .name
            .clone()
            .unwrap_or_else(|| format!("{}-{}-worker", config.project, agent.tool));
        if let Err(e) = db::agents::insert_agent(
            &pool,
            &agent_name,
            &agent.tool,
            &agent.role,
            "", // TODO: Plan 03 — command derived from tool
        )
        .await
        {
            failed.push((agent_name.clone(), format!("{e:#}")));
            continue;
        }

        if tmux::session_exists(&agent_name) {
            skipped += 1;
            skipped_names.push(agent_name.clone());
            continue; // Idempotent: skip already-running agents
        }

        match tmux::launch_agent(&agent_name, &agent.tool) { // TODO: Plan 03 — real command
            Ok(()) => launched += 1,
            Err(e) => failed.push((agent_name.clone(), format!("{e:#}"))),
        }
    }

    // 7. Output results
    let db_path_str = db_path.display().to_string();

    if json {
        let output = serde_json::json!({
            "launched": launched,
            "skipped": skipped,
            "failed": failed,
            "db_path": db_path_str,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!(
            "Initialized squad '{}' with {} agent(s)",
            config.project, launched
        );
        for name in &skipped_names {
            println!("  - {}: already running (skipped)", name);
        }
        for (name, error) in &failed {
            println!("  x {}: {}", name, error);
        }
        println!("  Database: {}", db_path_str);
    }

    // 8. Exit code: return Err only if ALL agents failed (including orchestrator)
    let total = config.agents.len() + 1; // +1 for orchestrator
    if !failed.is_empty() && failed.len() == total {
        anyhow::bail!("All {} agent(s) failed to launch", total);
    }

    Ok(())
}
