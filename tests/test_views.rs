mod helpers;

use squad_station::db;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

#[test]
fn test_views_module_compiles() {
    // Smoke test: this file compiles and test infra works
    assert!(true);
}

/// Helper: write a minimal squad.yml into `dir` with db_path pointing to `db_file`.
fn write_squad_yml(dir: &std::path::Path, db_file: &std::path::Path) {
    let db_path_str = db_file.to_str().expect("db path must be valid UTF-8");
    let yaml = format!(
        r#"project:
  name: test-squad
  db_path: "{db_path_str}"
orchestrator:
  name: test-orch
  provider: claude-code
  role: orchestrator
  command: "echo orch"
agents: []
"#
    );
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
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
        "INSERT INTO agents (id, name, provider, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent-alpha");

    sqlx::query(
        "INSERT INTO agents (id, name, provider, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-2").bind("agent-beta").bind("gemini-cli").bind("worker")
    .bind("echo beta").bind("dead").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent-beta");

    write_squad_yml(tmp.path(), &db_path);

    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
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
    assert!(stdout.contains("test-squad"), "must contain project name, got:\n{}", stdout);
    assert!(stdout.contains("idle"), "must contain 'idle', got:\n{}", stdout);
    assert!(stdout.contains("dead"), "must contain 'dead', got:\n{}", stdout);
}

#[tokio::test]
async fn test_status_json_output() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    sqlx::query(
        "INSERT INTO agents (id, name, provider, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent");

    write_squad_yml(tmp.path(), &db_path);

    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
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

    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert!(parsed.get("project").is_some(), "JSON must have 'project'");
    assert!(parsed.get("db_path").is_some(), "JSON must have 'db_path'");
    let agents = parsed.get("agents").expect("JSON must have 'agents'");
    assert!(agents.is_array(), "agents must be array");
    let first = &agents[0];
    assert!(first.get("name").is_some(), "agent must have 'name'");
    assert!(first.get("status").is_some(), "agent must have 'status'");
    assert!(first.get("pending_messages").is_some(), "agent must have 'pending_messages'");
}

#[tokio::test]
async fn test_status_pending_count() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let pool = setup_file_db(&db_path).await;

    sqlx::query(
        "INSERT INTO agents (id, name, provider, role, command, status, status_updated_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind("id-1").bind("agent-alpha").bind("claude-code").bind("worker")
    .bind("echo alpha").bind("idle").bind("2026-03-06T00:00:00Z").bind("2026-03-06T00:00:00Z")
    .execute(&pool).await.expect("insert agent");

    // Insert 3 pending messages
    for i in 0..3u32 {
        db::messages::insert_message(&pool, "agent-alpha", &format!("task {}", i), "normal")
            .await
            .expect("insert message");
    }

    write_squad_yml(tmp.path(), &db_path);

    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
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
    let pending = agents[0]["pending_messages"].as_u64().expect("pending_messages u64");
    assert_eq!(pending, 3, "expected 3 pending messages, got {}", pending);
}

#[tokio::test]
async fn test_status_empty_squad() {
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_path = tmp.path().join("station.db");
    let _pool = setup_file_db(&db_path).await;

    write_squad_yml(tmp.path(), &db_path);

    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
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
#[ignore] // RED — will pass after Task 3
async fn test_view_no_live_sessions() {
    // Agents in DB but no tmux sessions
    // Assert "No live" message
    todo!()
}
