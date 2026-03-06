use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub role: String,
    pub command: String,
    pub created_at: String,
    pub status: String,
    pub status_updated_at: String,
}

pub async fn insert_agent(
    pool: &SqlitePool,
    name: &str,
    provider: &str,
    role: &str,
    command: &str,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO agents (id, name, provider, role, command, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(name)
    .bind(provider)
    .bind(role)
    .bind(command)
    .bind(created_at)
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
pub async fn get_orchestrator(pool: &SqlitePool) -> anyhow::Result<Option<Agent>> {
    let agent = sqlx::query_as::<_, Agent>(
        "SELECT * FROM agents WHERE role = 'orchestrator' LIMIT 1"
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
    sqlx::query(
        "UPDATE agents SET status = ?, status_updated_at = ? WHERE name = ?"
    )
    .bind(status)
    .bind(&now)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(())
}
