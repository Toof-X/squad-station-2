use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

/// Set up an in-memory (or temp-file) SQLite pool with WAL mode and all migrations applied.
///
/// Note: SQLite WAL mode is silently downgraded to DELETE journal mode for ":memory:" databases
/// (SQLite limitation — WAL requires a real file). The pool is configured identically to production
/// (max_connections=1, single writer) to catch deadlocks and serialization issues.
pub async fn setup_test_db() -> SqlitePool {
    // Use a unique temp file per call so each test gets its own isolated DB (SAFE-01 compliance).
    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_owned();
    // Keep the file alive for the test duration by leaking the NamedTempFile handle.
    // The file is cleaned up when the process exits or the OS reclaims it.
    std::mem::forget(tmp);

    let opts = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("failed to create test pool");

    sqlx::migrate!("./src/db/migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    pool
}
