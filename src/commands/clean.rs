use anyhow::Result;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use crate::{config, tmux};

/// Compute the expected tmux session names for all agents in a squad config.
/// Mirrors the naming logic in init.rs: `{project}-{name_or_role}`.
pub fn compute_session_names(config: &config::SquadConfig) -> Vec<String> {
    let mut names = Vec::new();

    let orch_role = config
        .orchestrator
        .name
        .as_deref()
        .unwrap_or("orchestrator");
    names.push(format!("{}-{}", config.project, orch_role));

    for agent in &config.agents {
        let role_suffix = agent.name.as_deref().unwrap_or(&agent.role);
        names.push(format!("{}-{}", config.project, role_suffix));
    }

    names
}

/// Delete the DB file at the given path.
/// Returns `true` if the file was deleted, `false` if it did not exist.
pub fn delete_db_file(db_path: &Path) -> Result<bool> {
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Stop the watchdog daemon if running. Reads PID from .squad/watch.pid, kills process, deletes PID file.
/// Returns true if a watchdog was stopped.
pub fn stop_watchdog(squad_dir: &Path) -> bool {
    let pid_file = squad_dir.join("watch.pid");
    if !pid_file.exists() {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<i32>() {
            // Check if process is alive and kill it
            #[cfg(unix)]
            unsafe {
                if libc::kill(pid, 0) == 0 {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
        }
    }
    let _ = std::fs::remove_file(&pid_file);
    true
}

/// Kill all squad tmux sessions (agents + monitor).
/// Returns (killed_count, killed_names, skipped_names).
pub fn kill_all_sessions(config: &config::SquadConfig) -> Result<(u32, Vec<String>, Vec<String>)> {
    let session_names = compute_session_names(config);
    let mut killed = 0u32;
    let mut killed_names: Vec<String> = Vec::new();
    let mut skipped_names: Vec<String> = Vec::new();

    for name in &session_names {
        if tmux::session_exists(name) {
            tmux::kill_session(name)?;
            killed += 1;
            killed_names.push(name.clone());
        } else {
            skipped_names.push(name.clone());
        }
    }

    // Also kill the monitor session
    let monitor_name = format!("{}-monitor", config.project);
    if tmux::session_exists(&monitor_name) {
        tmux::kill_session(&monitor_name)?;
        killed += 1;
        killed_names.push(monitor_name);
    }

    Ok((killed, killed_names, skipped_names))
}

pub async fn run(config_path: PathBuf, yes: bool, delete_all: bool, json: bool) -> Result<()> {
    let config = config::load_config(&config_path)?;
    let db_path = config::resolve_db_path(&config)?;

    // .squad directory (parent of station.db)
    let squad_dir = db_path
        .parent()
        .unwrap_or(Path::new("."));

    if !yes {
        let prompt = if delete_all {
            format!(
                "Kill all squad sessions, stop watchdog, delete {} AND logs? [y/N]: ",
                db_path.display()
            )
        } else {
            format!(
                "Kill all squad sessions, stop watchdog, and delete {}? [y/N]: ",
                db_path.display()
            )
        };
        eprint!("{}", prompt);
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // 1. Stop watchdog daemon BEFORE deleting DB (a running watchdog + deleted DB = crash loop)
    let watchdog_stopped = stop_watchdog(squad_dir);

    // 2. Kill all tmux sessions
    let (killed, killed_names, skipped_names) = kill_all_sessions(&config)?;

    // 3. Delete the database
    let deleted = delete_db_file(&db_path)?;

    // 4. Delete watch.pid if still present
    let pid_file = squad_dir.join("watch.pid");
    if pid_file.exists() {
        let _ = std::fs::remove_file(&pid_file);
    }

    // 5. Optionally delete logs
    let logs_deleted = if delete_all {
        let log_dir = squad_dir.join("log");
        if log_dir.exists() {
            std::fs::remove_dir_all(&log_dir)?;
            true
        } else {
            false
        }
    } else {
        false
    };

    if json {
        let output = serde_json::json!({
            "project": config.project,
            "killed": killed,
            "skipped": skipped_names.len() as u32,
            "watchdog_stopped": watchdog_stopped,
            "db_path": db_path.display().to_string(),
            "db_deleted": deleted,
            "logs_deleted": logs_deleted,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║         Squad Station  •  Clean                              ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("  Project  : {}", config.project);
        println!();
        for name in &killed_names {
            println!("  [KILLED] {}", name);
        }
        for name in &skipped_names {
            println!("  [SKIP]   {} — not running", name);
        }
        if watchdog_stopped {
            println!("  [STOP]   watchdog daemon");
        }
        println!();
        println!(
            "  Database : {}",
            if deleted {
                "deleted"
            } else {
                "not found (skipped)"
            }
        );
        if delete_all {
            println!(
                "  Logs     : {}",
                if logs_deleted {
                    "deleted"
                } else {
                    "not found (skipped)"
                }
            );
        } else {
            println!("  Logs     : preserved (use --all to delete)");
        }
        println!();
    }

    Ok(())
}
