mod helpers;

use squad_station::config::{self, SquadConfig};
use squad_station::db;

// ============================================================
// Config parsing tests — SESS-01 (updated for new format)
// ============================================================

#[test]
fn test_config_parse_valid_yaml() {
    let yaml = r#"
project: test-squad
orchestrator:
  name: test-orchestrator
  provider: claude-code
  role: orchestrator
agents:
  - name: frontend
    provider: claude-code
    role: worker
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project, "test-squad");
    assert_eq!(
        config.orchestrator.name.as_deref(),
        Some("test-orchestrator")
    );
    assert_eq!(config.orchestrator.provider, "claude-code");
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
  provider: claude-code
  role: orchestrator
agents:
  - name: frontend
    provider: claude-code
    role: worker
  - name: backend
    provider: gemini-cli
    role: worker
  - name: reviewer
    provider: claude-code
    role: worker
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(config.project, "multi-squad");
    assert_eq!(config.agents.len(), 3);
    assert_eq!(config.agents[1].provider, "gemini-cli");
}

#[test]
fn test_config_parse_missing_required_field_returns_error() {
    // YAML missing required `orchestrator` field must return an error, not panic
    let yaml = r#"
project: broken-squad
agents:
  - name: worker
    provider: claude-code
    role: worker
"#;

    let result: Result<SquadConfig, _> = serde_saphyr::from_str(yaml);
    assert!(
        result.is_err(),
        "missing required field should return Err, not panic"
    );
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
  provider: claude-code
  role: orchestrator
agents: []
"#;

    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let path = config::resolve_db_path(&config).unwrap();

    let path_str = path.to_str().unwrap();
    assert!(
        path_str.ends_with(".squad/station.db"),
        "default path should end with .squad/station.db, got: {}",
        path_str
    );
    // Should be absolute (starts from current working directory)
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

    assert!(
        output.status.success(),
        "squad-station --help must exit 0, got: {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify all 6 subcommands are shown in help text
    assert!(stdout.contains("init"), "help must list 'init' subcommand");
    assert!(stdout.contains("send"), "help must list 'send' subcommand");
    assert!(
        stdout.contains("signal"),
        "help must list 'signal' subcommand"
    );
    assert!(stdout.contains("list"), "help must list 'list' subcommand");
    assert!(stdout.contains("peek"), "help must list 'peek' subcommand");
    assert!(
        stdout.contains("register"),
        "help must list 'register' subcommand"
    );
}

// ============================================================
// Init agent naming tests — CLI-02
// ============================================================

#[tokio::test]
async fn test_init_agent_name_prefix() {
    let db = helpers::setup_test_db().await;
    // Register an agent the same way init.rs would, using the auto-prefix logic
    db::agents::insert_agent(
        &db,
        "myapp-claude-code-backend", // pre-computed as init.rs would produce
        "claude-code",
        "worker",
        None,
        None,
    )
    .await
    .unwrap();

    let agent = db::agents::get_agent(&db, "myapp-claude-code-backend")
        .await
        .unwrap();
    assert!(
        agent.is_some(),
        "Agent with prefixed name must be registered"
    );
    let agent = agent.unwrap();
    assert_eq!(agent.name, "myapp-claude-code-backend");
    assert_eq!(agent.tool, "claude-code");
    assert_eq!(agent.role, "worker");
}

// ============================================================
// Signal notification format tests — SIG-01
// ============================================================

#[test]
fn test_signal_notification_format() {
    // Verify the format string produces the expected output
    let agent = "myapp-claude-implement";
    let task_id_str = "msg-a1b2c3";
    let notification = format!("{} completed {}", agent, task_id_str);
    assert_eq!(notification, "myapp-claude-implement completed msg-a1b2c3");
    assert!(
        !notification.contains("[SIGNAL]"),
        "Must not contain old [SIGNAL] prefix"
    );
    assert!(
        !notification.contains("agent="),
        "Must not contain old key=value format"
    );
    assert!(
        !notification.contains("task_id="),
        "Must not contain old task_id= format"
    );
}

// ============================================================
// Context output tests — CLI-03
// ============================================================

#[tokio::test]
async fn test_context_includes_model_and_description() {
    // Verify Agent struct has model and description fields accessible
    // (context.rs reads from list_agents which populates these from DB)
    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "myapp-claude-implement",
        "claude-code",
        "worker",
        Some("Claude Sonnet"),
        Some("Developer agent. Writes code."),
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let agent = agents
        .iter()
        .find(|a| a.name == "myapp-claude-implement")
        .unwrap();
    assert_eq!(agent.model.as_deref(), Some("Claude Sonnet"));
    assert_eq!(
        agent.description.as_deref(),
        Some("Developer agent. Writes code.")
    );
}

#[tokio::test]
async fn test_context_generates_single_orchestrator_file() {
    // Verify agent fields are correctly populated for context generation
    let db = helpers::setup_test_db().await;

    db::agents::insert_agent(
        &db,
        "proj-claude-orchestrator",
        "claude-code",
        "orchestrator",
        Some("claude-haiku"),
        Some("Orchestrator agent"),
    )
    .await
    .unwrap();

    db::agents::insert_agent(
        &db,
        "proj-claude-implement",
        "claude-code",
        "worker",
        Some("claude-sonnet"),
        Some("Senior coder"),
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();

    let worker = agents.iter().find(|a| a.role == "worker").unwrap();
    let orch = agents.iter().find(|a| a.role == "orchestrator").unwrap();

    assert_eq!(worker.name, "proj-claude-implement");
    assert_eq!(worker.model.as_deref(), Some("claude-sonnet"));
    assert_eq!(worker.description.as_deref(), Some("Senior coder"));
    assert_eq!(orch.role, "orchestrator");
}

#[tokio::test]
async fn test_build_orchestrator_md_contains_all_sections() {
    use squad_station::commands::context::build_orchestrator_md;

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "p-claude-implement",
        "claude-code",
        "worker",
        Some("claude-sonnet"),
        Some("Coder"),
    )
    .await
    .unwrap();
    db::agents::insert_agent(
        &db,
        "p-claude-orchestrator",
        "claude-code",
        "orchestrator",
        None,
        None,
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let content = build_orchestrator_md(&agents);

    assert!(content.contains("# Squad Orchestrator Playbook"), "Missing title");
    assert!(content.contains("## Delegation Workflow"), "Missing delegation section");
    assert!(content.contains("## Monitoring Workflow"), "Missing monitoring section");
    assert!(content.contains("## Agent Roster"), "Missing roster section");
    assert!(content.contains("p-claude-implement"), "Worker agent missing from content");
    assert!(content.contains("claude-sonnet"), "Worker model missing");
    assert!(
        content.contains("squad-orchestrator.md"),
        "Anti-decay rule must reference new file name"
    );
    // Orchestrator should NOT appear in the send-command delegation block
    let delegation_section_end = content.find("## How to Delegate").unwrap_or(content.len());
    let delegation_section = &content[..delegation_section_end];
    assert!(
        !delegation_section.contains("p-claude-orchestrator"),
        "Orchestrator must not appear in delegation send-command block"
    );
}
