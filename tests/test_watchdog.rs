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

// ---------------------------------------------------------------------------
// Task 2 tests — --dry-run lifecycle, flag validation, channels config
// ---------------------------------------------------------------------------

/// OPS-03: watch --dry-run starts the binary, runs at least one tick, and creates the log file.
#[tokio::test]
async fn test_watch_dry_run_exits_cleanly() {
    let tmp = tempfile::TempDir::new().unwrap();
    let squad_dir = tmp.path().join(".squad");
    std::fs::create_dir_all(&squad_dir).unwrap();

    let db_file = squad_dir.join("station.db");
    let pool = setup_file_db(&db_file).await;
    pool.close().await;

    write_squad_yml(tmp.path());

    // Spawn (not .output()) so we can kill it after a short delay.
    let mut child = cmd_in_dir(tmp.path(), &db_file)
        .args(["watch", "--dry-run", "--interval", "1", "--stall-threshold", "1"])
        .spawn()
        .expect("failed to spawn watch --dry-run process");

    // Let the watchdog run at least one tick cycle (interval=1s).
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Kill the process — it's an infinite loop by design.
    let _ = child.kill();
    let _ = child.wait();

    // The watchdog should have written its log file on startup.
    let log_file = squad_dir.join("log").join("watch.log");
    assert!(
        log_file.exists(),
        "watch --dry-run should create .squad/log/watch.log, but it was not found"
    );
}

/// Edge-case: watch --interval 0 --dry-run should not panic or crash immediately.
/// Clap accepts 0 as a valid u64; the binary enters the tick loop with 0-second sleep.
/// We just verify it can start without an immediate fatal error.
#[tokio::test]
async fn test_watch_invalid_interval_zero() {
    let tmp = tempfile::TempDir::new().unwrap();
    let squad_dir = tmp.path().join(".squad");
    std::fs::create_dir_all(&squad_dir).unwrap();

    let db_file = squad_dir.join("station.db");
    let pool = setup_file_db(&db_file).await;
    pool.close().await;

    write_squad_yml(tmp.path());

    // Spawn with interval=0 in dry-run mode; expect it to start without immediate crash.
    let mut child = cmd_in_dir(tmp.path(), &db_file)
        .args(["watch", "--interval", "0", "--dry-run"])
        .spawn()
        .expect("failed to spawn watch --interval 0 process");

    // Brief wait; if the process crashes immediately, wait() returns quickly.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // If the process is still running, kill it — that means it started successfully.
    let status = child.try_wait().expect("failed to poll child status");
    if let Some(exit_status) = status {
        // Process already exited — verify it didn't panic (non-zero from SIGKILL is 1/137).
        // Panics produce exit code 101 on Linux. Accept any exit that isn't 101.
        let code = exit_status.code().unwrap_or(0);
        assert_ne!(
            code, 101,
            "watch --interval 0 --dry-run should not panic (exit 101)"
        );
    } else {
        // Still running — good, kill it cleanly.
        let _ = child.kill();
        let _ = child.wait();
    }
}

/// ALERT-04: squad.yml with channels field parses correctly at the config level.
#[test]
fn test_watch_channels_in_squad_yml() {
    let tmp = tempfile::TempDir::new().unwrap();
    write_squad_yml_with_channels(tmp.path());

    let config_path = tmp.path().join("squad.yml");
    let config = squad_station::config::load_config(&config_path)
        .expect("failed to load squad.yml with channels");

    assert_eq!(
        config.orchestrator.channels,
        Some(vec!["plugin:telegram".to_string()]),
        "Expected orchestrator.channels = Some([\"plugin:telegram\"]), got: {:?}",
        config.orchestrator.channels
    );
}
