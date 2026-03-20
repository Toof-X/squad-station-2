use squad_station::config::{sanitize_session_name, SquadConfig};

#[test]
fn test_project_is_string() {
    let yaml = "project: myapp\norchestrator:\n  provider: claude-code\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.project, "myapp");
}

#[test]
fn test_model_description_optional() {
    let yaml = "project: p\norchestrator:\n  provider: claude-code\n  model: claude-opus\n  description: routes\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.orchestrator.model.as_deref(), Some("claude-opus"));
    assert_eq!(cfg.orchestrator.description.as_deref(), Some("routes"));

    let yaml2 = "project: p\norchestrator:\n  provider: claude-code\nagents: []";
    let cfg2: SquadConfig = serde_saphyr::from_str(yaml2).unwrap();
    assert!(cfg2.orchestrator.model.is_none());
}

#[test]
fn test_no_command_field() {
    let yaml = "project: p\norchestrator:\n  provider: claude-code\nagents:\n  - provider: gemini-cli\n    name: worker1";
    let result: Result<SquadConfig, _> = serde_saphyr::from_str(yaml);
    assert!(
        result.is_ok(),
        "Should parse without command field: {:?}",
        result.err()
    );
}

#[test]
fn test_provider_field() {
    let yaml = "project: p\norchestrator:\n  provider: claude-code\nagents:\n  - provider: gemini-cli\n    name: worker1";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.agents[0].provider, "gemini-cli");
}

#[test]
fn test_antigravity_provider_parses() {
    let yaml = "project: p\norchestrator:\n  provider: antigravity\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.orchestrator.provider, "antigravity");
}

#[test]
fn test_is_db_only_antigravity() {
    let yaml = "project: p\norchestrator:\n  provider: antigravity\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert!(cfg.orchestrator.is_db_only());
}

#[test]
fn test_is_db_only_claude_code_false() {
    let yaml = "project: p\norchestrator:\n  provider: claude-code\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert!(!cfg.orchestrator.is_db_only());
}

#[test]
fn test_sanitize_session_name_shell_metacharacters() {
    assert_eq!(sanitize_session_name("a'b"), "a-b");
    assert_eq!(sanitize_session_name("a`b"), "a-b");
    assert_eq!(sanitize_session_name("a$b"), "a-b");
    assert_eq!(sanitize_session_name("a;b"), "a-b");
    assert_eq!(sanitize_session_name("a(b)"), "a-b-");
    assert_eq!(sanitize_session_name("a|b"), "a-b");
    assert_eq!(sanitize_session_name("a&b"), "a-b");
    assert_eq!(sanitize_session_name("a\\b"), "a-b");
    assert_eq!(sanitize_session_name("a b"), "a-b");
    assert_eq!(sanitize_session_name("a/b"), "a-b");
}

#[test]
fn test_sanitize_session_name_injection_attempt() {
    let result = sanitize_session_name("foo'; rm -rf /; echo '");
    assert!(!result.contains('\''));
    assert!(!result.contains(';'));
    assert!(!result.contains(' '));
}
