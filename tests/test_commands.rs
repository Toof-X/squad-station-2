mod helpers;

use squad_station::config::{self, SquadConfig};

// ============================================================
// Config parsing tests — SESS-01 (updated for new format)
// ============================================================

#[test]
fn test_config_parse_valid_yaml() {
    let yaml = r#"
project: test-squad
orchestrator:
  name: test-orchestrator
  tool: claude-code
  role: orchestrator
agents:
  - name: frontend
    tool: claude-code
    role: worker
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project, "test-squad");
    assert_eq!(config.orchestrator.name.as_deref(), Some("test-orchestrator"));
    assert_eq!(config.orchestrator.tool, "claude-code");
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name.as_deref(), Some("frontend"));
    assert_eq!(config.agents[0].role, "worker");
}

#[test]
fn test_config_parse_multiple_agents() {
    let yaml = r#"
project: multi-squad
orchestrator:
  name: orch
  tool: claude-code
  role: orchestrator
agents:
  - name: frontend
    tool: claude-code
    role: worker
  - name: backend
    tool: gemini
    role: worker
  - name: reviewer
    tool: claude-code
    role: worker
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project, "multi-squad");
    assert_eq!(config.agents.len(), 3);
    assert_eq!(config.agents[1].tool, "gemini");
}

#[test]
fn test_config_parse_missing_required_field_returns_error() {
    // YAML missing required `orchestrator` field must return an error, not panic
    let yaml = r#"
project: broken-squad
agents:
  - name: worker
    tool: claude-code
    role: worker
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
project: my-project
orchestrator:
  name: orch
  tool: claude-code
  role: orchestrator
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
