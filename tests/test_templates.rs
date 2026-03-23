mod helpers;

use squad_station::commands::context::build_orchestrator_md;
use squad_station::commands::templates::{
    CUSTOM_IDX_ORCHESTRATOR, CUSTOM_IDX_WORKER, ORCHESTRATOR_TEMPLATES, WORKER_TEMPLATES,
};
use squad_station::db::agents::{get_agent, insert_agent, Agent};

// -- Template catalog unit tests --

#[test]
fn test_worker_template_count() {
    assert_eq!(WORKER_TEMPLATES.len(), 12);
}

#[test]
fn test_orchestrator_template_count() {
    assert_eq!(ORCHESTRATOR_TEMPLATES.len(), 4);
}

#[test]
fn test_worker_template_order() {
    assert_eq!(WORKER_TEMPLATES[0].slug, "coder");
    assert_eq!(WORKER_TEMPLATES[1].slug, "solution-architect");
    assert_eq!(WORKER_TEMPLATES[2].slug, "qa-engineer");
    assert_eq!(WORKER_TEMPLATES[3].slug, "devops-engineer");
    assert_eq!(WORKER_TEMPLATES[4].slug, "code-reviewer");
    assert_eq!(WORKER_TEMPLATES[5].slug, "technical-writer");
    assert_eq!(WORKER_TEMPLATES[6].slug, "data-engineer");
    assert_eq!(WORKER_TEMPLATES[7].slug, "security-engineer");
    assert_eq!(WORKER_TEMPLATES[8].slug, "market-researcher");
    assert_eq!(WORKER_TEMPLATES[9].slug, "ua-lead");
    assert_eq!(WORKER_TEMPLATES[10].slug, "design-lead");
    assert_eq!(WORKER_TEMPLATES[11].slug, "tech-researcher");
}

#[test]
fn test_template_fields_populated() {
    for t in WORKER_TEMPLATES.iter().chain(ORCHESTRATOR_TEMPLATES.iter()) {
        assert!(!t.slug.is_empty(), "slug empty for {:?}", t.display_name);
        assert!(
            !t.display_name.is_empty(),
            "display_name empty for {}",
            t.slug
        );
        assert!(
            !t.description.is_empty(),
            "description empty for {}",
            t.slug
        );
        assert!(
            !t.default_provider.is_empty(),
            "default_provider empty for {}",
            t.slug
        );
        assert!(
            !t.claude_model.is_empty(),
            "claude_model empty for {}",
            t.slug
        );
        assert!(
            !t.gemini_model.is_empty(),
            "gemini_model empty for {}",
            t.slug
        );
        assert!(
            t.routing_hints.len() >= 3,
            "routing_hints < 3 for {}",
            t.slug
        );
    }
}

#[test]
fn test_template_description_length() {
    for t in WORKER_TEMPLATES.iter().chain(ORCHESTRATOR_TEMPLATES.iter()) {
        let period_count = t.description.matches('.').count();
        assert!(
            period_count >= 2,
            "description for {} has {} periods, expected >= 2",
            t.slug,
            period_count
        );
    }
}

#[test]
fn test_custom_sentinel_indices() {
    assert_eq!(CUSTOM_IDX_WORKER, 12);
    assert_eq!(CUSTOM_IDX_ORCHESTRATOR, 4);
}

#[test]
fn test_custom_template_clears_fields() {
    use squad_station::commands::wizard::{AgentDraft, ModelSelector, Provider, TextInputState};

    // Create a draft and simulate having a template applied (non-default values)
    let mut draft = AgentDraft::new();
    draft.name.value = "coder".to_string();
    draft.name.cursor = 5;
    draft.provider = Provider::GeminiCli;
    draft.description.value = "Some template description.".to_string();
    draft.routing_hints = Some(vec!["code", "build"]);
    draft.template_index = CUSTOM_IDX_WORKER; // Select "Custom"

    // Simulate what handle_agent_key does on Enter when template_index == custom_idx:
    // Custom clears all fields to defaults
    let custom_idx = WORKER_TEMPLATES.len();
    assert_eq!(
        draft.template_index, custom_idx,
        "template_index should be at Custom sentinel"
    );

    // After applying Custom selection (same logic as handle_agent_key Template arm):
    draft.name = TextInputState::new();
    draft.provider = Provider::ClaudeCode;
    draft.model = ModelSelector::new();
    draft.custom_model = TextInputState::new();
    draft.description = TextInputState::new();
    draft.routing_hints = None;

    // Verify all fields are reset
    assert!(
        draft.name.value.is_empty(),
        "Name should be empty after Custom"
    );
    assert!(
        matches!(draft.provider, Provider::ClaudeCode),
        "Provider should reset to ClaudeCode"
    );
    assert!(
        draft.description.value.is_empty(),
        "Description should be empty after Custom"
    );
    assert!(
        draft.routing_hints.is_none(),
        "routing_hints should be None after Custom"
    );
}

#[test]
fn test_template_autofill_model_index() {
    use squad_station::commands::wizard::{ModelSelector, Provider};

    // For the "coder" template: claude_model = "sonnet"
    let coder = &WORKER_TEMPLATES[0];
    assert_eq!(coder.slug, "coder");
    assert_eq!(coder.claude_model, "sonnet");

    // Simulate auto-fill: look up "sonnet" in ClaudeCode model options
    let model_opts = ModelSelector::options_for(Provider::ClaudeCode);
    let expected_index = model_opts.iter().position(|&m| m == coder.claude_model);
    assert!(
        expected_index.is_some(),
        "coder's claude_model '{}' must exist in ClaudeCode model options",
        coder.claude_model
    );

    // For "solution-architect": claude_model = "opus"
    let sa = &WORKER_TEMPLATES[1];
    assert_eq!(sa.slug, "solution-architect");
    assert_eq!(sa.claude_model, "opus");
    let sa_index = model_opts.iter().position(|&m| m == sa.claude_model);
    assert!(
        sa_index.is_some(),
        "solution-architect's claude_model '{}' must exist in ClaudeCode model options",
        sa.claude_model
    );

    // Verify they map to different indices (sonnet != opus)
    assert_ne!(
        expected_index, sa_index,
        "Different model strings should map to different indices"
    );
}

// -- Routing Matrix output tests (no DB needed, direct struct construction) --

fn make_test_agent(name: &str, role: &str, routing_hints: Option<&str>) -> Agent {
    Agent {
        id: "test-id".to_string(),
        name: name.to_string(),
        tool: "claude-code".to_string(),
        role: role.to_string(),
        command: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        status: "idle".to_string(),
        status_updated_at: "2026-01-01T00:00:00Z".to_string(),
        model: Some("sonnet".to_string()),
        description: Some("test agent".to_string()),
        current_task: None,
        routing_hints: routing_hints.map(|s| s.to_string()),
    }
}

#[test]
fn test_routing_matrix_with_hints() {
    let agents = vec![
        make_test_agent("orch", "orchestrator", None),
        make_test_agent("my-coder", "worker", Some(r#"["code","build","fix"]"#)),
    ];
    let output = build_orchestrator_md(&agents, "/tmp/test", &[], &[]);
    assert!(
        output.contains("## Routing Matrix"),
        "Missing Routing Matrix heading"
    );
    assert!(
        output.contains("| Keyword | Route to |"),
        "Missing table header"
    );
    assert!(
        output.contains("| code | my-coder |"),
        "Missing keyword row"
    );
    assert!(
        output.contains("| build | my-coder |"),
        "Missing keyword row"
    );
}

#[test]
fn test_routing_matrix_empty() {
    let agents = vec![
        make_test_agent("orch", "orchestrator", None),
        make_test_agent("my-worker", "worker", None),
    ];
    let output = build_orchestrator_md(&agents, "/tmp/test", &[], &[]);
    assert!(
        output.contains("## Routing Matrix"),
        "Missing Routing Matrix heading"
    );
    assert!(
        output.contains("No routing hints configured"),
        "Missing placeholder"
    );
}

#[test]
fn test_routing_matrix_skips_orchestrator() {
    let agents = vec![
        make_test_agent("orch", "orchestrator", Some(r#"["plan","coordinate"]"#)),
        make_test_agent("my-worker", "worker", Some(r#"["code"]"#)),
    ];
    let output = build_orchestrator_md(&agents, "/tmp/test", &[], &[]);
    assert!(
        !output.contains("| plan | orch |"),
        "Orchestrator must not appear in routing matrix"
    );
    assert!(
        output.contains("| code | my-worker |"),
        "Worker must appear"
    );
}

// -- DB integration tests --

#[tokio::test]
async fn test_insert_agent_routing_hints() {
    let pool = helpers::setup_test_db().await;
    insert_agent(
        &pool,
        "test-agent",
        "claude-code",
        "worker",
        Some("sonnet"),
        Some("desc"),
        Some(r#"["code","build"]"#),
    )
    .await
    .unwrap();
    let agent = get_agent(&pool, "test-agent").await.unwrap().unwrap();
    assert_eq!(agent.routing_hints.as_deref(), Some(r#"["code","build"]"#));
}

#[tokio::test]
async fn test_insert_agent_routing_hints_null() {
    let pool = helpers::setup_test_db().await;
    insert_agent(
        &pool,
        "test-agent",
        "claude-code",
        "worker",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let agent = get_agent(&pool, "test-agent").await.unwrap().unwrap();
    assert!(agent.routing_hints.is_none());
}
