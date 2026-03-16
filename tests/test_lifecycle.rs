mod helpers;

use squad_station::db;

// ============================================================
// Context command tests — SESS-05
// ============================================================

/// Helper: write a minimal squad.yml into `dir` using the new format.
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

#[test]
fn test_context_output_contains_agents() {
    // SESS-05 (updated): context command writes a single .agent/workflows/squad-orchestrator.md
    // and prints a 1-line summary. (GAP-18 / PLAY-01: unified single-file output)
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    let output = cmd_with_db(&db_file)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "context command should exit 0, got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    // New behavior: 1-line summary to stdout
    assert!(
        stdout.contains("Generated") && stdout.contains("squad-orchestrator.md"),
        "context output must contain 1-line summary, got:\n{}",
        stdout
    );
    // Orchestrator context at .claude/commands/squad-orchestrator.md (GAP-02)
    assert!(
        tmp.path()
            .join(".claude/commands/squad-orchestrator.md")
            .exists(),
        ".claude/commands/squad-orchestrator.md must be created"
    );
}

#[test]
fn test_context_output_has_usage() {
    // SESS-05 (updated): context command writes a single unified squad-orchestrator.md.
    // Delegation instructions are now merged into squad-orchestrator.md (PLAY-01, PLAY-02).
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    let output = cmd_with_db(&db_file)
        .arg("context")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    assert!(
        output.status.success(),
        "context command should exit 0, got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    // Orchestrator context at .claude/commands/squad-orchestrator.md (GAP-02)
    let orchestrator_path = tmp.path().join(".claude/commands/squad-orchestrator.md");
    assert!(
        orchestrator_path.exists(),
        ".claude/commands/squad-orchestrator.md must be created"
    );
    let content = std::fs::read_to_string(&orchestrator_path).unwrap();
    assert!(
        content.contains("squad-station send"),
        "squad-orchestrator.md must include 'squad-station send' example, got:\n{}",
        content
    );
    assert!(
        content.contains("Session Routing"),
        "squad-orchestrator.md must contain 'Session Routing' section, got:\n{}",
        content
    );
}

// ============================================================
// Agents command status+duration format test — SESS-03
// ============================================================

#[test]
fn test_agents_command_shows_status_with_duration() {
    // SESS-03: agents command output shows status with a human-readable duration (e.g., "idle 0m").
    // format_status_with_duration is private; we verify the behavior through the agents subcommand.
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
    let db_file = tmp.path().join("station.db");
    write_squad_yml(tmp.path(), &db_file);

    // Register a worker agent via the register subcommand (uses SQUAD_STATION_DB for DB path)
    let reg = cmd_with_db(&db_file)
        .args([
            "register", "worker-a", "--role", "worker", "--tool", "claude",
        ])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run register");
    assert!(
        reg.status.success(),
        "register should succeed, got: {:?}\nstderr: {}",
        reg.status,
        String::from_utf8_lossy(&reg.stderr)
    );

    // Run agents command — no tmux session, so worker-a will be reconciled to "dead"
    let output = cmd_with_db(&db_file)
        .arg("agents")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "agents command should exit 0, got: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    // Duration pattern: status word followed by a number and "m" (or "h" for hours).
    // Since this runs immediately after register, the duration must be 0m or very small.
    let has_duration_pattern = stdout.contains("0m")
        || stdout.contains("1m")
        || stdout
            .split_whitespace()
            .any(|w| w.ends_with('m') || w.ends_with('h'));
    assert!(
        has_duration_pattern,
        "agents output must include status+duration (e.g., 'dead 0m'), got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("worker-a"),
        "agents output must list the registered agent, got:\n{}",
        stdout
    );
}

// ============================================================
// Signal guard tests — HOOK-03
// ============================================================

#[test]
fn test_signal_no_tmux_pane_exits_zero() {
    // HOOK-03: Guard 1 — when TMUX_PANE is unset, signal exits 0 silently
    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .arg("signal")
        .arg("some-agent")
        .env_remove("TMUX_PANE") // Ensure not set
        .output()
        .expect("failed to run binary");
    assert!(
        output.status.success(),
        "signal without TMUX_PANE should exit 0, got: {:?}",
        output.status
    );
    // Guard 1 should produce no stdout output (silent exit)
    assert!(
        output.stdout.is_empty(),
        "signal guard should produce no stdout output"
    );
}

// ============================================================
// Agent status lifecycle tests — SESS-03, SESS-04
// ============================================================

#[tokio::test]
async fn test_update_agent_status_dead_to_idle() {
    // SESS-04: agent can be revived from dead to idle (simulates tmux session reappearing)
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "agent-1", "claude", "worker", None, None)
        .await
        .unwrap();
    // Set to dead
    db::agents::update_agent_status(&pool, "agent-1", "dead")
        .await
        .unwrap();
    let agent = db::agents::get_agent(&pool, "agent-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, "dead");
    // Revive to idle (simulates tmux session reappearing)
    db::agents::update_agent_status(&pool, "agent-1", "idle")
        .await
        .unwrap();
    let agent = db::agents::get_agent(&pool, "agent-1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(agent.status, "idle");
}

// ============================================================
// Orchestrator detection tests — HOOK-01
// ============================================================

#[tokio::test]
async fn test_orchestrator_has_orchestrator_role() {
    // HOOK-01: get_orchestrator returns the agent with role = "orchestrator"
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "orch", "claude", "orchestrator", None, None)
        .await
        .unwrap();
    let orch = db::agents::get_orchestrator(&pool).await.unwrap().unwrap();
    assert_eq!(orch.role, "orchestrator");
    assert_eq!(orch.name, "orch");
}

#[tokio::test]
async fn test_get_orchestrator_returns_none_when_no_orchestrator() {
    // HOOK-01: get_orchestrator returns None if no orchestrator is registered
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "worker-1", "claude", "worker", None, None)
        .await
        .unwrap();
    let result = db::agents::get_orchestrator(&pool).await.unwrap();
    assert!(
        result.is_none(),
        "no orchestrator registered → get_orchestrator returns None"
    );
}

// ============================================================
// List agents status tests — SESS-04
// ============================================================

#[tokio::test]
async fn test_list_agents_includes_status() {
    // SESS-04: list_agents returns status for each agent
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "a1", "claude", "worker", None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "a2", "gemini", "worker", None, None)
        .await
        .unwrap();
    db::agents::update_agent_status(&pool, "a2", "busy")
        .await
        .unwrap();
    let agents = db::agents::list_agents(&pool).await.unwrap();
    assert_eq!(agents.len(), 2);
    let a1 = agents.iter().find(|a| a.name == "a1").unwrap();
    let a2 = agents.iter().find(|a| a.name == "a2").unwrap();
    assert_eq!(a1.status, "idle");
    assert_eq!(a2.status, "busy");
}

// ============================================================
// Signal Guard 2 test — HOOK-03
// ============================================================

#[test]
fn test_signal_guard_db_error_exits_zero_with_warning() {
    // HOOK-03: Guard 2 — when TMUX_PANE is set but no squad.yml exists, signal exits 0
    // and prints a warning to stderr (config/DB errors must not fail the provider).
    let tmp = tempfile::TempDir::new().expect("failed to create temp dir");

    let bin = env!("CARGO_BIN_EXE_squad-station");
    let output = std::process::Command::new(bin)
        .arg("signal")
        .arg("some-agent")
        .env("TMUX_PANE", "%0") // Guard 1 passes (TMUX_PANE is set)
        .current_dir(tmp.path()) // No squad.yml in this dir — Guard 2 triggers
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "signal with missing squad.yml should exit 0 (Guard 2), got: {:?}\nstderr: {}",
        output.status,
        stderr
    );
    assert!(
        stderr.contains("warning") || stderr.contains("Warning"),
        "signal Guard 2 must print a warning to stderr, got stderr: {:?}",
        stderr
    );
    // Stdout must be empty — warning goes to stderr only
    assert!(
        output.stdout.is_empty(),
        "signal Guard 2 must produce no stdout output, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
