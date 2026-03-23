pub mod agents;
pub mod messages;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub type Pool = SqlitePool;

/// Timeout layering (each fires in order, inner → outer):
///   busy_timeout  3s  — SQLite retries on SQLITE_BUSY
///   acquire       5s  — sqlx pool waits for a free connection
///   tokio         8s  — hard deadline, triggers WAL/SHM recovery
const BUSY_TIMEOUT: Duration = Duration::from_secs(3);
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

/// Connect to the SQLite database with WAL mode and single-writer pool (SAFE-01).
/// Wrapped in a tokio timeout to guarantee bounded execution even when
/// the OS-level flock on the SHM file blocks past SQLite's busy_timeout.
pub async fn connect(db_path: &Path) -> anyhow::Result<SqlitePool> {
    match tokio::time::timeout(CONNECT_TIMEOUT, connect_inner(db_path)).await {
        Ok(result) => result,
        Err(_) => try_wal_recovery(db_path, false).await,
    }
}

/// Connect to the SQLite database in read-only mode with a multi-reader pool.
/// Also wrapped in a tokio timeout with WAL/SHM recovery for the same flock issue.
/// Does NOT create the DB, set journal_mode, or run migrations.
pub async fn connect_readonly(db_path: &Path) -> anyhow::Result<SqlitePool> {
    match tokio::time::timeout(CONNECT_TIMEOUT, connect_readonly_inner(db_path)).await {
        Ok(result) => result,
        Err(_) => try_wal_recovery(db_path, true).await,
    }
}

// ── Internal connection builders ─────────────────────────────────────────────

async fn connect_inner(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(1) // CRITICAL: single writer, prevents async deadlock
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;

    Ok(pool)
}

async fn connect_readonly_inner(db_path: &Path) -> anyhow::Result<SqlitePool> {
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(opts)
        .await?;

    Ok(pool)
}

// ── WAL/SHM recovery ────────────────────────────────────────────────────────

/// Check if any live squad-station processes hold the DB file open.
/// Returns true if it is safe to remove WAL/SHM (no live holders).
fn wal_is_orphaned(db_path: &Path) -> bool {
    let db_str = match db_path.to_str() {
        Some(s) => s,
        None => return false, // can't check — assume not safe
    };
    // lsof exits 1 when no matches are found (= no holders = safe)
    match std::process::Command::new("lsof")
        .arg(db_str)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
    {
        Ok(s) => !s.success(), // exit 1 = no holders = orphaned
        Err(_) => false,       // lsof unavailable — assume not safe
    }
}

/// Attempt WAL/SHM recovery after a connection timeout.
/// Only removes files if no live process holds the DB (checked via lsof).
async fn try_wal_recovery(db_path: &Path, readonly: bool) -> anyhow::Result<SqlitePool> {
    let shm = db_path.with_extension("db-shm");
    let wal = db_path.with_extension("db-wal");

    if (shm.exists() || wal.exists()) && wal_is_orphaned(db_path) {
        eprintln!(
            "squad-station: DB connection timed out — clearing orphaned WAL/SHM and retrying"
        );
        let _ = std::fs::remove_file(&shm);
        let _ = std::fs::remove_file(&wal);

        let result = if readonly {
            tokio::time::timeout(CONNECT_TIMEOUT, connect_readonly_inner(db_path)).await
        } else {
            tokio::time::timeout(CONNECT_TIMEOUT, connect_inner(db_path)).await
        };
        result
            .map_err(|_| {
                anyhow::anyhow!(
                    "DB connection timed out after WAL recovery. \
                     Check for zombie squad-station processes: ps aux | grep squad-station"
                )
            })?
    } else {
        anyhow::bail!(
            "DB connection timed out ({}s). {}",
            CONNECT_TIMEOUT.as_secs(),
            if shm.exists() || wal.exists() {
                "WAL/SHM held by a live process — wait for it to finish or kill it."
            } else {
                "Check for zombie squad-station processes: ps aux | grep squad-station"
            }
        );
    }
}
