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

    if orchestrators.is_empty() {
        for w in &workers {
            for line in render_agent_box(w, false) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        return out;
    }

    const MAX_ROW_WIDTH: usize = 80;
    const GAP: usize = 2;

    for orch in &orchestrators {
        let mut orch_box = render_agent_box(orch, true);
        let orch_width = visible_len(&orch_box[0]);
        let orch_center = orch_width / 2;

        if !workers.is_empty() {
            // Replace the center char of the bottom border with ┬ to show the stem exit point
            let last = orch_box.len() - 1;
            orch_box[last] = replace_char_at(&orch_box[last], orch_center, '┬');
        }

        for line in &orch_box {
            out.push_str(line);
            out.push('\n');
        }

        if workers.is_empty() {
            continue;
        }

        // Pre-render worker boxes
        let worker_boxes: Vec<Vec<String>> =
            workers.iter().map(|w| render_agent_box(w, false)).collect();

        // Group boxes into rows
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut current_row: Vec<usize> = Vec::new();
        let mut current_width: usize = 0;

        for (idx, box_lines) in worker_boxes.iter().enumerate() {
            let box_width = visible_len(&box_lines[0]);
            let needed = if current_row.is_empty() {
                box_width
            } else {
                current_width + GAP + box_width
            };
            if !current_row.is_empty() && needed > MAX_ROW_WIDTH {
                rows.push(current_row);
                current_row = Vec::new();
            }
            current_row.push(idx);
            current_width = needed;
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        for (row_num, row_indices) in rows.iter().enumerate() {
            // Compute worker midpoints within this row
            let mut x_offset = 0usize;
            let mut worker_mids: Vec<usize> = Vec::new();
            for &idx in row_indices {
                let bw = visible_len(&worker_boxes[idx][0]);
                worker_mids.push(x_offset + bw / 2);
                x_offset += bw + GAP;
            }

            if row_num == 0 {
                // First row: full connector from orchestrator stem to worker midpoints
                for line in render_connector(orch_center, &worker_mids) {
                    out.push_str(&line);
                    out.push('\n');
                }
            } else {
                // Subsequent rows: simple ▼ directly above each worker box
                let canvas = worker_mids.last().copied().unwrap_or(0) + 1;
                let mut arrow_line: Vec<char> = vec![' '; canvas];
                for &m in &worker_mids {
                    arrow_line[m] = '▼';
                }
                out.push_str(&arrow_line.iter().collect::<String>());
                out.push('\n');
            }

            // Worker boxes side by side
            let row_boxes: Vec<&Vec<String>> =
                row_indices.iter().map(|&i| &worker_boxes[i]).collect();
            let max_height = row_boxes.iter().map(|b| b.len()).max().unwrap_or(0);
            for line_idx in 0..max_height {
                let mut row_str = String::new();
                for (col_idx, box_lines) in row_boxes.iter().enumerate() {
                    if col_idx > 0 {
                        row_str.push_str(&" ".repeat(GAP));
                    }
                    if line_idx < box_lines.len() {
                        row_str.push_str(&box_lines[line_idx]);
                    } else {
                        let bw = visible_len(&box_lines[0]);
                        row_str.push_str(&" ".repeat(bw));
                    }
                }
                out.push_str(&row_str);
                out.push('\n');
            }
        }
    }

    out
}

/// Replace the char at visible position `pos` in a plain (no-ANSI) string.
fn replace_char_at(s: &str, pos: usize, replacement: char) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| if i == pos { replacement } else { c })
        .collect()
}

/// Render connector lines from `orch_center` (x-position of orchestrator stem) down
/// to each worker midpoint in `worker_mids`.
///
/// Output lines (no trailing newline):
///   line1: │ at orch_center
///   line2: horizontal bar with proper box-drawing chars
///   line3: │ at each worker_mid
///   line4: ▼ at each worker_mid
fn render_connector(orch_center: usize, worker_mids: &[usize]) -> Vec<String> {
    if worker_mids.is_empty() {
        return vec![];
    }

    let left_most = orch_center.min(*worker_mids.first().unwrap());
    let right_most = orch_center.max(*worker_mids.last().unwrap());
    let canvas = right_most + 1;

    // Special case: single worker exactly at orch_center → straight line down
    if worker_mids.len() == 1 && worker_mids[0] == orch_center {
        let mut l1 = vec![' '; canvas];
        let mut l2 = vec![' '; canvas];
        l1[orch_center] = '│';
        l2[orch_center] = '▼';
        return vec![l1.iter().collect(), l2.iter().collect()];
    }

    // Line 1: │ descending from orchestrator stem
    let mut line1: Vec<char> = vec![' '; canvas];
    line1[orch_center] = '│';

    // Line 2: horizontal bar connecting orch_center to all worker_mids
    let mut line2: Vec<char> = vec![' '; canvas];
    for (i, ch) in line2
        .iter_mut()
        .enumerate()
        .take(right_most + 1)
        .skip(left_most)
    {
        let is_left = i == left_most;
        let is_right = i == right_most;
        let up = i == orch_center;
        let down = worker_mids.contains(&i);
        let left = !is_left;
        let right = !is_right;
        *ch = box_char(up, down, left, right);
    }

    // Line 3: │ at each worker midpoint
    let mut line3: Vec<char> = vec![' '; canvas];
    for &m in worker_mids {
        line3[m] = '│';
    }

    // Line 4: ▼ at each worker midpoint
    let mut line4: Vec<char> = vec![' '; canvas];
    for &m in worker_mids {
        line4[m] = '▼';
    }

    vec![
        line1.iter().collect(),
        line2.iter().collect(),
        line3.iter().collect(),
        line4.iter().collect(),
    ]
}

/// Select the correct box-drawing character given which directions connect.
fn box_char(up: bool, down: bool, left: bool, right: bool) -> char {
    match (up, down, left, right) {
        (false, false, _, _) => '─',
        (false, true, false, true) => '┌',
        (false, true, true, false) => '┐',
        (false, true, true, true) => '┬',
        (false, true, false, false) => '│',
        (true, false, false, true) => '└',
        (true, false, true, false) => '┘',
        (true, false, true, true) => '┴',
        (true, false, false, false) => '│',
        (true, true, false, true) => '├',
        (true, true, true, false) => '┤',
        (true, true, true, true) => '┼',
        (true, true, false, false) => '│',
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(name: &str, tool: &str, role: &str, model: Option<&str>, status: &str) -> Agent {
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
            routing_hints: None,
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
        assert!(
            output.contains("ORCHESTRATOR"),
            "Expected ORCHESTRATOR in output, got:\n{}",
            output
        );
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
        assert!(
            output.contains("my-worker"),
            "Expected agent name my-worker"
        );
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
        assert!(
            count >= 2,
            "Expected at least 2 'tool: ' prefixes, got {}",
            count
        );
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
        assert!(
            output.contains('▼'),
            "Expected ▼ arrow in output with workers"
        );
    }

    #[test]
    fn test_render_diagram_empty_agents_no_panic() {
        let agents: Vec<Agent> = vec![];
        let output = render_diagram(&agents);
        assert!(output.is_empty(), "Expected empty output for no agents");
    }

    #[test]
    fn test_render_diagram_orchestrator_only_no_arrows() {
        let agents = vec![make_agent("orch", "claude", "orchestrator", None, "idle")];
        let output = render_diagram(&agents);
        assert!(output.contains("ORCHESTRATOR"), "Expected ORCHESTRATOR");
        assert!(
            !output.contains('▼'),
            "Expected no ▼ arrows with no workers"
        );
    }

    #[test]
    fn test_render_diagram_model_none_omitted() {
        let agents = vec![make_agent("orch", "claude", "orchestrator", None, "idle")];
        let output = render_diagram(&agents);
        assert!(
            !output.contains("model:"),
            "Expected no 'model:' when model is None"
        );
    }

    #[test]
    fn test_render_diagram_model_some_shown() {
        let agents = vec![make_agent(
            "orch",
            "claude",
            "orchestrator",
            Some("claude-sonnet"),
            "idle",
        )];
        let output = render_diagram(&agents);
        assert!(
            output.contains("model: claude-sonnet"),
            "Expected 'model: claude-sonnet' in output"
        );
    }

    #[test]
    fn test_render_diagram_multirow_visual() {
        let agents = vec![
            make_agent("orch", "claude", "orchestrator", None, "idle"),
            make_agent("w1", "claude", "worker", None, "idle"),
            make_agent("w2", "claude", "worker", None, "dead"),
            make_agent("w3", "gemini", "worker", None, "idle"),
            make_agent("w4-long-name", "claude", "worker", None, "busy"),
            make_agent("w5", "gemini", "worker", None, "dead"),
        ];
        let output = render_diagram(&agents);
        println!("{}", output);
        assert!(output.contains("ORCHESTRATOR"));
    }
}
