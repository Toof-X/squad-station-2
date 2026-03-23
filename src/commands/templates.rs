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
// Worker templates (12 entries)
// ----------------------------------------------------------------------------

pub const WORKER_TEMPLATES: &[AgentTemplate] = &[
    // ── Engineering agents ───────────────────────────────────────────────
    AgentTemplate {
        slug: "coder",
        display_name: "Coder",
        description: "Writes and modifies source code: implements features, fixes bugs, \
                      and handles frontend, backend, and mobile development. Deliverables \
                      are working code changes, not plans or reviews. Route here for any \
                      task whose primary output is new or changed source code.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "code", "implement", "build", "bugfix", "feature", "frontend", "backend",
            "mobile", "refactor",
        ],
    },
    AgentTemplate {
        slug: "solution-architect",
        display_name: "Solution Architect",
        description: "Produces architecture decisions, system-design documents, and technical \
                      plans. Evaluates tradeoffs, defines component boundaries, and addresses \
                      technical debt at the system level. Route here when the deliverable is \
                      a design document or architectural recommendation, not code or a PR review.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "architecture",
            "system-design",
            "tradeoff",
            "technical-debt",
            "component",
            "diagram",
            "rfc",
        ],
    },
    AgentTemplate {
        slug: "qa-engineer",
        display_name: "QA Engineer",
        description: "Writes and maintains tests: unit, integration, and end-to-end. \
                      Investigates regressions, measures coverage, and produces quality reports. \
                      Route here when the primary output is test code, a test plan, or a \
                      bug-investigation report — not production code fixes.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "test", "qa", "regression", "coverage", "e2e", "assertion", "test-plan",
        ],
    },
    AgentTemplate {
        slug: "devops-engineer",
        display_name: "DevOps Engineer",
        description: "Manages CI/CD pipelines, infrastructure-as-code, containers, and \
                      deployment automation. Works with Docker, Kubernetes, cloud providers, \
                      and monitoring. Route here for any task touching build pipelines, release \
                      processes, or infrastructure configuration.",
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
        description: "Reviews pull requests and enforces coding standards. Checks for \
                      correctness, readability, security, and convention adherence. Deliverables \
                      are review comments and approval decisions. Route here when you need a \
                      PR reviewed — not for writing code or designing systems.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "review", "pr", "pull-request", "feedback", "standards", "lint", "approve",
        ],
    },
    AgentTemplate {
        slug: "technical-writer",
        display_name: "Technical Writer",
        description: "Creates and maintains documentation: API references, README files, \
                      changelogs, and developer guides. Ensures docs stay in sync with code. \
                      Route here when the primary deliverable is written documentation, not \
                      code or design artifacts.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-flash",
        routing_hints: &["docs", "documentation", "readme", "api-docs", "changelog", "guide"],
    },
    AgentTemplate {
        slug: "data-engineer",
        display_name: "Data Engineer",
        description: "Designs database schemas, writes SQL migrations, and builds data \
                      pipelines and ETL processes. Handles analytics queries and data modelling. \
                      Route here when the work centres on data storage, retrieval, transformation, \
                      or reporting — not application-level code.",
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
                      auth mechanisms and encryption. Ensures compliance and secure coding \
                      practices. Route here when the task has a primary security or compliance \
                      objective — not general code reviews.",
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
    // ── Business & strategy agents ───────────────────────────────────────
    AgentTemplate {
        slug: "market-researcher",
        display_name: "Market Researcher",
        description: "Conducts market research, target audience analysis, and competitor \
                      studies. Defines product use cases, user personas, and feature priorities \
                      based on market data. Route here for competitor benchmarks, audience \
                      segmentation, or positioning strategy — not technical research or UA channels.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "market",
            "competitor",
            "audience",
            "persona",
            "positioning",
            "use-case",
            "benchmark",
        ],
    },
    AgentTemplate {
        slug: "ua-lead",
        display_name: "UA Lead",
        description: "Plans user-acquisition channels, growth tactics, and monetisation \
                      models. Optimises conversion funnels and aligns growth initiatives \
                      with revenue targets. Route here for acquisition strategy, channel \
                      evaluation, or pricing models — not market research or visual design.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "acquisition",
            "growth",
            "channel",
            "monetisation",
            "conversion",
            "retention",
            "funnel",
            "pricing",
        ],
    },
    AgentTemplate {
        slug: "design-lead",
        display_name: "Design Lead",
        description: "Owns visual identity, UI/UX strategy, and design systems. Creates \
                      wireframes, prototypes, and mockups. Ensures brand consistency and \
                      cohesive user experience. Route here for visual design, UX flows, or \
                      brand assets — not system architecture or frontend code.",
        default_provider: "claude-code",
        claude_model: "sonnet",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "ui",
            "ux",
            "visual",
            "brand",
            "wireframe",
            "prototype",
            "mockup",
            "design-system",
        ],
    },
    AgentTemplate {
        slug: "tech-researcher",
        display_name: "Tech Researcher",
        description: "Researches technology stacks, frameworks, and services to find \
                      cost-effective solutions that reduce capital investment. Produces \
                      comparison matrices and build-vs-buy recommendations. Route here for \
                      stack selection or tool evaluation — not for architecture decisions \
                      or hands-on implementation.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "tech-stack",
            "evaluate",
            "cost-analysis",
            "framework",
            "tooling",
            "build-vs-buy",
            "comparison",
        ],
    },
];

// ----------------------------------------------------------------------------
// Orchestrator templates (4 entries)
// ----------------------------------------------------------------------------

pub const ORCHESTRATOR_TEMPLATES: &[AgentTemplate] = &[
    AgentTemplate {
        slug: "project-manager",
        display_name: "Project Manager",
        description: "Coordinates delivery: tracks task progress, manages milestones, and \
                      enforces deadlines. Distributes work based on agent availability and \
                      pending queues. Choose this orchestrator when the focus is planning \
                      and delivery coordination rather than technical or product decisions.",
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
        description: "Provides technical authority: makes architecture decisions, enforces \
                      coding standards, and resolves technical conflicts across the team. \
                      Routes tasks based on technical domain and agent expertise. Choose this \
                      orchestrator when routing requires deep technical judgment.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "architecture",
            "technical",
            "decision",
            "standard",
            "mentor",
            "conflict",
        ],
    },
    AgentTemplate {
        slug: "scrum-master",
        display_name: "Scrum Master",
        description: "Facilitates process: manages sprint backlog, removes blockers, and \
                      tracks velocity. Ensures agents follow the agreed workflow and ceremonies. \
                      Choose this orchestrator when the primary responsibility is process \
                      facilitation and blocker removal rather than direct task decisions.",
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
    AgentTemplate {
        slug: "product-owner",
        display_name: "Product Owner",
        description: "Owns product vision: prioritises backlog by business value, defines \
                      acceptance criteria, and ensures deliverables align with stakeholder \
                      expectations. Routes tasks to maximise product impact across engineering, \
                      design, and business agents. Choose this orchestrator for cross-functional \
                      teams mixing technical and business roles.",
        default_provider: "claude-code",
        claude_model: "opus",
        gemini_model: "gemini-2.5-pro",
        routing_hints: &[
            "prioritise",
            "backlog",
            "stakeholder",
            "requirement",
            "acceptance",
            "vision",
            "impact",
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
