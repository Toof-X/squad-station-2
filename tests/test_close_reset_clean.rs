mod helpers;

use squad_station::commands::clean;
use squad_station::commands::close;
use squad_station::config::SquadConfig;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================
// close::compute_session_names — pure function tests
// ============================================================

#[test]
fn close_session_names_includes_orchestrator_and_workers() {
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
    let names = close::compute_session_names(&config);
    assert!(names.contains(&"myapp-master".to_string()));
    assert!(names.contains(&"myapp-worker".to_string()));
    assert_eq!(names.len(), 2);
}

#[test]
fn close_session_names_orchestrator_defaults_to_role_when_no_name() {
    let yaml = r#"
project: proj
orchestrator:
  provider: claude-code
  role: orchestrator
agents: []
"#;
    let config: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    let names = close::compute_session_names(&config);
    assert_eq!(names, vec!["proj-orchestrator"]);
}

#[test]
fn close_session_names_multiple_agents_with_different_providers() {
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
    let names = close::compute_session_names(&config);
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"app-orchestrator".to_string()));
    assert!(names.contains(&"app-frontend".to_string()));
    assert!(names.contains(&"app-backend".to_string()));
}

#[test]
fn close_session_names_agent_falls_back_to_role_when_no_name() {
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
    let names = close::compute_session_names(&config);
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
    let result = clean::run(PathBuf::from("nonexistent-squad.yml"), true, false).await;
    assert!(result.is_err(), "run must error when config file not found");
}

// ============================================================
// close::run — error handling
// ============================================================

#[tokio::test]
async fn close_run_errors_on_missing_config() {
    let result = close::run(PathBuf::from("nonexistent-squad.yml"), false).await;
    assert!(result.is_err(), "run must error when config file not found");
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
