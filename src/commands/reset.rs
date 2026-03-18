use anyhow::Result;
use std::path::PathBuf;

use crate::{config, tmux};

use super::clean;
use super::close;

pub async fn run(config_path: PathBuf, no_relaunch: bool, json: bool) -> Result<()> {
    let config = config::load_config(&config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let session_names = close::compute_session_names(&config);

    // Kill all sessions
    let mut killed = 0u32;
    let mut skipped = 0u32;

    for name in &session_names {
        if tmux::session_exists(name) {
            tmux::kill_session(name)?;
            killed += 1;
        } else {
            skipped += 1;
        }
    }

    // Delete the database
    let deleted = clean::delete_db_file(&db_path)?;

    // Optionally relaunch (re-init)
    if !no_relaunch {
        if !json {
            println!();
            println!("  Relaunching squad...");
        }
        return super::init::run(config_path, json, false).await;
    }

    if json {
        let output = serde_json::json!({
            "project": config.project,
            "killed": killed,
            "skipped": skipped,
            "db_deleted": deleted,
            "relaunched": false,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║         Squad Station  •  Reset                              ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("  Project  : {}", config.project);
        println!("  Sessions : {} killed, {} skipped", killed, skipped);
        println!(
            "  Database : {}",
            if deleted {
                "deleted"
            } else {
                "not found (skipped)"
            }
        );
        println!();
        println!("Run `squad-station init` to relaunch agents.");
        println!();
    }

    Ok(())
}
