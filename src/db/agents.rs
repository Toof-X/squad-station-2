use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub tool: String, // AGNT-03: renamed from provider
    pub role: String,
    #[allow(dead_code)]
    pub command: Option<String>, // legacy column; schema has NOT NULL but we ignore value
    pub created_at: String,
    pub status: String,
    pub status_updated_at: String,
    pub model: Option<String>,        // AGNT-01
    pub description: Option<String>,  // AGNT-01
    pub current_task: Option<String>, // AGNT-02: FK to messages.id
}

pub async fn insert_agent(
    pool: &SqlitePool,
    name: &str,
    tool: &str,
    role: &str,
    model: Option<&str>,
    description: Option<&str>,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO agents (id, name, tool, role, command, model, description, created_at, status_updated_at) \
         VALUES (?, ?, ?, ?, '', ?, ?, ?, ?) \
         ON CONFLICT(name) DO UPDATE SET tool = excluded.tool, role = excluded.role, \
         model = excluded.model, description = excluded.description"
    )
    .bind(id)
    .bind(name)
    .bind(tool)
    .bind(role)
    .bind(model)
    .bind(description)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_agent(pool: &SqlitePool, name: &str) -> anyhow::Result<Option<Agent>> {
    let agent = sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(agent)
}

pub async fn list_agents(pool: &SqlitePool) -> anyhow::Result<Vec<Agent>> {
    let agents = sqlx::query_as::<_, Agent>("SELECT * FROM agents ORDER BY name")
        .fetch_all(pool)
        .await?;
    Ok(agents)
}

/// Find the orchestrator agent (role = 'orchestrator') for notification purposes.
/// Prefers non-dead orchestrators so stale records from previous inits don't shadow
/// the active one (e.g. my-project-orchestrator dead vs squad-test-project-orchestrator idle).
pub async fn get_orchestrator(pool: &SqlitePool) -> anyhow::Result<Option<Agent>> {
    let agent = sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents WHERE role = 'orchestrator' \
         ORDER BY CASE WHEN status = 'dead' THEN 1 ELSE 0 END, created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(agent)
}

/// Update agent lifecycle status. Valid values: "idle" | "busy" | "dead"
pub async fn update_agent_status(
    pool: &SqlitePool,
    name: &str,
    status: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE agents SET status = ?, status_updated_at = ? WHERE name = ?")
        .bind(status)
        .bind(&now)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete all agents from the DB. Used before re-registering on overwrite.
pub async fn delete_all_agents(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM agents").execute(pool).await?;
    Ok(())
}
