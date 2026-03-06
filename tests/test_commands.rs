mod helpers;

use squad_station::config::{self, SquadConfig};

// ============================================================
// Config parsing tests — SESS-01
// ============================================================

#[test]
fn test_config_parse_valid_yaml() {
    let yaml = r#"
project:
  name: test-squad
orchestrator:
  name: test-orchestrator
  provider: claude-code
  role: orchestrator
  command: "claude --dangerously-skip-permissions"
agents:
  - name: frontend
    provider: claude-code
    role: worker
    command: "claude --dangerously-skip-permissions"
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project.name, "test-squad");
    assert_eq!(config.orchestrator.name, "test-orchestrator");
    assert_eq!(config.orchestrator.provider, "claude-code");
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name, "frontend");
    assert_eq!(config.agents[0].role, "worker");
}

#[test]
fn test_config_parse_multiple_agents() {
    let yaml = r#"
project:
  name: multi-squad
orchestrator:
  name: orch
  provider: claude-code
  role: orchestrator
  command: "claude"
agents:
  - name: frontend
    provider: claude-code
    role: worker
    command: "claude"
  - name: backend
    provider: gemini
    role: worker
    command: "gemini"
  - name: reviewer
    provider: claude-code
    role: worker
    command: "claude"
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project.name, "multi-squad");
    assert_eq!(config.agents.len(), 3);
    assert_eq!(config.agents[1].provider, "gemini");
}

#[test]
fn test_config_parse_with_db_path() {
    let yaml = r#"
project:
  name: custom-db
  db_path: "/custom/path/station.db"
orchestrator:
  name: orch
  provider: claude-code
  role: orchestrator
  command: "claude"
agents: []
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project.db_path.as_deref(), Some("/custom/path/station.db"));
}

#[test]
fn test_config_parse_missing_required_field_returns_error() {
    // YAML missing required `orchestrator` field must return an error, not panic
    let yaml = r#"
project:
  name: broken-squad
agents:
  - name: worker
    provider: claude-code
    role: worker
    command: "claude"
"#;

    let result: Result<SquadConfig, _> = serde_saphyr::from_str(yaml);
    assert!(result.is_err(), "missing required field should return Err, not panic");
}

// ============================================================
// DB path resolution tests — SESS-01
// ============================================================

#[test]
fn test_db_path_resolution_default() {
    let yaml = r#"
project:
  name: my-project
orchestrator:
  name: orch
  provider: claude-code
  role: orchestrator
  command: "claude"
agents: []
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let path = config::resolve_db_path(&config).unwrap();

    let path_str = path.to_str().unwrap();
    assert!(
        path_str.contains(".agentic-squad/my-project/station.db"),
        "default path should contain .agentic-squad/<project>/station.db, got: {}",
        path_str
    );
    // Should be absolute (starts from home directory)
    assert!(path.is_absolute(), "resolved DB path must be absolute");
}

#[test]
fn test_db_path_resolution_custom() {
    let yaml = r#"
project:
  name: my-project
  db_path: "/tmp/test-squad/station.db"
orchestrator:
  name: orch
  provider: claude-code
  role: orchestrator
  command: "claude"
agents: []
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let path = config::resolve_db_path(&config).unwrap();

    assert_eq!(
        path.to_str().unwrap(),
        "/tmp/test-squad/station.db",
        "custom db_path must be used as-is"
    );
}

// ============================================================
// SIGPIPE test — SAFE-04
// ============================================================

#[test]
fn test_sigpipe_binary_starts() {
    // SAFE-04: verify the binary starts cleanly (SIGPIPE handler doesn't crash startup)
    // and shows help text — implicitly tests that main() SIGPIPE reset doesn't panic.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_squad-station"))
        .arg("--help")
        .output()
        .expect("failed to run squad-station binary");

    assert!(output.status.success(), "squad-station --help must exit 0, got: {:?}", output.status);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify all 6 subcommands are shown in help text
    assert!(stdout.contains("init"), "help must list 'init' subcommand");
    assert!(stdout.contains("send"), "help must list 'send' subcommand");
    assert!(stdout.contains("signal"), "help must list 'signal' subcommand");
    assert!(stdout.contains("list"), "help must list 'list' subcommand");
    assert!(stdout.contains("peek"), "help must list 'peek' subcommand");
    assert!(stdout.contains("register"), "help must list 'register' subcommand");
}
