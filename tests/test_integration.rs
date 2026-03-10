mod helpers;

use squad_station::commands;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use squad_station::db;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a minimal squad.yml into `dir` using the new format (project as plain string).
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

/// Create a Command for the binary with SQUAD_STATION_DB set to point at the test DB.
fn cmd_with_db(db_path: &std::path::Path) -> std::process::Command {
    let mut c = std::process::Command::new(bin());
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

fn bin() -> String {
    env!("CARGO_BIN_EXE_squad-station").to_string()
}

// ============================================================
// Register command integration tests
// ============================================================

#[test]
fn test_register_text_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    let output = cmd_with_db(&db_file)
        .args([
            "register",
            "my-worker",
            "--role",
            "worker",
            "--tool",
            "claude-code",
        ])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Registered agent 'my-worker'"),
        "got: {}",
        stdout
    );
    assert!(stdout.contains("role=worker"), "got: {}", stdout);
    assert!(stdout.contains("tool=claude-code"), "got: {}", stdout);
}

#[test]
fn test_register_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    let output = cmd_with_db(&db_file)
        .args(["register", "my-worker", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");
    assert_eq!(parsed["registered"], true);
    assert_eq!(parsed["name"], "my-worker");
    assert_eq!(parsed["role"], "worker"); // default role
}

#[test]
fn test_register_idempotent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    let args = [
        "register",
        "dup-agent",
        "--role",
        "worker",
        "--tool",
        "test",
    ];

    // First registration
    let out1 = cmd_with_db(&db_file)
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out1.status.success());

    // Second registration — same name, should succeed (INSERT OR IGNORE)
    let out2 = cmd_with_db(&db_file)
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out2.status.success(), "duplicate register must not fail");
}

#[test]
fn test_register_no_squad_yml_fails() {
    let tmp = tempfile::TempDir::new().unwrap();
    // No squad.yml, no SQUAD_STATION_DB env var

    let output = std::process::Command::new(bin())
        .args(["register", "agent"])
        .current_dir(tmp.path())
        .env_remove("SQUAD_STATION_DB")
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "register without squad.yml or env var must fail"
    );
}

#[test]
fn test_register_via_env_var() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    // No squad.yml — but set SQUAD_STATION_DB env var

    // First create the DB via a squad.yml setup, then use env var
    write_squad_yml(tmp.path(), &db_file);

    // Register first agent normally to initialize DB via SQUAD_STATION_DB
    let init = cmd_with_db(&db_file)
        .args(["register", "init-agent"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(init.status.success());

    // Now remove squad.yml and use env var alone
    std::fs::remove_file(tmp.path().join("squad.yml")).unwrap();

    let output = std::process::Command::new(bin())
        .args(["register", "env-agent"])
        .current_dir(tmp.path())
        .env("SQUAD_STATION_DB", db_file.to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "register via SQUAD_STATION_DB should work, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("env-agent"), "got: {}", stdout);
}

// ============================================================
// List command integration tests
// ============================================================

#[tokio::test]
async fn test_list_text_output_with_messages() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "implement feature X",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "fix bug Y",
        "high",
    )
    .await
    .unwrap();

    // Close the pool before running the binary (single-writer)
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Table header
    assert!(
        stdout.contains("ID"),
        "table must have ID column, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("TO"),
        "table must have TO column, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("STATUS"),
        "table must have STATUS column, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("PRIORITY"),
        "table must have PRIORITY column, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("TASK"),
        "table must have TASK column, got:\n{}",
        stdout
    );
    // Data
    assert!(
        stdout.contains("worker-1"),
        "must contain agent name, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("implement feature X"),
        "must contain task text, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("fix bug Y"),
        "must contain task text, got:\n{}",
        stdout
    );
}

#[tokio::test]
async fn test_list_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "test task",
        "urgent",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["list", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");
    assert!(parsed.is_array(), "JSON output must be an array");
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["agent_name"], "worker-1");
    assert_eq!(arr[0]["task"], "test task");
    assert_eq!(arr[0]["priority"], "urgent");
    assert_eq!(arr[0]["status"], "processing");
}

#[tokio::test]
async fn test_list_empty_shows_no_messages() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("list")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("No messages found"),
        "empty list must say 'No messages found', got:\n{}",
        stdout
    );
}

#[tokio::test]
async fn test_list_filter_by_agent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "alpha", "claude", "worker", None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "beta", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "alpha",
        "task_request",
        "alpha task",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "beta",
        "task_request",
        "beta task",
        "normal",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["list", "--agent", "alpha", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1, "filter should return only alpha's messages");
    assert_eq!(arr[0]["agent_name"], "alpha");
}

#[tokio::test]
async fn test_list_filter_by_status() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker",
        "task_request",
        "task 1",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker",
        "task_request",
        "task 2",
        "normal",
    )
    .await
    .unwrap();
    // Complete the most recent one
    db::messages::update_status(&pool, "worker").await.unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["list", "--status", "completed", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "completed");
}

#[tokio::test]
async fn test_list_with_limit() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker", "claude", "worker", None, None)
        .await
        .unwrap();
    for i in 0..10u32 {
        db::messages::insert_message(
            &pool,
            "orchestrator",
            "worker",
            "task_request",
            &format!("task {}", i),
            "normal",
        )
        .await
        .unwrap();
    }
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["list", "--limit", "3", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 3, "limit=3 must return exactly 3 messages");
}

// ============================================================
// Peek command integration tests
// ============================================================

#[tokio::test]
async fn test_peek_text_with_pending_task() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "do something important",
        "high",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["peek", "worker-1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("do something important"), "got: {}", stdout);
    assert!(
        stdout.contains("high"),
        "must show priority, got: {}",
        stdout
    );
}

#[tokio::test]
async fn test_peek_json_with_pending_task() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "json task",
        "urgent",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["peek", "worker-1", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");
    assert_eq!(parsed["task"], "json task");
    assert_eq!(parsed["priority"], "urgent");
    assert_eq!(parsed["status"], "processing");
    assert!(parsed["id"].is_string(), "must have id field");
}

#[tokio::test]
async fn test_peek_no_pending_text() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["peek", "worker-1"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("No pending tasks"), "got: {}", stdout);
}

#[tokio::test]
async fn test_peek_no_pending_json() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["peek", "worker-1", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["pending"], false);
    assert_eq!(parsed["agent"], "worker-1");
}

#[tokio::test]
async fn test_peek_priority_ordering() {
    // Peek must return the highest-priority pending message
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "normal task",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "high task",
        "high",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "worker-1",
        "task_request",
        "urgent task",
        "urgent",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["peek", "worker-1", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        parsed["priority"], "urgent",
        "peek must return urgent task first"
    );
    assert_eq!(parsed["task"], "urgent task");
}

// ============================================================
// Send command error path tests
// ============================================================

#[tokio::test]
async fn test_send_agent_not_found() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["send", "nonexistent-agent", "--body", "do something"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "send to nonexistent agent must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Agent not found"),
        "error must say 'Agent not found', got: {}",
        stderr
    );
}

#[tokio::test]
async fn test_send_no_tmux_session() {
    // Agent exists in DB but has no tmux session
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "offline-agent", "claude", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["send", "offline-agent", "--body", "do something"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "send to agent without tmux session must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("tmux session not running"),
        "error must mention tmux session, got: {}",
        stderr
    );
}

// ============================================================
// Signal command integration tests (DB-level flow)
// ============================================================

#[tokio::test]
async fn test_signal_completes_message_and_resets_status() {
    // Full signal flow: insert agent + message, signal, verify message completed + agent idle
    let pool = helpers::setup_test_db().await;

    db::agents::insert_agent(&pool, "sig-agent", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "sig-agent",
        "task_request",
        "a task",
        "normal",
    )
    .await
    .unwrap();
    db::agents::update_agent_status(&pool, "sig-agent", "busy")
        .await
        .unwrap();

    // Signal: complete the message
    let rows = db::messages::update_status(&pool, "sig-agent")
        .await
        .unwrap();
    assert_eq!(rows, 1, "one message should be completed");

    // After signal, agent should be reset to idle
    db::agents::update_agent_status(&pool, "sig-agent", "idle")
        .await
        .unwrap();
    let agent = db::agents::get_agent(&pool, "sig-agent")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, "idle");

    // Message should be completed
    let msgs = db::messages::list_messages(&pool, Some("sig-agent"), Some("completed"), 10)
        .await
        .unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].status, "completed");
}

#[tokio::test]
async fn test_signal_multiple_messages_completes_most_recent() {
    // Signal should complete only the most recent pending message
    let pool = helpers::setup_test_db().await;

    db::agents::insert_agent(&pool, "multi-agent", "claude", "worker", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "multi-agent",
        "task_request",
        "task 1",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "multi-agent",
        "task_request",
        "task 2",
        "normal",
    )
    .await
    .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "multi-agent",
        "task_request",
        "task 3",
        "normal",
    )
    .await
    .unwrap();

    // Signal once — should complete only 1 message
    let rows = db::messages::update_status(&pool, "multi-agent")
        .await
        .unwrap();
    assert_eq!(rows, 1);

    let pending = db::messages::list_messages(&pool, Some("multi-agent"), Some("processing"), 100)
        .await
        .unwrap();
    assert_eq!(pending.len(), 2, "2 messages should still be processing");

    let completed = db::messages::list_messages(&pool, Some("multi-agent"), Some("completed"), 100)
        .await
        .unwrap();
    assert_eq!(completed.len(), 1, "1 message should be completed");
}

#[tokio::test]
async fn test_signal_orchestrator_self_signal_guard() {
    // Orchestrator signaling itself should be a no-op (HOOK-01 guard)
    // Tested via binary: signal with TMUX_PANE set but agent is orchestrator
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    // Register as orchestrator
    db::agents::insert_agent(&pool, "orch-test", "claude", "orchestrator", None, None)
        .await
        .unwrap();
    db::messages::insert_message(
        &pool,
        "orchestrator",
        "orch-test",
        "task_request",
        "orch task",
        "normal",
    )
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["signal", "orch-test"])
        .env("TMUX_PANE", "%0")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "orchestrator self-signal must exit 0"
    );
    // The message should NOT have been completed (guard blocks signal)
    let pool2 = setup_file_db(&db_path).await;
    let pending = db::messages::list_messages(&pool2, Some("orch-test"), Some("processing"), 10)
        .await
        .unwrap();
    assert_eq!(
        pending.len(),
        1,
        "orchestrator self-signal must not complete the message"
    );
}

#[tokio::test]
async fn test_signal_unregistered_agent_guard() {
    // Signal for an unregistered agent should silently exit 0 (HOOK-03 guard)
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["signal", "ghost-agent"])
        .env("TMUX_PANE", "%0")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "signal for unregistered agent must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "signal for unregistered agent must produce no stdout"
    );
}

// ============================================================
// Agents command integration tests
// ============================================================

#[tokio::test]
async fn test_agents_json_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "agent-1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "agent-2", "gemini", "orchestrator", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .args(["agents", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("must be valid JSON");
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Should include name, role, tool, status fields
    for agent in arr {
        assert!(agent["name"].is_string());
        assert!(agent["role"].is_string());
        assert!(agent["tool"].is_string());
        assert!(agent["status"].is_string());
    }
}

#[tokio::test]
async fn test_agents_empty_squad() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("agents")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("No agents registered"), "got: {}", stdout);
}

// ============================================================
// Context command with agents
// ============================================================

#[tokio::test]
async fn test_context_lists_registered_agents() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "context must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Updated: check file contents instead of stdout
    let roster_path = tmp.path().join(".agent/workflows/squad-roster.md");
    assert!(roster_path.exists(), ".agent/workflows/squad-roster.md must exist");
    let roster = std::fs::read_to_string(&roster_path).unwrap();
    assert!(
        roster.contains("ctx-worker"),
        "roster must contain agent name, got:\n{}",
        roster
    );
}

#[tokio::test]
async fn test_context_generates_delegate_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "context must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let delegate_path = tmp.path().join(".agent/workflows/squad-delegate.md");
    assert!(delegate_path.exists(), ".agent/workflows/squad-delegate.md must exist");

    let content = std::fs::read_to_string(&delegate_path).unwrap();
    assert!(
        content.contains("ctx-worker"),
        "delegate.md must contain agent name, got:\n{}",
        content
    );
    assert!(
        content.contains("squad-station send"),
        "delegate.md must contain squad-station send command, got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_delegate_content() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let delegate_path = tmp.path().join(".agent/workflows/squad-delegate.md");
    let content = std::fs::read_to_string(&delegate_path).unwrap();
    assert!(
        content.contains("BEHAVIORAL RULE"),
        "delegate.md must contain BEHAVIORAL RULE header, got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_generates_monitor_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "context must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let monitor_path = tmp.path().join(".agent/workflows/squad-monitor.md");
    assert!(monitor_path.exists(), ".agent/workflows/squad-monitor.md must exist");

    let content = std::fs::read_to_string(&monitor_path).unwrap();
    assert!(
        content.contains("squad-station agents"),
        "monitor.md must contain squad-station agents command, got:\n{}",
        content
    );
    assert!(
        content.contains("squad-station list"),
        "monitor.md must contain squad-station list command, got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_monitor_content() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let monitor_path = tmp.path().join(".agent/workflows/squad-monitor.md");
    let content = std::fs::read_to_string(&monitor_path).unwrap();
    assert!(
        content.contains("Anti-Context-Decay"),
        "monitor.md must contain Anti-Context-Decay section, got:\n{}",
        content
    );
    assert!(
        content.contains("re-read") && content.contains(".agent/workflows"),
        "monitor.md must contain re-read .agent/workflows instruction, got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_generates_roster_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(
        &pool,
        "ctx-worker",
        "claude-code",
        "worker",
        None,
        None,
    )
    .await
    .unwrap();
    // Update model and description via SQL since insert_agent may not take those
    sqlx::query(
        "UPDATE agents SET model = 'claude-sonnet', description = 'Test agent' WHERE name = 'ctx-worker'"
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    let output = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "context must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let roster_path = tmp.path().join(".agent/workflows/squad-roster.md");
    assert!(roster_path.exists(), ".agent/workflows/squad-roster.md must exist");

    let content = std::fs::read_to_string(&roster_path).unwrap();
    assert!(
        content.contains("ctx-worker"),
        "roster.md must contain agent name, got:\n{}",
        content
    );
    assert!(
        content.contains("claude-sonnet"),
        "roster.md must contain model, got:\n{}",
        content
    );
    assert!(
        content.contains("Test agent"),
        "roster.md must contain description, got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_roster_content() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let roster_path = tmp.path().join(".agent/workflows/squad-roster.md");
    let content = std::fs::read_to_string(&roster_path).unwrap();
    assert!(
        content.contains("| Agent |"),
        "roster.md must contain Markdown table header '| Agent |', got:\n{}",
        content
    );
}

#[tokio::test]
async fn test_context_idempotent() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    db::agents::insert_agent(&pool, "ctx-worker", "claude-code", "worker", None, None)
        .await
        .unwrap();
    pool.close().await;

    write_squad_yml(tmp.path(), &db_path);

    // First run
    let out1 = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "first context run must exit 0, stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    // Second run — overwrite must be safe
    let out2 = cmd_with_db(&db_path)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "second context run must exit 0, stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    // Files must still exist
    assert!(tmp.path().join(".agent/workflows/squad-delegate.md").exists());
    assert!(tmp.path().join(".agent/workflows/squad-monitor.md").exists());
    assert!(tmp.path().join(".agent/workflows/squad-roster.md").exists());
}

// ============================================================
// Full workflow integration test
// ============================================================

#[tokio::test]
async fn test_full_workflow_register_send_peek_signal() {
    // End-to-end DB-level workflow: register → send → peek → signal → verify
    let pool = helpers::setup_test_db().await;

    // 1. Register agent
    db::agents::insert_agent(&pool, "e2e-agent", "claude", "worker", None, None)
        .await
        .unwrap();
    let agent = db::agents::get_agent(&pool, "e2e-agent").await.unwrap();
    assert!(agent.is_some());
    assert_eq!(agent.unwrap().status, "idle");

    // 2. Send task (DB operations only, skip tmux)
    let msg_id = db::messages::insert_message(
        &pool,
        "orchestrator",
        "e2e-agent",
        "task_request",
        "build the feature",
        "high",
    )
    .await
    .unwrap();
    assert_eq!(msg_id.len(), 36); // UUID
    db::agents::update_agent_status(&pool, "e2e-agent", "busy")
        .await
        .unwrap();

    // 3. Peek — should return the task
    let peeked = db::messages::peek_message(&pool, "e2e-agent")
        .await
        .unwrap();
    assert!(peeked.is_some());
    assert_eq!(peeked.unwrap().task, "build the feature");

    // 4. Agent is busy
    let agent = db::agents::get_agent(&pool, "e2e-agent")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, "busy");

    // 5. Signal completion
    let rows = db::messages::update_status(&pool, "e2e-agent")
        .await
        .unwrap();
    assert_eq!(rows, 1);
    db::agents::update_agent_status(&pool, "e2e-agent", "idle")
        .await
        .unwrap();

    // 6. Verify final state
    let agent = db::agents::get_agent(&pool, "e2e-agent")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, "idle");

    let peeked = db::messages::peek_message(&pool, "e2e-agent")
        .await
        .unwrap();
    assert!(peeked.is_none(), "no pending tasks after signal");

    let completed = db::messages::list_messages(&pool, Some("e2e-agent"), Some("completed"), 10)
        .await
        .unwrap();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].task, "build the feature");
}

// ============================================================
// Antigravity provider integration tests (AGNT-02 / AGNT-03)
// ============================================================

fn write_antigravity_squad_yml(dir: &std::path::Path, _db_file: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  provider: antigravity
  role: orchestrator
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
}

#[tokio::test]
async fn test_signal_antigravity_orchestrator_db_only() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml(tmp.path(), &db_file);
    // Register orchestrator and a worker agent in DB directly
    let pool = setup_file_db(&db_file).await;
    db::agents::insert_agent(&pool, "test-squad-antigravity-test-orch", "antigravity", "orchestrator", None, None).await.unwrap();
    db::agents::insert_agent(&pool, "test-squad-claude-code-worker", "claude-code", "worker", None, None).await.unwrap();
    // Send a task to the worker
    db::messages::insert_message(&pool, "orchestrator", "test-squad-claude-code-worker", "task_request", "test task", "normal").await.unwrap();
    pool.close().await;
    // Signal the worker
    let output = cmd_with_db(&db_file)
        .args(["signal", "test-squad-claude-code-worker", "--json"])
        .env("TMUX_PANE", "%0")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "signal must exit 0: {:?}", output);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["signaled"], true);
    assert_eq!(json["orchestrator_notified"], false, "antigravity orch must NOT be notified via tmux");
}

#[tokio::test]
async fn test_signal_antigravity_message_completed() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml(tmp.path(), &db_file);
    let pool = setup_file_db(&db_file).await;
    db::agents::insert_agent(&pool, "test-squad-antigravity-test-orch", "antigravity", "orchestrator", None, None).await.unwrap();
    db::agents::insert_agent(&pool, "test-squad-claude-code-worker", "claude-code", "worker", None, None).await.unwrap();
    db::messages::insert_message(&pool, "orchestrator", "test-squad-claude-code-worker", "task_request", "do work", "normal").await.unwrap();
    pool.close().await;
    let output = cmd_with_db(&db_file)
        .args(["signal", "test-squad-claude-code-worker"])
        .env("TMUX_PANE", "%0")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    // Verify DB state: message completed, agent idle
    let pool2 = setup_file_db(&db_file).await;
    let msg: (String,) = sqlx::query_as("SELECT status FROM messages WHERE agent_name = ? ORDER BY created_at DESC LIMIT 1")
        .bind("test-squad-claude-code-worker")
        .fetch_one(&pool2).await.unwrap();
    assert_eq!(msg.0, "completed");
    let agent: (String,) = sqlx::query_as("SELECT status FROM agents WHERE name = ?")
        .bind("test-squad-claude-code-worker")
        .fetch_one(&pool2).await.unwrap();
    assert_eq!(agent.0, "idle");
}

// ============================================================
// AGNT-03: init skips tmux launch for antigravity orchestrator
// ============================================================

#[tokio::test]
async fn test_init_antigravity_orchestrator_skips_tmux() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml(tmp.path(), &db_file);
    let output = cmd_with_db(&db_file)
        .args(["init", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "init must exit 0: {:?}\nstderr: {}", output, String::from_utf8_lossy(&output.stderr));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    // Orchestrator is db-only: launched count for total init must be 0 (no tmux sessions)
    assert_eq!(json["launched"], 0, "no tmux sessions launched for antigravity-only squad");
    assert_eq!(json["failed"], serde_json::json!([]), "no failures");
}

#[tokio::test]
async fn test_init_antigravity_registers_in_db() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml(tmp.path(), &db_file);
    let output = cmd_with_db(&db_file)
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "init must exit 0: {:?}\nstderr: {}", output, String::from_utf8_lossy(&output.stderr));
    // Orchestrator must be in DB even though no tmux session created
    let pool = setup_file_db(&db_file).await;
    let rec: Option<(String, String)> = sqlx::query_as(
        "SELECT tool, role FROM agents WHERE name = 'test-squad-antigravity-test-orch'"
    )
    .fetch_optional(&pool).await.unwrap();
    assert!(rec.is_some(), "orchestrator must be registered in DB");
    let (tool, role) = rec.unwrap();
    assert_eq!(tool, "antigravity");
    assert_eq!(role, "orchestrator");
}

#[tokio::test]
async fn test_init_antigravity_log_message() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml(tmp.path(), &db_file);
    let output = cmd_with_db(&db_file)
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "init must exit 0: {:?}\nstderr: {}", output, String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("db-only"),
        "stdout must contain 'db-only' to explain DB-only registration. Got: {stdout}"
    );
    assert!(
        !stdout.contains("already running"),
        "db-only message must be distinct from already-running message. Got: {stdout}"
    );
}

// ============================================================
// HOOK-01: signal auto-detection tests
// ============================================================

#[tokio::test]
async fn test_signal_no_args_no_tmux() {
    // HOOK-01: signal with no agent name provided must exit 0 silently
    let result = commands::signal::run(None, false).await;
    assert!(result.is_ok(), "signal with no args must exit 0");
}

// ============================================================
// HOOK-03 / HOOK-04: init settings.json hook merge tests
// ============================================================

/// Write a squad.yml with antigravity orchestrator so no tmux sessions are spawned.
fn write_antigravity_squad_yml_for_hook(dir: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  provider: antigravity
  role: orchestrator
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
}

#[test]
fn test_init_hook_prints_instructions() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_file = tmp.path().join("station.db");
    write_antigravity_squad_yml_for_hook(tmp.path());

    let output = cmd_with_db(&db_file)
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init must exit 0\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("squad-station signal $(tmux display-message -p '#S')"),
        "stdout must contain hook command instructions. Got: {}",
        stdout
    );
    assert!(
        stdout.contains("Stop"),
        "stdout must mention the Stop event name. Got: {}",
        stdout
    );
}

#[tokio::test]
async fn test_signal_via_tmux_pane() {
    // HOOK-01: signal now requires an explicit session name (passed via hook as
    // $(tmux display-message -p '#S')); TMUX_PANE resolution is no longer used.
    // Full E2E is covered by e2e_cli.sh.
    eprintln!("test_signal_via_tmux_pane: E2E covered by e2e_cli.sh");
}

#[tokio::test]
async fn test_signal_pane_id_as_arg() {
    // Pane ID resolution via TMUX_PANE is no longer supported. The hook now passes
    // the session name directly via $(tmux display-message -p '#S').
    // E2E behavior is covered by e2e_cli.sh.
    eprintln!("test_signal_pane_id_as_arg: E2E covered by e2e_cli.sh");
}
