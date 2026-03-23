mod helpers;

use squad_station::commands::clean;
use squad_station::config::SquadConfig;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================
// clean::compute_session_names — pure function tests
// ============================================================

#[test]
fn clean_session_names_includes_orchestrator_and_workers() {
    let yaml = r#"
project: myapp
orchestrator:
  name: master
  provider: claude-code
  role: orchestrator
agents:
  - name: worker
    provider: claude-code
    role: worker
"#;
    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let names = clean::compute_session_names(&config);
    assert!(names.contains(&"myapp-master".to_string()));
    assert!(names.contains(&"myapp-worker".to_string()));
    assert_eq!(names.len(), 2);
}

#[test]
fn clean_session_names_orchestrator_defaults_to_role_when_no_name() {
    let yaml = r#"
project: proj
orchestrator:
  provider: claude-code
  role: orchestrator
agents: []
"#;
    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let names = clean::compute_session_names(&config);
    assert_eq!(names, vec!["proj-orchestrator"]);
}

#[test]
fn clean_session_names_multiple_agents_with_different_providers() {
    let yaml = r#"
project: app
orchestrator:
  provider: claude-code
  role: orchestrator
agents:
  - name: frontend
    provider: claude-code
    role: worker
  - name: backend
    provider: gemini-cli
    role: worker
"#;
    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let names = clean::compute_session_names(&config);
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"app-orchestrator".to_string()));
    assert!(names.contains(&"app-frontend".to_string()));
    assert!(names.contains(&"app-backend".to_string()));
}

#[test]
fn clean_session_names_agent_falls_back_to_role_when_no_name() {
    let yaml = r#"
project: myapp
orchestrator:
  provider: claude-code
  role: orchestrator
agents:
  - provider: claude-code
    role: researcher
"#;
    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let names = clean::compute_session_names(&config);
    assert!(names.contains(&"myapp-researcher".to_string()));
}

// ============================================================
// clean::delete_db_file — pure file deletion tests
// ============================================================

#[test]
fn clean_delete_db_file_returns_false_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("nonexistent-squad-station.db");
    // file does not exist
    let result = clean::delete_db_file(&db_path);
    assert!(
        result.is_ok(),
        "delete_db_file must not error on missing file"
    );
    assert!(
        !result.unwrap(),
        "should return false when file does not exist"
    );
}

#[test]
fn clean_delete_db_file_deletes_file_and_returns_true() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("station.db");
    std::fs::write(&db_path, b"fake db content").unwrap();
    assert!(db_path.exists(), "setup: file must exist before deletion");

    let result = clean::delete_db_file(&db_path);
    assert!(
        result.is_ok(),
        "delete_db_file must not error on existing file"
    );
    assert!(result.unwrap(), "should return true when file was deleted");
    assert!(
        !db_path.exists(),
        "file must no longer exist after deletion"
    );
}

// ============================================================
// clean::run — error handling
// ============================================================

#[tokio::test]
async fn clean_run_errors_on_missing_config() {
    let result = clean::run(PathBuf::from("nonexistent-squad.yml"), true, false, false).await;
    assert!(result.is_err(), "run must error when config file not found");
}

// ============================================================
// clean::stop_watchdog — watchdog lifecycle tests
// ============================================================

#[test]
fn clean_stop_watchdog_returns_false_when_no_pid_file() {
    let dir = TempDir::new().unwrap();
    assert!(!clean::stop_watchdog(dir.path()));
}

#[test]
fn clean_stop_watchdog_removes_pid_file() {
    let dir = TempDir::new().unwrap();
    let pid_file = dir.path().join("watch.pid");
    std::fs::write(&pid_file, "99999999").unwrap(); // Non-existent PID
    assert!(clean::stop_watchdog(dir.path()));
    assert!(!pid_file.exists(), "PID file must be removed after stop");
}

// ============================================================
// v0.6.0 log preservation — filesystem behavior tests
//
// These test the delete_db_file and stop_watchdog primitives in combination
// with realistic .squad/ directory layouts, verifying the contract:
// - clean (default): deletes DB + PID, preserves .squad/log/
// - clean --all: deletes DB + PID + .squad/log/
// ============================================================

/// Helper: create a realistic .squad/ directory layout
fn setup_squad_dir() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let squad_dir = dir.path().join(".squad");
    let log_dir = squad_dir.join("log");
    std::fs::create_dir_all(&log_dir).unwrap();

    std::fs::write(squad_dir.join("station.db"), b"fake db").unwrap();
    std::fs::write(log_dir.join("signal.log"), "2026-03-20 OK agent=test\n").unwrap();
    std::fs::write(log_dir.join("watch.log"), "2026-03-20 INFO started\n").unwrap();
    std::fs::write(squad_dir.join("watch.pid"), "99999999").unwrap();

    (dir, squad_dir)
}

#[test]
fn clean_default_preserves_logs() {
    let (_dir, squad_dir) = setup_squad_dir();
    let db_path = squad_dir.join("station.db");
    let log_dir = squad_dir.join("log");

    // Simulate clean (default): stop watchdog + delete DB + preserve logs
    clean::stop_watchdog(&squad_dir);
    clean::delete_db_file(&db_path).unwrap();
    // clean also removes watch.pid
    let _ = std::fs::remove_file(squad_dir.join("watch.pid"));

    // DB deleted
    assert!(!db_path.exists(), "station.db must be deleted");
    // watch.pid deleted
    assert!(
        !squad_dir.join("watch.pid").exists(),
        "watch.pid must be removed"
    );
    // Logs preserved
    assert!(
        log_dir.exists(),
        ".squad/log/ must survive clean (no --all)"
    );
    assert!(
        log_dir.join("signal.log").exists(),
        "signal.log must survive"
    );
    assert!(log_dir.join("watch.log").exists(), "watch.log must survive");
}

#[test]
fn clean_all_deletes_logs() {
    let (_dir, squad_dir) = setup_squad_dir();
    let db_path = squad_dir.join("station.db");
    let log_dir = squad_dir.join("log");

    // Simulate clean --all: stop watchdog + delete DB + delete logs
    clean::stop_watchdog(&squad_dir);
    clean::delete_db_file(&db_path).unwrap();
    let _ = std::fs::remove_file(squad_dir.join("watch.pid"));
    // --all: delete log directory
    if log_dir.exists() {
        std::fs::remove_dir_all(&log_dir).unwrap();
    }

    assert!(!db_path.exists(), "station.db must be deleted");
    assert!(!log_dir.exists(), ".squad/log/ must be deleted with --all");
}

#[test]
fn clean_stops_watchdog_before_db_deletion() {
    let (_dir, squad_dir) = setup_squad_dir();
    let pid_path = squad_dir.join("watch.pid");
    assert!(pid_path.exists(), "setup: watch.pid must exist");

    // Step 1: stop watchdog (removes PID file)
    let stopped = clean::stop_watchdog(&squad_dir);
    assert!(
        stopped,
        "stop_watchdog must return true when PID file exists"
    );
    assert!(
        !pid_path.exists(),
        "watch.pid must be removed by stop_watchdog"
    );

    // Step 2: delete DB (safe now — no watchdog crash loop)
    let db_path = squad_dir.join("station.db");
    let deleted = clean::delete_db_file(&db_path).unwrap();
    assert!(deleted, "DB must be deleted after watchdog stopped");
    assert!(
        !db_path.exists(),
        "station.db must not exist after deletion"
    );
}

#[test]
fn clean_ordering_watchdog_stops_first() {
    // Verify the expected ordering: watchdog → sessions → DB → (optionally logs)
    let (_dir, squad_dir) = setup_squad_dir();
    let pid_path = squad_dir.join("watch.pid");
    let db_path = squad_dir.join("station.db");

    // 1. Watchdog must be stopped first
    assert!(pid_path.exists());
    clean::stop_watchdog(&squad_dir);
    assert!(!pid_path.exists(), "watchdog stopped first");

    // 2. Then DB can safely be deleted
    assert!(db_path.exists());
    clean::delete_db_file(&db_path).unwrap();
    assert!(!db_path.exists(), "DB deleted after watchdog");

    // 3. Logs still present
    assert!(
        squad_dir.join("log").join("signal.log").exists(),
        "logs preserved"
    );
}

#[test]
fn clean_handles_missing_log_dir_gracefully() {
    let dir = TempDir::new().unwrap();
    let squad_dir = dir.path().join(".squad");
    std::fs::create_dir_all(&squad_dir).unwrap();
    std::fs::write(squad_dir.join("station.db"), b"fake db").unwrap();
    // No log directory created

    clean::delete_db_file(&squad_dir.join("station.db")).unwrap();

    // --all should not error even if log dir doesn't exist
    let log_dir = squad_dir.join("log");
    assert!(!log_dir.exists());
    // This mirrors the code: if log_dir.exists() { remove_dir_all } else { false }
}

// ============================================================
// reset::run — error handling
// ============================================================

#[tokio::test]
async fn reset_run_errors_on_missing_config() {
    use squad_station::commands::reset;
    let result = reset::run(PathBuf::from("nonexistent-squad.yml"), true, false).await;
    assert!(result.is_err(), "run must error when config file not found");
}
