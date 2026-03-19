// templates.rs — Static role template data for the agent wizard (Phase 24)
// Provides pre-defined agent configurations that users can select when creating agents.
// Templates supply slug, display name, description, default provider, per-provider model
// mappings, and routing hint keywords for the orchestrator routing matrix.

/// Static data for a role template shown in the wizard's template selector.
pub struct AgentTemplate {
    /// URL-safe identifier (e.g. "coder", "qa-engineer")
    pub slug: &'static str,
    /// Human-readable name shown in the selector list
    pub display_name: &'static str,
    /// 2-3 sentence description shown in the preview pane
    pub description: &'static str,
    /// Default provider: "claude-code" | "gemini-cli"
    pub default_provider: &'static str,
    /// Claude model shorthand ("sonnet" | "opus") — used when provider = claude-code
    pub claude_model: &'static str,
    /// Gemini model string — used when provider = gemini-cli
    pub gemini_model: &'static str,
    /// Keywords the orchestrator uses to route tasks to this agent type
    pub routing_hints: &'static [&'static str],
}

// ----------------------------------------------------------------------------
// Worker templates (8 entries)
// ----------------------------------------------------------------------------

pub const WORKER_TEMPLATES: &[AgentTemplate] = &[
    AgentTemplate {
        slug: "coder",
        display_name: "Coder",
        description: "Broad umbrella for all implementation work across the stack. \
                      Handles frontend, backend, and mobile feature development, bug fixes, \
                      and general coding tasks. Use this role when the work is primarily writing \
                      or modifying source code.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "code", "implement", "build", "fix", "feature", "bug", "frontend", "backend",
            "mobile",
        ],
    },
    AgentTemplate {
        slug: "solution-architect",
        display_name: "Solution Architect",
        description: "Covers technical leadership, architecture design, and solution planning. \
                      Suitable for tech lead responsibilities, high-level system design, \
                      refactoring strategies, and addressing technical debt. Use this role when \
                      decisions require broad system-level thinking.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "architecture",
            "design",
            "system",
            "plan",
            "review",
            "technical-debt",
            "refactor",
        ],
    },
    AgentTemplate {
        slug: "qa-engineer",
        display_name: "QA Engineer",
        description: "Specialises in testing, quality assurance, and test automation. \
                      Writes unit, integration, and end-to-end tests, investigates regressions, \
                      and ensures adequate coverage. Use this role whenever the primary output \
                      is test code or a quality report.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &["test", "qa", "quality", "bug", "regression", "coverage", "e2e"],
    },
    AgentTemplate {
        slug: "devops-engineer",
        display_name: "DevOps Engineer",
        description: "Handles CI/CD pipelines, infrastructure-as-code, and deployment automation. \
                      Works with Docker, Kubernetes, cloud providers, and monitoring tooling. \
                      Use this role for any task touching build, release, or infrastructure \
                      configuration.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "deploy",
            "ci",
            "cd",
            "pipeline",
            "docker",
            "kubernetes",
            "infrastructure",
            "monitoring",
        ],
    },
    AgentTemplate {
        slug: "code-reviewer",
        display_name: "Code Reviewer",
        description: "Performs thorough code reviews, enforces coding standards, and provides \
                      constructive feedback on pull requests. Checks for correctness, readability, \
                      security, and adherence to team conventions. Use this role when the \
                      primary output is review comments or quality feedback.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &["review", "pr", "pull-request", "feedback", "standards", "lint"],
    },
    AgentTemplate {
        slug: "technical-writer",
        display_name: "Technical Writer",
        description: "Creates and maintains technical documentation, API references, README files, \
                      changelogs, and developer guides. Ensures docs stay in sync with code changes \
                      and are accessible to the intended audience. Use this role when the primary \
                      deliverable is documentation rather than code.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-flash",
        routing_hints: &["docs", "documentation", "readme", "api-docs", "changelog", "guide"],
    },
    AgentTemplate {
        slug: "data-engineer",
        display_name: "Data Engineer",
        description: "Designs database schemas, writes SQL migrations, and builds data pipelines. \
                      Handles ETL processes, analytics queries, and data modelling. Use this role \
                      when the work centres on data storage, retrieval, transformation, or \
                      reporting.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "data",
            "database",
            "sql",
            "migration",
            "etl",
            "schema",
            "analytics",
        ],
    },
    AgentTemplate {
        slug: "security-engineer",
        display_name: "Security Engineer",
        description: "Conducts security audits, assesses vulnerabilities, and implements \
                      authentication and authorisation mechanisms. Ensures encryption, compliance, \
                      and secure coding practices are followed throughout the codebase. Use this \
                      role for any task with a primary security or compliance objective.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "security",
            "audit",
            "vulnerability",
            "auth",
            "encryption",
            "compliance",
        ],
    },
];

// ----------------------------------------------------------------------------
// Orchestrator templates (3 entries)
// ----------------------------------------------------------------------------

pub const ORCHESTRATOR_TEMPLATES: &[AgentTemplate] = &[
    AgentTemplate {
        slug: "project-manager",
        display_name: "Project Manager",
        description: "Coordinates project activities, tracks task progress, and manages \
                      milestones and priorities. Ensures the team has clarity on deadlines \
                      and keeps stakeholders informed. Use this role when the orchestrator \
                      focuses on planning and delivery coordination rather than technical \
                      decisions.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "coordinate",
            "plan",
            "track",
            "milestone",
            "priority",
            "deadline",
        ],
    },
    AgentTemplate {
        slug: "tech-lead",
        display_name: "Tech Lead",
        description: "Provides technical leadership, makes architecture decisions, and enforces \
                      coding standards across the team. Mentors other agents, reviews critical \
                      design choices, and resolves technical conflicts. Use this role when the \
                      orchestrator needs strong technical authority and hands-on involvement in \
                      code quality.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "architecture",
            "technical",
            "decision",
            "review",
            "standard",
            "mentor",
        ],
    },
    AgentTemplate {
        slug: "scrum-master",
        display_name: "Scrum Master",
        description: "Facilitates agile ceremonies, manages the sprint backlog, and removes \
                      blockers for the team. Tracks velocity, organises stand-ups and \
                      retrospectives, and ensures the team follows the agreed process. Use this \
                      role when the orchestrator's primary responsibility is process facilitation \
                      rather than direct task execution.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "sprint",
            "standup",
            "retrospective",
            "blocker",
            "velocity",
            "backlog",
        ],
    },
];

// ----------------------------------------------------------------------------
// Sentinel indices
// ----------------------------------------------------------------------------

/// Index value used to represent "Custom" in the worker template selector.
/// Equal to WORKER_TEMPLATES.len() so it sits just past the last real entry.
pub const CUSTOM_IDX_WORKER: usize = WORKER_TEMPLATES.len();

/// Index value used to represent "Custom" in the orchestrator template selector.
/// Equal to ORCHESTRATOR_TEMPLATES.len() so it sits just past the last real entry.
pub const CUSTOM_IDX_ORCHESTRATOR: usize = ORCHESTRATOR_TEMPLATES.len();
