use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Allowed provider values for squad.yml
const VALID_PROVIDERS: &[&str] = &["antigravity", "claude-code", "gemini-cli"];

/// Valid model identifiers per provider (provider → allowed model slugs)
fn valid_models_for(provider: &str) -> Option<&'static [&'static str]> {
    match provider {
        "claude-code" => Some(&[
            "opus",
            "sonnet",
            "haiku",
            "claude-sonnet-4-6",
            "claude-opus-4-6",
            "claude-haiku-4-5",
        ]),
        "gemini-cli" => Some(&[
            "gemini-3.1-pro-preview",
            "gemini-3-flash-preview",
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
        ]),
        _ => None, // no model validation for providers that don't support a model override
    }
}

/// SDD (Software Design Document) workflow configuration
#[derive(Deserialize, Debug, Clone)]
pub struct SddConfig {
    pub name: String,
    pub playbook: String, // absolute path to playbook .md file
}

/// Top-level squad configuration
#[derive(Deserialize, Debug)]
pub struct SquadConfig {
    pub project: String,             // CONF-01: plain string (not a nested struct)
    pub sdd: Option<Vec<SddConfig>>, // optional SDD workflow configs
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

impl SquadConfig {
    /// Validate all agent configs (orchestrator + workers).
    /// Returns a descriptive error on the first invalid provider or model found.
    pub fn validate(&self) -> Result<()> {
        let label = self.orchestrator.name.as_deref().unwrap_or("orchestrator");
        validate_agent_config(label, &self.orchestrator)?;
        for agent in &self.agents {
            let label = agent.name.as_deref().unwrap_or(&agent.role);
            validate_agent_config(label, agent)?;
        }
        Ok(())
    }
}

/// Agent configuration (used for both orchestrator and worker agents)
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub name: Option<String>, // optional; orchestrator name auto-derived in Phase 5
    pub provider: String,     // CONF-04: provider name (e.g. claude-code, gemini-cli, antigravity)
    #[serde(default = "default_role")]
    pub role: String,
    pub model: Option<String>, // CONF-02: optional model override
    pub description: Option<String>, // CONF-02: optional description
                               // command field is REMOVED (CONF-03: provider infers launch command)
}

/// Sanitize a string for use as a tmux session name.
/// Replaces shell metacharacters and tmux-special chars with `-` to prevent
/// both tmux targeting issues and shell injection in `sh -c` commands.
pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '.' | ':' | '"' | '\'' | '`' | '$' | ';' | '(' | ')' | '|' | '&' | '<' | '>'
            | '\\' | ' ' | '\n' | '\0' | '/' => '-',
            _ => c,
        })
        .collect()
}

impl AgentConfig {
    /// Returns true when the agent uses DB-only mode (no tmux session).
    /// Currently only "antigravity" is DB-only. All other provider values use tmux.
    pub fn is_db_only(&self) -> bool {
        self.provider == "antigravity"
    }
}

fn default_role() -> String {
    "worker".to_string()
}

/// Validate provider and (optionally) model for a single agent config.
/// Known providers get model validation; unknown providers get a warning but proceed.
fn validate_agent_config(label: &str, agent: &AgentConfig) -> Result<()> {
    if !VALID_PROVIDERS.contains(&agent.provider.as_str()) {
        // Unknown provider: warn but don't fail — allows extensibility
        eprintln!(
            "Warning: Unknown provider '{}' for agent '{}'. \
             Known providers: {}. Proceeding without model validation.",
            agent.provider,
            label,
            VALID_PROVIDERS.join(", ")
        );
        return Ok(()); // skip model validation for unknown providers
    }

    // Model validation (only for known providers with a model list)
    if let Some(model) = &agent.model {
        if let Some(valid_models) = valid_models_for(&agent.provider) {
            if !valid_models.contains(&model.as_str()) {
                bail!(
                    "Invalid model '{}' for provider '{}' (agent '{}'). Valid models are: {}.",
                    model,
                    agent.provider,
                    label,
                    valid_models.join(", ")
                );
            }
        }
    }

    Ok(())
}

/// Walk up the directory tree to find `squad.yml`, returning the project root directory.
/// Similar to how git finds `.git/` or cargo finds `Cargo.toml`.
pub fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()
        .map_err(|e| anyhow!("Cannot determine current directory: {}", e))?;
    loop {
        if dir.join("squad.yml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("squad.yml not found in current directory or any parent directory. Run 'squad-station init' with a squad.yml config file.");
        }
    }
}

/// Load squad configuration from a YAML file and validate its contents.
/// If the default `squad.yml` path doesn't exist, walks up the directory tree.
/// Explicit non-default paths (e.g. `/tmp/custom.yml`) are NOT searched up the tree.
pub fn load_config(path: &Path) -> Result<SquadConfig> {
    let is_default_path = path == Path::new("squad.yml");
    let config_path = if path.exists() {
        path.to_path_buf()
    } else if is_default_path {
        // Walk up the directory tree to find squad.yml (supports orchestrator subdirectory)
        find_project_root()?.join("squad.yml")
    } else {
        // Explicit path given but not found — don't walk up
        path.to_path_buf()
    };
    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!("squad.yml not found in current directory or any parent directory. Run 'squad-station init' with a squad.yml config file.")
        } else {
            anyhow!("Failed to read {}: {}", config_path.display(), e)
        }
    })?;
    let config: SquadConfig = serde_saphyr::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

/// Resolve the DB path. Uses project root (where squad.yml lives), not CWD.
/// SQUAD_STATION_DB env var overrides the default path (useful for testing).
pub fn resolve_db_path(_config: &SquadConfig) -> Result<PathBuf> {
    let db_path = if let Ok(env_path) = std::env::var("SQUAD_STATION_DB") {
        PathBuf::from(env_path)
    } else {
        let project_root = find_project_root()?;
        project_root.join(".squad").join("station.db")
    };

    // Ensure the parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(provider: &str, model: Option<&str>) -> AgentConfig {
        AgentConfig {
            name: None,
            provider: provider.to_string(),
            role: "worker".to_string(),
            model: model.map(str::to_string),
            description: None,
        }
    }

    #[test]
    fn valid_provider_no_model() {
        assert!(validate_agent_config("orch", &make_agent("claude-code", None)).is_ok());
        assert!(validate_agent_config("orch", &make_agent("gemini-cli", None)).is_ok());
        assert!(validate_agent_config("orch", &make_agent("antigravity", None)).is_ok());
    }

    #[test]
    fn unknown_provider_warns_but_succeeds() {
        // GAP-03: unknown providers warn to stderr but don't fail
        assert!(validate_agent_config("agent1", &make_agent("gemini", None)).is_ok());
        assert!(validate_agent_config("agent1", &make_agent("codex", None)).is_ok());
        assert!(validate_agent_config("agent1", &make_agent("opencode", None)).is_ok());
    }

    #[test]
    fn valid_model_accepted() {
        assert!(validate_agent_config("a", &make_agent("claude-code", Some("sonnet"))).is_ok());
        assert!(validate_agent_config(
            "a",
            &make_agent("gemini-cli", Some("gemini-3-flash-preview"))
        )
        .is_ok());
    }

    #[test]
    fn full_model_ids_accepted_claude() {
        assert!(
            validate_agent_config("a", &make_agent("claude-code", Some("claude-sonnet-4-6")))
                .is_ok()
        );
        assert!(
            validate_agent_config("a", &make_agent("claude-code", Some("claude-opus-4-6")))
                .is_ok()
        );
        assert!(
            validate_agent_config("a", &make_agent("claude-code", Some("claude-haiku-4-5")))
                .is_ok()
        );
    }

    #[test]
    fn full_model_ids_accepted_gemini() {
        assert!(
            validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-2.5-pro"))).is_ok()
        );
        assert!(
            validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-2.5-flash")))
                .is_ok()
        );
        assert!(
            validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-2.5-flash-lite")))
                .is_ok()
        );
    }

    #[test]
    fn invalid_model_rejected() {
        let err = validate_agent_config("a", &make_agent("claude-code", Some("claude-code-2")))
            .unwrap_err();
        assert!(err.to_string().contains("opus, sonnet, haiku"));

        let err =
            validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-pro"))).unwrap_err();
        assert!(err
            .to_string()
            .contains("gemini-3.1-pro-preview, gemini-3-flash-preview"));
    }

    #[test]
    fn antigravity_model_not_validated() {
        // antigravity has no known model list — any model value (or none) is accepted
        assert!(
            validate_agent_config("orch", &make_agent("antigravity", Some("anything"))).is_ok()
        );
    }

    #[test]
    fn sanitize_session_name_replaces_dots() {
        assert_eq!(sanitize_session_name("my.app-worker"), "my-app-worker");
    }

    #[test]
    fn sanitize_session_name_replaces_colons() {
        assert_eq!(sanitize_session_name("proj:v2-agent"), "proj-v2-agent");
    }

    #[test]
    fn sanitize_session_name_replaces_quotes() {
        assert_eq!(
            sanitize_session_name(r#"proj"name-agent"#),
            "proj-name-agent"
        );
    }

    #[test]
    fn sanitize_session_name_clean_passthrough() {
        assert_eq!(
            sanitize_session_name("squad-station-implement"),
            "squad-station-implement"
        );
    }

    #[test]
    fn deny_unknown_fields_in_agent_config() {
        // BUG-16: unknown fields like tmux-session must be rejected
        let yaml = r#"
project: test
orchestrator:
  provider: claude-code
  role: orchestrator
agents:
  - name: worker
    provider: claude-code
    role: worker
    tmux-session: custom-name
"#;
        let result: Result<SquadConfig, _> = serde_saphyr::from_str(yaml);
        assert!(
            result.is_err(),
            "unknown field 'tmux-session' should be rejected"
        );
    }
}
