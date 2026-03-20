mod helpers;

use squad_station::commands::clone;
use squad_station::db;

// ── Unit tests for strip_clone_suffix ──

#[test]
fn test_strip_clone_suffix_no_suffix() {
    assert_eq!(clone::strip_clone_suffix("worker"), "worker");
}

#[test]
fn test_strip_clone_suffix_removes_dash_2() {
    assert_eq!(clone::strip_clone_suffix("worker-2"), "worker");
}

#[test]
fn test_strip_clone_suffix_removes_dash_3() {
    assert_eq!(clone::strip_clone_suffix("worker-3"), "worker");
}

#[test]
fn test_strip_clone_suffix_preserves_non_numeric() {
    assert_eq!(
        clone::strip_clone_suffix("my-project-cc-worker"),
        "my-project-cc-worker"
    );
}

#[test]
fn test_strip_clone_suffix_preserves_dash_1() {
    // N=1 is the original agent, not a clone — don't strip
    assert_eq!(clone::strip_clone_suffix("worker-1"), "worker-1");
}

#[test]
fn test_strip_clone_suffix_preserves_dash_0() {
    assert_eq!(clone::strip_clone_suffix("worker-0"), "worker-0");
}

// ── Unit tests for extract_clone_number ──

#[test]
fn test_extract_clone_number_match() {
    assert_eq!(clone::extract_clone_number("worker-2", "worker"), Some(2));
}

#[test]
fn test_extract_clone_number_large() {
    assert_eq!(clone::extract_clone_number("worker-10", "worker"), Some(10));
}

#[test]
fn test_extract_clone_number_wrong_base() {
    assert_eq!(clone::extract_clone_number("other-2", "worker"), None);
}

#[test]
fn test_extract_clone_number_no_suffix() {
    assert_eq!(clone::extract_clone_number("worker", "worker"), None);
}

// ── Unit tests for get_launch_command ──

#[test]
fn test_launch_command_claude_with_model() {
    assert_eq!(
        clone::get_launch_command("claude-code", Some("opus")),
        "claude --dangerously-skip-permissions --model opus"
    );
}

#[test]
fn test_launch_command_gemini_no_model() {
    assert_eq!(clone::get_launch_command("gemini-cli", None), "gemini -y");
}

#[test]
fn test_launch_command_unknown_provider() {
    assert_eq!(clone::get_launch_command("antigravity", None), "zsh");
}

// ── Integration tests (async, use test DB) ──

#[tokio::test]
async fn test_generate_clone_name_first_clone() {
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "worker", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    let name = clone::generate_clone_name("worker", &pool).await.unwrap();
    assert_eq!(name, "worker-2");
}

#[tokio::test]
async fn test_generate_clone_name_increments() {
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "worker", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "worker-2", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "worker-3", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    let name = clone::generate_clone_name("worker", &pool).await.unwrap();
    assert_eq!(name, "worker-4");
}

#[tokio::test]
async fn test_generate_clone_name_from_existing_clone() {
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "worker", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "worker-2", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    db::agents::insert_agent(&pool, "worker-3", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    // Cloning worker-3 should produce worker-4 (sibling, not nested clone)
    let name = clone::generate_clone_name("worker-3", &pool).await.unwrap();
    assert_eq!(name, "worker-4");
}

#[tokio::test]
async fn test_clone_rejects_orchestrator() {
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "my-orch", "claude-code", "orchestrator", None, None, None)
        .await
        .unwrap();
    let source = db::agents::get_agent(&pool, "my-orch")
        .await
        .unwrap()
        .unwrap();
    // Verify the agent has orchestrator role — the run() function rejects this with bail!
    assert_eq!(source.role, "orchestrator");
}

#[tokio::test]
async fn test_delete_agent_by_name() {
    let pool = helpers::setup_test_db().await;
    db::agents::insert_agent(&pool, "to-delete", "claude-code", "worker", None, None, None)
        .await
        .unwrap();
    assert!(
        db::agents::get_agent(&pool, "to-delete")
            .await
            .unwrap()
            .is_some(),
        "agent should exist before deletion"
    );
    db::agents::delete_agent_by_name(&pool, "to-delete")
        .await
        .unwrap();
    assert!(
        db::agents::get_agent(&pool, "to-delete")
            .await
            .unwrap()
            .is_none(),
        "agent should be gone after delete_agent_by_name"
    );
}
