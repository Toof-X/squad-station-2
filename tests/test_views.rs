mod helpers;

use crossterm::event::KeyCode;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use squad_station::commands::ui::{App, FocusPanel};
use squad_station::db;

// ---------------------------------------------------------------------------
// Helper: mock Agent
// ---------------------------------------------------------------------------

fn mock_agent(name: &str, status: &str) -> squad_station::db::agents::Agent {
    squad_station::db::agents::Agent {
        id: "test-id".into(),
        name: name.into(),
        tool: "test".into(),
        role: "worker".into(),
        command: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        status: status.into(),
        status_updated_at: "2026-01-01T00:00:00Z".into(),
        model: None,
        description: None,
        current_task: None,
    }
}

// ---------------------------------------------------------------------------
// TUI app state unit tests
// ---------------------------------------------------------------------------

#[test]
fn test_ui_app_new() {
    let app = App::new();
    assert!(app.agents.is_empty(), "agents should be empty");
    assert!(!app.quit, "quit should be false");
    assert_eq!(app.agent_list_state.selected(), None, "no selection");
    assert_eq!(
        app.focus,
        FocusPanel::AgentPanel,
        "focus should be AgentPanel"
    );
}

#[test]
fn test_ui_navigation_next() {
    let mut app = App::new();
    app.agents = vec![
        mock_agent("a", "idle"),
        mock_agent("b", "busy"),
        mock_agent("c", "dead"),
    ];
    // First call from None -> 0
    app.select_next();
    assert_eq!(app.agent_list_state.selected(), Some(0));
    app.select_next();
    assert_eq!(app.agent_list_state.selected(), Some(1));
    app.select_next();
    assert_eq!(app.agent_list_state.selected(), Some(2));
    // Wrap around
    app.select_next();
    assert_eq!(app.agent_list_state.selected(), Some(0));
}

#[test]
fn test_ui_navigation_prev() {
    let mut app = App::new();
    app.agents = vec![
        mock_agent("a", "idle"),
        mock_agent("b", "busy"),
        mock_agent("c", "dead"),
    ];
    app.agent_list_state.select(Some(0));
    // From 0 -> wraps to 2
    app.select_previous();
    assert_eq!(app.agent_list_state.selected(), Some(2));
    app.select_previous();
    assert_eq!(app.agent_list_state.selected(), Some(1));
    app.select_previous();
    assert_eq!(app.agent_list_state.selected(), Some(0));
}

#[test]
fn test_ui_quit_key_q() {
    let mut app = App::new();
    app.handle_key(KeyCode::Char('q'));
    assert!(app.quit, "quit should be true after 'q'");
}

#[test]
fn test_ui_quit_key_esc() {
    let mut app = App::new();
    app.handle_key(KeyCode::Esc);
    assert!(app.quit, "quit should be true after Esc");
}

#[test]
fn test_ui_toggle_focus() {
    let mut app = App::new();
    assert_eq!(app.focus, FocusPanel::AgentPanel);
    app.toggle_focus();
    assert_eq!(app.focus, FocusPanel::MessagePanel);
    app.toggle_focus();
    assert_eq!(app.focus, FocusPanel::AgentPanel);
}

#[test]
fn test_ui_navigation_empty() {
    let mut app = App::new();
    // Should not panic with empty agents
    app.select_next();
    assert_eq!(app.agent_list_state.selected(), None);
    app.select_previous();
    assert_eq!(app.agent_list_state.selected(), None);
}

#[test]
fn test_views_module_compiles() {
    // Smoke test: this file compiles and test infra works
    assert!(true);
}

/// Helper: write a minimal squad.yml into `dir` using the new format (project as plain string).
/// Use SQUAD_STATION_DB env var to point commands at the test DB file.
fn write_squad_yml(dir: &std::path::Path, _db_file: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  provider: claude-code
  role: orchestrator
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
}

/// Create a Command for the binary with SQUAD_STATION_DB set to the test DB.
fn cmd_with_db(db_path: &std::path::Path) -> std::process::Command {
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let mut c = std::process::Command::new(bin);
    c.env(
        "SQUAD_STATION_DB",
        db_path.to_str().expect("db path must be valid UTF-8"),
    );
    c
}

/// Create a real SQLite file pool with migrations applied.
async fn setup_file_db(path: &std::path::Path) -> sqlx::SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("failed to create pool");
    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await
        .expect("migrations failed");
    pool
}

#[tokio::test]
async fn test_status_text_output() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    // Insert agents
    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent-alpha");

    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-2").bind("agent-beta").bind("gemini-cli").bind("worker")
    .bind("echo beta").bind("dead").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent-beta");

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("status")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "status command should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("test-squad"),
        "must contain project name, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("idle"),
        "must contain 'idle', got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("dead"),
        "must contain 'dead', got:\n{}",
        stdout
    );
}

#[tokio::test]
async fn test_status_json_output() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent");

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["status", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "status --json should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert!(parsed.get("project").is_some(), "JSON must have 'project'");
    assert!(parsed.get("db_path").is_some(), "JSON must have 'db_path'");
    let agents = parsed.get("agents").expect("JSON must have 'agents'");
    assert!(agents.is_array(), "agents must be array");
    let first = &agents[0];
    assert!(first.get("name").is_some(), "agent must have 'name'");
    assert!(first.get("status").is_some(), "agent must have 'status'");
    assert!(
        first.get("pending_messages").is_some(),
        "agent must have 'pending_messages'"
    );
}

#[tokio::test]
async fn test_status_pending_count() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent");

    // Insert 3 pending messages
    for i in 0..3u32 {
        db::messages::insert_message(
            &pool,
            "orchestrator",
            "agent-alpha",
            "task_request",
            &format!("task {}", i),
            "normal",
            None,
        )
        .await
        .expect("insert message");
    }

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["status", "--json"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "status --json should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let agents = parsed["agents"].as_array().expect("agents array");
    assert_eq!(agents.len(), 1);
    let pending = agents[0]["pending_messages"]
        .as_u64()
        .expect("pending_messages u64");
    assert_eq!(pending, 3, "expected 3 pending messages, got {}", pending);
}

#[tokio::test]
async fn test_status_empty_squad() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("status")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "status should exit 0 with no agents, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("No agents registered."),
        "must say 'No agents registered.', got:\n{}",
        stdout
    );
}

#[tokio::test]
async fn test_view_no_live_sessions() {
    // Agents in DB but no tmux sessions running in test environment
    // view command should print "No live agent sessions to display."
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent");

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("view")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "view command should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("No live"),
        "output must contain 'No live', got:\n{}",
        stdout
    );
}
