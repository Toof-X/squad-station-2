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

    // 9a. Create .squad/log/ directory for signal and watchdog logs
    {
        let log_dir = db_path
            .parent()
            .unwrap_or(std::path::Path::new(".squad"))
            .join("log");
        let _ = std::fs::create_dir_all(&log_dir);
    }

    // 9. Hook setup: auto-install for ALL providers used in the squad (not just orchestrator).
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

        // Collect all unique providers across orchestrator + workers
        let mut providers_seen: Vec<String> = vec![config.orchestrator.provider.clone()];
        for agent in &config.agents {
            if !providers_seen.contains(&agent.provider) {
                providers_seen.push(agent.provider.clone());
            }
        }

        let mut any_hooks_installed = false;
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
                (".claude/settings.json", "Stop", "*"),
                (".claude/settings.json", "Notification", "permission_prompt"),
                (".claude/settings.json", "PostToolUse", "AskUserQuestion"),
                (".gemini/settings.json", "AfterAgent", "*"),
                (".gemini/settings.json", "Notification", "*"),
            ];
            for &(settings_path, hook_event, matcher) in hook_providers {
                print_hook_instructions(settings_path, hook_event, matcher);
            }
        }

        let hook_installed = any_hooks_installed;

        // Ask user if they want auto-inject of orchestrator context on session start/compact/clear.
        // Only prompt when base hooks were successfully auto-installed (supported provider).
        if hook_installed {
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
            print!("\n  Enable auto-inject? [y/N] ");
            use std::io::Write;
            std::io::stdout().flush().ok();

            let mut answer = String::new();
            if std::io::stdin().read_line(&mut answer).is_ok()
                && answer.trim().eq_ignore_ascii_case("y")
            {
                let project_root = config_path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(std::path::Path::new("."));
                match install_session_start_hook(&config.orchestrator.provider, project_root) {
                    Ok(true) => println!("  SessionStart hook: installed"),
                    Ok(false) => println!("  SessionStart hook: skipped (unsupported provider)"),
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

        // Auto-start watchdog daemon for self-healing
        match crate::commands::watch::run(30, 5, true, false).await {
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

/// Build the agent name resolution shell snippet.
/// Primary: $SQUAD_AGENT_NAME (set at tmux launch, deterministic).
/// Fallback: $TMUX_PANE + list-panes (server command, more reliable than display-message).
fn agent_resolve_snippet() -> &'static str {
    r#"AGENT=${SQUAD_AGENT_NAME:-$(tmux list-panes -t "$TMUX_PANE" -F '#S' 2>/dev/null | head -1)}"#
}

/// Install Claude Code hooks: Stop (signal) + Notification (notify) + PostToolUse (AskUserQuestion)
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
///
/// Critical Gemini CLI differences:
/// - Uses AfterAgent (not Stop) for completion signals
/// - Stdout MUST be valid JSON (golden rule) — all signal output goes to log file
/// - printf '{}' outputs empty JSON object = "continue normally"
/// - Uses ${AGENT:-__none__} to avoid shell short-circuit skipping printf
fn install_gemini_hooks(settings_file: &str) -> anyhow::Result<bool> {
    let mut settings = read_or_create_settings(settings_file)?;
    let resolve = agent_resolve_snippet();

    // Gemini CLI: ALL signal output redirected to log. stdout MUST be valid JSON.
    let signal_cmd = format!(
        r#"{}; squad-station signal "${{AGENT:-__none__}}" >>.squad/log/signal.log 2>&1; printf '{{}}'"#,
        resolve
    );
    let notify_cmd = format!(
        r#"{}; squad-station notify --body 'Agent needs input' --agent "${{AGENT:-__none__}}" >>.squad/log/signal.log 2>&1; printf '{{}}'"#,
        resolve
    );

    settings["hooks"]["AfterAgent"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": signal_cmd,
            "name": "squad-signal",
            "description": "Signal task completion to squad-station",
            "timeout": 30000
        }]
    }]);

    settings["hooks"]["Notification"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": notify_cmd,
            "name": "squad-notify",
            "description": "Forward permission prompt to orchestrator",
            "timeout": 30000
        }]
    }]);

    std::fs::write(settings_file, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Install SessionStart hook for auto-injecting orchestrator context.
/// Called separately from base hooks because it requires user opt-in.
fn install_session_start_hook(
    provider: &str,
    project_root: &std::path::Path,
) -> anyhow::Result<bool> {
    let rel_path = match provider {
        "claude-code" => ".claude/settings.json",
        "gemini-cli" => ".gemini/settings.json",
        _ => return Ok(false),
    };

    let settings_path = project_root.join(rel_path);
    let settings_str = settings_path.to_string_lossy();
    let mut settings = read_or_create_settings(&settings_str)?;
    let inject_cmd = "squad-station context --inject";

    settings["hooks"]["SessionStart"] = serde_json::json!([{
        "matcher": "",
        "hooks": [{"type": "command", "command": inject_cmd}]
    }]);

    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Validate that a model string is safe for use as a CLI argument.
/// Only allows alphanumeric characters, dots, dashes, underscores, and colons.
fn is_safe_model_value(model: &str) -> bool {
    !model.is_empty()
        && model
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':')
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

        // Base hooks must NOT include SessionStart (opt-in via install_session_start_hook)
        assert!(
            settings["hooks"]["SessionStart"].is_null(),
            "SessionStart must not be installed by base hooks"
        );
    }

    #[test]
    fn test_install_claude_hooks_uses_squad_agent_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let settings_file = tmp.path().join(".claude").join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_claude_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let stop_cmd = settings["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(
            stop_cmd.contains("SQUAD_AGENT_NAME"),
            "Stop hook must use $SQUAD_AGENT_NAME: {}",
            stop_cmd
        );
        assert!(
            stop_cmd.contains("list-panes"),
            "Stop hook must have tmux list-panes fallback: {}",
            stop_cmd
        );
        assert!(
            stop_cmd.contains(".squad/log/signal.log"),
            "Stop hook must log to .squad/log/signal.log: {}",
            stop_cmd
        );
        assert!(
            !stop_cmd.contains("display-message"),
            "Stop hook must NOT use fragile tmux display-message: {}",
            stop_cmd
        );
    }

    #[test]
    fn test_install_claude_hooks_no_json_stdout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let settings_file = tmp.path().join(".claude").join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_claude_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let stop_cmd = settings["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(
            !stop_cmd.contains("printf"),
            "Claude Code hook must NOT add printf '{{}}' — stdout is ignored: {}",
            stop_cmd
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
        // SessionStart must NOT be added by base hooks
        assert!(settings["hooks"]["SessionStart"].is_null());
    }

    #[test]
    fn test_install_gemini_hooks_json_stdout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_gemini_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let signal_cmd = settings["hooks"]["AfterAgent"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        // Must end with printf '{}' for valid JSON stdout
        assert!(
            signal_cmd.contains("printf '{}'"),
            "Gemini hook MUST output valid JSON via printf: {}",
            signal_cmd
        );
        // Must redirect signal stdout to log (not to Gemini's stdout)
        assert!(
            signal_cmd.contains(">>.squad/log/signal.log 2>&1"),
            "Gemini hook must redirect signal output to log: {}",
            signal_cmd
        );
    }

    #[test]
    fn test_install_gemini_hooks_uses_afteragent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_gemini_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Must use AfterAgent, NOT Stop
        assert!(
            settings["hooks"]["AfterAgent"].is_array(),
            "Gemini must use AfterAgent hook"
        );
        assert!(
            settings["hooks"]["Stop"].is_null(),
            "Gemini must NOT use Stop hook"
        );
    }

    #[test]
    fn test_install_gemini_hooks_has_name_and_timeout() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_gemini_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let hook = &settings["hooks"]["AfterAgent"][0]["hooks"][0];
        assert_eq!(
            hook["name"].as_str().unwrap(),
            "squad-signal",
            "Gemini hook must have name field"
        );
        assert!(
            hook["description"].is_string(),
            "Gemini hook must have description field"
        );
        assert_eq!(
            hook["timeout"].as_u64().unwrap(),
            30000,
            "Gemini hook must have timeout field"
        );
    }

    #[test]
    fn test_install_gemini_hooks_uses_squad_agent_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        let settings_str = settings_file.to_str().unwrap();

        install_gemini_hooks(settings_str).unwrap();

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        let signal_cmd = settings["hooks"]["AfterAgent"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(
            signal_cmd.contains("SQUAD_AGENT_NAME"),
            "Gemini hook must use $SQUAD_AGENT_NAME: {}",
            signal_cmd
        );
        assert!(
            !signal_cmd.contains("display-message"),
            "Gemini hook must NOT use fragile tmux display-message: {}",
            signal_cmd
        );
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
        let settings_file = claude_dir.join("settings.json");
        // Pre-populate with base hooks
        std::fs::write(&settings_file, r#"{"hooks":{"Stop":[]}}"#).unwrap();

        let result = install_session_start_hook("claude-code", tmp.path());
        assert!(result.unwrap());

        let content = std::fs::read_to_string(&settings_file).unwrap();
        let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

        // SessionStart hook installed
        let ss = &settings["hooks"]["SessionStart"];
        assert!(ss.is_array(), "SessionStart hook must exist");
        let ss_cmd = ss[0]["hooks"][0]["command"].as_str().unwrap();
        assert_eq!(ss_cmd, "squad-station context --inject");

        // Existing hooks preserved
        assert!(settings["hooks"]["Stop"].is_array());
    }

    #[test]
    fn test_install_session_start_hook_gemini() {
        let tmp = tempfile::TempDir::new().unwrap();
        let gemini_dir = tmp.path().join(".gemini");
        std::fs::create_dir_all(&gemini_dir).unwrap();
        let settings_file = gemini_dir.join("settings.json");
        std::fs::write(&settings_file, r#"{"hooks":{"AfterAgent":[]}}"#).unwrap();

        let result = install_session_start_hook("gemini-cli", tmp.path());
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
        assert!(!install_session_start_hook("antigravity", tmp.path()).unwrap());
        assert!(!install_session_start_hook("unknown-tool", tmp.path()).unwrap());
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
