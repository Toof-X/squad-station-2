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
    db::agents::insert_agent(
        &pool,
        &config.orchestrator.name,
        &config.orchestrator.provider,
        "orchestrator",
        &config.orchestrator.command,
    )
    .await?;

    // 5. Launch orchestrator tmux session (if not already running)
    let orch_name = &config.orchestrator.name;
    let orch_launched = if tmux::session_exists(orch_name) {
        false
    } else {
        tmux::launch_agent(orch_name, &config.orchestrator.command)?;
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
        if let Err(e) = db::agents::insert_agent(
            &pool,
            &agent.name,
            &agent.provider,
            &agent.role,
            &agent.command,
        )
        .await
        {
            failed.push((agent.name.clone(), format!("{e:#}")));
            continue;
        }

        if tmux::session_exists(&agent.name) {
            skipped += 1;
            skipped_names.push(agent.name.clone());
            continue; // Idempotent: skip already-running agents
        }

        match tmux::launch_agent(&agent.name, &agent.command) {
            Ok(()) => launched += 1,
            Err(e) => failed.push((agent.name.clone(), format!("{e:#}"))),
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
        let project_name = &config.project.name;
        println!(
            "Initialized squad '{}' with {} agent(s)",
            project_name, launched
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
