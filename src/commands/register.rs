use crate::{config, db};

pub async fn run(
    name: String,
    role: String,
    tool: String, // TODO: Plan 03 — command will be derived from tool
    json: bool,
) -> anyhow::Result<()> {
    // 1. Resolve DB path — look for squad.yml in current directory
    let config_path = std::path::Path::new("squad.yml");
    let db_path = if config_path.exists() {
        let cfg = config::load_config(config_path)?;
        config::resolve_db_path(&cfg)?
    } else if let Ok(env_path) = std::env::var("SQUAD_STATION_DB") {
        std::path::PathBuf::from(env_path)
    } else {
        anyhow::bail!(
            "No squad.yml found in current directory. Run 'squad-station init' first, or set SQUAD_STATION_DB env var."
        );
    };

    // 2. Connect to DB
    let pool = db::connect(&db_path).await?;

    // 3. Insert agent — INSERT OR IGNORE means duplicate name is a no-op, not an error
    db::agents::insert_agent(&pool, &name, &tool, &role, None, None).await?;

    // 4. Output result
    if json {
        let output = serde_json::json!({
            "registered": true,
            "name": name,
            "role": role,
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!(
            "Registered agent '{}' (role={}, tool={})",
            name, role, tool
        );
    }

    Ok(())
}
