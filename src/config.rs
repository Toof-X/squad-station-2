use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level squad configuration
#[derive(Deserialize, Debug)]
pub struct SquadConfig {
    pub project: String,              // CONF-01: plain string (not a nested struct)
    pub orchestrator: AgentConfig,
    pub agents: Vec<AgentConfig>,
}

/// Agent configuration (used for both orchestrator and worker agents)
#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    pub name: Option<String>,         // optional; orchestrator name auto-derived in Phase 5
    pub tool: String,                 // CONF-04: renamed from provider
    #[serde(default = "default_role")]
    pub role: String,
    pub model: Option<String>,        // CONF-02: optional model override
    pub description: Option<String>,  // CONF-02: optional description
    // command field is REMOVED (CONF-03: tool infers launch command)
}

fn default_role() -> String {
    "worker".to_string()
}

/// Load squad configuration from a YAML file
pub fn load_config(path: &Path) -> Result<SquadConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: SquadConfig = serde_saphyr::from_str(&content)?;
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
            .join(&config.project)   // config.project is now a String directly
            .join("station.db")
    };

    // Ensure the parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(db_path)
}
