mod helpers;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a minimal squad.yml into `dir`.
fn write_squad_yml(dir: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  provider: claude-code
  role: orchestrator
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml");
}

/// Write a squad.yml with channels configured on the orchestrator.
fn write_squad_yml_with_channels(dir: &std::path::Path) {
    let yaml = r#"project: test-squad
orchestrator:
  name: test-orch
  provider: claude-code
  role: orchestrator
  channels:
    - "plugin:telegram"
agents: []
"#;
    std::fs::write(dir.join("squad.yml"), yaml).expect("failed to write squad.yml with channels");
}

/// Create a Command for the binary pointing at the test DB, with cwd set to `dir`.
fn cmd_in_dir(dir: &std::path::Path, db_path: &std::path::Path) -> std::process::Command {
    let mut c = std::process::Command::new(bin());
    c.env(
        "SQUAD_STATION_DB",
        db_path.to_str().expect("db path must be valid UTF-8"),
    );
    c.current_dir(dir);
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

// ---------------------------------------------------------------------------
// Task 1 tests — --status and --help CLI tests
// ---------------------------------------------------------------------------

/// OPS-01: watch --status with no PID file reports "No watchdog daemon running".
#[tokio::test]
async fn test_watch_status_no_daemon() {
    let tmp = tempfile::TempDir::new().unwrap();
    let squad_dir = tmp.path().join(".squad");
    std::fs::create_dir_all(&squad_dir).unwrap();

    let db_file = squad_dir.join("station.db");
    let pool = setup_file_db(&db_file).await;
    pool.close().await;

    write_squad_yml(tmp.path());

    // No PID file exists — watch --status should report no daemon.
    let output = cmd_in_dir(tmp.path(), &db_file)
        .args(["watch", "--status"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "watch --status should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("No watchdog daemon running"),
        "Expected 'No watchdog daemon running' in stdout, got: {}",
        stdout
    );
}

/// OPS-01: watch --status with a stale PID file cleans it up and reports "stale PID" or "not running".
#[tokio::test]
async fn test_watch_status_stale_pid() {
    let tmp = tempfile::TempDir::new().unwrap();
    let squad_dir = tmp.path().join(".squad");
    std::fs::create_dir_all(&squad_dir).unwrap();

    let db_file = squad_dir.join("station.db");
    let pool = setup_file_db(&db_file).await;
    pool.close().await;

    write_squad_yml(tmp.path());

    // Write a stale PID file with a PID that almost certainly does not exist.
    let pid_file = squad_dir.join("watch.pid");
    std::fs::write(&pid_file, "999999").unwrap();
    assert!(pid_file.exists(), "PID file should exist before --status");

    let output = cmd_in_dir(tmp.path(), &db_file)
        .args(["watch", "--status"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "watch --status with stale PID should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("stale PID") || stdout.contains("not running"),
        "Expected 'stale PID' or 'not running' in stdout, got: {}",
        stdout
    );
    // The PID file should have been cleaned up.
    assert!(
        !pid_file.exists(),
        "Stale PID file should be removed after --status"
    );
}

/// OPS-02: watch --help lists all 8 expected flags.
#[test]
fn test_watch_help_lists_all_flags() {
    let output = std::process::Command::new(bin())
        .args(["watch", "--help"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in &[
        "interval",
        "stall-threshold",
        "cooldown",
        "debounce",
        "dry-run",
        "status",
        "daemon",
        "stop",
    ] {
        assert!(
            stdout.contains(flag),
            "Expected flag '{}' in watch --help output, got: {}",
            flag,
            stdout
        );
    }
}

/// OPS-02: watch --help exits with code 0.
#[test]
fn test_watch_help_exit_code() {
    let output = std::process::Command::new(bin())
        .args(["watch", "--help"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "watch --help should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
