use anyhow::Result;
use std::path::PathBuf;

use crate::config;

use super::clean;

pub async fn run(config_path: PathBuf, no_relaunch: bool, json: bool) -> Result<()> {
    let config = config::load_config(&config_path)?;
    let db_path = config::resolve_db_path(&config)?;

    // Kill all sessions
    let (killed, _, _) = clean::kill_all_sessions(&config)?;
    let session_count = clean::compute_session_names(&config).len() as u32;
    let skipped = session_count.saturating_sub(killed);

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
