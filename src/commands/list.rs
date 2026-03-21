use crate::{config, db};
use owo_colors::OwoColorize;
use owo_colors::Stream;

pub async fn run(
    agent: Option<String>,
    status: Option<String>,
    limit: u32,
    json: bool,
) -> anyhow::Result<()> {
    // 1. Resolve DB path
    let config = config::load_config(std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE))?;
    let db_path = config::resolve_db_path(&config)?;

    // 2. Connect to DB
    let pool = db::connect(&db_path).await?;

    // 3. Query messages with filters
    let messages =
        db::messages::list_messages(&pool, agent.as_deref(), status.as_deref(), limit).await?;

    if json {
        // JSON mode: serialize full messages array
        println!("{}", serde_json::to_string_pretty(&messages)?);
        return Ok(());
    }

    // Table mode
    if messages.is_empty() {
        println!("No messages found.");
        return Ok(());
    }

    // Column widths: ID=8, AGENT=15, STATUS=10, PRIORITY=8, TASK=42, CREATED=10
    print_table_header();
    for msg in &messages {
        print_table_row(msg);
    }

    Ok(())
}

fn print_table_header() {
    println!(
        "{:<8}  {:<15}  {:<15}  {:<10}  {:<8}  {:<42}  {:<10}",
        "ID", "FROM", "TO", "STATUS", "PRIORITY", "TASK", "CREATED"
    );
}

fn print_table_row(msg: &db::messages::Message) {
    // ID: first 8 chars of UUID
    let id_short = if msg.id.len() >= 8 {
        &msg.id[..8]
    } else {
        &msg.id
    };

    // TASK: truncate to 40 chars with '...' suffix if longer
    let task_display = if msg.task.len() > 40 {
        format!("{}...", &msg.task[..40])
    } else {
        msg.task.clone()
    };

    // CREATED: extract date portion from RFC3339 timestamp (first 10 chars = YYYY-MM-DD)
    let created_display = if msg.created_at.len() >= 10 {
        &msg.created_at[..10]
    } else {
        &msg.created_at
    };

    // FROM / TO columns: use directional fields, fall back to agent_name for legacy rows
    let from_display = msg.from_agent.as_deref().unwrap_or("-");
    let to_display = msg.to_agent.as_deref().unwrap_or(msg.agent_name.as_str());

    // STATUS: colorize when terminal supports it.
    // ANSI codes add invisible bytes so we pad the raw status text manually,
    // then append the colored string (without fmt padding, which would count escape bytes).
    let status_raw = &msg.status;
    let status_colored = colorize_status(status_raw);
    // Pad the raw text to STATUS width (10), then replace raw text with colored text
    let status_cell = pad_colored(status_raw, &status_colored, 10);

    println!(
        "{:<8}  {:<15}  {:<15}  {}  {:<8}  {:<42}  {:<10}",
        id_short,
        from_display,
        to_display,
        status_cell,
        msg.priority,
        task_display,
        created_display,
    );
}

/// Build a padded cell where visible width is based on `raw` length but output uses `colored`.
fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = width.saturating_sub(raw_len);
    format!("{}{}", colored, " ".repeat(padding))
}

fn colorize_status(status: &str) -> String {
    match status {
        "processing" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.yellow())
        ),
        "pending" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.yellow())
        ),
        "completed" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.green())
        ),
        "failed" => format!("{}", status.if_supports_color(Stream::Stdout, |s| s.red())),
        _ => status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_colored_adds_spaces() {
        let result = pad_colored("hi", "hi", 10);
        assert_eq!(result, "hi        ");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_pad_colored_no_padding_when_exact() {
        let result = pad_colored("hello", "hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_pad_colored_no_padding_when_longer() {
        let result = pad_colored("toolong", "toolong", 3);
        assert_eq!(result, "toolong");
    }

    #[test]
    fn test_pad_colored_with_ansi_codes() {
        // Colored string is longer due to ANSI escape codes,
        // but padding is based on raw string length
        let raw = "ok";
        let colored = "\x1b[32mok\x1b[0m"; // green "ok"
        let result = pad_colored(raw, colored, 6);
        // Should have 4 spaces of padding based on raw length (2), not colored length
        assert!(result.starts_with("\x1b[32mok\x1b[0m"));
        assert!(result.ends_with("    "));
    }

    #[test]
    fn test_colorize_status_processing() {
        let result = colorize_status("processing");
        assert!(result.contains("processing"));
    }

    #[test]
    fn test_colorize_status_pending() {
        let result = colorize_status("pending");
        assert!(result.contains("pending"));
    }

    #[test]
    fn test_colorize_status_completed() {
        let result = colorize_status("completed");
        assert!(result.contains("completed"));
    }

    #[test]
    fn test_colorize_status_failed() {
        let result = colorize_status("failed");
        assert!(result.contains("failed"));
    }

    #[test]
    fn test_colorize_status_unknown() {
        let result = colorize_status("something_else");
        assert_eq!(result, "something_else");
    }
}
