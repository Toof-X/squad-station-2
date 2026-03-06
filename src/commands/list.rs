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
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
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
        "{:<8}  {:<15}  {:<10}  {:<8}  {:<42}  {:<10}",
        "ID", "AGENT", "STATUS", "PRIORITY", "TASK", "CREATED"
    );
}

fn print_table_row(msg: &db::messages::Message) {
    // ID: first 8 chars of UUID
    let id_short = if msg.id.len() >= 8 { &msg.id[..8] } else { &msg.id };

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

    // STATUS: colorize when terminal supports it.
    // ANSI codes add invisible bytes so we pad the raw status text manually,
    // then append the colored string (without fmt padding, which would count escape bytes).
    let status_raw = &msg.status;
    let status_colored = colorize_status(status_raw);
    // Pad the raw text to STATUS width (10), then replace raw text with colored text
    let status_cell = pad_colored(status_raw, &status_colored, 10);

    println!(
        "{:<8}  {:<15}  {}  {:<8}  {:<42}  {:<10}",
        id_short,
        msg.agent_name,
        status_cell,
        msg.priority,
        task_display,
        created_display,
    );
}

/// Build a padded cell where visible width is based on `raw` length but output uses `colored`.
fn pad_colored(raw: &str, colored: &str, width: usize) -> String {
    let raw_len = raw.len();
    let padding = if raw_len < width { width - raw_len } else { 0 };
    format!("{}{}", colored, " ".repeat(padding))
}

fn colorize_status(status: &str) -> String {
    match status {
        "pending" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.yellow())
        ),
        "completed" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.green())
        ),
        "failed" => format!(
            "{}",
            status.if_supports_color(Stream::Stdout, |s| s.red())
        ),
        _ => status.to_string(),
    }
}
