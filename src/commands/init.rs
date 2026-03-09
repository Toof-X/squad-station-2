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
    let orch_role = config
        .orchestrator
        .name
        .as_deref()
        .unwrap_or("orchestrator");
    let orch_name = format!(
        "{}-{}-{}",
        config.project, config.orchestrator.provider, orch_role
    );
    db::agents::insert_agent(
        &pool,
        &orch_name,
        &config.orchestrator.provider,
        "orchestrator",
        config.orchestrator.model.as_deref(),
        config.orchestrator.description.as_deref(),
    )
    .await?;

    // 5. Launch orchestrator tmux session (or skip if db-only provider)
    let mut db_only_names: Vec<String> = vec![];
    let orch_launched = if config.orchestrator.is_db_only() {
        // Antigravity: DB-only orchestrator — register to DB only, no tmux session.
        db_only_names.push(orch_name.clone());
        false
    } else if tmux::session_exists(&orch_name) {
        false
    } else {
        let cmd = get_launch_command(&config.orchestrator.provider);
        tmux::launch_agent(&orch_name, cmd)?;
        true
    };
    let orch_skipped = !orch_launched && !config.orchestrator.is_db_only();

    // 6. Register and launch each worker agent — continue on partial failure
    let mut failed: Vec<(String, String)> = vec![];
    let mut skipped_names: Vec<String> = vec![];
    let mut launched: u32 = if orch_launched { 1 } else { 0 };
    let mut skipped: u32 = if orch_skipped { 1 } else { 0 };

    if orch_skipped {
        skipped_names.push(orch_name.clone());
    }

    for agent in &config.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        let agent_name = format!("{}-{}-{}", config.project, agent.provider, role_suffix);
        if let Err(e) = db::agents::insert_agent(
            &pool,
            &agent_name,
            &agent.provider,
            &agent.role,
            agent.model.as_deref(),
            agent.description.as_deref(),
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

        let cmd = get_launch_command(&agent.provider);
        match tmux::launch_agent(&agent_name, cmd) {
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
        for name in &db_only_names {
            println!("  {}: db-only (antigravity orchestrator — no tmux session)", name);
        }
        for (name, error) in &failed {
            println!("  x {}: {}", name, error);
        }
        println!("  Database: {}", db_path_str);
    }

    // 8. Exit code: return Err only if ALL agents failed (including orchestrator)
    // DB-only orchestrator is excluded from total: it is never launched and never fails.
    let total = config.agents.len() + if config.orchestrator.is_db_only() { 0 } else { 1 };
    if !failed.is_empty() && failed.len() == total {
        anyhow::bail!("All {} agent(s) failed to launch", total);
    }

    // 9. Hook setup: merge into settings.json or print instructions
    // In JSON mode, skip stdout instructions (to preserve machine-parseable output).
    if !json {
        let providers: &[(&str, &str, &str)] = &[
            (".claude/settings.json", "Stop", "*"),
            (".claude/settings.json", "Notification", "permission_prompt"),
            (".claude/settings.json", "Notification", "idle_prompt"),
            (".gemini/settings.json", "AfterAgent", "*"),
            (".gemini/settings.json", "Notification", "*"),
        ];
        for &(settings_path, hook_event, matcher) in providers {
            print_hook_instructions(settings_path, hook_event, matcher);
        }
    }

    Ok(())
}

fn get_launch_command(provider: &str) -> &str {
    match provider {
        "claude-code" => "claude",
        "gemini-cli" => "gemini",
        other => other,
    }
}


fn print_hook_instructions(settings_path: &str, event: &str, matcher: &str) {
    println!(
        "\nHook setup instructions for {} (event: {}):\n\n  \
        Create the file with the following content, or add to your existing hooks:\n\n  \
        {{\n    \"hooks\": {{\n      \"{}\": [\n        \
        {{ \"matcher\": \"{}\", \"hooks\": [ {{ \"type\": \"command\", \"command\": \"squad-station signal $(tmux display-message -p '#S')\" }} ] }}\n      \
        ]\n    }}\n  }}",
        settings_path, event, event, matcher
    );
}
