use crate::{config, db};
use crate::db::agents::Agent;

fn build_delegate_md(agents: &[Agent]) -> String {
    let mut out = String::new();

    out.push_str("# Squad Delegation Workflow\n\n");
    out.push_str("> BEHAVIORAL RULE: You are an orchestrator. Do not implement tasks yourself.\n");
    out.push_str("> Delegate to agents using `squad-station send`. Poll for completion.\n");
    out.push_str("> These rules survive context compression — re-read this file if context resets.\n\n");
    out.push_str("## Registered Agents\n\n");

    for agent in agents {
        if agent.role == "orchestrator" {
            continue;
        }
        let display_model = agent
            .model
            .as_deref()
            .unwrap_or(&agent.tool);
        out.push_str(&format!("### {} ({})\n", agent.name, display_model));
        if let Some(ref desc) = agent.description {
            out.push_str(&format!("{}\n", desc));
        }
        out.push_str(&format!("Role: {}\n\n", agent.role));
        out.push_str("```\n");
        out.push_str(&format!("squad-station send {} --body \"...\"\n", agent.name));
        out.push_str(&format!("tmux capture-pane -t {} -p\n", agent.name));
        out.push_str("```\n\n");
    }

    out.push_str("## How to Delegate\n\n");
    out.push_str("1. Select agent based on task type\n");
    out.push_str("2. `squad-station send <agent> --body \"<task>\"`\n");
    out.push_str("3. Poll: `squad-station agents` to check status\n");
    out.push_str("4. Read output: `tmux capture-pane -t <agent> -p`\n");
    out.push_str("5. Completion: `squad-station list --agent <agent>` to verify\n");

    out
}

fn build_monitor_md() -> String {
    let mut out = String::new();

    out.push_str("# Squad Monitor Workflow\n\n");
    out.push_str("> BEHAVIORAL RULE: Poll, don't push. Agents signal DB on completion.\n");
    out.push_str("> You do not receive push notifications if using antigravity provider.\n\n");
    out.push_str("## How to Poll\n\n");
    out.push_str("Check all agent statuses:\n");
    out.push_str("```\n");
    out.push_str("squad-station agents\n");
    out.push_str("```\n\n");
    out.push_str("Check pending/completed messages:\n");
    out.push_str("```\n");
    out.push_str("squad-station list --limit 20\n");
    out.push_str("```\n\n");
    out.push_str("Read agent output:\n");
    out.push_str("```\n");
    out.push_str("tmux capture-pane -t <agent-name> -p\n");
    out.push_str("```\n\n");
    out.push_str("## Anti-Context-Decay Rules\n\n");
    out.push_str("- If you lose context, re-read `.agent/workflows/` files\n");
    out.push_str("- Check `squad-station status` for current state\n");
    out.push_str("- Never assume a task is done — verify with `squad-station list`\n");
    out.push_str("- Re-read `.agent/workflows/squad-roster.md` to confirm agent names\n");

    out
}

fn build_roster_md(agents: &[Agent]) -> String {
    let mut out = String::new();

    out.push_str("# Squad Roster\n\n");
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

pub async fn run() -> anyhow::Result<()> {
    // 1. Connect to DB
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch agents — read-only, no tmux reconciliation
    let agents = db::agents::list_agents(&pool).await?;

    // 3. Create directory
    std::fs::create_dir_all(".agent/workflows")?;

    // 4. Write squad-delegate.md
    let delegate_content = build_delegate_md(&agents);
    std::fs::write(".agent/workflows/squad-delegate.md", delegate_content)?;

    // 5. Write squad-monitor.md
    let monitor_content = build_monitor_md();
    std::fs::write(".agent/workflows/squad-monitor.md", monitor_content)?;

    // 6. Write squad-roster.md
    let roster_content = build_roster_md(&agents);
    std::fs::write(".agent/workflows/squad-roster.md", roster_content)?;

    // 7. Print summary
    println!("Generated .agent/workflows/ (3 files)");

    Ok(())
}
