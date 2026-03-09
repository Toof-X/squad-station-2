use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Allowed provider values for squad.yml
const VALID_PROVIDERS: &[&str] = &["antigravity", "claude-code", "gemini-cli"];

/// Valid model identifiers per provider (provider → allowed model slugs)
fn valid_models_for(provider: &str) -> Option<&'static [&'static str]> {
    match provider {
        "claude-code" => Some(&["opus", "sonnet", "haiku"]),
        "gemini-cli" => Some(&["gemini-3.1-pro-preview", "gemini-3-flash-preview"]),
        _ => None, // no model validation for providers that don't support a model override
    }
}

/// Top-level squad configuration
#[derive(Deserialize, Debug)]
pub struct SquadConfig {
    pub project: String, // CONF-01: plain string (not a nested struct)
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

impl SquadConfig {
    /// Validate all agent configs (orchestrator + workers).
    /// Returns a descriptive error on the first invalid provider or model found.
    pub fn validate(&self) -> Result<()> {
        let label = self
            .orchestrator
            .name
            .as_deref()
            .unwrap_or("orchestrator");
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
pub struct AgentConfig {
    pub name: Option<String>, // optional; orchestrator name auto-derived in Phase 5
    pub provider: String,     // CONF-04: provider name (e.g. claude-code, gemini-cli, antigravity)
    #[serde(default = "default_role")]
    pub role: String,
    pub model: Option<String>,       // CONF-02: optional model override
    pub description: Option<String>, // CONF-02: optional description
                                     // command field is REMOVED (CONF-03: provider infers launch command)
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
fn validate_agent_config(label: &str, agent: &AgentConfig) -> Result<()> {
    // Provider whitelist check
    if !VALID_PROVIDERS.contains(&agent.provider.as_str()) {
        bail!(
            "Invalid provider '{}' for agent '{}'. Valid providers are: {}.",
            agent.provider,
            label,
            VALID_PROVIDERS.join(", ")
        );
    }

    // Model validation (only for providers that have a known model list)
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

/// Load squad configuration from a YAML file and validate its contents.
pub fn load_config(path: &Path) -> Result<SquadConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: SquadConfig = serde_saphyr::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

/// Resolve the DB path from config or use the default.
/// SQUAD_STATION_DB env var overrides the default path (useful for testing).
pub fn resolve_db_path(config: &SquadConfig) -> Result<PathBuf> {
    let db_path = if let Ok(env_path) = std::env::var("SQUAD_STATION_DB") {
        PathBuf::from(env_path)
    } else {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot determine home directory"))?;
        home.join(".agentic-squad")
            .join(&config.project) // config.project is now a String directly
            .join("station.db")
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
    fn invalid_provider_rejected() {
        let err = validate_agent_config("agent1", &make_agent("gemini", None)).unwrap_err();
        assert!(err.to_string().contains("antigravity, claude-code, gemini-cli"));
        assert!(err.to_string().contains("agent1"));
    }

    #[test]
    fn valid_model_accepted() {
        assert!(validate_agent_config("a", &make_agent("claude-code", Some("sonnet"))).is_ok());
        assert!(validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-3-flash-preview"))).is_ok());
    }

    #[test]
    fn invalid_model_rejected() {
        let err = validate_agent_config("a", &make_agent("claude-code", Some("claude-code-2"))).unwrap_err();
        assert!(err.to_string().contains("opus, sonnet, haiku"));

        let err = validate_agent_config("a", &make_agent("gemini-cli", Some("gemini-pro"))).unwrap_err();
        assert!(err.to_string().contains("gemini-3.1-pro-preview, gemini-3-flash-preview"));
    }

    #[test]
    fn antigravity_model_not_validated() {
        // antigravity has no known model list — any model value (or none) is accepted
        assert!(validate_agent_config("orch", &make_agent("antigravity", Some("anything"))).is_ok());
    }
}
