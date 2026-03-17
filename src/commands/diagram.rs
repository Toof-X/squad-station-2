use crate::commands::helpers::{colorize_agent_status, pad_colored};
use crate::db::agents::Agent;
use owo_colors::OwoColorize;
use owo_colors::Stream;

/// Print the agent fleet diagram to stdout.
pub fn print_diagram(agents: &[Agent]) {
    print!("{}", render_diagram(agents));
}

/// Build the full diagram as a string (for testing).
pub fn render_diagram(agents: &[Agent]) -> String {
    if agents.is_empty() {
        return String::new();
    }

    let orchestrators: Vec<&Agent> = agents.iter().filter(|a| a.role == "orchestrator").collect();
    let workers: Vec<&Agent> = agents.iter().filter(|a| a.role != "orchestrator").collect();

    let mut out = String::new();
    out.push_str("\nAgent Fleet:\n");

    // Render orchestrator box(es)
    for orch in &orchestrators {
        let box_lines = render_agent_box(orch, true);
        for line in &box_lines {
            out.push_str(line);
            out.push('\n');
        }
    }

    if workers.is_empty() {
        return out;
    }

    // Render workers in rows of up to ~80 chars width, with arrow rows above each row
    const MAX_ROW_WIDTH: usize = 80;
    const GAP: usize = 2;

    // Pre-render all worker boxes
    let worker_boxes: Vec<Vec<String>> = workers.iter().map(|w| render_agent_box(w, false)).collect();

    // Group boxes into rows
    let mut rows: Vec<Vec<&Vec<String>>> = Vec::new();
    let mut current_row: Vec<&Vec<String>> = Vec::new();
    let mut current_width: usize = 0;

    for box_lines in &worker_boxes {
        let box_width = box_lines.first().map(|l| visible_len(l)).unwrap_or(0);
        let needed = if current_row.is_empty() {
            box_width
        } else {
            current_width + GAP + box_width
        };

        if !current_row.is_empty() && needed > MAX_ROW_WIDTH {
            rows.push(current_row);
            current_row = Vec::new();
            current_width = 0;
        }

        current_row.push(box_lines);
        current_width = if current_row.len() == 1 {
            box_width
        } else {
            current_width + GAP + box_width
        };
    }
    if !current_row.is_empty() {
        rows.push(current_row);
    }

    for row in &rows {
        // Arrow rows
        let arrow_lines = render_arrow_row(row, GAP);
        for line in &arrow_lines {
            out.push_str(line);
            out.push('\n');
        }

        // Worker boxes side by side
        let max_height = row.iter().map(|b| b.len()).max().unwrap_or(0);
        for line_idx in 0..max_height {
            let mut row_str = String::new();
            for (col_idx, box_lines) in row.iter().enumerate() {
                if col_idx > 0 {
                    row_str.push_str(&" ".repeat(GAP));
                }
                if line_idx < box_lines.len() {
                    row_str.push_str(&box_lines[line_idx]);
                } else {
                    // Pad with spaces to maintain alignment
                    let box_width = box_lines.first().map(|l| visible_len(l)).unwrap_or(0);
                    row_str.push_str(&" ".repeat(box_width));
                }
            }
            out.push_str(&row_str);
            out.push('\n');
        }
    }

    out
}

/// Compute the visible (non-ANSI) length of a string.
fn visible_len(s: &str) -> usize {
    // Strip ANSI escape codes for width measurement
    let mut len = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Skip ANSI escape sequence
            i += 2;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            i += 1; // skip 'm'
        } else {
            // Count UTF-8 multibyte characters as single visible chars
            if bytes[i] & 0x80 == 0 {
                len += 1;
                i += 1;
            } else if bytes[i] & 0xE0 == 0xC0 {
                len += 1;
                i += 2;
            } else if bytes[i] & 0xF0 == 0xE0 {
                len += 1;
                i += 3;
            } else if bytes[i] & 0xF8 == 0xF0 {
                len += 1;
                i += 4;
            } else {
                i += 1;
            }
        }
    }
    len
}

/// Build content lines for a box (plain + colored versions).
fn build_content_lines(agent: &Agent, is_orchestrator: bool) -> Vec<(String, String)> {
    let mut lines: Vec<(String, String)> = Vec::new();

    if is_orchestrator {
        let raw = "ORCHESTRATOR".to_string();
        let colored = format!(
            "{}",
            "ORCHESTRATOR".if_supports_color(Stream::Stdout, |s| s.bold())
        );
        lines.push((raw, colored));
    }

    // Agent name
    lines.push((agent.name.clone(), agent.name.clone()));

    // Tool + optional model
    let tool_model_raw = match &agent.model {
        Some(m) if !m.is_empty() => format!("tool: {}  model: {}", agent.tool, m),
        _ => format!("tool: {}", agent.tool),
    };
    lines.push((tool_model_raw.clone(), tool_model_raw));

    // Status badge
    let raw_status = format!("[{}]", agent.status);
    let colored_status = format!("[{}]", colorize_agent_status(&agent.status));
    lines.push((raw_status, colored_status));

    lines
}

/// Render an agent as a box. Returns a Vec of strings (one per row).
fn render_agent_box(agent: &Agent, is_orchestrator: bool) -> Vec<String> {
    let content = build_content_lines(agent, is_orchestrator);

    // Box width based on max raw content width
    let max_raw_len = content.iter().map(|(raw, _)| raw.len()).max().unwrap_or(0);
    let inner_width = max_raw_len;
    let box_width = inner_width + 4; // "│ " + content + " │"

    let mut lines = Vec::new();

    // Top border
    lines.push(format!("┌{}┐", "─".repeat(box_width - 2)));

    // Content lines
    for (raw, colored) in &content {
        lines.push(format!("│ {} │", pad_colored(raw, colored, inner_width)));
    }

    // Bottom border
    lines.push(format!("└{}┘", "─".repeat(box_width - 2)));

    lines
}

/// Render the arrow rows connecting orchestrator to a row of workers.
fn render_arrow_row(worker_boxes: &[&Vec<String>], gap: usize) -> Vec<String> {
    // Compute the x-offset midpoint for each box within the combined row
    let mut midpoints: Vec<usize> = Vec::new();
    let mut x_offset = 0;
    for box_lines in worker_boxes {
        let box_width = box_lines.first().map(|l| visible_len(l)).unwrap_or(0);
        midpoints.push(x_offset + box_width / 2);
        x_offset += box_width + gap;
    }

    let total_width = x_offset.saturating_sub(gap);

    // Line 1: │ at each midpoint
    let mut line1 = vec![b' '; total_width];
    // Line 2: ▼ at each midpoint
    let mut line2 = vec![b' '; total_width];

    for &mid in &midpoints {
        if mid < line1.len() {
            line1[mid] = b'|';
            line2[mid] = b'V';
        }
    }

    // Convert to strings with Unicode chars substituted
    let s1: String = line1
        .iter()
        .map(|&c| if c == b'|' { '│' } else { ' ' })
        .collect();
    let s2: String = line2
        .iter()
        .map(|&c| if c == b'V' { '▼' } else { ' ' })
        .collect();

    vec![s1, s2]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(
        name: &str,
        tool: &str,
        role: &str,
        model: Option<&str>,
        status: &str,
    ) -> Agent {
        Agent {
            id: "test".to_string(),
            name: name.to_string(),
            tool: tool.to_string(),
            role: role.to_string(),
            command: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            status: status.to_string(),
            status_updated_at: "2026-01-01T00:00:00Z".to_string(),
            model: model.map(|s| s.to_string()),
            description: None,
            current_task: None,
        }
    }

    #[test]
    fn test_render_diagram_contains_orchestrator_label() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("worker1", "claude", "worker", None, "idle"),
            make_agent("worker2", "claude", "worker", None, "busy"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains("ORCHESTRATOR"), "Expected ORCHESTRATOR in output, got:\n{}", output);
    }

    #[test]
    fn test_render_diagram_contains_box_drawing_chars() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("worker1", "claude", "worker", None, "idle"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains('┌'), "Missing ┌");
        assert!(output.contains('─'), "Missing ─");
        assert!(output.contains('┐'), "Missing ┐");
        assert!(output.contains('│'), "Missing │");
        assert!(output.contains('└'), "Missing └");
        assert!(output.contains('┘'), "Missing ┘");
    }

    #[test]
    fn test_render_diagram_contains_agent_names() {
        let agents = vec![
            make_agent("my-orch", "claude", "orchestrator", None, "idle"),
            make_agent("my-worker", "gemini", "worker", None, "busy"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains("my-orch"), "Expected agent name my-orch");
        assert!(output.contains("my-worker"), "Expected agent name my-worker");
    }

    #[test]
    fn test_render_diagram_contains_tool_prefix() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("worker1", "gemini", "worker", None, "idle"),
        ];
        let output = render_diagram(&agents);
        // Both agents have "tool: " prefix
        let count = output.matches("tool: ").count();
        assert!(count >= 2, "Expected at least 2 'tool: ' prefixes, got {}", count);
    }

    #[test]
    fn test_render_diagram_contains_status_badges() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("w1", "claude", "worker", None, "busy"),
            make_agent("w2", "claude", "worker", None, "dead"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains("[idle]"), "Expected [idle] in output");
        assert!(output.contains("[busy]"), "Expected [busy] in output");
        assert!(output.contains("[dead]"), "Expected [dead] in output");
    }

    #[test]
    fn test_render_diagram_contains_arrow_when_workers_exist() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("worker1", "claude", "worker", None, "idle"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains('▼'), "Expected ▼ arrow in output with workers");
    }

    #[test]
    fn test_render_diagram_empty_agents_no_panic() {
        let agents: Vec<Agent> = vec![];
        let output = render_diagram(&agents);
        assert!(output.is_empty(), "Expected empty output for no agents");
    }

    #[test]
    fn test_render_diagram_orchestrator_only_no_arrows() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains("ORCHESTRATOR"), "Expected ORCHESTRATOR");
        assert!(!output.contains('▼'), "Expected no ▼ arrows with no workers");
    }

    #[test]
    fn test_render_diagram_model_none_omitted() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
        ];
        let output = render_diagram(&agents);
        assert!(!output.contains("model:"), "Expected no 'model:' when model is None");
    }

    #[test]
    fn test_render_diagram_model_some_shown() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", Some("claude-sonnet"), "idle"),
        ];
        let output = render_diagram(&agents);
        assert!(output.contains("model: claude-sonnet"), "Expected 'model: claude-sonnet' in output");
    }
}
