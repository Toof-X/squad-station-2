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
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
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
#[derive(serde::Serialize)]
struct SquadYmlSdd<'a> {
    name: &'a str,
    playbook: String,
}

#[derive(serde::Serialize)]
struct SquadYmlAgent<'a> {
    provider: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    role: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channels: Option<Vec<String>>,
}

#[derive(serde::Serialize)]
struct SquadYmlDoc<'a> {
    project: &'a str,
    sdd: Vec<SquadYmlSdd<'a>>,
    orchestrator: SquadYmlAgent<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    agents: Vec<SquadYmlAgent<'a>>,
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
        let single = SquadYmlAgent {
            provider: &agent.provider,
            name: if agent.name.is_empty() {
                None
            } else {
                Some(&agent.name)
            },
            role: "worker",
            model: agent.model.as_deref(),
            description: agent.description.as_deref(),
            channels: agent.channels.clone(),
        };
        let yaml_str = serde_saphyr::to_string(&single).unwrap_or_default();
        let cleaned = yaml_str.replace("---\n", "");
        let mut lines = cleaned.lines();
        if let Some(first) = lines.next() {
            result.push_str(&format!("  - {}\n", first));
        }
        for line in lines {
            result.push_str(&format!("    {}\n", line));
        }
    }
    result
}

pub async fn run(mut config_path: PathBuf, json: bool, tui: bool) -> anyhow::Result<()> {
    let mut purge_db_on_init = false;
    // Track the project directory when wizard creates it (for cd prompt at end)
    let mut project_dir: Option<String> = None;
    // Carries routing_hints from wizard result to DB insertion.
    // routing_hints are NOT stored in squad.yml, so they must be kept separately.
    let mut wizard_routing_hints: Option<std::collections::HashMap<String, Option<String>>> = None;
    // Deferred SDD playbook creation: save workflow so .squad/ is only created after setup
    let mut deferred_sdd: Option<crate::commands::wizard::SddWorkflow> = None;

    if !config_path.exists() {
        if tui {
            // --tui: run interactive wizard to generate squad.yml
            match crate::commands::wizard::run().await? {
                Some(result) => {
                    // Change to install directory (already includes project name as last component)
                    let install_dir = std::path::PathBuf::from(&result.install_dir);
                    std::fs::create_dir_all(&install_dir)?;
                    project_dir = Some(
                        install_dir
                            .canonicalize()
                            .unwrap_or(install_dir.clone())
                            .to_string_lossy()
                            .to_string(),
                    );
                    std::env::set_current_dir(&install_dir)?;
                    // Initialize git repo so Claude Code recognizes the project root
                    // and can find .claude/commands/ slash commands
                    if !install_dir.join(".git").exists() {
                        let _ = std::process::Command::new("git")
                            .args(["init", "-q"])
                            .current_dir(&install_dir)
                            .status();
                    }
                    config_path = install_dir.join(crate::config::DEFAULT_CONFIG_FILE);
                    // Capture routing hints before result fields are moved into yaml generation
                    wizard_routing_hints = Some(extract_routing_hints(&result));
                    // Defer SDD playbook creation — .squad/ will be created after setup completes
                    deferred_sdd = Some(result.sdd);
                    let yaml = generate_squad_yml(&result);
                    std::fs::write(&config_path, &yaml)?;
                    println!(
                        "Generated squad.yml for project '{}' in {}",
                        result.project, result.install_dir
                    );
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
                    kill_config_sessions(&old_cfg).await;
                }
                purge_db_on_init = true;
                match crate::commands::wizard::run().await? {
                    Some(result) => {
                        // Capture routing hints before result fields are moved into yaml generation
                        wizard_routing_hints = Some(extract_routing_hints(&result));
                        let yaml = generate_squad_yml(&result);
                        std::fs::write(&config_path, &yaml)?;
                        create_sdd_playbook(&config_path, result.sdd);
                        install_sdd_if_needed(result.sdd, &result.orchestrator.provider).ok();
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
                            let workers = cfg
                                .agents
                                .iter()
                                .map(|a| {
                                    let n = a.name.as_deref().unwrap_or("worker");
                                    format!("{} ({})", n, a.provider)
                                })
                                .collect();
                            (Some(orch), workers)
                        }
                        Err(_) => (None, vec![]),
                    };
                match crate::commands::wizard::run_worker_only(
                    existing_orchestrator,
                    existing_workers,
                )
                .await?
                {
                    Some(new_workers) => {
                        // Carry routing hints for new workers by agent name
                        let worker_hints: std::collections::HashMap<String, Option<String>> =
                            new_workers
                                .iter()
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

    // Compute project root from config_path
    let project_root = config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));

    let is_new_tui_project = project_dir.is_some();

    // 2. Resolve DB path and create .squad/
    let db_path = if is_new_tui_project {
        // TUI new-project: create .squad/ directly — sessions launch AFTER full setup
        let squad_dir = project_root.join(".squad");
        std::fs::create_dir_all(&squad_dir)?;

        // Create SDD playbook + install SDD locally if needed
        if let Some(sdd) = deferred_sdd {
            create_sdd_playbook(&config_path, sdd);
            install_sdd_if_needed(sdd, &config.orchestrator.provider).ok();
        }

        // Create log directory
        let _ = std::fs::create_dir_all(squad_dir.join("log"));

        squad_dir.join("station.db")
    } else {
        config::resolve_db_path(&config)?
    };

    // 3. Connect to DB (creates file + runs migrations)
    let pool = db::connect(&db_path).await?;

    // On overwrite: purge stale agents so the DB matches the new config exactly
    if purge_db_on_init {
        let _ = db::agents::delete_all_agents(&pool).await;
    }

    // 4. Register ALL agents to DB first (before launching any sessions)
    let orch_role = config
        .orchestrator
        .name
        .as_deref()
        .unwrap_or("orchestrator");
    let orch_name = config::sanitize_session_name(&format!("{}-{}", config.project, orch_role));
    let orch_hints = wizard_routing_hints
        .as_ref()
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

    let mut failed: Vec<(String, String)> = vec![];
    for agent in &config.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        let agent_name =
            config::sanitize_session_name(&format!("{}-{}", config.project, role_suffix));
        let hints = wizard_routing_hints
            .as_ref()
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
        }
    }

    // 4b. Clean stale agents: delete any DB agents not in current config
    {
        let mut expected_names: Vec<String> = vec![orch_name.clone()];
        for agent in &config.agents {
            let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
            expected_names.push(config::sanitize_session_name(&format!(
                "{}-{}",
                config.project, role_suffix
            )));
        }
        let all_agents = db::agents::list_agents(&pool).await?;
        for agent in &all_agents {
            if !expected_names.contains(&agent.name) {
                let _ = db::agents::delete_agent_by_name(&pool, &agent.name).await;
            }
        }
    }

    // 5. Setup hooks, context, watchdog — BEFORE launching sessions (TUI new-project)
    //    For non-TUI, this happens after session launch (existing behavior preserved).
    let mut any_hooks_installed = false;
    #[allow(unused_assignments)]
    let mut monitor_created = false;
    let monitor_name = format!("{}-monitor", config.project);

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

        // For TUI new-project: install hooks + generate context BEFORE launching sessions
        if is_new_tui_project {
            println!("\n{}", green("══════════════════════════════════"));
            println!("  {}", bold("Setting up project..."));
            println!("{}\n", green("══════════════════════════════════"));

            // Install hooks
            let mut providers_seen: Vec<String> = vec![config.orchestrator.provider.clone()];
            for agent in &config.agents {
                if !providers_seen.contains(&agent.provider) {
                    providers_seen.push(agent.provider.clone());
                }
            }
            for provider in &providers_seen {
                match auto_install_hooks(provider) {
                    Ok(true) => {
                        any_hooks_installed = true;
                        println!("  Hooks: installed for {}", provider);
                    }
                    Ok(false) => {
                        println!("  Hooks: skipped for {} (unsupported provider)", provider);
                    }
                    Err(e) => {
                        println!("  Hooks: failed for {} ({})", provider, e);
                    }
                }
            }

            // Auto-inject prompt
            if any_hooks_installed {
                println!();
                println!(
                    "  {}",
                    bold("Auto-inject orchestrator context on session start?")
                );
                println!(
                    "  When enabled, the orchestrator automatically receives its role and agent roster"
                );
                println!("  whenever the AI starts a new session, resumes, or compacts context.");
                println!(
                    "  If disabled, you must manually run {} each time.",
                    yellow("/squad-orchestrator")
                );
                print!("\n  Enable auto-inject? [Y/n] ");
                use std::io::Write;
                std::io::stdout().flush().ok();

                let mut answer = String::new();
                if std::io::stdin().read_line(&mut answer).is_ok()
                    && !answer.trim().eq_ignore_ascii_case("n")
                {
                    match install_session_start_hook(
                        &config.orchestrator.provider,
                        project_root,
                        &orch_name,
                    ) {
                        Ok(true) => println!("  SessionStart hook: installed"),
                        Ok(false) => {
                            println!("  SessionStart hook: skipped (unsupported provider)")
                        }
                        Err(e) => println!("  SessionStart hook: failed ({})", e),
                    }
                } else {
                    println!("  SessionStart hook: skipped");
                }
            }

            // Generate full context (DB has all agents now)
            println!("\nGenerating orchestrator context...");
            if let Err(e) = crate::commands::context::run(false).await {
                println!("Warning: Failed to generate context files: {}", e);
            }

            // Start watchdog
            match crate::commands::watch::run(30, 5, true, false, false, false, 600, 3).await {
                Ok(()) => println!("  Watchdog: started (30s interval)"),
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("already running") {
                        println!("  Watchdog: already running");
                    } else {
                        println!("  Watchdog: failed to start ({})", e);
                    }
                }
            }
        }

        // 6. Launch tmux sessions
        let mut db_only_names: Vec<String> = vec![];
        let mut skipped_names: Vec<String> = vec![];
        let mut launched: u32 = 0;
        let mut skipped: u32 = 0;

        // Launch orchestrator
        let orch_launched = if config.orchestrator.is_db_only() {
            db_only_names.push(orch_name.clone());
            false
        } else if tmux::session_exists(&orch_name).await {
            skipped += 1;
            skipped_names.push(orch_name.clone());
            false
        } else {
            let project_root_str = project_root.to_string_lossy().to_string();
            let cmd = get_launch_command(&config.orchestrator);
            tmux::launch_agent_in_dir(&orch_name, &cmd, &project_root_str).await?;
            launched += 1;
            true
        };
        let _ = orch_launched; // used for counting only

        // Launch workers
        for agent in &config.agents {
            let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
            let agent_name =
                config::sanitize_session_name(&format!("{}-{}", config.project, role_suffix));

            // Skip agents that failed registration
            if failed.iter().any(|(n, _)| n == &agent_name) {
                continue;
            }

            if tmux::session_exists(&agent_name).await {
                skipped += 1;
                skipped_names.push(agent_name.clone());
                continue;
            }

            let project_root_str = project_root.to_string_lossy().to_string();
            let cmd = get_launch_command(agent);
            match tmux::launch_agent_in_dir(&agent_name, &cmd, &project_root_str).await {
                Ok(()) => launched += 1,
                Err(e) => failed.push((agent_name.clone(), format!("{e:#}"))),
            }
        }

        // 7. Create monitor session
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
        tmux::kill_session(&monitor_name).await?;
        monitor_created = if !monitor_sessions.is_empty() {
            tmux::create_view_session(&monitor_name, &monitor_sessions)
                .await
                .is_ok()
        } else {
            false
        };

        // 8. Output results
        let db_path_str = db_path.display().to_string();
        let total_agents = config.agents.len() + 1;
        println!(
            "\nInitialized squad '{}' with {} agent(s) ({} launched, {} skipped)",
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

        // For non-TUI: run hooks/context/watch AFTER sessions (existing behavior)
        if !is_new_tui_project {
            // Create log directory
            let log_dir = db_path
                .parent()
                .unwrap_or(std::path::Path::new(".squad"))
                .join("log");
            let _ = std::fs::create_dir_all(&log_dir);

            // Install SDD locally if not already installed
            if let Some(sdd_configs) = &config.sdd {
                install_sdd_from_config(sdd_configs, &config.orchestrator.provider);
            }

            println!("\n{}", green("══════════════════════════════════"));
            println!("  {}", bold("Squad Setup Complete"));
            println!("{}\n", green("══════════════════════════════════"));

            let mut providers_seen: Vec<String> = vec![config.orchestrator.provider.clone()];
            for agent in &config.agents {
                if !providers_seen.contains(&agent.provider) {
                    providers_seen.push(agent.provider.clone());
                }
            }
            for provider in &providers_seen {
                match auto_install_hooks(provider) {
                    Ok(true) => {
                        any_hooks_installed = true;
                        println!("  Hooks: installed for {}", provider);
                    }
                    Ok(false) => {
                        println!("  Hooks: skipped for {} (unsupported provider)", provider);
                    }
                    Err(e) => {
                        println!("  Hooks: failed for {} ({})", provider, e);
                    }
                }
            }

            if !any_hooks_installed {
                println!("Please manually configure the following hooks to enable task completion signals:\n");
                let hook_providers: &[(&str, &str, &str)] = &[
                    (".claude/settings.local.json", "Stop", "*"),
                    (
                        ".claude/settings.local.json",
                        "Notification",
                        "permission_prompt",
                    ),
                    (
                        ".claude/settings.local.json",
                        "PostToolUse",
                        "AskUserQuestion",
                    ),
                    (".gemini/settings.json", "AfterAgent", "*"),
                    (".gemini/settings.json", "Notification", "*"),
                ];
                for &(settings_path, hook_event, matcher) in hook_providers {
                    print_hook_instructions(settings_path, hook_event, matcher);
                }
            }

            if any_hooks_installed {
                println!();
                println!(
                    "  {}",
                    bold("Auto-inject orchestrator context on session start?")
                );
                println!(
                    "  When enabled, the orchestrator automatically receives its role and agent roster"
                );
                println!("  whenever the AI starts a new session, resumes, or compacts context.");
                println!(
                    "  If disabled, you must manually run {} each time.",
                    yellow("/squad-orchestrator")
                );
                print!("\n  Enable auto-inject? [Y/n] ");
                use std::io::Write;
                std::io::stdout().flush().ok();

                let mut answer = String::new();
                if std::io::stdin().read_line(&mut answer).is_ok()
                    && !answer.trim().eq_ignore_ascii_case("n")
                {
                    match install_session_start_hook(
                        &config.orchestrator.provider,
                        project_root,
                        &orch_name,
                    ) {
                        Ok(true) => println!("  SessionStart hook: installed"),
                        Ok(false) => {
                            println!("  SessionStart hook: skipped (unsupported provider)")
                        }
                        Err(e) => println!("  SessionStart hook: failed ({})", e),
                    }
                } else {
                    println!("  SessionStart hook: skipped");
                }
            }

            println!("\nGenerating orchestrator context...");
            if let Err(e) = crate::commands::context::run(false).await {
                println!("Warning: Failed to generate context files: {}", e);
            }
        }

        // 9. Get Started + final status (both paths)
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

        if !is_new_tui_project {
            // Non-TUI: start watchdog after sessions
            match crate::commands::watch::run(30, 5, true, false, false, false, 600, 3).await {
                Ok(()) => println!("  Watchdog: started (30s interval)"),
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("already running") {
                        println!("  Watchdog: already running");
                    } else {
                        println!("  Watchdog: failed to start ({})", e);
                    }
                }
            }
            println!();
        }

        // Reconcile agent statuses before printing diagram
        crate::commands::helpers::reconcile_agent_statuses(&pool).await?;
        let agents = db::agents::list_agents(&pool).await?;
        crate::commands::diagram::print_diagram(&agents);

        // Write project directory to temp file for parent process (run.js) to cd into
        if let Some(ref dir) = project_dir {
            let marker = std::env::temp_dir().join(".squad-project-dir");
            let _ = std::fs::write(&marker, dir);
        }
    } else {
        // JSON mode: launch sessions, output JSON, skip interactive setup
        let mut db_only_names: Vec<String> = vec![];
        let mut skipped_names: Vec<String> = vec![];
        let mut launched: u32 = 0;
        let mut skipped: u32 = 0;

        let orch_launched = if config.orchestrator.is_db_only() {
            db_only_names.push(orch_name.clone());
            false
        } else if tmux::session_exists(&orch_name).await {
            skipped += 1;
            skipped_names.push(orch_name.clone());
            false
        } else {
            let project_root_str = project_root.to_string_lossy().to_string();
            let cmd = get_launch_command(&config.orchestrator);
            tmux::launch_agent_in_dir(&orch_name, &cmd, &project_root_str).await?;
            launched += 1;
            true
        };
        let _ = orch_launched;

        for agent in &config.agents {
            let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
            let agent_name =
                config::sanitize_session_name(&format!("{}-{}", config.project, role_suffix));
            if failed.iter().any(|(n, _)| n == &agent_name) {
                continue;
            }
            if tmux::session_exists(&agent_name).await {
                skipped += 1;
                continue;
            }
            let project_root_str = project_root.to_string_lossy().to_string();
            let cmd = get_launch_command(agent);
            match tmux::launch_agent_in_dir(&agent_name, &cmd, &project_root_str).await {
                Ok(()) => launched += 1,
                Err(e) => failed.push((agent_name.clone(), format!("{e:#}"))),
            }
        }

        let mut monitor_sessions: Vec<String> = vec![];
        if !config.orchestrator.is_db_only() {
            monitor_sessions.push(orch_name.clone());
        }
        for agent in &config.agents {
            let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
            monitor_sessions.push(config::sanitize_session_name(&format!(
                "{}-{}",
                config.project, role_suffix
            )));
        }
        tmux::kill_session(&monitor_name).await?;
        monitor_created = if !monitor_sessions.is_empty() {
            tmux::create_view_session(&monitor_name, &monitor_sessions)
                .await
                .is_ok()
        } else {
            false
        };

        let output = serde_json::json!({
            "launched": launched,
            "skipped": skipped,
            "failed": failed,
            "db_path": db_path.display().to_string(),
            "monitor": if monitor_created { Some(&monitor_name) } else { None },
        });
        println!("{}", serde_json::to_string(&output)?);
    }

    Ok(())
}

/// Kill all tmux sessions (orchestrator + workers + monitor) for a given config.
/// Used before overwriting squad.yml so stale sessions don't persist.
async fn kill_config_sessions(cfg: &config::SquadConfig) {
    let orch_role = cfg.orchestrator.name.as_deref().unwrap_or("orchestrator");
    let orch_name = config::sanitize_session_name(&format!("{}-{}", cfg.project, orch_role));
    let monitor_name = config::sanitize_session_name(&format!("{}-monitor", cfg.project));

    let _ = tmux::kill_session(&orch_name).await;
    let _ = tmux::kill_session(&monitor_name).await;

    for agent in &cfg.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        let session_name =
            config::sanitize_session_name(&format!("{}-{}", cfg.project, role_suffix));
        let _ = tmux::kill_session(&session_name).await;
    }
}

/// Write the SDD playbook file to `.squad/sdd/<name>-playbook.md`.
/// Creates the directory if it doesn't exist. Skips silently if the file already exists.
fn create_sdd_playbook(config_path: &std::path::Path, sdd: crate::commands::wizard::SddWorkflow) {
    let project_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    let sdd_dir = project_dir.join(".squad").join("sdd");
    if let Err(e) = std::fs::create_dir_all(&sdd_dir) {
        eprintln!("Warning: could not create .squad/sdd/: {}", e);
        return;
    }
    let filename = format!("{}-playbook.md", sdd.as_str());
    let playbook_path = sdd_dir.join(&filename);
    if playbook_path.exists() {
        return; // already present — don't overwrite user edits
    }
    if let Err(e) = std::fs::write(&playbook_path, sdd.playbook_content()) {
        eprintln!(
            "Warning: could not write {}: {}",
            playbook_path.display(),
            e
        );
    }
}

/// Run the SDD local installer if the SDD is not already installed.
/// Checks detect_dirs to skip if already present. Runs the non-interactive install command.
/// Returns Ok(true) if installed, Ok(false) if skipped, Err on failure.
fn install_sdd_if_needed(
    sdd: crate::commands::wizard::SddWorkflow,
    provider: &str,
) -> anyhow::Result<bool> {
    // Check if already installed
    let cwd = std::env::current_dir().unwrap_or_default();
    for dir in sdd.detect_dirs() {
        if cwd.join(dir).exists() {
            return Ok(false); // already installed
        }
    }

    let args = match sdd.install_command(provider) {
        Some(args) => args,
        None => {
            // No automated installer (e.g. Superpower)
            eprintln!(
                "  SDD '{}': no automated installer — follow the playbook's install instructions manually",
                sdd.as_str()
            );
            return Ok(false);
        }
    };

    println!("  SDD '{}': installing locally...", sdd.as_str());

    let status = std::process::Command::new(args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("  SDD '{}': installed successfully", sdd.as_str());
            Ok(true)
        }
        Ok(s) => {
            eprintln!(
                "  SDD '{}': installer exited with {}",
                sdd.as_str(),
                s.code().unwrap_or(-1)
            );
            Ok(false)
        }
        Err(e) => {
            eprintln!(
                "  SDD '{}': failed to run installer ({}). Install manually: {}",
                sdd.as_str(),
                e,
                args.join(" ")
            );
            Ok(false)
        }
    }
}

/// Install SDD from squad.yml config (non-wizard path).
/// Resolves SddConfig.name to SddWorkflow and runs the installer.
fn install_sdd_from_config(sdd_configs: &[crate::config::SddConfig], provider: &str) {
    for sdd_cfg in sdd_configs {
        if let Some(sdd) = crate::commands::wizard::SddWorkflow::from_name(&sdd_cfg.name) {
            match install_sdd_if_needed(sdd, provider) {
                Ok(true) => {}  // message already printed
                Ok(false) => {} // skipped or manual
                Err(e) => eprintln!("  SDD '{}': error ({})", sdd_cfg.name, e),
            }
        }
    }
}

/// Generate a squad.yml YAML string from a completed WizardResult.
/// Optional fields (name, model, description) are omitted when empty/None.
fn generate_squad_yml(result: &crate::commands::wizard::WizardResult) -> String {
    let sdd_name = result.sdd.as_str();

    let doc = SquadYmlDoc {
        project: &result.project,
        sdd: vec![SquadYmlSdd {
            name: sdd_name,
            playbook: format!(".squad/sdd/{}-playbook.md", sdd_name),
        }],
        orchestrator: SquadYmlAgent {
            provider: &result.orchestrator.provider,
            name: if result.orchestrator.name.is_empty() {
                None
            } else {
                Some(&result.orchestrator.name)
            },
            role: "orchestrator",
            model: result.orchestrator.model.as_deref(),
            description: result.orchestrator.description.as_deref(),
            channels: result.orchestrator.channels.clone(),
        },
        agents: result
            .agents
            .iter()
            .map(|a| SquadYmlAgent {
                provider: &a.provider,
                name: if a.name.is_empty() {
                    None
                } else {
                    Some(&a.name)
                },
                role: "worker",
                model: a.model.as_deref(),
                description: a.description.as_deref(),
                channels: a.channels.clone(),
            })
            .collect(),
    };

    serde_saphyr::to_string(&doc)
        .unwrap_or_else(|_| "".to_string())
        .replace("---\n", "")
}

fn auto_install_hooks(provider: &str) -> anyhow::Result<bool> {
    match provider {
        "claude-code" => install_claude_hooks(".claude/settings.local.json"),
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

/// Build the agent name resolution shell snippet.
/// Primary: $SQUAD_AGENT_NAME (set at tmux launch, deterministic).
/// Fallback: $TMUX_PANE + list-panes (server command, more reliable than display-message).
fn agent_resolve_snippet() -> &'static str {
    r#"AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -1)}"#
}

/// Install Claude Code hooks: Stop (signal) + Notification (notify) + PostToolUse (AskUserQuestion)
/// Appends to existing hook arrays instead of overwriting them.
fn install_claude_hooks(settings_file: &str) -> anyhow::Result<bool> {
    let mut settings = read_or_create_settings(settings_file)?;
    let resolve = agent_resolve_snippet();

    // Claude Code: stdout is ignored, errors to log file. Always exit 0.
    let signal_cmd = format!(
        r#"{}; [ -n "$AGENT" ] && squad-station signal "$AGENT" 2>>.squad/log/signal.log || true"#,
        resolve
    );
    let notify_cmd = format!(
        r#"{}; [ -n "$AGENT" ] && squad-station notify --body 'Agent needs input' --agent "$AGENT" || true"#,
        resolve
    );

    // Stop hook — agent finished task → signal completion
    append_hook_entry(
        &mut settings,
        "Stop",
        serde_json::json!({
            "matcher": "",
            "hooks": [{"type": "command", "command": signal_cmd}]
        }),
        "squad-station signal",
    );

    // Notification hook — agent needs permission approval → notify orchestrator
    append_hook_entry(
        &mut settings,
        "Notification",
        serde_json::json!({
            "matcher": "permission_prompt",
            "hooks": [{"type": "command", "command": notify_cmd}]
        }),
        "squad-station notify",
    );
    append_hook_entry(
        &mut settings,
        "Notification",
        serde_json::json!({
            "matcher": "elicitation_dialog",
            "hooks": [{"type": "command", "command": notify_cmd}]
        }),
        "squad-station notify",
    );

    // PostToolUse hook — agent is asking the user a question → notify orchestrator.
    append_hook_entry(
        &mut settings,
        "PostToolUse",
        serde_json::json!({
            "matcher": "AskUserQuestion",
            "hooks": [{"type": "command", "command": notify_cmd}]
        }),
        "squad-station notify",
    );

    std::fs::write(settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Install Gemini CLI hooks: AfterAgent (signal) + Notification (notify)
///
/// Critical Gemini CLI differences:
/// - Uses AfterAgent (not Stop) for completion signals
/// - Stdout MUST be valid JSON (golden rule) — all signal output goes to log file
/// - printf '{}' outputs empty JSON object = "continue normally"
/// - Uses ${AGENT:-__none__} to avoid shell short-circuit skipping printf
fn install_gemini_hooks(settings_file: &str) -> anyhow::Result<bool> {
    let mut settings = read_or_create_settings(settings_file)?;
    let resolve = agent_resolve_snippet();

    // Gemini CLI: signal command outputs {} to stdout in non-TTY (hook) context.
    // No redirect or printf needed — signal handles JSON output natively.
    let signal_cmd = format!(
        r#"{}; squad-station signal "${{AGENT:-__none__}}""#,
        resolve
    );
    let notify_cmd = format!(
        r#"{}; squad-station notify --body 'Agent needs input' --agent "${{AGENT:-__none__}}" 2>/dev/null; printf '{{}}'"#,
        resolve
    );

    append_hook_entry(
        &mut settings,
        "AfterAgent",
        serde_json::json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": signal_cmd,
                "name": "squad-signal",
                "description": "Signal task completion to squad-station",
                "timeout": 30000
            }]
        }),
        "squad-station signal",
    );

    append_hook_entry(
        &mut settings,
        "Notification",
        serde_json::json!({
            "matcher": "",
            "hooks": [{
                "type": "command",
                "command": notify_cmd,
                "name": "squad-notify",
                "description": "Forward permission prompt to orchestrator",
                "timeout": 30000
            }]
        }),
        "squad-station notify",
    );

    std::fs::write(settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Append a hook entry to an existing hook array, skipping if an identical matcher+marker combo exists.
/// Returns true if the entry was added, false if it already existed.
fn append_hook_entry(
    settings: &mut serde_json::Value,
    hook_event: &str,
    entry: serde_json::Value,
    marker: &str,
) -> bool {
    let arr = settings["hooks"][hook_event]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let entry_matcher = entry["matcher"].as_str().unwrap_or("");

    // Check if an entry with the same matcher AND marker command already exists
    let already_exists = arr.iter().any(|e| {
        let same_matcher = e["matcher"].as_str().unwrap_or("") == entry_matcher;
        let has_marker = e["hooks"]
            .as_array()
            .map(|hooks| {
                hooks.iter().any(|h| {
                    h["command"]
                        .as_str()
                        .map(|c| c.contains(marker))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        same_matcher && has_marker
    });

    if already_exists {
        return false;
    }

    let mut new_arr = arr;
    new_arr.push(entry);
    settings["hooks"][hook_event] = serde_json::Value::Array(new_arr);
    true
}

/// Install SessionStart hook for auto-injecting orchestrator context.
/// Called separately from base hooks because it requires user opt-in.
/// Appends to existing SessionStart hooks instead of overwriting.
/// For Claude Code: uses fast `cat` of pre-generated context file (instant, no binary startup).
/// For Gemini CLI: uses `squad-station context --inject` (needs provider-specific formatting).
fn install_session_start_hook(
    provider: &str,
    project_root: &std::path::Path,
    orch_session_name: &str,
) -> anyhow::Result<bool> {
    let (rel_path, inject_cmd, search_key) = match provider {
        "claude-code" => {
            // Fast cat: only inject for orchestrator session, instant file read
            let cmd = format!(
                r#"[ "$SQUAD_AGENT_NAME" = "{}" ] && cat .claude/commands/squad-orchestrator.md || true"#,
                orch_session_name
            );
            (".claude/settings.local.json", cmd, "squad-orchestrator")
        }
        "gemini-cli" => (
            ".gemini/settings.json",
            "squad-station context --inject".to_string(),
            "squad-station context",
        ),
        _ => return Ok(false),
    };

    let settings_path = project_root.join(rel_path);
    let settings_str = settings_path.to_string_lossy();
    let mut settings = read_or_create_settings(&settings_str)?;

    let entry = serde_json::json!({
        "matcher": "",
        "hooks": [{"type": "command", "command": inject_cmd}]
    });

    let added = append_hook_entry(&mut settings, "SessionStart", entry, search_key);

    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    Ok(added)
}

/// Validate that a model string is safe for use as a CLI argument.
/// Only allows alphanumeric characters, dots, dashes, underscores, and colons.
fn is_safe_model_value(model: &str) -> bool {
    !model.is_empty()
        && model.chars().all(|c| {
            c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':' || c == '@'
        })
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
                if is_safe_model_value(model) {
                    cmd.push_str(&format!(" --model {}", model));
                } else {
                    eprintln!(
                        "squad-station: warning: skipping unsafe model value: {:?}",
                        model
                    );
                }
            }
            if let Some(channels) = &agent.channels {
                for ch in channels {
                    if is_safe_model_value(ch) {
                        cmd.push_str(&format!(" --channels {}", ch));
                    } else {
                        eprintln!(
                            "squad-station: warning: skipping unsafe channel value: {:?}",
                            ch
                        );
                    }
                }
            }
            cmd
        }
        "gemini-cli" => {
            let mut cmd = "gemini -y".to_string();
            if let Some(model) = &agent.model {
                if is_safe_model_value(model) {
                    cmd.push_str(&format!(" --model {}", model));
                } else {
                    eprintln!(
                        "squad-station: warning: skipping unsafe model value: {:?}",
                        model
                    );
                }
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
            channels: None,
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
            channels: None,
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
        assert!(
            result.contains("provider: gemini-cli"),
            "Must contain new gemini-cli worker"
        );
        assert!(
            result.contains("provider: claude-code"),
            "Must contain existing + new claude-code"
        );
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
        assert!(
            result.contains("name: my-agent"),
            "Named worker must have name field"
        );
    }

    #[test]
    fn test_append_workers_to_yaml_omits_name_when_empty() {
        let existing = "agents:\n";
        let workers = vec![make_worker("claude-code", "")];
        let result = append_workers_to_yaml(existing, &workers);
        assert!(
            !result.contains("name:"),
            "Unnamed worker must not have name field"
        );
    }

    #[test]
    fn test_append_workers_to_yaml_includes_model_when_present() {
        let existing = "agents:\n";
        let workers = vec![make_worker_with_model("claude-code", "", "sonnet")];
        let result = append_workers_to_yaml(existing, &workers);
        assert!(
            result.contains("model: sonnet"),
            "Model must appear in output"
        );
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
                channels: Some(vec!["plugin:telegram@claude-plugins-official".to_string()]),
            },
            agents: vec![AgentInput {
                name: "backend".to_string(),
                role: "worker".to_string(),
                provider: "gemini-cli".to_string(),
                model: Some("gemini-2.5-pro".to_string()),
                description: None,
                routing_hints: None,
                channels: None,
            }],
        }
    }

    #[test]
    fn test_generate_squad_yml_contains_required_sections() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(
            yaml.starts_with("project: my-project\n"),
            "YAML must start with project: line"
        );
        assert!(yaml.contains("sdd:"), "YAML must contain sdd section");
        assert!(
            yaml.contains("orchestrator:"),
            "YAML must contain orchestrator section"
        );
        assert!(yaml.contains("agents:"), "YAML must contain agents section");
    }

    #[test]
    fn test_generate_squad_yml_sdd_playbook_path() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(
            yaml.contains("playbook: .squad/sdd/gsd-playbook.md")
                || yaml.contains("playbook: \".squad/sdd/gsd-playbook.md\""),
            "SDD playbook path must be correct, got:\n{}",
            yaml
        );
    }

    #[test]
    fn test_generate_squad_yml_orchestrator_fields() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(
            yaml.contains("role: orchestrator"),
            "orchestrator must have role: orchestrator"
        );
        assert!(
            yaml.contains("provider: claude-code"),
            "orchestrator provider must be set"
        );
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
        let orch_start = lines
            .iter()
            .position(|l| l.trim() == "orchestrator:")
            .unwrap();
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
        let model_lines_in_agents: Vec<&str> = agent_lines
            .iter()
            .filter(|l| l.contains("model:"))
            .copied()
            .collect();
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
        assert_eq!(notif[0]["matcher"].as_str().unwrap(), "permission_prompt");
        assert_eq!(notif[1]["matcher"].as_str().unwrap(), "elicitation_dialog");

        // Verify PostToolUse hook exists with AskUserQuestion matcher
        let ptu = &settings["hooks"]["PostToolUse"];
        assert!(ptu.is_array(), "PostToolUse hook must exist");
        assert_eq!(ptu[0]["matcher"].as_str().unwrap(), "AskUserQuestion");

        // Verify the command calls notify with the standard pattern
        let cmd = ptu[0]["hooks"][0]["command"].as_str().unwrap();
        assert!(
            cmd.contains("squad-station notify"),
            "PostToolUse command must call squad-station notify"
        );

        // Base hooks must NOT include SessionStart (opt-in via install_session_start_hook)
        assert!(
            settings["hooks"]["SessionStart"].is_null(),
            "SessionStart must not be installed by base hooks"
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
        std::fs::write(
            &settings_file,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

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
        // SessionStart must NOT be added by base hooks
        assert!(settings["hooks"]["SessionStart"].is_null());
    }

    #[test]
    fn test_install_gemini_hooks_excludes_session_start() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_gemini_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify base hooks exist
        assert!(
            settings["hooks"]["AfterAgent"].is_array(),
            "AfterAgent hook must exist"
        );
        assert!(
            settings["hooks"]["Notification"].is_array(),
            "Notification hook must exist"
        );
        // SessionStart must NOT be installed by base hooks
        assert!(
            settings["hooks"]["SessionStart"].is_null(),
            "SessionStart must not be installed by base hooks"
        );
    }

    #[test]
    fn test_install_session_start_hook_claude() {
        let tmp = tempfile::TempDir::new().unwrap();
        let claude_dir = tmp.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        // SessionStart hook goes to settings.local.json (trusted, no approval needed)
        let settings_file = claude_dir.join("settings.local.json");

        let result = install_session_start_hook("claude-code", tmp.path(), "test-orch");
        assert!(result.unwrap());

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // SessionStart hook installed with fast cat command
        let ss = &settings["hooks"]["SessionStart"];
        assert!(ss.is_array(), "SessionStart hook must exist");
        let ss_cmd = ss[0]["hooks"][0]["command"].as_str().unwrap();
        assert!(
            ss_cmd.contains("cat .claude/commands/squad-orchestrator.md"),
            "Must use fast cat"
        );
        assert!(
            ss_cmd.contains("test-orch"),
            "Must check orchestrator session name"
        );
    }

    #[test]
    fn test_install_session_start_hook_gemini() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        std::fs::write(&settings_file, r#"{"hooks":{"AfterAgent":[]}}"#).unwrap();

        let result = install_session_start_hook("gemini-cli", tmp.path(), "test-orch");
        assert!(result.unwrap());

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let ss = &settings["hooks"]["SessionStart"];
        assert!(ss.is_array(), "SessionStart hook must exist");
        let ss_cmd = ss[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(ss_cmd, "squad-station context --inject");

        // Existing hooks preserved
        assert!(settings["hooks"]["AfterAgent"].is_array());
    }

    #[test]
    fn test_install_session_start_hook_unknown_provider_returns_false() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!install_session_start_hook("antigravity", tmp.path(), "x").unwrap());
        assert!(!install_session_start_hook("unknown-tool", tmp.path(), "x").unwrap());
    }

    #[test]
    fn test_is_safe_model_value_valid() {
        assert!(is_safe_model_value("claude-opus"));
        assert!(is_safe_model_value("gemini-3.1-pro-preview"));
        assert!(is_safe_model_value("gpt_4o:latest"));
    }

    #[test]
    fn test_is_safe_model_value_rejects_injection() {
        assert!(!is_safe_model_value("opus; rm -rf /"));
        assert!(!is_safe_model_value("model$(whoami)"));
        assert!(!is_safe_model_value("model`id`"));
        assert!(!is_safe_model_value(""));
    }

    #[test]
    fn test_get_launch_command_claude_with_channels() {
        let agent = config::AgentConfig {
            name: None,
            provider: "claude-code".to_string(),
            role: "orchestrator".to_string(),
            model: None,
            description: None,
            channels: Some(vec!["plugin:telegram@claude-plugins-official".to_string()]),
        };
        let cmd = get_launch_command(&agent);
        assert_eq!(
            cmd,
            "claude --dangerously-skip-permissions --channels plugin:telegram@claude-plugins-official"
        );
    }

    #[test]
    fn test_get_launch_command_claude_with_model_and_channels() {
        let agent = config::AgentConfig {
            name: None,
            provider: "claude-code".to_string(),
            role: "orchestrator".to_string(),
            model: Some("opus".to_string()),
            description: None,
            channels: Some(vec!["plugin:telegram@claude-plugins-official".to_string()]),
        };
        let cmd = get_launch_command(&agent);
        assert_eq!(
            cmd,
            "claude --dangerously-skip-permissions --model opus --channels plugin:telegram@claude-plugins-official"
        );
    }

    #[test]
    fn test_get_launch_command_claude_no_channels() {
        let agent = config::AgentConfig {
            name: None,
            provider: "claude-code".to_string(),
            role: "orchestrator".to_string(),
            model: None,
            description: None,
            channels: None,
        };
        let cmd = get_launch_command(&agent);
        assert_eq!(cmd, "claude --dangerously-skip-permissions");
    }

    #[test]
    fn test_get_launch_command_gemini_ignores_channels() {
        let agent = config::AgentConfig {
            name: None,
            provider: "gemini-cli".to_string(),
            role: "worker".to_string(),
            model: None,
            description: None,
            channels: Some(vec!["plugin:telegram@claude-plugins-official".to_string()]),
        };
        let cmd = get_launch_command(&agent);
        assert_eq!(cmd, "gemini -y");
    }

    #[test]
    fn test_generate_squad_yml_includes_channels() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        assert!(
            yaml.contains("channels:"),
            "YAML must contain channels section for orchestrator, got:\n{}",
            yaml
        );
        assert!(
            yaml.contains("plugin:telegram@claude-plugins-official"),
            "YAML must contain plugin:telegram@claude-plugins-official channel, got:\n{}",
            yaml
        );
    }

    #[test]
    fn test_generate_squad_yml_roundtrips_with_channels() {
        let result = make_wizard_result();
        let yaml = generate_squad_yml(&result);
        let config: Result<crate::config::SquadConfig, _> = serde_saphyr::from_str(&yaml);
        assert!(
            config.is_ok(),
            "Generated YAML with channels must parse, got: {:?}",
            config.err()
        );
        let config = config.unwrap();
        let channels = config
            .orchestrator
            .channels
            .as_ref()
            .expect("orchestrator must have channels");
        assert_eq!(
            channels,
            &vec!["plugin:telegram@claude-plugins-official".to_string()]
        );
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
