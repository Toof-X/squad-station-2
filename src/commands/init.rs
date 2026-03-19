use std::io::IsTerminal;
use std::path::PathBuf;

use owo_colors::OwoColorize;
use owo_colors::Stream;

use crate::{config, db, tmux};

/// Choice returned by the re-init prompt when squad.yml already exists.
enum ReinitChoice {
    Overwrite,
    AddAgents,
    Abort,
}

/// Display a re-init menu and read a single keypress from stdin.
/// Returns the user's choice. Ctrl+C and Esc both map to Abort.
fn prompt_reinit() -> anyhow::Result<ReinitChoice> {
    use crossterm::{event, terminal};

    println!("squad.yml already exists. What would you like to do?");
    println!("  [o] Overwrite — replace with new wizard config");
    println!("  [a] Add agents — add more workers to existing config");
    println!("  [q] Abort — exit without changes");

    terminal::enable_raw_mode()?;
    let choice = loop {
        match event::read() {
            Ok(event::Event::Key(key)) => {
                if key.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    crossterm::event::KeyCode::Char('o') => break ReinitChoice::Overwrite,
                    crossterm::event::KeyCode::Char('a') => break ReinitChoice::AddAgents,
                    crossterm::event::KeyCode::Char('q') => break ReinitChoice::Abort,
                    crossterm::event::KeyCode::Esc => break ReinitChoice::Abort,
                    crossterm::event::KeyCode::Char('c')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break ReinitChoice::Abort;
                    }
                    _ => {} // ignore other keys, loop again
                }
            }
            Ok(_) => {} // non-key events: ignore
            Err(e) => {
                terminal::disable_raw_mode()?;
                return Err(e.into());
            }
        }
    };
    terminal::disable_raw_mode()?;
    Ok(choice)
}

/// Append new worker agent entries to existing squad.yml content.
/// Preserves all existing content; new entries are appended at the end.
/// If new_workers is empty, returns the original content unchanged.
fn append_workers_to_yaml(
    existing_yaml: &str,
    new_workers: &[crate::commands::wizard::AgentInput],
) -> String {
    let mut result = existing_yaml.to_string();
    if new_workers.is_empty() {
        return result;
    }
    // Ensure trailing newline before appending
    if !result.ends_with('\n') {
        result.push('\n');
    }
    for agent in new_workers {
        result.push_str(&format!("  - provider: {}\n", agent.provider));
        if !agent.name.is_empty() {
            result.push_str(&format!("    name: {}\n", agent.name));
        }
        result.push_str("    role: worker\n");
        if let Some(ref model) = agent.model {
            result.push_str(&format!("    model: {}\n", model));
        }
        if let Some(ref desc) = agent.description {
            result.push_str(&format!("    description: {}\n", desc));
        }
    }
    result
}

pub async fn run(mut config_path: PathBuf, json: bool, tui: bool) -> anyhow::Result<()> {
    let mut purge_db_on_init = false;
    // Carries routing_hints from wizard result to DB insertion.
    // routing_hints are NOT stored in squad.yml, so they must be kept separately.
    let mut wizard_routing_hints: Option<std::collections::HashMap<String, Option<String>>> = None;

    if !config_path.exists() {
        if tui {
            // --tui: run interactive wizard to generate squad.yml
            match crate::commands::wizard::run().await? {
                Some(result) => {
                    // Change to install directory if different from CWD
                    let install_dir = std::path::PathBuf::from(&result.install_dir);
                    if install_dir.is_absolute() || install_dir.exists() {
                        std::fs::create_dir_all(&install_dir)?;
                        std::env::set_current_dir(&install_dir)?;
                        config_path = install_dir.join("squad.yml");
                    }
                    // Capture routing hints before result fields are moved into yaml generation
                    wizard_routing_hints = Some(extract_routing_hints(&result));
                    let yaml = generate_squad_yml(&result);
                    std::fs::write(&config_path, &yaml)?;
                    create_sdd_playbook(&config_path, &result);
                    println!("Generated squad.yml for project '{}' in {}", result.project, result.install_dir);
                    // Fall through to load_config below
                }
                None => {
                    println!("Init cancelled.");
                    return Ok(());
                }
            }
        } else {
            // Non-TUI: notify that squad.yml is missing and exit
            eprintln!(
                "No squad.yml found. Run `squad-station init --tui` to create one interactively."
            );
            return Ok(());
        }
    } else if tui && std::io::stdin().is_terminal() {
        // --tui + squad.yml exists: show re-init prompt
        match prompt_reinit()? {
            ReinitChoice::Overwrite => {
                // Kill existing sessions from the old config before overwriting
                if let Ok(old_cfg) = crate::config::load_config(&config_path) {
                    kill_config_sessions(&old_cfg);
                }
                purge_db_on_init = true;
                match crate::commands::wizard::run().await? {
                    Some(result) => {
                        // Capture routing hints before result fields are moved into yaml generation
                        wizard_routing_hints = Some(extract_routing_hints(&result));
                        let yaml = generate_squad_yml(&result);
                        std::fs::write(&config_path, &yaml)?;
                        create_sdd_playbook(&config_path, &result);
                        println!("Replaced squad.yml for project '{}'", result.project);
                        // Fall through to load_config below
                    }
                    None => {
                        println!("Init cancelled.");
                        return Ok(());
                    }
                }
            }
            ReinitChoice::AddAgents => {
                // Parse existing config so the wizard can show old agents on the Review page
                let existing = std::fs::read_to_string(&config_path)?;
                let (existing_orchestrator, existing_workers) =
                    match crate::config::load_config(&config_path) {
                        Ok(cfg) => {
                            let orch = {
                                let n = cfg.orchestrator.name.as_deref().unwrap_or("orchestrator");
                                format!("{} ({})", n, cfg.orchestrator.provider)
                            };
                            let workers = cfg.agents.iter().map(|a| {
                                let n = a.name.as_deref().unwrap_or("worker");
                                format!("{} ({})", n, a.provider)
                            }).collect();
                            (Some(orch), workers)
                        }
                        Err(_) => (None, vec![]),
                    };
                match crate::commands::wizard::run_worker_only(existing_orchestrator, existing_workers).await? {
                    Some(new_workers) => {
                        // Carry routing hints for new workers by agent name
                        let worker_hints: std::collections::HashMap<String, Option<String>> =
                            new_workers.iter()
                                .map(|w| (w.name.clone(), w.routing_hints.clone()))
                                .collect();
                        wizard_routing_hints = Some(worker_hints);
                        let updated = append_workers_to_yaml(&existing, &new_workers);
                        std::fs::write(&config_path, &updated)?;
                        println!("Added {} worker(s) to squad.yml", new_workers.len());
                        // Fall through to load_config below
                    }
                    None => {
                        println!("Init cancelled.");
                        return Ok(());
                    }
                }
            }
            ReinitChoice::Abort => {
                println!("Init aborted.");
                return Ok(());
            }
        }
    }
    // Non-TUI + squad.yml exists: fall through directly to load_config

    // 1. Parse squad.yml
    let config = config::load_config(&config_path)?;

    // 2. Resolve DB path
    let db_path = config::resolve_db_path(&config)?;

    // 3. Connect to DB (creates file + runs migrations)
    let pool = db::connect(&db_path).await?;

    // On overwrite: purge stale agents so the DB matches the new config exactly
    if purge_db_on_init {
        let _ = db::agents::delete_all_agents(&pool).await;
    }

    // 4. Register orchestrator with hardcoded role="orchestrator"
    let orch_role = config
        .orchestrator
        .name
        .as_deref()
        .unwrap_or("orchestrator");
    let orch_name = config::sanitize_session_name(&format!("{}-{}", config.project, orch_role));
    // Look up routing hints by the raw role name (orch_role) not the sanitized session name
    let orch_hints = wizard_routing_hints.as_ref()
        .and_then(|m| m.get(orch_role).cloned())
        .flatten();
    db::agents::insert_agent(
        &pool,
        &orch_name,
        &config.orchestrator.provider,
        "orchestrator",
        config.orchestrator.model.as_deref(),
        config.orchestrator.description.as_deref(),
        orch_hints.as_deref(),
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
        let hints = wizard_routing_hints.as_ref()
            .and_then(|m| m.get(role_suffix).cloned())
            .flatten();
        if let Err(e) = db::agents::insert_agent(
            &pool,
            &agent_name,
            &agent.provider,
            &agent.role,
            agent.model.as_deref(),
            agent.description.as_deref(),
            hints.as_deref(),
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
                (".claude/settings.json", "PostToolUse", "AskUserQuestion"),
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

        // Reconcile agent statuses before printing diagram
        crate::commands::helpers::reconcile_agent_statuses(&pool).await?;
        let agents = db::agents::list_agents(&pool).await?;
        crate::commands::diagram::print_diagram(&agents);
    }

    Ok(())
}

/// Kill all tmux sessions (orchestrator + workers + monitor) for a given config.
/// Used before overwriting squad.yml so stale sessions don't persist.
fn kill_config_sessions(cfg: &config::SquadConfig) {
    let orch_role = cfg.orchestrator.name.as_deref().unwrap_or("orchestrator");
    let orch_name = config::sanitize_session_name(&format!("{}-{}", cfg.project, orch_role));
    let monitor_name = config::sanitize_session_name(&format!("{}-monitor", cfg.project));

    let _ = tmux::kill_session(&orch_name);
    let _ = tmux::kill_session(&monitor_name);

    for agent in &cfg.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        let session_name = config::sanitize_session_name(&format!("{}-{}", cfg.project, role_suffix));
        let _ = tmux::kill_session(&session_name);
    }
}

/// Write the SDD playbook file to `.squad/sdd/<name>-playbook.md`.
/// Creates the directory if it doesn't exist. Skips silently if the file already exists.
fn create_sdd_playbook(
    config_path: &std::path::Path,
    result: &crate::commands::wizard::WizardResult,
) {
    let project_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    let sdd_dir = project_dir.join(".squad").join("sdd");
    if let Err(e) = std::fs::create_dir_all(&sdd_dir) {
        eprintln!("Warning: could not create .squad/sdd/: {}", e);
        return;
    }
    let filename = format!("{}-playbook.md", result.sdd.as_str());
    let playbook_path = sdd_dir.join(&filename);
    if playbook_path.exists() {
        return; // already present — don't overwrite user edits
    }
    if let Err(e) = std::fs::write(&playbook_path, result.sdd.playbook_content()) {
        eprintln!("Warning: could not write {}: {}", playbook_path.display(), e);
    }
}

/// Generate a squad.yml YAML string from a completed WizardResult.
/// Optional fields (name, model, description) are omitted when empty/None.
fn generate_squad_yml(result: &crate::commands::wizard::WizardResult) -> String {
    let mut yaml = format!("project: {}\n", result.project);

    // SDD section
    let sdd_name = result.sdd.as_str();
    yaml.push_str(&format!(
        "\nsdd:\n  - name: {}\n    playbook: \".squad/sdd/{}-playbook.md\"\n",
        sdd_name, sdd_name
    ));

    // Orchestrator section
    yaml.push_str("\norchestrator:\n");
    yaml.push_str(&format!("  provider: {}\n", result.orchestrator.provider));
    if !result.orchestrator.name.is_empty() {
        yaml.push_str(&format!("  name: {}\n", result.orchestrator.name));
    }
    yaml.push_str("  role: orchestrator\n");
    if let Some(ref model) = result.orchestrator.model {
        yaml.push_str(&format!("  model: {}\n", model));
    }
    if let Some(ref desc) = result.orchestrator.description {
        yaml.push_str(&format!("  description: {}\n", desc));
    }

    // Agents section
    yaml.push_str("\nagents:\n");
    for agent in &result.agents {
        yaml.push_str(&format!("  - provider: {}\n", agent.provider));
        if !agent.name.is_empty() {
            yaml.push_str(&format!("    name: {}\n", agent.name));
        }
        yaml.push_str("    role: worker\n");
        if let Some(ref model) = agent.model {
            yaml.push_str(&format!("    model: {}\n", model));
        }
        if let Some(ref desc) = agent.description {
            yaml.push_str(&format!("    description: {}\n", desc));
        }
    }

    yaml
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

/// Install Claude Code hooks: Stop (signal) + Notification (notify) + PostToolUse (AskUserQuestion)
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
        },
        {
            "matcher": "elicitation_dialog",
            "hooks": [{"type": "command", "command": notify_cmd}]
        }
    ]);

    // PostToolUse hook — agent is asking the user a question → notify orchestrator.
    // Orchestrator reads the actual question via capture-pane.
    settings["hooks"]["PostToolUse"] = serde_json::json!([
        {
            "matcher": "AskUserQuestion",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::wizard::{AgentInput, SddWorkflow, WizardResult};

    fn make_worker(provider: &str, name: &str) -> AgentInput {
        AgentInput {
            name: name.to_string(),
            role: "worker".to_string(),
            provider: provider.to_string(),
            model: None,
            description: None,
            routing_hints: None,
        }
    }

    fn make_worker_with_model(provider: &str, name: &str, model: &str) -> AgentInput {
        AgentInput {
            name: name.to_string(),
            role: "worker".to_string(),
            provider: provider.to_string(),
            model: Some(model.to_string()),
            description: None,
            routing_hints: None,
        }
    }

    #[test]
    fn test_append_workers_to_yaml_adds_entries() {
        let existing = "project: test\nagents:\n  - provider: claude-code\n    role: worker\n";
        let new_workers = vec![
            make_worker("gemini-cli", "worker2"),
            make_worker("claude-code", "worker3"),
        ];
        let result = append_workers_to_yaml(existing, &new_workers);
        assert!(result.contains("provider: gemini-cli"), "Must contain new gemini-cli worker");
        assert!(result.contains("provider: claude-code"), "Must contain existing + new claude-code");
        // Count occurrences of "provider:" — should be 3 (1 existing + 2 new)
        let count = result.matches("provider:").count();
        assert_eq!(count, 3, "Expected 3 provider entries, got {}", count);
    }

    #[test]
    fn test_append_workers_to_yaml_preserves_existing() {
        let existing = "project: my-project\norchestrator:\n  provider: claude-code\n  role: orchestrator\nagents:\n  - provider: gemini-cli\n    role: worker\n";
        let new_workers = vec![make_worker("claude-code", "new-worker")];
        let result = append_workers_to_yaml(existing, &new_workers);
        // Existing content must appear at the start unchanged
        assert!(
            result.starts_with(existing),
            "Result must start with existing YAML content.\nExisting: {}\nResult: {}",
            existing,
            result
        );
    }

    #[test]
    fn test_append_workers_to_yaml_empty_workers() {
        let existing = "project: test\nagents:\n  - provider: claude-code\n    role: worker\n";
        let result = append_workers_to_yaml(existing, &[]);
        // Should be identical to input (or at most have trailing newline)
        assert!(
            result.starts_with(existing),
            "Empty workers must not modify existing content"
        );
        assert_eq!(
            result.matches("provider:").count(),
            1,
            "Must have exactly 1 provider entry when no workers added"
        );
    }

    #[test]
    fn test_append_workers_to_yaml_includes_name_when_nonempty() {
        let existing = "agents:\n";
        let workers = vec![make_worker("claude-code", "my-agent")];
        let result = append_workers_to_yaml(existing, &workers);
        assert!(result.contains("name: my-agent"), "Named worker must have name field");
    }

    #[test]
    fn test_append_workers_to_yaml_omits_name_when_empty() {
        let existing = "agents:\n";
        let workers = vec![make_worker("claude-code", "")];
        let result = append_workers_to_yaml(existing, &workers);
        assert!(!result.contains("name:"), "Unnamed worker must not have name field");
    }

    #[test]
    fn test_append_workers_to_yaml_includes_model_when_present() {
        let existing = "agents:\n";
        let workers = vec![make_worker_with_model("claude-code", "", "sonnet")];
        let result = append_workers_to_yaml(existing, &workers);
        assert!(result.contains("model: sonnet"), "Model must appear in output");
    }

    fn make_wizard_result() -> WizardResult {
        WizardResult {
            install_dir: ".".to_string(),
            project: "my-project".to_string(),
            sdd: SddWorkflow::GetShitDone,
            orchestrator: AgentInput {
                name: "orch".to_string(),
                role: "orchestrator".to_string(),
                provider: "claude-code".to_string(),
                model: Some("sonnet".to_string()),
                description: Some("main orchestrator".to_string()),
                routing_hints: None,
            },
            agents: vec![AgentInput {
                name: "backend".to_string(),
                role: "worker".to_string(),
                provider: "gemini-cli".to_string(),
                model: Some("gemini-2.5-pro".to_string()),
                description: None,
                routing_hints: None,
            }],
        }
    }

    #[test]
    fn test_generate_squad_yml_contains_required_sections() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(yaml.starts_with("project: my-project\n"), "YAML must start with project: line");
        assert!(yaml.contains("sdd:"), "YAML must contain sdd section");
        assert!(yaml.contains("orchestrator:"), "YAML must contain orchestrator section");
        assert!(yaml.contains("agents:"), "YAML must contain agents section");
    }

    #[test]
    fn test_generate_squad_yml_sdd_playbook_path() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(
            yaml.contains("playbook: \".squad/sdd/gsd-playbook.md\""),
            "SDD playbook path must be correct, got:\n{}",
            yaml
        );
    }

    #[test]
    fn test_generate_squad_yml_orchestrator_fields() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(yaml.contains("role: orchestrator"), "orchestrator must have role: orchestrator");
        assert!(yaml.contains("provider: claude-code"), "orchestrator provider must be set");
        assert!(
            yaml.contains("model: sonnet"),
            "orchestrator model must be set"
        );
    }

    #[test]
    fn test_generate_squad_yml_omits_empty_name() {
        let mut result = make_wizard_result();
        result.orchestrator.name = "".to_string();
        result.agents[0].name = "".to_string();
        let yaml = generate_squad_yml(&result);
        // Name should not appear if it's empty
        // The orchestrator line itself won't have a name: field
        let lines: Vec<&str> = yaml.lines().collect();
        // Check no "name: " line appears in orchestrator section (between "orchestrator:" and "agents:")
        let orch_start = lines.iter().position(|l| l.trim() == "orchestrator:").unwrap();
        let agents_start = lines.iter().position(|l| l.trim() == "agents:").unwrap();
        let orch_lines: Vec<&str> = lines[orch_start..agents_start].to_vec();
        assert!(
            !orch_lines.iter().any(|l| l.contains("name:")),
            "Empty name must be omitted from orchestrator section"
        );
    }

    #[test]
    fn test_generate_squad_yml_omits_none_model() {
        let mut result = make_wizard_result();
        result.agents[0].model = None;
        let yaml = generate_squad_yml(&result);
        let lines: Vec<&str> = yaml.lines().collect();
        let agents_start = lines.iter().position(|l| l.trim() == "agents:").unwrap();
        let agent_lines: Vec<&str> = lines[agents_start..].to_vec();
        // The worker agent had no model; model: line should not appear in agents section
        // But the orchestrator still has a model, so we check count
        let model_lines_in_agents: Vec<&str> =
            agent_lines.iter().filter(|l| l.contains("model:")).copied().collect();
        assert!(
            model_lines_in_agents.is_empty(),
            "None model must be omitted from worker section"
        );
    }

    #[test]
    fn test_generate_squad_yml_roundtrips_through_serde() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        let config: Result<crate::config::SquadConfig, _> = serde_saphyr::from_str(&yaml);
        assert!(
            config.is_ok(),
            "Generated YAML must parse as SquadConfig, got error: {:?}",
            config.err()
        );
        let config = config.unwrap();
        assert_eq!(config.project, "my-project");
    }

    #[test]
    fn test_install_claude_hooks_includes_post_tool_use() {
        let tmp = tempfile::TempDir::new().unwrap();
        let settings_file = tmp.path().join(".claude").join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_claude_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify Stop hook exists
        assert!(settings["hooks"]["Stop"].is_array(), "Stop hook must exist");

        // Verify Notification hook exists with both matchers
        let notif = &settings["hooks"]["Notification"];
        assert!(notif.is_array(), "Notification hook must exist");
        assert_eq!(notif.as_array().unwrap().len(), 2);
        assert_eq!(
            notif[0]["matcher"].as_str().unwrap(),
            "permission_prompt"
        );
        assert_eq!(
            notif[1]["matcher"].as_str().unwrap(),
            "elicitation_dialog"
        );

        // Verify PostToolUse hook exists with AskUserQuestion matcher
        let ptu = &settings["hooks"]["PostToolUse"];
        assert!(ptu.is_array(), "PostToolUse hook must exist");
        assert_eq!(
            ptu[0]["matcher"].as_str().unwrap(),
            "AskUserQuestion"
        );

        // Verify the command calls notify with the standard pattern
        let cmd = ptu[0]["hooks"][0]["command"].as_str().unwrap();
        assert!(
            cmd.contains("squad-station notify"),
            "PostToolUse command must call squad-station notify"
        );
    }

    #[test]
    fn test_install_claude_hooks_preserves_existing_settings() {
        let tmp = tempfile::TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let settings_file = claude_dir.join("settings.json");

        // Pre-populate with existing settings
        let existing = serde_json::json!({
            "customKey": "preserved",
            "hooks": {
                "PreToolUse": [{"matcher": "Bash", "hooks": []}]
            }
        });
        std::fs::write(&settings_file, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let settings_str = settings_file.to_str().unwrap();
        install_claude_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Existing keys preserved
        assert_eq!(settings["customKey"].as_str().unwrap(), "preserved");
        // Existing hooks preserved
        assert!(settings["hooks"]["PreToolUse"].is_array());
        // New hooks added
        assert!(settings["hooks"]["PostToolUse"].is_array());
        assert!(settings["hooks"]["Stop"].is_array());
        assert!(settings["hooks"]["Notification"].is_array());
    }
}

/// Extract routing_hints map from wizard result for DB insertion.
/// Returns HashMap<raw_agent_name, Option<String>> for orchestrator + all workers.
/// Keys are the raw names from the wizard (e.g. "orch", "coder") not sanitized session names.
/// routing_hints are NOT stored in squad.yml so they must be captured before
/// the WizardResult is consumed by generate_squad_yml.
fn extract_routing_hints(
    result: &crate::commands::wizard::WizardResult,
) -> std::collections::HashMap<String, Option<String>> {
    let mut map = std::collections::HashMap::new();
    // Orchestrator: key is the raw name (matches orch_role variable in run())
    let orch_name = result.orchestrator.name.clone();
    map.insert(orch_name, result.orchestrator.routing_hints.clone());
    for agent in &result.agents {
        // Workers: key is the raw name (matches role_suffix variable in the agents loop)
        map.insert(agent.name.clone(), agent.routing_hints.clone());
    }
    map
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
