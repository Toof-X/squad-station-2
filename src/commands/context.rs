use crate::config::SddConfig;
use crate::db::agents::Agent;
use crate::{config, db};
use serde_json;

// ── Fleet Status types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentResult {
    Ok,                                                     // overlap found — checkmark
    Warning { task_preview: String, role: String },         // zero overlap — warning
    None,                                                   // no task or no description
}

#[derive(Debug, Clone)]
pub struct AgentMetrics {
    pub agent_name: String,
    pub pending_count: i64,
    pub busy_for: String,       // "idle", "<1m", "5m", "1h 30m", "2d 4h"
    pub alignment: AlignmentResult,
}

/// Returns how long an agent has been busy as a human-readable string.
/// Returns "idle" if status is not "busy". Returns "<1m", "Xm", "Xh Ym", or "Xd Yh".
pub fn format_busy_duration(status: &str, status_updated_at: &str) -> String {
    if status != "busy" {
        return "idle".to_string();
    }
    match chrono::DateTime::parse_from_rfc3339(status_updated_at) {
        Ok(t) => {
            let dur = chrono::Utc::now().signed_duration_since(t);
            let total_mins = dur.num_minutes();
            let total_hours = dur.num_hours();
            let days = total_hours / 24;
            let hours = total_hours % 24;
            let mins = total_mins % 60;
            if total_mins < 1 {
                "<1m".to_string()
            } else if total_hours < 1 {
                format!("{}m", total_mins)
            } else if days < 1 {
                format!("{}h {}m", total_hours, mins)
            } else {
                format!("{}d {}h", days, hours)
            }
        }
        Err(_) => "?".to_string(),
    }
}

/// Compute keyword overlap between a task body and an agent's description.
/// Returns AlignmentResult::Ok if any non-stop-word tokens overlap,
/// Warning with preview/role if zero overlap, or None if task or description is missing.
pub fn compute_alignment(task_body: &str, description: Option<&str>) -> AlignmentResult {
    if task_body.is_empty() {
        return AlignmentResult::None;
    }
    let desc = match description {
        Some(d) if !d.is_empty() => d,
        _ => return AlignmentResult::None,
    };

    const STOP_WORDS: &[&str] = &[
        "the", "a", "an", "and", "to", "for", "in", "of", "is", "on",
        "it", "with", "as", "at", "by", "from", "that", "this",
    ];

    let tokenize = |s: &str| -> std::collections::HashSet<String> {
        s.split_whitespace()
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| !w.is_empty() && !STOP_WORDS.contains(&w.as_str()))
            .collect()
    };

    let task_tokens = tokenize(task_body);
    let desc_tokens = tokenize(desc);
    let overlap: usize = task_tokens.intersection(&desc_tokens).count();

    if overlap > 0 {
        AlignmentResult::Ok
    } else {
        // Truncate task body to ~30 chars for preview
        let preview: String = task_body.chars().take(30).collect();
        let task_preview = if task_body.len() > 30 {
            format!("{}...", preview.trim_end())
        } else {
            preview
        };
        // Extract role from first few words of description
        let role: String = desc
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        AlignmentResult::Warning { task_preview, role }
    }
}

pub fn build_orchestrator_md(
    agents: &[Agent],
    project_root: &str,
    sdd_configs: &[SddConfig],
    metrics: &[AgentMetrics],
) -> String {
    let mut out = String::new();

    // Collect worker agents
    let workers: Vec<&Agent> = agents.iter().filter(|a| a.role != "orchestrator").collect();

    // ── Role ─────────────────────────────────────────────────────────────
    out.push_str("You are the orchestrator. You DO NOT directly write code, modify files, or run workflows.\n");
    out.push_str("You COORDINATE agents on behalf of the user via `squad-station send`.\n\n");

    // ── PRE-FLIGHT ───────────────────────────────────────────────────────
    out.push_str("## PRE-FLIGHT — Execute IMMEDIATELY before any task\n\n");
    if !sdd_configs.is_empty() {
        out.push_str("> Read and fully internalize the SDD playbook(s). Do not skip or skim.\n\n");
        for sdd in sdd_configs {
            out.push_str(&format!("- [ ] Read `{}`\n", sdd.playbook));
        }
        out.push_str("\n");
        out.push_str("Only proceed after reading. The playbook defines your workflow.\n\n");
    }
    out.push_str(&format!("- [ ] Project root: `{}`\n", project_root));
    out.push_str("- [ ] Verify agents are alive: `squad-station agents`\n\n");

    // ── Completion Notification ──────────────────────────────────────────
    out.push_str("## Completion Notification (Automatic)\n\n");
    out.push_str("Agents have a stop hook configured. When an agent completes a task, the hook\n");
    out.push_str(
        "**automatically sends a signal** back to your session. You **DO NOT need to**:\n",
    );
    out.push_str("- Continuously poll `tmux capture-pane` to track progress.\n");
    out.push_str("- Run `sleep`, `squad-station list`, or `squad-station agents` in a loop.\n");
    out.push_str("- Use the `Agent` tool to spawn subagents.\n\n");
    out.push_str("After assigning a task, **stop and wait for the signal**:\n\n");
    out.push_str("```\n");
    out.push_str("[SQUAD SIGNAL] Agent '<name>' completed task <id>. Read output: tmux capture-pane -t <name> -p | Next: squad-station status\n");
    out.push_str("```\n\n");
    out.push_str("Only proactively check (`capture-pane`) if you suspect the agent is stuck for an unusually long time.\n\n");

    // ── Fleet Status ──────────────────────────────────────────────────────
    // Only render if metrics were provided (caller fetched from DB)
    let fleet_metrics: Vec<&AgentMetrics> = metrics
        .iter()
        .filter(|m| {
            // Exclude orchestrator and dead agents
            agents.iter().any(|a| {
                a.name == m.agent_name && a.role != "orchestrator" && a.status != "dead"
            })
        })
        .collect();

    if !fleet_metrics.is_empty() {
        out.push_str("## Fleet Status\n\n");
        out.push_str("| Agent | Pending | Busy For | Alignment |\n");
        out.push_str("|-------|---------|----------|-----------|\n");
        for m in &fleet_metrics {
            let alignment_str = match &m.alignment {
                AlignmentResult::Ok => "\u{2705}".to_string(),
                AlignmentResult::Warning { task_preview, role } => {
                    format!(
                        "\u{26a0}\u{fe0f} '{}' \u{2192} {}",
                        task_preview, role
                    )
                }
                AlignmentResult::None => "\u{2014}".to_string(),
            };
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                m.agent_name, m.pending_count, m.busy_for, alignment_str
            ));
        }
        out.push_str("\n");

        // Routing hints
        out.push_str("- Prefer agents with 0 pending tasks\n");
        out.push_str(
            "- \u{26a0}\u{fe0f} alignment = task may be misrouted \u{2014} verify before sending\n",
        );
        out.push_str("- Re-query if this context is >5 minutes old\n\n");

        // Re-query commands blockquote (INTEL-04)
        out.push_str("> **Live re-query commands:**\n");
        out.push_str("> ```\n");
        out.push_str("> squad-station agents          # agent status + busy duration\n");
        out.push_str("> squad-station list --status processing  # pending queue\n");
        out.push_str("> squad-station status          # fleet overview\n");
        out.push_str("> squad-station context         # regenerate this file\n");
        out.push_str("> ```\n\n");
    }

    // ── Context Management ─────────────────────────────────────────────
    out.push_str("## Context Management — `/clear`\n\n");
    out.push_str("You MUST send `/clear` to an agent BEFORE dispatching a new task if ANY of these conditions are true:\n\n");
    out.push_str("### Mandatory `/clear` Triggers\n\n");
    out.push_str("1. **Topic shift** — The new task is on a DIFFERENT topic/feature than the agent's last completed task.\n");
    out.push_str("   Examples: bug fix → new feature, UI work → backend work, different file areas.\n\n");
    out.push_str("2. **Task count threshold** — The agent has completed 3 or more consecutive tasks without a `/clear`.\n");
    out.push_str("   Count resets after each `/clear`.\n\n");
    out.push_str("3. **Agent hint** — The agent's output mentions context issues, suggests clearing,\n");
    out.push_str("   or shows signs of confusion (referencing old/irrelevant code).\n\n");
    out.push_str("### `/clear` Checklist (run BEFORE every `squad-station send`)\n\n");
    out.push_str("□ Is this a topic shift from the agent's last task? → /clear\n");
    out.push_str("□ Has the agent done 3+ tasks since last /clear? → /clear\n");
    out.push_str("□ Did the agent hint at context issues? → /clear\n");
    out.push_str("□ None of the above? → send task directly (no /clear needed)\n\n");
    out.push_str("### How to `/clear`\n\n");
    out.push_str("```bash\nsquad-station send <agent-name> --body \"/clear\"\n```\n\n");
    out.push_str("After `/clear`, the agent has ZERO memory. You MUST re-inject enough context\n");
    out.push_str("in the next task body so the agent can execute independently.\n\n");

    // ── Session Routing ──────────────────────────────────────────────────
    out.push_str("## Session Routing\n\n");
    out.push_str("Based on the nature of the work, independently decide the correct agent:\n\n");
    for agent in &workers {
        let desc = agent.description.as_deref().unwrap_or(&agent.role);
        let model = agent.model.as_deref().unwrap_or(&agent.tool);
        out.push_str(&format!("- **{}** ({}) — {}\n", agent.name, model, desc));
    }
    out.push_str("\n**Routing rules:**\n");
    out.push_str("- Reasoning, architecture, planning, review → brainstorm/planning agent\n");
    out.push_str("- Coding, implement, fix, build, deploy → implementation agent\n");
    out.push_str("- **Parallel** only when tasks are independent. **Sequential** when one output feeds another.\n\n");

    // -- Routing Matrix --
    out.push_str("## Routing Matrix\n\n");
    let hinted_agents: Vec<(&Agent, Vec<String>)> = agents
        .iter()
        .filter(|a| a.role != "orchestrator")
        .filter_map(|a| {
            a.routing_hints.as_ref().and_then(|h| {
                serde_json::from_str::<Vec<String>>(h).ok().map(|kws| (a, kws))
            })
        })
        .collect();

    if hinted_agents.is_empty() {
        out.push_str("No routing hints configured — use templates during init for keyword-based routing\n\n");
    } else {
        out.push_str("| Keyword | Route to |\n");
        out.push_str("|---------|----------|\n");
        for (agent, keywords) in &hinted_agents {
            for kw in keywords {
                out.push_str(&format!("| {} | {} |\n", kw, agent.name));
            }
        }
        out.push_str("\n");
    }

    // ── SDD Orchestration ────────────────────────────────────────────────
    if !sdd_configs.is_empty() {
        out.push_str("## SDD Orchestration\n\n");
        out.push_str("The agents have SDD tools (slash commands, workflows) installed in their sessions. **You do NOT.**\n");
        out.push_str("Your job is to send the playbook's commands to the correct agent. Do not run them yourself.\n\n");
        out.push_str("**How it works:**\n");
        out.push_str("1. Read the playbook (PRE-FLIGHT) → identify the workflow steps and their slash commands\n");
        out.push_str("2. For each step: decide which agent handles it (see Session Routing)\n");
        out.push_str("3. Send the slash command as the task body:\n");
        out.push_str("   ```\n");
        if let Some(first_worker) = workers.first() {
            out.push_str(&format!(
                "   squad-station send {} --body \"/command-name\"\n",
                first_worker.name
            ));
        }
        out.push_str("   ```\n");
        out.push_str("4. STOP. Wait for `[SQUAD SIGNAL]`.\n");
        out.push_str("5. Read output → evaluate → send next step to the appropriate agent.\n\n");
        out.push_str("**CRITICAL:**\n");
        out.push_str("- Do NOT send raw task descriptions like \"build the login page\".\n");
        out.push_str("- Do NOT run slash commands, workflows, or Agent subagents yourself.\n");
        out.push_str(
            "- Send the playbook's exact commands. The agent knows how to execute them.\n\n",
        );
    }

    // ── Sending Tasks ────────────────────────────────────────────────────
    out.push_str("## Sending Tasks\n\n");
    out.push_str("```bash\n");
    for agent in &workers {
        out.push_str(&format!(
            "squad-station send {} --body \"<command or task>\"\n",
            agent.name
        ));
    }
    out.push_str("```\n\n");

    // ── Full Context Transfer ────────────────────────────────────────────
    out.push_str("## Full Context Transfer\n\n");
    out.push_str("When transferring results from one agent to another:\n");
    out.push_str("- Capture ENTIRE output: `tmux capture-pane -t <agent> -p -S -`\n");
    out.push_str("- Include complete context in the next task body.\n");
    out.push_str("- **Self-check:** \"If the target agent had NO other context, could it execute correctly?\" If NO → add more.\n\n");

    // ── Workflow Completion Discipline ────────────────────────────────────
    out.push_str("## Workflow Completion Discipline\n\n");
    out.push_str("- **NEVER** interrupt a running agent to move on.\n");
    out.push_str("- **WAIT** for the `[SQUAD SIGNAL]` before evaluating results.\n");
    out.push_str("- Only after the signal → read output → decide next step per playbook.\n\n");

    // ── QA Gate ──────────────────────────────────────────────────────────
    out.push_str("## QA Gate\n\n");
    out.push_str("After receiving `[SQUAD SIGNAL]`:\n");
    out.push_str("1. `tmux capture-pane -t <agent> -p -S -` — read full output\n");
    out.push_str(
        "2. If agent reported errors or asked technical questions → answer via follow-up task\n",
    );
    out.push_str("3. If agent asked business/requirements questions → forward to user (HITL)\n");
    out.push_str("4. `squad-station list --agent <agent>` — confirm status is `completed`\n");
    out.push_str("5. Run the `/clear` checklist (see Context Management) — if ANY condition matches,\n");
    out.push_str("   send `/clear` to the agent BEFORE dispatching the next task.\n");
    out.push_str(
        "6. Proceed to next playbook step, or report to user if workflow is complete.\n\n",
    );

    // ── Agent Roster ─────────────────────────────────────────────────────
    out.push_str("## Agent Roster\n\n");
    out.push_str("| Agent | Model | Role | Description |\n");
    out.push_str("|-------|-------|------|-------------|\n");
    for agent in agents {
        let model = agent.model.as_deref().unwrap_or("\u{2014}");
        let desc = agent.description.as_deref().unwrap_or("\u{2014}");
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            agent.name, model, agent.role, desc
        ));
    }

    out
}

/// Detect the current tmux session name. Returns None if not in tmux.
pub fn detect_tmux_session() -> Option<String> {
    std::process::Command::new("tmux")
        .args(["display-message", "-p", "#S"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let name = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            } else {
                None
            }
        })
}

/// Format orchestrator content for stdout injection based on provider.
/// Claude Code: raw markdown. Gemini CLI: JSON with hookSpecificOutput.additionalContext.
pub fn format_inject_output(provider: &str, content: &str) -> String {
    match provider {
        "gemini-cli" => {
            let json = serde_json::json!({
                "hookSpecificOutput": {
                    "additionalContext": content
                }
            });
            serde_json::to_string(&json).unwrap_or_default()
        }
        _ => content.to_string(),
    }
}

pub async fn run(inject: bool) -> anyhow::Result<()> {
    let project_root = config::find_project_root()?;
    let config = config::load_config(&project_root.join("squad.yml"))?;

    if inject {
        return run_inject(&project_root, &config).await;
    }

    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    let agents = db::agents::list_agents(&pool).await?;

    // Build per-agent metrics for Fleet Status (INTEL-01 through INTEL-03)
    let mut metrics = Vec::new();
    for agent in &agents {
        // Skip orchestrator and dead agents — they won't render anyway,
        // but skipping avoids unnecessary DB queries
        if agent.role == "orchestrator" || agent.status == "dead" {
            continue;
        }

        // INTEL-01: Pending message count
        let pending_count = db::messages::count_processing(&pool, &agent.name).await?;

        // INTEL-02: Busy duration
        let busy_for = format_busy_duration(&agent.status, &agent.status_updated_at);

        // INTEL-03: Task-role alignment from most recent processing message
        let alignment = match db::messages::peek_message(&pool, &agent.name).await? {
            Some(msg) => compute_alignment(&msg.task, agent.description.as_deref()),
            None => AlignmentResult::None,
        };

        metrics.push(AgentMetrics {
            agent_name: agent.name.clone(),
            pending_count,
            busy_for,
            alignment,
        });
    }

    let project_root_str = project_root.to_string_lossy().to_string();
    let sdd_configs = config.sdd.as_deref().unwrap_or(&[]);
    // INTEL-05: metrics passed as parameter — build_orchestrator_md remains pure
    let prompt_content = build_orchestrator_md(&agents, &project_root_str, sdd_configs, &metrics);

    // Write slash command in provider-specific format and directory
    let (cmd_subdir, filename, file_content) = match config.orchestrator.provider.as_str() {
        "gemini-cli" => {
            // Gemini CLI: TOML format with description + prompt fields
            let toml = format!(
                "description = \"Squad Station orchestrator — coordinate AI agent squads\"\n\
                 prompt = \"\"\"\n{}\n\"\"\"",
                prompt_content
            );
            (".gemini/commands", "squad-orchestrator.toml", toml)
        }
        _ => {
            // Claude Code: plain markdown
            (".claude/commands", "squad-orchestrator.md", prompt_content)
        }
    };
    let cmd_dir = project_root.join(cmd_subdir);
    std::fs::create_dir_all(&cmd_dir)?;
    let context_path = cmd_dir.join(filename);
    std::fs::write(&context_path, &file_content)?;

    println!("Generated {}", context_path.display());
    Ok(())
}

/// Hook injection mode: output orchestrator context to stdout for SessionStart hooks.
/// Injects if squad.yml exists. When in tmux, skips worker sessions (only orchestrator gets context).
async fn run_inject(
    project_root: &std::path::Path,
    config: &config::SquadConfig,
) -> anyhow::Result<()> {
    // Only inject in orchestrator session — workers must not get orchestrator context.
    if let Some(session_name) = detect_tmux_session() {
        let orch_role = config
            .orchestrator
            .name
            .as_deref()
            .unwrap_or("orchestrator");
        let orch_name =
            config::sanitize_session_name(&format!("{}-{}", config.project, orch_role));
        if session_name != orch_name {
            return Ok(()); // Not the orchestrator — silent exit
        }
    } else {
        return Ok(()); // Not in tmux — silent exit
    }

    // Generate content
    let db_path = config::resolve_db_path(config)?;
    let pool = db::connect(&db_path).await?;
    let agents = db::agents::list_agents(&pool).await?;

    let project_root_str = project_root.to_string_lossy().to_string();
    let sdd_configs = config.sdd.as_deref().unwrap_or(&[]);
    let content = build_orchestrator_md(&agents, &project_root_str, sdd_configs, &[]);

    // Output in provider-appropriate format.
    // Use the current session's provider (not orchestrator's) so each tool gets its format.
    let provider = match detect_tmux_session() {
        Some(session_name) => {
            agents
                .iter()
                .find(|a| a.name == session_name)
                .map(|a| a.tool.clone())
                .unwrap_or_else(|| config.orchestrator.provider.clone())
        }
        None => config.orchestrator.provider.clone(),
    };
    print!("{}", format_inject_output(&provider, &content));
    Ok(())
}
