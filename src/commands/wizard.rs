// wizard.rs — TUI wizard for interactive squad.yml configuration
// Implementation added in Task 2; data types and validation are here.

// ----------------------------------------------------------------------------
// Public API types
// ----------------------------------------------------------------------------

pub struct WizardResult {
    pub project: String,
    pub agents: Vec<AgentInput>,
}

pub struct AgentInput {
    pub role: String,
    pub tool: String, // "claude-code" | "gemini-cli" | "antigravity"
    pub model: Option<String>,
    pub description: Option<String>,
}

// ----------------------------------------------------------------------------
// Tool enum
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    ClaudeCode,
    GeminiCli,
    Antigravity,
}

impl Tool {
    pub fn cycle_next(self) -> Self {
        match self {
            Tool::ClaudeCode => Tool::GeminiCli,
            Tool::GeminiCli => Tool::Antigravity,
            Tool::Antigravity => Tool::ClaudeCode,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Tool::ClaudeCode => "claude-code",
            Tool::GeminiCli => "gemini-cli",
            Tool::Antigravity => "antigravity",
        }
    }
}

// ----------------------------------------------------------------------------
// TextInputState
// ----------------------------------------------------------------------------

pub struct TextInputState {
    pub value: String,
    pub error: Option<String>,
}

impl TextInputState {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            error: None,
        }
    }

    pub fn push(&mut self, c: char) {
        self.value.push(c);
        self.error = None;
    }

    pub fn pop(&mut self) {
        self.value.pop();
        self.error = None;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// AgentDraft
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum AgentField {
    Role,
    Tool,
    Model,
    Description,
}

pub struct AgentDraft {
    pub role: TextInputState,
    pub tool: Tool,
    pub model: TextInputState,
    pub description: TextInputState,
    pub focused_field: AgentField,
}

impl AgentDraft {
    pub fn new() -> Self {
        Self {
            role: TextInputState::new(),
            tool: Tool::ClaudeCode,
            model: TextInputState::new(),
            description: TextInputState::new(),
            focused_field: AgentField::Role,
        }
    }
}

impl Default for AgentDraft {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// WizardPage and WizardState
// ----------------------------------------------------------------------------

enum WizardPage {
    ProjectName,
    AgentCount,
    AgentConfig { index: usize },
    Summary,
}

struct WizardState {
    page: WizardPage,
    project_input: TextInputState,
    count_input: TextInputState,
    agent_count: usize,
    agents: Vec<AgentDraft>,
}

impl WizardState {
    fn new() -> Self {
        Self {
            page: WizardPage::ProjectName,
            project_input: TextInputState::new(),
            count_input: TextInputState::new(),
            agent_count: 0,
            agents: Vec::new(),
        }
    }

    fn into_result(self) -> WizardResult {
        WizardResult {
            project: self.project_input.value.trim().to_string(),
            agents: self
                .agents
                .into_iter()
                .map(|d| AgentInput {
                    role: d.role.value.trim().to_string(),
                    tool: d.tool.as_str().to_string(),
                    model: if d.model.value.trim().is_empty() {
                        None
                    } else {
                        Some(d.model.value.trim().to_string())
                    },
                    description: if d.description.value.trim().is_empty() {
                        None
                    } else {
                        Some(d.description.value.trim().to_string())
                    },
                })
                .collect(),
        }
    }
}

// ----------------------------------------------------------------------------
// Validation functions
// ----------------------------------------------------------------------------

fn validate_project_name(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Project name is required".to_string())
    } else {
        Ok(())
    }
}

fn validate_count(input: &str) -> Result<usize, String> {
    input
        .trim()
        .parse::<usize>()
        .map_err(|_| "Please enter a whole number (e.g. 2)".to_string())
        .and_then(|n| {
            if n >= 1 {
                Ok(n)
            } else {
                Err("Agent count must be at least 1".to_string())
            }
        })
}

fn validate_role(input: &str) -> Result<(), String> {
    if input.trim().is_empty() {
        Err("Role is required".to_string())
    } else {
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// Public run function (stub — full implementation in Task 2)
// ----------------------------------------------------------------------------

pub async fn run() -> anyhow::Result<Option<WizardResult>> {
    // Full implementation in Task 2
    todo!("Wizard TUI implementation")
}

// ----------------------------------------------------------------------------
// Unit tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_push() {
        let mut s = TextInputState::new();
        assert_eq!(s.value, "");
        s.push('a');
        s.push('b');
        assert_eq!(s.value, "ab");
    }

    #[test]
    fn test_text_input_pop() {
        let mut s = TextInputState::new();
        s.push('a');
        s.push('b');
        s.pop();
        assert_eq!(s.value, "a");
        s.pop();
        assert_eq!(s.value, "");
        // pop on empty should not panic
        s.pop();
        assert_eq!(s.value, "");
    }

    #[test]
    fn test_text_input_error() {
        let mut s = TextInputState::new();
        s.error = Some("err".to_string());
        s.clear_error();
        assert_eq!(s.error, None);
    }

    #[test]
    fn test_validate_count_valid() {
        assert_eq!(validate_count("3"), Ok(3));
        assert_eq!(validate_count("1"), Ok(1));
    }

    #[test]
    fn test_validate_count_invalid() {
        let err = validate_count("0").unwrap_err();
        assert!(
            err.contains("at least 1"),
            "Expected 'at least 1' in: {}",
            err
        );

        let err = validate_count("abc").unwrap_err();
        assert!(
            err.contains("whole number"),
            "Expected 'whole number' in: {}",
            err
        );

        let err = validate_count("").unwrap_err();
        assert!(
            err.contains("whole number"),
            "Expected 'whole number' in: {}",
            err
        );
    }

    #[test]
    fn test_validate_role_valid() {
        assert!(validate_role("backend").is_ok());
        assert!(validate_role("  lead  ").is_ok());
    }

    #[test]
    fn test_validate_role_empty() {
        let err = validate_role("").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );

        let err = validate_role("   ").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );
    }

    #[test]
    fn test_tool_cycle() {
        assert_eq!(Tool::ClaudeCode.cycle_next(), Tool::GeminiCli);
        assert_eq!(Tool::GeminiCli.cycle_next(), Tool::Antigravity);
        assert_eq!(Tool::Antigravity.cycle_next(), Tool::ClaudeCode);
    }

    #[test]
    fn test_tool_as_str() {
        assert_eq!(Tool::ClaudeCode.as_str(), "claude-code");
        assert_eq!(Tool::GeminiCli.as_str(), "gemini-cli");
        assert_eq!(Tool::Antigravity.as_str(), "antigravity");
    }

    #[test]
    fn test_tool_display_matches_valid_providers() {
        let valid_providers = &["antigravity", "claude-code", "gemini-cli"];
        assert!(
            valid_providers.contains(&Tool::ClaudeCode.as_str()),
            "ClaudeCode.as_str() not in VALID_PROVIDERS"
        );
        assert!(
            valid_providers.contains(&Tool::GeminiCli.as_str()),
            "GeminiCli.as_str() not in VALID_PROVIDERS"
        );
        assert!(
            valid_providers.contains(&Tool::Antigravity.as_str()),
            "Antigravity.as_str() not in VALID_PROVIDERS"
        );
    }

    #[test]
    fn test_validate_project_name() {
        let err = validate_project_name("").unwrap_err();
        assert!(
            err.contains("required"),
            "Expected 'required' in: {}",
            err
        );

        assert!(validate_project_name("my-project").is_ok());
    }
}
