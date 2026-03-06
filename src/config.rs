use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level squad configuration
#[derive(Deserialize, Debug)]
pub struct SquadConfig {
    pub project: ProjectConfig,
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

/// Project-level configuration
#[derive(Deserialize, Debug)]
pub struct ProjectConfig {
    pub name: String,
    /// Optional custom DB path; defaults to ~/.agentic-squad/<name>/station.db
    pub db_path: Option<String>,
}

/// Agent configuration (used for both orchestrator and worker agents)
#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    pub name: String,
    /// Provider label only — no built-in mappings (e.g., "claude-code", "gemini")
    pub provider: String,
    pub role: String,
    /// Actual launch command
    pub command: String,
}

/// Load squad configuration from a YAML file
pub fn load_config(path: &Path) -> Result<SquadConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: SquadConfig = serde_saphyr::from_str(&content)?;
    Ok(config)
}

/// Resolve the DB path from config or use the default
pub fn resolve_db_path(config: &SquadConfig) -> Result<PathBuf> {
    let db_path = if let Some(ref custom_path) = config.project.db_path {
        PathBuf::from(custom_path)
    } else {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot determine home directory"))?;
        home.join(".agentic-squad")
            .join(&config.project.name)
            .join("station.db")
    };

    // Ensure the parent directory exists (Pitfall 6)
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(db_path)
}
