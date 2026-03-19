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
    let content = build_orchestrator_md(&agents, "/project/root", &[], &[]);

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
    let content = build_orchestrator_md(&agents, "/project/root", &sdd, &[]);

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
    let content = build_orchestrator_md(&agents, "/project/root", &[], &[]);

    assert!(
        !content.contains("## SDD Orchestration"),
        "SDD section should not appear when no SDDs configured"
    );
}

// ============================================================
// AgentMetrics, AlignmentResult, compute_alignment, format_busy_duration tests
// ============================================================

#[test]
fn test_compute_alignment_ok_overlap() {
    use squad_station::commands::context::{compute_alignment, AlignmentResult};
    let result = compute_alignment(
        "fix CSS grid layout bug",
        Some("Frontend engineer specializing in React and CSS"),
    );
    assert_eq!(result, AlignmentResult::Ok, "CSS overlap should return Ok");
}

#[test]
fn test_compute_alignment_warning_no_overlap() {
    use squad_station::commands::context::{compute_alignment, AlignmentResult};
    let result = compute_alignment(
        "fix CSS grid layout bug",
        Some("Backend API developer for database operations"),
    );
    match result {
        AlignmentResult::Warning { task_preview, role } => {
            assert!(
                task_preview.contains("fix CSS"),
                "task_preview should contain 'fix CSS', got: {}",
                task_preview
            );
            assert!(
                !role.is_empty(),
                "role should be non-empty"
            );
        }
        other => panic!("Expected Warning, got {:?}", other),
    }
}

#[test]
fn test_compute_alignment_none_empty_task() {
    use squad_station::commands::context::{compute_alignment, AlignmentResult};
    let result = compute_alignment("", Some("some description"));
    assert_eq!(
        result,
        AlignmentResult::None,
        "empty task should return None"
    );
}

#[test]
fn test_compute_alignment_none_no_description() {
    use squad_station::commands::context::{compute_alignment, AlignmentResult};
    let result = compute_alignment("deploy the app", None);
    assert_eq!(
        result,
        AlignmentResult::None,
        "no description should return None"
    );
}

#[test]
fn test_compute_alignment_stop_words_filtered() {
    use squad_station::commands::context::{compute_alignment, AlignmentResult};
    // "the", "and", "to", "for", "in", "of" are all stop words — no real overlap
    let result = compute_alignment(
        "the and to for in of",
        Some("the and to for in of is on it"),
    );
    // All tokens are stop words — should produce Warning or None (no meaningful overlap)
    assert!(
        result == AlignmentResult::None || matches!(result, AlignmentResult::Warning { .. }),
        "stop-word-only tokens should not produce Ok alignment, got: {:?}",
        result
    );
}

#[test]
fn test_format_busy_duration_5m() {
    use squad_station::commands::context::format_busy_duration;
    let ts = (chrono::Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
    let result = format_busy_duration("busy", &ts);
    assert_eq!(result, "5m", "5 minutes ago should return '5m', got: {}", result);
}

#[test]
fn test_format_busy_duration_1h30m() {
    use squad_station::commands::context::format_busy_duration;
    let ts = (chrono::Utc::now() - chrono::Duration::minutes(90)).to_rfc3339();
    let result = format_busy_duration("busy", &ts);
    assert_eq!(result, "1h 30m", "90 minutes ago should return '1h 30m', got: {}", result);
}

#[test]
fn test_format_busy_duration_2d4h() {
    use squad_station::commands::context::format_busy_duration;
    let ts = (chrono::Utc::now() - chrono::Duration::hours(52)).to_rfc3339();
    let result = format_busy_duration("busy", &ts);
    assert_eq!(result, "2d 4h", "52 hours ago should return '2d 4h', got: {}", result);
}

#[test]
fn test_format_busy_duration_idle_status() {
    use squad_station::commands::context::format_busy_duration;
    let ts = chrono::Utc::now().to_rfc3339();
    let result = format_busy_duration("idle", &ts);
    assert_eq!(result, "idle", "non-busy status should return 'idle', got: {}", result);
}

#[test]
fn test_format_busy_duration_less_than_1m() {
    use squad_station::commands::context::format_busy_duration;
    let ts = (chrono::Utc::now() - chrono::Duration::seconds(30)).to_rfc3339();
    let result = format_busy_duration("busy", &ts);
    assert_eq!(result, "<1m", "30 seconds ago should return '<1m', got: {}", result);
}

#[test]
fn test_agent_metrics_struct() {
    use squad_station::commands::context::{AgentMetrics, AlignmentResult};
    let m = AgentMetrics {
        agent_name: "my-agent".to_string(),
        pending_count: 3,
        busy_for: "5m".to_string(),
        alignment: AlignmentResult::Ok,
    };
    assert_eq!(m.agent_name, "my-agent");
    assert_eq!(m.pending_count, 3);
    assert_eq!(m.busy_for, "5m");
    assert_eq!(m.alignment, AlignmentResult::Ok);
}

// ============================================================
// Fleet Status rendering tests — INTEL-01..05
// ============================================================

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_table() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "proj-orchestrator",
        "claude-code",
        "orchestrator",
        None,
        None,
    )
    .await
    .unwrap();
    db::agents::insert_agent(
        &db,
        "proj-frontend",
        "claude-code",
        "worker",
        Some("claude-sonnet"),
        Some("Frontend engineer"),
    )
    .await
    .unwrap();
    db::agents::insert_agent(
        &db,
        "proj-backend",
        "claude-code",
        "worker",
        Some("claude-sonnet"),
        Some("Backend engineer"),
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![
        AgentMetrics {
            agent_name: "proj-frontend".to_string(),
            pending_count: 3,
            busy_for: "5m".to_string(),
            alignment: AlignmentResult::Ok,
        },
        AgentMetrics {
            agent_name: "proj-backend".to_string(),
            pending_count: 0,
            busy_for: "idle".to_string(),
            alignment: AlignmentResult::None,
        },
    ];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    assert!(
        content.contains("## Fleet Status"),
        "Fleet Status header should be present"
    );
    assert!(
        content.contains("| Agent | Pending | Busy For | Alignment |"),
        "Fleet Status table header should be present"
    );
    assert!(
        content.contains("proj-frontend"),
        "proj-frontend worker should appear in Fleet Status"
    );
    assert!(
        content.contains("proj-backend"),
        "proj-backend worker should appear in Fleet Status"
    );
    assert!(content.contains("| 3 |"), "pending_count=3 should appear");
    assert!(content.contains("| 5m |"), "busy_for='5m' should appear");
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_excludes_orchestrator() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "proj-orchestrator",
        "claude-code",
        "orchestrator",
        None,
        None,
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![AgentMetrics {
        agent_name: "proj-orchestrator".to_string(),
        pending_count: 0,
        busy_for: "idle".to_string(),
        alignment: AlignmentResult::None,
    }];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    // Fleet Status section should not appear if only orchestrator metrics provided
    // (orchestrator is excluded from the table)
    let fleet_section_exists = content.contains("## Fleet Status");
    if fleet_section_exists {
        // If Fleet Status section exists, the orchestrator agent should NOT appear in the table
        let fleet_start = content.find("## Fleet Status").unwrap();
        let fleet_end = content[fleet_start..]
            .find("\n## ")
            .map(|i| fleet_start + i)
            .unwrap_or(content.len());
        let fleet_section = &content[fleet_start..fleet_end];
        assert!(
            !fleet_section.contains("proj-orchestrator"),
            "Orchestrator should NOT appear in Fleet Status table rows"
        );
    }
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_excludes_dead() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "proj-dead-worker",
        "claude-code",
        "worker",
        None,
        None,
    )
    .await
    .unwrap();
    // Mark worker as dead
    db::agents::update_agent_status(&db, "proj-dead-worker", "dead")
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![AgentMetrics {
        agent_name: "proj-dead-worker".to_string(),
        pending_count: 0,
        busy_for: "idle".to_string(),
        alignment: AlignmentResult::None,
    }];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    // Dead agent should not appear in Fleet Status table
    let fleet_section_exists = content.contains("## Fleet Status");
    if fleet_section_exists {
        let fleet_start = content.find("## Fleet Status").unwrap();
        let fleet_end = content[fleet_start..]
            .find("\n## ")
            .map(|i| fleet_start + i)
            .unwrap_or(content.len());
        let fleet_section = &content[fleet_start..fleet_end];
        assert!(
            !fleet_section.contains("proj-dead-worker"),
            "Dead agent should NOT appear in Fleet Status table rows"
        );
    }
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_empty_metrics() {
    use squad_station::commands::context::build_orchestrator_md;

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(&db, "proj-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let content = build_orchestrator_md(&agents, "/project/root", &[], &[]);

    assert!(
        !content.contains("## Fleet Status"),
        "Fleet Status section should NOT appear when metrics is empty"
    );
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_alignment_warning() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(
        &db,
        "proj-worker",
        "claude-code",
        "worker",
        None,
        Some("Backend engineer"),
    )
    .await
    .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![AgentMetrics {
        agent_name: "proj-worker".to_string(),
        pending_count: 1,
        busy_for: "2m".to_string(),
        alignment: AlignmentResult::Warning {
            task_preview: "fix CSS...".to_string(),
            role: "Backend engineer".to_string(),
        },
    }];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    assert!(
        content.contains("## Fleet Status"),
        "Fleet Status should appear with warning metric"
    );
    // Warning emoji \u{26a0}\u{fe0f} = ⚠️
    assert!(
        content.contains('\u{26a0}'),
        "Warning emoji should appear for Warning alignment"
    );
    assert!(
        content.contains("fix CSS"),
        "task_preview text should appear in Fleet Status"
    );
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_requery_commands() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(&db, "proj-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![AgentMetrics {
        agent_name: "proj-worker".to_string(),
        pending_count: 0,
        busy_for: "idle".to_string(),
        alignment: AlignmentResult::None,
    }];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    assert!(
        content.contains("squad-station agents"),
        "Re-query commands should include 'squad-station agents'"
    );
    assert!(
        content.contains("squad-station list --status processing"),
        "Re-query commands should include 'squad-station list --status processing'"
    );
    assert!(
        content.contains("squad-station status"),
        "Re-query commands should include 'squad-station status'"
    );
    assert!(
        content.contains("squad-station context"),
        "Re-query commands should include 'squad-station context'"
    );
}

#[tokio::test]
async fn test_build_orchestrator_md_fleet_status_section_order() {
    use squad_station::commands::context::{build_orchestrator_md, AgentMetrics, AlignmentResult};

    let db = helpers::setup_test_db().await;
    db::agents::insert_agent(&db, "proj-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();

    let agents = db::agents::list_agents(&db).await.unwrap();
    let metrics = vec![AgentMetrics {
        agent_name: "proj-worker".to_string(),
        pending_count: 0,
        busy_for: "idle".to_string(),
        alignment: AlignmentResult::None,
    }];

    let content = build_orchestrator_md(&agents, "/project/root", &[], &metrics);

    let preflight_pos = content.find("## PRE-FLIGHT").expect("PRE-FLIGHT section must exist");
    let fleet_pos = content.find("## Fleet Status").expect("Fleet Status section must exist");
    let routing_pos = content
        .find("## Session Routing")
        .expect("Session Routing section must exist");

    assert!(
        preflight_pos < fleet_pos,
        "Fleet Status must appear after PRE-FLIGHT"
    );
    assert!(
        fleet_pos < routing_pos,
        "Fleet Status must appear before Session Routing"
    );
}

// ============================================================
// End-to-end metrics pipeline integration test — INTEL-01..05
// ============================================================

#[tokio::test]
async fn test_context_metrics_pipeline_end_to_end() {
    use squad_station::commands::context::{
        build_orchestrator_md, compute_alignment, format_busy_duration,
        AgentMetrics, AlignmentResult,
    };

    let db = helpers::setup_test_db().await;

    // Set up agents: 1 orchestrator + 2 workers (1 busy, 1 idle)
    db::agents::insert_agent(
        &db, "proj-orchestrator", "claude-code", "orchestrator", None, None,
    ).await.unwrap();
    db::agents::insert_agent(
        &db, "proj-worker-a", "claude-code", "worker",
        Some("sonnet"), Some("Frontend engineer for React and CSS"),
    ).await.unwrap();
    db::agents::insert_agent(
        &db, "proj-worker-b", "claude-code", "worker",
        Some("opus"), Some("Backend API developer"),
    ).await.unwrap();

    // Make worker-a busy
    db::agents::update_agent_status(&db, "proj-worker-a", "busy").await.unwrap();

    // Send 2 processing messages to worker-a, 1 to worker-b
    db::messages::insert_message(
        &db, "proj-orchestrator", "proj-worker-a", "task_request",
        "Fix the CSS grid layout for the dashboard", "normal", None,
    ).await.unwrap();
    db::messages::insert_message(
        &db, "proj-orchestrator", "proj-worker-a", "task_request",
        "Add responsive breakpoints", "normal", None,
    ).await.unwrap();
    db::messages::insert_message(
        &db, "proj-orchestrator", "proj-worker-b", "task_request",
        "Optimize database query performance", "normal", None,
    ).await.unwrap();

    // Simulate the metrics assembly that run() does
    let agents = db::agents::list_agents(&db).await.unwrap();
    let mut metrics = Vec::new();
    for agent in &agents {
        if agent.role == "orchestrator" || agent.status == "dead" {
            continue;
        }
        let pending = db::messages::count_processing(&db, &agent.name).await.unwrap();
        let busy_for = format_busy_duration(&agent.status, &agent.status_updated_at);
        let alignment = match db::messages::peek_message(&db, &agent.name).await.unwrap() {
            Some(msg) => compute_alignment(&msg.task, agent.description.as_deref()),
            None => AlignmentResult::None,
        };
        metrics.push(AgentMetrics {
            agent_name: agent.name.clone(),
            pending_count: pending,
            busy_for,
            alignment,
        });
    }

    let content = build_orchestrator_md(&agents, "/test/project", &[], &metrics);

    // INTEL-01: Pending counts appear
    assert!(content.contains("| proj-worker-a | 2 |"), "Worker A should show 2 pending, got:\n{}", content);
    assert!(content.contains("| proj-worker-b | 1 |"), "Worker B should show 1 pending, got:\n{}", content);

    // INTEL-02: Busy For column present
    // worker-b is idle (should show "idle")
    assert!(content.contains("idle"), "Worker B should show idle");

    // INTEL-03: Alignment check
    // worker-a: "Fix CSS grid layout..." vs "Frontend engineer for React and CSS" — overlap on "css" → checkmark
    assert!(content.contains('\u{2705}'), "Worker A should have checkmark alignment (CSS overlap)");

    // INTEL-04: Re-query commands
    assert!(content.contains("squad-station agents"), "Missing re-query: agents");
    assert!(content.contains("squad-station list --status processing"), "Missing re-query: list");
    assert!(content.contains("squad-station status"), "Missing re-query: status");
    assert!(content.contains("squad-station context"), "Missing re-query: context");

    // Orchestrator excluded from Fleet Status
    let fleet_section = content.split("## Fleet Status").nth(1).unwrap_or("");
    let session_section_start = fleet_section.find("## Session Routing").unwrap_or(fleet_section.len());
    let fleet_only = &fleet_section[..session_section_start];
    assert!(!fleet_only.contains("proj-orchestrator"), "Orchestrator should not be in Fleet Status");

    // Section ordering: Fleet Status between PRE-FLIGHT and Session Routing
    let fleet_pos = content.find("## Fleet Status").expect("Fleet Status section missing");
    let preflight_pos = content.find("PRE-FLIGHT").expect("PRE-FLIGHT missing");
    let routing_pos = content.find("## Session Routing").expect("Session Routing missing");
    assert!(fleet_pos > preflight_pos, "Fleet Status should come after PRE-FLIGHT");
    assert!(fleet_pos < routing_pos, "Fleet Status should come before Session Routing");
}
