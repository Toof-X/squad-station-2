use squad_station::config::SquadConfig;

#[test]
fn test_project_is_string() {
    let yaml = "project: myapp\norchestrator:\n  tool: claude-code\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.project, "myapp");
}

#[test]
fn test_model_description_optional() {
    let yaml = "project: p\norchestrator:\n  tool: claude-code\n  model: claude-opus\n  description: routes\nagents: []";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.orchestrator.model.as_deref(), Some("claude-opus"));
    assert_eq!(cfg.orchestrator.description.as_deref(), Some("routes"));

    let yaml2 = "project: p\norchestrator:\n  tool: claude-code\nagents: []";
    let cfg2: SquadConfig = serde_saphyr::from_str(yaml2).unwrap();
    assert!(cfg2.orchestrator.model.is_none());
}

#[test]
fn test_no_command_field() {
    let yaml = "project: p\norchestrator:\n  tool: claude-code\nagents:\n  - tool: gemini\n    name: worker1";
    let result: Result<SquadConfig, _> = serde_saphyr::from_str(yaml);
    assert!(result.is_ok(), "Should parse without command field: {:?}", result.err());
}

#[test]
fn test_tool_field() {
    let yaml = "project: p\norchestrator:\n  tool: claude-code\nagents:\n  - tool: gemini\n    name: worker1";
    let cfg: SquadConfig = serde_saphyr::from_str(yaml).unwrap();
    assert_eq!(cfg.agents[0].tool, "gemini");
}
