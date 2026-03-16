use std::path::PathBuf;

use owo_colors::OwoColorize;
use owo_colors::Stream;

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
    let orch_name = config::sanitize_session_name(&format!("{}-{}", config.project, orch_role));
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
        // Orchestrator launches at project root.
        // Context loaded via /squad-orchestrator slash command.
        let project_root = db_path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(std::path::Path::new("."));
        let project_root_str = project_root.to_string_lossy().to_string();
        let cmd = get_launch_command(&config.orchestrator);
        tmux::launch_agent_in_dir(&orch_name, &cmd, &project_root_str)?;
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
        let agent_name =
            config::sanitize_session_name(&format!("{}-{}", config.project, role_suffix));
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

        // GAP-05: Workers launch at project root directory
        let project_root = db_path
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(std::path::Path::new("."));
        let project_root_str = project_root.to_string_lossy().to_string();
        let cmd = get_launch_command(agent);
        match tmux::launch_agent_in_dir(&agent_name, &cmd, &project_root_str) {
            Ok(()) => launched += 1,
            Err(e) => failed.push((agent_name.clone(), format!("{e:#}"))),
        }
    }

    // 7. Create monitor session with interactive panes for all agents
    let monitor_name = format!("{}-monitor", config.project);
    let mut monitor_sessions: Vec<String> = vec![];
    if !config.orchestrator.is_db_only() {
        monitor_sessions.push(orch_name.clone());
    }
    for agent in &config.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        let agent_name =
            config::sanitize_session_name(&format!("{}-{}", config.project, role_suffix));
        monitor_sessions.push(agent_name);
    }
    // Kill existing monitor session before recreating
    tmux::kill_session(&monitor_name)?;
    let monitor_created = if !monitor_sessions.is_empty() {
        tmux::create_view_session(&monitor_name, &monitor_sessions).is_ok()
    } else {
        false
    };

    // 8. Output results
    let db_path_str = db_path.display().to_string();

    if json {
        let output = serde_json::json!({
            "launched": launched,
            "skipped": skipped,
            "failed": failed,
            "db_path": db_path_str,
            "monitor": if monitor_created { Some(&monitor_name) } else { None },
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        let total_agents = config.agents.len() + 1; // workers + orchestrator
        println!(
            "Initialized squad '{}' with {} agent(s) ({} launched, {} skipped)",
            config.project, total_agents, launched, skipped
        );
        for name in &skipped_names {
            println!("  - {}: already running (skipped)", name);
        }
        for name in &db_only_names {
            println!(
                "  {}: db-only (antigravity orchestrator — no tmux session)",
                name
            );
        }
        for (name, error) in &failed {
            println!("  x {}: {}", name, error);
        }
        println!("  Database: {}", db_path_str);
    }

    // 8. Exit code: return Err only if ALL agents failed (including orchestrator)
    // DB-only orchestrator is excluded from total: it is never launched and never fails.
    let total = config.agents.len()
        + if config.orchestrator.is_db_only() {
            0
        } else {
            1
        };
    if !failed.is_empty() && failed.len() == total {
        anyhow::bail!("All {} agent(s) failed to launch", total);
    }

    // 9. Hook setup: auto-install or print instructions
    // In JSON mode, skip stdout instructions (to preserve machine-parseable output).
    if !json {
        let green = |s: &str| {
            s.if_supports_color(Stream::Stdout, |s| s.green())
                .to_string()
        };
        let cyan = |s: &str| {
            s.if_supports_color(Stream::Stdout, |s| s.cyan())
                .to_string()
        };
        let yellow = |s: &str| {
            s.if_supports_color(Stream::Stdout, |s| s.yellow())
                .to_string()
        };
        let bold = |s: &str| {
            s.if_supports_color(Stream::Stdout, |s| s.bold())
                .to_string()
        };

        println!("\n{}", green("══════════════════════════════════"));
        println!("  {}", bold("Squad Setup Complete"));
        println!("{}\n", green("══════════════════════════════════"));

        let hook_installed = auto_install_hooks(&config.orchestrator.provider).unwrap_or(false);
        if hook_installed {
            println!("  Hooks: installed to settings file");
        } else {
            println!("Please manually configure the following hooks to enable task completion signals:\n");
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

        println!("\nGenerating orchestrator context...");
        if let Err(e) = crate::commands::context::run().await {
            println!("Warning: Failed to generate context files: {}", e);
        }

        println!("\n{}", bold("Get Started:"));
        println!();
        println!("  1. Attach to the orchestrator session:");
        println!("     {}", cyan(&format!("tmux attach -t {}", orch_name)));
        println!();
        println!("  2. Load the orchestrator context by typing:");
        println!("     {}", yellow("/squad-orchestrator"));
        if monitor_created {
            println!();
            println!("  Monitor all agents (interactive panes):");
            println!("     {}", cyan(&format!("tmux attach -t {}", monitor_name)));
        }
        println!();
        println!("  Monitor all agents (read-only view):");
        println!("     {}", cyan("squad-station view"));
        println!();
    }

    Ok(())
}

fn auto_install_hooks(provider: &str) -> anyhow::Result<bool> {
    match provider {
        "claude-code" => install_claude_hooks(".claude/settings.json"),
        "gemini-cli" => install_gemini_hooks(".gemini/settings.json"),
        _ => Ok(false), // unknown provider: skip auto-install
    }
}

/// Read or create a settings JSON file, returning the parsed value.
/// Creates a .bak backup if the file already exists.
fn read_or_create_settings(settings_file: &str) -> anyhow::Result<serde_json::Value> {
    let settings_path = std::path::Path::new(settings_file);

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match std::fs::read_to_string(settings_path) {
        Ok(content) => {
            std::fs::write(settings_path.with_extension("json.bak"), &content)?;
            match serde_json::from_str(&content) {
                Ok(v) => Ok(v),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse {}: {}. Starting fresh.",
                        settings_file, e
                    );
                    Ok(serde_json::json!({}))
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(e) => Err(e.into()),
    }
}

/// Install Claude Code hooks: Stop (signal) + Notification (notify)
fn install_claude_hooks(settings_file: &str) -> anyhow::Result<bool> {
    let mut settings = read_or_create_settings(settings_file)?;
    let signal_cmd = "squad-station signal $(tmux display-message -p '#S')";
    let notify_cmd =
        "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')";

    // Stop hook — agent finished task → signal completion
    settings["hooks"]["Stop"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{"type": "command", "command": signal_cmd}]
    }]);

    // Notification hook — agent needs permission approval → notify orchestrator
    // Only permission_prompt triggers notify. idle_prompt must NOT trigger notify
    // because idle = agent finished and is waiting for next task, which causes a
    // notification loop: idle → notify orchestrator → orchestrator sends task → idle → notify...
    settings["hooks"]["Notification"] = serde_json::json!([
        {
            "matcher": "permission_prompt",
            "hooks": [{"type": "command", "command": notify_cmd}]
        }
    ]);

    std::fs::write(settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Install Gemini CLI hooks: AfterAgent (signal) + Notification (notify)
fn install_gemini_hooks(settings_file: &str) -> anyhow::Result<bool> {
    let mut settings = read_or_create_settings(settings_file)?;
    let signal_cmd = "squad-station signal $(tmux display-message -p '#S')";
    let notify_cmd =
        "squad-station notify --body 'Agent needs input' --agent $(tmux display-message -p '#S')";

    settings["hooks"]["AfterAgent"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{"type": "command", "command": signal_cmd}]
    }]);

    settings["hooks"]["Notification"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{"type": "command", "command": notify_cmd}]
    }]);

    std::fs::write(settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Build the launch command for a tmux session based on provider and model.
/// Claude Code: `claude --dangerously-skip-permissions --model <model>`
/// Gemini CLI: `gemini -y --model <model>`
/// Unknown/no model: plain `zsh` shell
fn get_launch_command(agent: &config::AgentConfig) -> String {
    match agent.provider.as_str() {
        "claude-code" => {
            let mut cmd = "claude --dangerously-skip-permissions".to_string();
            if let Some(model) = &agent.model {
                cmd.push_str(&format!(" --model {}", model));
            }
            cmd
        }
        "gemini-cli" => {
            let mut cmd = "gemini -y".to_string();
            if let Some(model) = &agent.model {
                cmd.push_str(&format!(" --model {}", model));
            }
            cmd
        }
        _ => "zsh".to_string(), // Unknown provider: open shell, user launches manually
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
