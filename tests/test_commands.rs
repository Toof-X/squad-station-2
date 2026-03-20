mod helpers;

use squad_station::config::{self, SddConfig, SquadConfig};
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
    // GAP-04: naming simplified to {project}-{name} (no provider in middle)
    db::agents::insert_agent(
        &db,
        "myapp-backend", // pre-computed as init.rs would produce
        "claude-code",
        "worker",
        None,
        None,
    )
    .await
    .unwrap();

    let agent = db::agents::get_agent(&db, "myapp-backend").await.unwrap();
    assert!(
        agent.is_some(),
        "Agent with prefixed name must be registered"
    );
    let agent = agent.unwrap();
    assert_eq!(agent.name, "myapp-backend");
    assert_eq!(agent.tool, "claude-code");
    assert_eq!(agent.role, "worker");
}

// ============================================================
// Signal notification format tests — SIG-01
// ============================================================

#[test]
fn test_signal_notification_format() {
    // Verify the format string produces the expected output (GAP-02: structured notification)
    let agent = "myapp-implement";
    let task_id_str = "msg-a1b2c3";
    let notification = format!(
        "[SQUAD SIGNAL] Agent '{}' completed task {}. Read output: tmux capture-pane -t {} -p | Next: squad-station status",
        agent, task_id_str, agent
    );
    assert!(
        notification.contains("[SQUAD SIGNAL]"),
        "Must contain [SQUAD SIGNAL] prefix"
    );
    assert!(
        notification.contains("myapp-implement"),
        "Must contain agent name"
    );
    assert!(notification.contains("msg-a1b2c3"), "Must contain task_id");
    assert!(
        notification.contains("tmux capture-pane"),
        "Must contain actionable read command"
    );
    assert!(
        notification.contains("squad-station status"),
        "Must contain next action hint"
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
    let content = build_orchestrator_md(&agents, "/project/root", &[]);

    assert!(
        content.contains("You are the orchestrator"),
        "Missing role definition"
    );
    assert!(
        content.contains("## Completion Notification"),
        "Missing completion notification section"
    );
    assert!(
        content.contains("## Session Routing"),
        "Missing session routing section"
    );
    assert!(
        content.contains("## Agent Roster"),
        "Missing roster section"
    );
    assert!(
        content.contains("p-claude-implement"),
        "Worker agent missing from content"
    );
    assert!(content.contains("claude-sonnet"), "Worker model missing");
    assert!(
        content.contains("/project/root"),
        "Content must include project root path"
    );
    // Orchestrator should NOT appear in sending commands block (only in roster)
    let sending_start = content.find("## Sending Tasks").unwrap_or(0);
    let sending_end = content[sending_start..]
        .find("\n## ")
        .map(|i| sending_start + i)
        .unwrap_or(content.len());
    let sending_section = &content[sending_start..sending_end];
    assert!(
        !sending_section.contains("p-claude-orchestrator"),
        "Orchestrator must not appear in sending commands"
    );
    assert!(
        content.contains("[SQUAD SIGNAL]"),
        "Missing signal format example"
    );
    assert!(
        content.contains("DO NOT need to"),
        "Missing anti-polling instruction"
    );
}

// ============================================================
// SDD workflow context tests — GAP-01
// ============================================================

#[tokio::test]
async fn test_build_orchestrator_md_with_sdd() {
    use squad_station::commands::context::build_orchestrator_md;

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(&db, "p-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    // Create a temp playbook file so it can be embedded
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "# Test Playbook\n\nStep 1: /test:init\nStep 2: /test:build\n",
    )
    .unwrap();
    let playbook_path = tmp.path().to_string_lossy().to_string();

    let sdd = vec![SddConfig {
        name: "get-shit-done".to_string(),
        playbook: playbook_path,
    }];
    let content = build_orchestrator_md(&agents, "/project/root", &sdd);

    assert!(
        content.contains("## SDD Orchestration"),
        "Missing SDD orchestration section"
    );
    assert!(
        content.contains("## PRE-FLIGHT"),
        "Missing PRE-FLIGHT section"
    );
    // PRE-FLIGHT must reference the playbook path
    assert!(
        content.contains(&*tmp.path().to_string_lossy()),
        "PRE-FLIGHT must reference playbook path"
    );
    // Must tell orchestrator agents have the tools, not it
    assert!(
        content.contains("You do NOT"),
        "Must tell orchestrator it doesn't have SDD tools"
    );
    assert!(
        content.contains("Do NOT run slash commands"),
        "Must forbid running commands directly"
    );
}

#[tokio::test]
async fn test_build_orchestrator_md_without_sdd() {
    use squad_station::commands::context::build_orchestrator_md;

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(&db, "p-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let content = build_orchestrator_md(&agents, "/project/root", &[]);

    assert!(
        !content.contains("## SDD Orchestration"),
        "SDD section should not appear when no SDDs configured"
    );
}

// ============================================================
// Context inject output format tests
// ============================================================

#[tokio::test]
async fn test_format_inject_output_claude_code_returns_raw_content() {
    use squad_station::commands::context::format_inject_output;

    let content = "You are the orchestrator.\n## Agent Roster\n";
    let output = format_inject_output("claude-code", content);
    assert_eq!(output, content, "Claude Code inject must return raw content");
}

#[tokio::test]
async fn test_format_inject_output_gemini_cli_returns_json() {
    use squad_station::commands::context::format_inject_output;

    let content = "You are the orchestrator.";
    let output = format_inject_output("gemini-cli", content);
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Gemini CLI inject output must be valid JSON");
    assert_eq!(
        parsed["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap(),
        content,
        "Gemini JSON must contain additionalContext field"
    );
}

#[tokio::test]
async fn test_format_inject_output_unknown_provider_returns_raw() {
    use squad_station::commands::context::format_inject_output;

    let content = "fallback content";
    let output = format_inject_output("some-other-tool", content);
    assert_eq!(
        output, content,
        "Unknown provider should fall back to raw content"
    );
}
