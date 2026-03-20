pub mod agents;
pub mod messages;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub type Pool = SqlitePool;

/// Connect to the SQLite database with WAL mode and single-writer pool (SAFE-01)
pub async fn connect(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1) // CRITICAL: single writer, prevents async deadlock
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;

    Ok(pool)
}
