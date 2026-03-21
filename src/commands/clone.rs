use crate::{commands::context, config, db, tmux};

pub async fn run(source_name: String, json: bool) -> anyhow::Result<()> {
    // 1. Resolve DB path (same pattern as register.rs)
    let config_path = std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE);
    let db_path = if config_path.exists() {
        let cfg = config::load_config(config_path)?;
        config::resolve_db_path(&cfg)?
    } else if let Ok(env_path) = std::env::var("SQUAD_STATION_DB") {
        std::path::PathBuf::from(env_path)
    } else {
        anyhow::bail!("No squad.yml found in current directory. Run 'squad-station init' first, or set SQUAD_STATION_DB env var.");
    };

    let pool = db::connect(&db_path).await?;

    // 2. Fetch source agent from DB
    let source = db::agents::get_agent(&pool, &source_name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not found", source_name))?;

    // 3. CLONE-04: Reject orchestrator cloning
    if source.role == "orchestrator" {
        anyhow::bail!("cannot clone orchestrator agent");
    }

    // 4. CLONE-02: Generate auto-incremented name
    let clone_name_raw = generate_clone_name(&source.name, &pool).await?;
    let clone_name = config::sanitize_session_name(&clone_name_raw);

    // 5. CLONE-03: DB-first — insert clone agent record
    db::agents::insert_agent(
        &pool,
        &clone_name,
        &source.tool,
        &source.role,
        source.model.as_deref(),
        source.description.as_deref(),
        source.routing_hints.as_deref(),
    )
    .await?;

    // 6. Launch tmux session (skip for antigravity/DB-only agents)
    if source.tool != "antigravity" {
        let project_root = config::find_project_root()?;
        let project_root_str = project_root.to_string_lossy().to_string();
        let cmd = get_launch_command(&source.tool, source.model.as_deref());

        if let Err(e) = tmux::launch_agent_in_dir(&clone_name, &cmd, &project_root_str).await {
            // CLONE-03: Rollback DB record on tmux failure
            let _ = db::agents::delete_agent_by_name(&pool, &clone_name).await;
            anyhow::bail!("Clone failed: tmux session launch error: {e:#}");
        }
    }

    // 7. Output result
    if json {
        // CLONE-05: Auto-regenerate context (best-effort)
        let context_ok = context::run(false).await.is_ok();
        let output = serde_json::json!({
            "cloned": true,
            "source": source_name,
            "name": clone_name,
            "context_regenerated": context_ok,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("Cloned {} -> {}", source_name, clone_name);
        // CLONE-05: Auto-regenerate context (best-effort)
        match context::run(false).await {
            Ok(()) => println!("Regenerated squad-orchestrator.md"),
            Err(e) => eprintln!(
                "Warning: context regeneration failed: {e:#}. Run 'squad-station context' manually."
            ),
        }
    }

    Ok(())
}

/// Generate the next clone name by finding the highest existing N suffix.
///
/// Rules from CONTEXT.md:
/// - Append `-N` suffix: first clone gets `-2`, next `-3`, etc.
/// - No gap-filling — always increment from highest existing N
/// - Check uniqueness against both DB agents table AND live tmux sessions
/// - If source name already ends with `-N`, strip suffix to find base name
///   (cloning `worker-3` produces `worker-4`, not `worker-3-2`)
pub async fn generate_clone_name(
    source_name: &str,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<String> {
    let base = strip_clone_suffix(source_name);

    // Collect all existing names from DB and tmux
    let db_agents = db::agents::list_agents(pool).await?;
    let db_names: Vec<&str> = db_agents.iter().map(|a| a.name.as_str()).collect();
    let tmux_names = tmux::list_live_session_names().await;

    // Find highest N among names matching `{base}-N`
    let mut max_n: u32 = 1; // base agent is implicitly "-1"
    for name in db_names
        .iter()
        .copied()
        .chain(tmux_names.iter().map(|s| s.as_str()))
    {
        if let Some(n) = extract_clone_number(name, base) {
            if n > max_n {
                max_n = n;
            }
        }
    }

    // Also count the base name itself as N=1
    Ok(format!("{}-{}", base, max_n + 1))
}

/// Strip trailing `-N` suffix if present (where N is a positive integer >= 2).
/// Examples: "worker-3" -> "worker", "my-project-cc-worker" -> "my-project-cc-worker"
pub fn strip_clone_suffix(name: &str) -> &str {
    if let Some(pos) = name.rfind('-') {
        let suffix = &name[pos + 1..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            let n: u32 = suffix.parse().unwrap_or(0);
            if n >= 2 {
                return &name[..pos];
            }
        }
    }
    name
}

/// Extract clone number N from a name matching `{base}-N` pattern.
/// Returns None if name doesn't match the pattern.
pub fn extract_clone_number(name: &str, base: &str) -> Option<u32> {
    let suffix = name.strip_prefix(base)?.strip_prefix('-')?;
    if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

/// Build launch command from agent tool + model (replicates init.rs get_launch_command logic).
/// Cannot reuse init.rs version directly because it takes &AgentConfig, not DB fields.
pub fn get_launch_command(tool: &str, model: Option<&str>) -> String {
    match tool {
        "claude-code" => {
            let mut cmd = "claude --dangerously-skip-permissions".to_string();
            if let Some(m) = model {
                cmd.push_str(&format!(" --model {}", m));
            }
            cmd
        }
        "gemini-cli" => {
            let mut cmd = "gemini -y".to_string();
            if let Some(m) = model {
                cmd.push_str(&format!(" --model {}", m));
            }
            cmd
        }
        _ => "zsh".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_clone_suffix_with_clone_number() {
        assert_eq!(strip_clone_suffix("worker-2"), "worker");
        assert_eq!(strip_clone_suffix("worker-3"), "worker");
        assert_eq!(strip_clone_suffix("worker-10"), "worker");
    }

    #[test]
    fn test_strip_clone_suffix_preserves_base_and_one_suffix() {
        // Names ending in -1 are NOT clones (convention: clones start at -2)
        assert_eq!(strip_clone_suffix("worker-1"), "worker-1");
        assert_eq!(strip_clone_suffix("worker"), "worker");
        assert_eq!(strip_clone_suffix("my-project-cc-worker"), "my-project-cc-worker");
    }

    #[test]
    fn test_extract_clone_number() {
        assert_eq!(extract_clone_number("worker-2", "worker"), Some(2));
        assert_eq!(extract_clone_number("worker-10", "worker"), Some(10));
        assert_eq!(extract_clone_number("worker", "worker"), None);
        assert_eq!(extract_clone_number("other-2", "worker"), None);
        assert_eq!(extract_clone_number("worker-abc", "worker"), None);
    }

    #[test]
    fn test_get_launch_command_claude() {
        let cmd = get_launch_command("claude-code", None);
        assert_eq!(cmd, "claude --dangerously-skip-permissions");
    }

    #[test]
    fn test_get_launch_command_claude_with_model() {
        let cmd = get_launch_command("claude-code", Some("claude-opus-4-5"));
        assert_eq!(cmd, "claude --dangerously-skip-permissions --model claude-opus-4-5");
    }

    #[test]
    fn test_get_launch_command_gemini() {
        let cmd = get_launch_command("gemini-cli", None);
        assert_eq!(cmd, "gemini -y");
    }

    #[test]
    fn test_get_launch_command_unknown() {
        let cmd = get_launch_command("antigravity", None);
        assert_eq!(cmd, "zsh");
    }
}
