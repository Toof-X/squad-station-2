mod helpers;

#[test]
fn test_views_module_compiles() {
    // Smoke test: this file compiles and test infra works
    assert!(true);
}

#[tokio::test]
#[ignore] // RED — will pass after Task 2 implements status command
async fn test_status_text_output() {
    // Set up DB with 2 agents (one idle, one dead)
    // Run `squad-station status` pointing at test DB
    // Assert exit code 0, output contains project name, "idle", "dead"
    todo!()
}

#[tokio::test]
#[ignore] // RED — will pass after Task 2
async fn test_status_json_output() {
    // Run `squad-station status --json`, parse JSON
    // Assert "project", "db_path", "agents" fields exist
    todo!()
}

#[tokio::test]
#[ignore] // RED — will pass after Task 2
async fn test_status_pending_count() {
    // Insert 3 pending messages, run status --json
    // Assert agent pending_messages == 3
    todo!()
}

#[tokio::test]
#[ignore] // RED — will pass after Task 2
async fn test_status_empty_squad() {
    // No agents, run status, assert "No agents registered."
    todo!()
}

#[tokio::test]
#[ignore] // RED — will pass after Task 3
async fn test_view_no_live_sessions() {
    // Agents in DB but no tmux sessions
    // Assert "No live" message
    todo!()
}
