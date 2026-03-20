use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Message {
    pub id: String,
    pub agent_name: String, // legacy column — kept for transition (nullable after migration)
    pub from_agent: Option<String>, // MSGS-01
    pub to_agent: Option<String>, // MSGS-01
    #[sqlx(rename = "type")]
    pub msg_type: String, // MSGS-02: "task_request" | "task_completed" | "notify"
    pub task: String,       // keep field name (body is column alias — keep task column)
    pub status: String,     // MSGS-03: default is now 'processing'
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>, // MSGS-04
    pub thread_id: Option<String>,    // GAP-07: group related messages
}

pub async fn insert_message(
    pool: &SqlitePool,
    from_agent: &str, // MSGS-01
    to_agent: &str,   // MSGS-01
    msg_type: &str,   // MSGS-02 ("task_request")
    body: &str,       // task content (stored in `task` column for now)
    priority: &str,
    thread_id: Option<&str>, // GAP-07: group related messages
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    // If no thread_id provided, this message starts a new thread (thread_id = msg id)
    let effective_thread_id = thread_id.unwrap_or(&id);
    // agent_name is set to to_agent value for backward compat with peek_message and update_status subqueries
    sqlx::query(
        "INSERT INTO messages (id, agent_name, from_agent, to_agent, type, task, status, priority, created_at, updated_at, thread_id) \
         VALUES (?, ?, ?, ?, ?, ?, 'processing', ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(to_agent)    // agent_name = to_agent for legacy compat
    .bind(from_agent)
    .bind(to_agent)
    .bind(msg_type)
    .bind(body)
    .bind(priority)
    .bind(&now)
    .bind(&now)
    .bind(effective_thread_id)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Mark the oldest processing message for this agent as completed (FIFO order).
/// Respects priority ordering: urgent > high > normal, then oldest first.
/// Returns the number of rows affected (0 = already completed, not an error — MSG-03 idempotency).
///
/// Uses a subquery to identify the target row because SQLite does not support
/// `UPDATE ... ORDER BY ... LIMIT` without a compile-time flag (SQLITE_ENABLE_UPDATE_DELETE_LIMIT).
pub async fn update_status(pool: &SqlitePool, agent_name: &str) -> anyhow::Result<u64> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE messages SET status = 'completed', updated_at = ?, completed_at = ? \
         WHERE id = (\
           SELECT id FROM messages \
           WHERE agent_name = ? AND status = 'processing' \
           ORDER BY CASE priority WHEN 'urgent' THEN 1 WHEN 'high' THEN 2 ELSE 3 END, created_at ASC LIMIT 1\
         )",
    )
    .bind(&now)
    .bind(&now)
    .bind(agent_name)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_messages(
    pool: &SqlitePool,
    agent: Option<&str>,
    status: Option<&str>,
    limit: u32,
) -> anyhow::Result<Vec<Message>> {
    // Build query dynamically with optional WHERE clauses
    let mut query = String::from("SELECT * FROM messages");
    let mut conditions = Vec::new();
    if agent.is_some() {
        conditions.push("agent_name = ?");
    }
    if status.is_some() {
        conditions.push("status = ?");
    }
    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }
    query.push_str(" ORDER BY created_at DESC LIMIT ?");

    let mut q = sqlx::query_as::<_, Message>(&query);
    if let Some(a) = agent {
        q = q.bind(a);
    }
    if let Some(s) = status {
        q = q.bind(s);
    }
    q = q.bind(limit);

    let messages = q.fetch_all(pool).await?;
    Ok(messages)
}

/// Count remaining processing messages for an agent.
pub async fn count_processing(pool: &SqlitePool, agent_name: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE agent_name = ? AND status = 'processing'",
    )
    .bind(agent_name)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Complete a specific message by ID. Returns rows affected (0 if already completed or not found).
/// Used by signal.rs to complete the exact task pointed to by agent.current_task,
/// instead of the FIFO-based update_status which can race with /clear.
pub async fn complete_by_id(pool: &SqlitePool, message_id: &str) -> anyhow::Result<u64> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE messages SET status = 'completed', updated_at = ?, completed_at = ? \
         WHERE id = ? AND status = 'processing'",
    )
    .bind(&now)
    .bind(&now)
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Count all processing messages across all agents.
pub async fn count_processing_all(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM messages WHERE status = 'processing'")
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Get the ID of the most recently completed message for an agent.
/// Used by signal.rs FIFO fallback to identify which task was just completed.
pub async fn last_completed_id(
    pool: &SqlitePool,
    agent_name: &str,
) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM messages WHERE agent_name = ? AND status = 'completed' \
         ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(agent_name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id,)| id))
}

/// Count total messages (all statuses). Used by watchdog for activity detection.
pub async fn total_count(pool: &SqlitePool) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Get the most recent updated_at timestamp across all messages.
/// Used by watchdog for global stall detection.
pub async fn last_activity_timestamp(pool: &SqlitePool) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT MAX(updated_at) FROM messages")
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(ts,)| ts))
}

/// Peek at the highest-priority processing message for an agent.
/// Priority ordering: urgent > high > normal.
pub async fn peek_message(pool: &SqlitePool, agent_name: &str) -> anyhow::Result<Option<Message>> {
    let message = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE agent_name = ? AND status = 'processing' ORDER BY CASE priority WHEN 'urgent' THEN 1 WHEN 'high' THEN 2 ELSE 3 END, created_at ASC LIMIT 1"
    )
    .bind(agent_name)
    .fetch_optional(pool)
    .await?;
    Ok(message)
}
