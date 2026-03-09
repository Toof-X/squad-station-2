use crate::{config, db, tmux};

pub async fn run(json: bool) -> anyhow::Result<()> {
    // 1. Load config + connect
    let config = config::load_config(std::path::Path::new("squad.yml"))?;
    let db_path = config::resolve_db_path(&config)?;
    let pool = db::connect(&db_path).await?;

    // 2. Fetch agents
    let agents = db::agents::list_agents(&pool).await?;

    // 3. Get live tmux sessions
    let live_sessions = tmux::list_live_session_names();

    // 4. Filter: keep only agents whose name appears in live sessions
    let live_agent_names: Vec<String> = agents
        .iter()
        .filter(|a| live_sessions.contains(&a.name))
        .map(|a| a.name.clone())
        .collect();

    if live_agent_names.is_empty() {
        if json {
            println!(r#"{{"message":"No live agent sessions to display."}}"#);
        } else {
            println!("No live agent sessions to display.");
        }
        return Ok(());
    }

    let n = live_agent_names.len();

    // 5. Kill existing monitor session for this project (idempotent)
    let monitor_session = format!("squad-monitor-{}", config.project);
    tmux::kill_session(&monitor_session)?;

    // 6. Create new monitor session with tiled panes (TMUX= unset per pane to allow nested attach)
    tmux::create_view_session(&monitor_session, &live_agent_names)?;

    if json {
        println!(
            r#"{{"message":"Created {} with {} panes","session":"{}","panes":{}}}"#,
            monitor_session, n, monitor_session, n
        );
    } else {
        println!(
            "Created monitor session '{}' with {} pane(s)",
            monitor_session, n
        );
        println!("Attach with: tmux attach -t {}", monitor_session);
    }

    Ok(())
}
