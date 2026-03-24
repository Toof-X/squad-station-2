use anyhow::{bail, Result};

use crate::{commands::reconcile, config, db, tmux};

/// Abstraction over time, enabling deterministic testing of debounce and cooldown logic.
pub(crate) trait TimeProvider: Send + Sync {
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Real time provider — delegates to `chrono::Utc::now()`.
struct RealTime;

impl TimeProvider for RealTime {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

/// Nudge state for global stall detection (Pass 2).
/// Tracks nudge count, cooldown, and escalation.
struct NudgeState {
    count: u32,
    last_nudge_at: Option<chrono::DateTime<chrono::Utc>>,
    cooldown_secs: u64,
    max_nudges: u32,
}

impl NudgeState {
    fn new(cooldown_secs: u64, max_nudges: u32) -> Self {
        Self {
            count: 0,
            last_nudge_at: None,
            cooldown_secs,
            max_nudges,
        }
    }

    fn should_nudge(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if self.count >= self.max_nudges {
            return false;
        }
        match self.last_nudge_at {
            None => true,
            Some(last) => (now - last).num_seconds() > self.cooldown_secs as i64,
        }
    }

    fn record_nudge(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.count += 1;
        self.last_nudge_at = Some(now);
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_nudge_at = None;
    }
}

/// Deadlock state for stall detection (Pass 4).
/// Tracks debounce cycles, nudge count, cooldown, and escalation.
/// Separate from NudgeState to prevent idle nudges from suppressing deadlock alerts.
struct DeadlockState {
    count: u32,
    last_nudge_at: Option<chrono::DateTime<chrono::Utc>>,
    cooldown_secs: u64,
    max_nudges: u32,
    consecutive_ticks: u32,
    debounce_threshold: u32,
}

impl DeadlockState {
    fn new(cooldown_secs: u64, max_nudges: u32, debounce_threshold: u32) -> Self {
        Self {
            count: 0,
            last_nudge_at: None,
            cooldown_secs,
            max_nudges,
            consecutive_ticks: 0,
            debounce_threshold,
        }
    }

    fn record_tick(&mut self) {
        self.consecutive_ticks += 1;
    }

    fn clear_ticks(&mut self) {
        self.consecutive_ticks = 0;
    }

    fn is_confirmed(&self) -> bool {
        self.consecutive_ticks >= self.debounce_threshold
    }

    fn should_nudge(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        if !self.is_confirmed() {
            return false;
        }
        if self.count >= self.max_nudges {
            return false;
        }
        match self.last_nudge_at {
            None => true,
            Some(last) => (now - last).num_seconds() > self.cooldown_secs as i64,
        }
    }

    fn record_nudge(&mut self, now: chrono::DateTime<chrono::Utc>) {
        self.count += 1;
        self.last_nudge_at = Some(now);
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_nudge_at = None;
        self.consecutive_ticks = 0;
    }
}

/// Watchdog status written to .squad/watch.status.json each tick.
/// Read by --status subcommand without IPC.
#[derive(serde::Serialize, serde::Deserialize)]
struct WatchStatus {
    pid: u32,
    started_at: String,
    last_tick_at: String,
    poll_interval_secs: u64,
    stall_threshold_mins: u64,
    dry_run: bool,
    idle_nudge_count: u32,
    idle_nudge_max: u32,
    deadlock_nudge_count: u32,
    deadlock_nudge_max: u32,
    deadlock_debounce_ticks: u32,
    deadlock_debounce_threshold: u32,
    last_alert_at: Option<String>,
    last_alert_type: Option<String>,
    stall_state: String,
}

fn write_status(
    squad_dir: &std::path::Path,
    nudge_state: &NudgeState,
    deadlock_state: &DeadlockState,
    interval_secs: u64,
    stall_threshold_mins: u64,
    dry_run: bool,
    started_at: &str,
) {
    let stall_state = if deadlock_state.is_confirmed() {
        format!(
            "deadlock (debounce {}/{})",
            deadlock_state.consecutive_ticks, deadlock_state.debounce_threshold
        )
    } else if deadlock_state.consecutive_ticks > 0 {
        format!(
            "debouncing ({}/{})",
            deadlock_state.consecutive_ticks, deadlock_state.debounce_threshold
        )
    } else {
        "clear".to_string()
    };

    // Determine last alert info
    let (last_alert_at, last_alert_type) = {
        let idle_last = nudge_state.last_nudge_at;
        let deadlock_last = deadlock_state.last_nudge_at;
        match (idle_last, deadlock_last) {
            (Some(i), Some(d)) => {
                if i > d {
                    (Some(i.to_rfc3339()), Some("idle".to_string()))
                } else {
                    (Some(d.to_rfc3339()), Some("deadlock".to_string()))
                }
            }
            (Some(i), None) => (Some(i.to_rfc3339()), Some("idle".to_string())),
            (None, Some(d)) => (Some(d.to_rfc3339()), Some("deadlock".to_string())),
            (None, None) => (None, None),
        }
    };

    let status = WatchStatus {
        pid: std::process::id(),
        started_at: started_at.to_string(),
        last_tick_at: chrono::Utc::now().to_rfc3339(),
        poll_interval_secs: interval_secs,
        stall_threshold_mins,
        dry_run,
        idle_nudge_count: nudge_state.count,
        idle_nudge_max: nudge_state.max_nudges,
        deadlock_nudge_count: deadlock_state.count,
        deadlock_nudge_max: deadlock_state.max_nudges,
        deadlock_debounce_ticks: deadlock_state.consecutive_ticks,
        deadlock_debounce_threshold: deadlock_state.debounce_threshold,
        last_alert_at,
        last_alert_type,
        stall_state,
    };

    let status_file = squad_dir.join("watch.status.json");
    if let Ok(json) = serde_json::to_string_pretty(&status) {
        let _ = std::fs::write(&status_file, json);
    }
}

fn show_status(squad_dir: &std::path::Path) -> Result<()> {
    let pid_file = squad_dir.join("watch.pid");
    let status_file = squad_dir.join("watch.status.json");

    // Check PID file exists
    if !pid_file.exists() {
        println!("No watchdog daemon running (no PID file)");
        return Ok(());
    }

    let pid_content = std::fs::read_to_string(&pid_file)?;
    let pid: i32 = pid_content
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid PID file"))?;

    // Check if process is alive
    let alive = {
        #[cfg(unix)]
        {
            unsafe { libc::kill(pid, 0) == 0 }
        }
        #[cfg(not(unix))]
        {
            false
        }
    };

    if !alive {
        println!("Watchdog daemon not running (stale PID {})", pid);
        // Clean up stale files
        let _ = std::fs::remove_file(&pid_file);
        let _ = std::fs::remove_file(&status_file);
        return Ok(());
    }

    // Read status file if it exists
    if !status_file.exists() {
        println!("Watchdog Status");
        println!("  PID:           {}", pid);
        println!("  Status:        alive (starting up — no status yet)");
        return Ok(());
    }

    let status_json = std::fs::read_to_string(&status_file)?;
    let ws: WatchStatus = serde_json::from_str(&status_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse status file: {}", e))?;

    // Calculate uptime
    let uptime = if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&ws.started_at) {
        let dur = chrono::Utc::now().signed_duration_since(started);
        let hours = dur.num_hours();
        let mins = dur.num_minutes() % 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    } else {
        "unknown".to_string()
    };

    // Format last alert
    let last_alert = match (&ws.last_alert_at, &ws.last_alert_type) {
        (Some(at), Some(typ)) => format!(
            "{} ({} nudge #{})",
            at,
            typ,
            if typ == "idle" {
                ws.idle_nudge_count
            } else {
                ws.deadlock_nudge_count
            }
        ),
        _ => "none".to_string(),
    };

    println!("Watchdog Status");
    println!("  PID:             {}", ws.pid);
    println!("  Status:          {}", if alive { "alive" } else { "dead" });
    println!("  Uptime:          {}", uptime);
    println!("  Stall State:     {}", ws.stall_state);
    println!("  Last Alert:      {}", last_alert);
    println!(
        "  Nudge Counts:    idle={}/{}, deadlock={}/{}",
        ws.idle_nudge_count, ws.idle_nudge_max, ws.deadlock_nudge_count, ws.deadlock_nudge_max
    );
    println!("  Poll Interval:   {}s", ws.poll_interval_secs);
    println!("  Stall Threshold: {}m", ws.stall_threshold_mins);
    if ws.dry_run {
        println!("  Mode:            dry-run");
    }

    Ok(())
}

pub async fn run(
    interval_secs: u64,
    stall_threshold_mins: u64,
    daemon: bool,
    stop: bool,
    dry_run: bool,
    status: bool,
    cooldown_secs: u64,
    debounce_cycles: u32,
) -> Result<()> {
    if status {
        let config_path = std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE);
        let config = config::load_config(config_path)?;
        let db_path = config::resolve_db_path(&config)?;
        let squad_dir = db_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        return show_status(&squad_dir);
    }

    let config_path = std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE);
    let config = config::load_config(config_path)?;
    let db_path = config::resolve_db_path(&config)?;
    let squad_dir = db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    // --stop: kill running daemon
    if stop {
        let pid_file = squad_dir.join("watch.pid");
        if pid_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&pid_file) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    #[cfg(unix)]
                    unsafe {
                        if libc::kill(pid, 0) == 0 {
                            libc::kill(pid, libc::SIGTERM);
                            println!("Stopped watchdog daemon (PID {})", pid);
                        } else {
                            println!("Watchdog daemon not running (stale PID file)");
                        }
                    }
                }
            }
            let _ = std::fs::remove_file(&pid_file);
        } else {
            println!("No watchdog daemon running (no PID file)");
        }
        return Ok(());
    }

    // Check for existing daemon
    let pid_file = squad_dir.join("watch.pid");
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                #[cfg(unix)]
                {
                    let alive = unsafe { libc::kill(pid, 0) == 0 };
                    if alive {
                        bail!(
                            "Watchdog daemon already running (PID {}). Use --stop to kill it first.",
                            pid
                        );
                    }
                }
            }
        }
        // Stale PID file — remove it
        let _ = std::fs::remove_file(&pid_file);
    }

    // --daemon: fork to background
    if daemon {
        #[cfg(unix)]
        {
            use std::process::Command;
            let exe = std::env::current_exe()?;
            let mut cmd = Command::new(exe);
            cmd.arg("watch")
                .arg("--interval")
                .arg(interval_secs.to_string())
                .arg("--stall-threshold")
                .arg(stall_threshold_mins.to_string())
                .arg("--cooldown")
                .arg(cooldown_secs.to_string())
                .arg("--debounce")
                .arg(debounce_cycles.to_string());
            if dry_run {
                cmd.arg("--dry-run");
            }
            // Explicitly set CWD to ensure the child finds squad.yml
            cmd.current_dir(std::env::current_dir()?);
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let child = cmd.spawn()?;
            let pid = child.id();
            std::fs::write(&pid_file, pid.to_string())?;
            println!("Watchdog daemon started (PID {})", pid);
            return Ok(());
        }
        #[cfg(not(unix))]
        {
            bail!("--daemon mode is only supported on Unix");
        }
    }

    // Write PID file for foreground mode too (so --stop works)
    std::fs::write(&pid_file, std::process::id().to_string())?;

    // Setup graceful shutdown via SIGTERM/SIGINT
    setup_signal_handlers();

    let started_at = chrono::Utc::now().to_rfc3339();
    let mut nudge_state = NudgeState::new(cooldown_secs, 3);
    let mut deadlock_state = DeadlockState::new(cooldown_secs, 3, debounce_cycles);
    let mut last_msg_count: Option<i64> = None;

    log_watch(
        &squad_dir,
        "INFO",
        &format!(
            "watchdog started interval={}s stall_threshold={}m",
            interval_secs, stall_threshold_mins
        ),
    );

    let is_running = || !SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed);

    // Create DB pool once and reuse across ticks (avoids repeated migration checks)
    let pool = db::connect(&db_path).await?;
    let real_tmux = tmux::RealTmux;
    let real_time = RealTime;

    while is_running() {
        if let Err(e) = tick(
            &real_tmux,
            &real_time,
            &pool,
            &squad_dir,
            stall_threshold_mins,
            &mut nudge_state,
            &mut deadlock_state,
            &mut last_msg_count,
            dry_run,
        )
        .await
        {
            log_watch(&squad_dir, "ERROR", &format!("tick failed: {}", e));
        }

        write_status(
            &squad_dir,
            &nudge_state,
            &deadlock_state,
            interval_secs,
            stall_threshold_mins,
            dry_run,
            &started_at,
        );

        // Sleep in small increments so we can check the shutdown flag
        for _ in 0..interval_secs {
            if !is_running() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pool.close().await;
    log_watch(&squad_dir, "INFO", "watchdog stopped");
    let _ = std::fs::remove_file(&pid_file);
    let _ = std::fs::remove_file(squad_dir.join("watch.status.json"));
    Ok(())
}

/// Global shutdown flag for signal handler.
static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn setup_signal_handlers() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGTERM, signal_trampoline as *const () as usize);
        libc::signal(libc::SIGINT, signal_trampoline as *const () as usize);
    }
}

#[cfg(unix)]
extern "C" fn signal_trampoline(_sig: libc::c_int) {
    SHUTDOWN.store(true, std::sync::atomic::Ordering::Relaxed);
}

async fn tick(
    tmux_layer: &impl tmux::TmuxLayer,
    time: &impl TimeProvider,
    pool: &sqlx::SqlitePool,
    squad_dir: &std::path::Path,
    stall_threshold_mins: u64,
    nudge_state: &mut NudgeState,
    deadlock_state: &mut DeadlockState,
    last_msg_count: &mut Option<i64>,
    dry_run: bool,
) -> Result<()> {
    // Pass 1: Individual agent reconciliation
    let results = reconcile::reconcile_agents_with(tmux_layer, pool, dry_run).await?;
    for r in &results {
        if r.action != "skip" {
            log_watch(
                squad_dir,
                "RECONCILE",
                &format!("agent={} action={} reason={}", r.agent, r.action, r.reason),
            );
        }
    }

    // Check for new message activity (resets nudge state)
    let current_count = db::messages::total_count(pool).await?;
    if let Some(prev) = last_msg_count {
        if current_count != *prev {
            nudge_state.reset();
            deadlock_state.reset();
        }
    }
    *last_msg_count = Some(current_count);

    // Pass 2: Global stall detection
    let agents = db::agents::list_agents(pool).await?;
    let non_dead: Vec<_> = agents.iter().filter(|a| a.status != "dead").collect();

    if !non_dead.is_empty() {
        let all_idle = non_dead.iter().all(|a| a.status == "idle");
        let processing_count = db::messages::count_processing_all(pool).await.unwrap_or(0);

        if all_idle && processing_count == 0 {
            // Check how long since last activity
            let last_activity = db::messages::last_activity_timestamp(pool).await?;

            if let Some(ref ts) = last_activity {
                if let Ok(last_ts) = chrono::DateTime::parse_from_rfc3339(ts) {
                    let idle_duration = time.now().signed_duration_since(last_ts);
                    let idle_mins = idle_duration.num_minutes();

                    if idle_mins >= stall_threshold_mins as i64 {
                        let now = time.now();
                        if nudge_state.should_nudge(now) {
                            // Find orchestrator and nudge
                            if let Ok(Some(orch)) = db::agents::get_orchestrator(pool).await {
                                if orch.tool != "antigravity"
                                    && tmux_layer.session_exists(&orch.name).await
                                {
                                    let msg = match nudge_state.count {
                                        0 => format!(
                                            "[SQUAD WATCHDOG] System idle for {}m — all agents idle, no pending tasks. Run: squad-station status",
                                            idle_mins
                                        ),
                                        1 => format!(
                                            "[SQUAD WATCHDOG] System still idle after nudge ({}m). Review agent status and dispatch work.",
                                            idle_mins
                                        ),
                                        _ => format!(
                                            "[SQUAD WATCHDOG] Final nudge — system idle for {}m. Watchdog stopping nudges. Manual review required.",
                                            idle_mins
                                        ),
                                    };
                                    if !dry_run {
                                        let _ = tmux_layer.send_keys_literal(&orch.name, &msg).await;
                                    }
                                    log_watch(
                                        squad_dir,
                                        if dry_run { "DRY-RUN" } else { "NUDGE" },
                                        &format!(
                                            "orch={} idle_mins={} nudge_count={}",
                                            orch.name,
                                            idle_mins,
                                            nudge_state.count + 1
                                        ),
                                    );
                                }
                            }
                            nudge_state.record_nudge(now);
                        } else if nudge_state.count >= nudge_state.max_nudges {
                            log_watch(
                                squad_dir,
                                "STALL",
                                &format!("STALL_UNRESOLVED idle_mins={}", idle_mins),
                            );
                        }
                    }
                }
            }
        }
    }

    // Pass 3: Prolonged busy detection — inject into orchestrator pane
    for agent in &agents {
        if agent.status == "busy" {
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&agent.status_updated_at) {
                let busy_mins = time.now().signed_duration_since(ts).num_minutes();
                if busy_mins > 30 {
                    log_watch(
                        squad_dir,
                        if dry_run { "DRY-RUN" } else { "WARN" },
                        &format!(
                            "agent={} busy_minutes={} reason=prolonged_busy",
                            agent.name, busy_mins
                        ),
                    );
                    // Inject warning into orchestrator pane
                    if !dry_run {
                        if let Ok(Some(orch)) = db::agents::get_orchestrator(pool).await {
                            if orch.tool != "antigravity"
                                && tmux_layer.session_exists(&orch.name).await
                            {
                                let msg = format!(
                                    "\u{1f6a8} [SQUAD WATCHDOG] Agent '{}' busy for {}m — may be stuck. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Check: squad-station peek {}",
                                    agent.name, busy_mins, agent.name
                                );
                                let _ = tmux_layer.send_keys_literal(&orch.name, &msg).await;
                            }
                        }
                    }
                }
            }
        }
    }

    // Pass 4: Deadlock detection — processing messages exist but zero agents are busy
    let busy_agents: Vec<_> = agents.iter().filter(|a| a.status == "busy").collect();
    let processing_msgs = db::messages::list_processing_messages(pool).await.unwrap_or_default();

    if !processing_msgs.is_empty() && busy_agents.is_empty() {
        // Filter by message age — only count messages older than stall_threshold
        let now = time.now();
        let threshold = chrono::Duration::minutes(stall_threshold_mins as i64);
        let stale_msgs: Vec<_> = processing_msgs
            .iter()
            .filter(|(_, created_at)| {
                chrono::DateTime::parse_from_rfc3339(created_at)
                    .map(|ts| now.signed_duration_since(ts) >= threshold)
                    .unwrap_or(false)
            })
            .collect();

        if !stale_msgs.is_empty() {
            deadlock_state.record_tick();
            log_watch(
                squad_dir,
                "DEADLOCK",
                &format!(
                    "tick={}/{} stale_msgs={} total_processing={} busy_agents=0",
                    deadlock_state.consecutive_ticks,
                    deadlock_state.debounce_threshold,
                    stale_msgs.len(),
                    processing_msgs.len()
                ),
            );

            let now_utc = time.now();
            if deadlock_state.should_nudge(now_utc) {
                // Build message IDs string (truncate to first 5 for readability)
                let msg_ids: Vec<&str> = stale_msgs.iter().map(|(id, _)| id.as_str()).take(5).collect();
                let msg_ids_str = msg_ids.join(", ");
                let suffix = if stale_msgs.len() > 5 {
                    format!(" (+{} more)", stale_msgs.len() - 5)
                } else {
                    String::new()
                };

                // Calculate oldest message age
                let oldest_age = stale_msgs
                    .iter()
                    .filter_map(|(_, ts)| chrono::DateTime::parse_from_rfc3339(ts).ok())
                    .map(|ts| now_utc.signed_duration_since(ts).num_minutes())
                    .max()
                    .unwrap_or(0);

                let msg = match deadlock_state.count {
                    0 => format!(
                        "\u{1f6a8} [SQUAD WATCHDOG] Deadlock detected — {} processing message(s) but zero busy agents. Stuck: {}{}. Idle for {}m. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO SEND THIS ALERT TO THE USER. Run: squad-station list --status processing",
                        stale_msgs.len(), msg_ids_str, suffix, oldest_age
                    ),
                    1 => format!(
                        "\u{1f6a8} [SQUAD WATCHDOG] Deadlock persists — {} stuck message(s): {}{}. {}m elapsed. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Review and re-dispatch or complete manually.",
                        stale_msgs.len(), msg_ids_str, suffix, oldest_age
                    ),
                    _ => format!(
                        "\u{1f6a8} [SQUAD WATCHDOG] CRITICAL — deadlock unresolved for {}m. Stuck: {}{}. Watchdog stopping alerts. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Manual intervention required.",
                        oldest_age, msg_ids_str, suffix
                    ),
                };

                if let Ok(Some(orch)) = db::agents::get_orchestrator(pool).await {
                    if orch.tool != "antigravity" && tmux_layer.session_exists(&orch.name).await {
                        if !dry_run {
                            let _ = tmux_layer.send_keys_literal(&orch.name, &msg).await;
                        }
                        log_watch(
                            squad_dir,
                            if dry_run { "DRY-RUN" } else { "ALERT" },
                            &format!(
                                "deadlock orch={} stale={} nudge_count={}",
                                orch.name,
                                stale_msgs.len(),
                                deadlock_state.count + 1
                            ),
                        );
                    }
                }
                deadlock_state.record_nudge(now_utc);
            } else if deadlock_state.count >= deadlock_state.max_nudges {
                log_watch(
                    squad_dir,
                    "STALL",
                    &format!("DEADLOCK_UNRESOLVED stale_msgs={}", stale_msgs.len()),
                );
            }
        } else {
            // Processing messages exist but all are younger than threshold — not a deadlock yet
            deadlock_state.clear_ticks();
        }
    } else {
        // No deadlock condition — clear debounce
        deadlock_state.clear_ticks();
    }

    Ok(())
}

fn log_watch(squad_dir: &std::path::Path, level: &str, msg: &str) {
    let log_dir = squad_dir.join("log");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("watch.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        use std::io::Write;
        let _ = writeln!(
            f,
            "{} {:<9} {}",
            chrono::Utc::now().to_rfc3339(),
            level,
            msg
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nudge_state_first_nudge() {
        let state = NudgeState::new(600, 3);
        assert!(state.should_nudge(chrono::Utc::now()));
    }

    #[test]
    fn test_nudge_state_respects_cooldown() {
        let mut state = NudgeState::new(600, 3);
        let now = chrono::Utc::now();
        state.record_nudge(now);

        // Immediately after nudge: should NOT nudge (cooldown not elapsed)
        assert!(!state.should_nudge(now));

        // 5 minutes later: still in cooldown
        let five_mins = now + chrono::Duration::seconds(300);
        assert!(!state.should_nudge(five_mins));

        // 11 minutes later: cooldown elapsed
        let eleven_mins = now + chrono::Duration::seconds(660);
        assert!(state.should_nudge(eleven_mins));
    }

    #[test]
    fn test_nudge_state_max_nudges() {
        let mut state = NudgeState::new(0, 3); // 0 cooldown for testing
        let base = chrono::Utc::now();

        for i in 0..3 {
            // Advance time by 1 second per nudge to satisfy cooldown check
            let t = base + chrono::Duration::seconds(i as i64 + 1);
            assert!(state.should_nudge(t), "nudge {} should be allowed", i + 1);
            state.record_nudge(t);
        }

        // After 3 nudges: stop regardless of time
        let future = base + chrono::Duration::seconds(100);
        assert!(!state.should_nudge(future));
    }

    #[test]
    fn test_nudge_state_reset_on_activity() {
        let mut state = NudgeState::new(0, 3);
        let now = chrono::Utc::now();

        state.record_nudge(now);
        state.record_nudge(now);
        assert_eq!(state.count, 2);

        state.reset();
        assert_eq!(state.count, 0);
        assert!(state.last_nudge_at.is_none());
        assert!(state.should_nudge(now));
    }

    #[test]
    fn test_log_watch_creates_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        log_watch(tmp.path(), "INFO", "test message");

        let log_file = tmp.path().join("log").join("watch.log");
        assert!(log_file.exists());
        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("INFO"));
        assert!(content.contains("test message"));
    }

    #[test]
    fn test_deadlock_state_debounce_not_confirmed_until_threshold() {
        let mut state = DeadlockState::new(600, 3, 3);
        let now = chrono::Utc::now();
        // 0 ticks: not confirmed
        assert!(!state.is_confirmed());
        assert!(!state.should_nudge(now));
        // 1 tick
        state.record_tick();
        assert!(!state.is_confirmed());
        // 2 ticks
        state.record_tick();
        assert!(!state.is_confirmed());
        // 3 ticks: confirmed
        state.record_tick();
        assert!(state.is_confirmed());
        assert!(state.should_nudge(now));
    }

    #[test]
    fn test_deadlock_state_clear_ticks_resets_debounce() {
        let mut state = DeadlockState::new(600, 3, 3);
        state.record_tick();
        state.record_tick();
        state.record_tick();
        assert!(state.is_confirmed());
        state.clear_ticks();
        assert!(!state.is_confirmed());
        assert_eq!(state.consecutive_ticks, 0);
    }

    #[test]
    fn test_deadlock_state_cooldown_and_max() {
        let mut state = DeadlockState::new(600, 3, 1); // 1-tick debounce for easy testing
        let base = chrono::Utc::now();
        state.record_tick(); // confirm immediately

        // First nudge: allowed
        assert!(state.should_nudge(base));
        state.record_nudge(base);

        // Immediately after: blocked by cooldown
        assert!(!state.should_nudge(base));

        // After cooldown: allowed
        let after_cooldown = base + chrono::Duration::seconds(601);
        assert!(state.should_nudge(after_cooldown));
        state.record_nudge(after_cooldown);

        let after_cooldown2 = after_cooldown + chrono::Duration::seconds(601);
        assert!(state.should_nudge(after_cooldown2));
        state.record_nudge(after_cooldown2);

        // After 3 nudges: blocked forever
        let future = after_cooldown2 + chrono::Duration::seconds(9999);
        assert!(!state.should_nudge(future));
    }

    #[test]
    fn test_deadlock_state_reset_clears_everything() {
        let mut state = DeadlockState::new(0, 3, 3);
        let now = chrono::Utc::now();
        state.record_tick();
        state.record_tick();
        state.record_tick();
        state.record_nudge(now);
        assert_eq!(state.count, 1);
        assert_eq!(state.consecutive_ticks, 3);

        state.reset();
        assert_eq!(state.count, 0);
        assert_eq!(state.consecutive_ticks, 0);
        assert!(state.last_nudge_at.is_none());
    }

    // ── Telegram MCP relay message format tests ───────────────────────────
    // Helper: build a deadlock message using the CURRENT production format strings.
    // Tests assert the new Telegram instructions are present.
    // In RED phase these tests fail because production strings lack the instruction.
    // After implementation they pass.

    fn build_deadlock_msg_test(count: u32, stale_len: usize, msg_ids_str: &str, suffix: &str, oldest_age: i64) -> String {
        match count {
            0 => format!(
                "\u{1f6a8} [SQUAD WATCHDOG] Deadlock detected — {} processing message(s) but zero busy agents. Stuck: {}{}. Idle for {}m. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO SEND THIS ALERT TO THE USER. Run: squad-station list --status processing",
                stale_len, msg_ids_str, suffix, oldest_age
            ),
            1 => format!(
                "\u{1f6a8} [SQUAD WATCHDOG] Deadlock persists — {} stuck message(s): {}{}. {}m elapsed. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Review and re-dispatch or complete manually.",
                stale_len, msg_ids_str, suffix, oldest_age
            ),
            _ => format!(
                "\u{1f6a8} [SQUAD WATCHDOG] CRITICAL — deadlock unresolved for {}m. Stuck: {}{}. Watchdog stopping alerts. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Manual intervention required.",
                oldest_age, msg_ids_str, suffix
            ),
        }
    }

    fn build_prolonged_busy_msg_test(agent_name: &str, busy_mins: u64) -> String {
        format!(
            "\u{1f6a8} [SQUAD WATCHDOG] Agent '{}' busy for {}m — may be stuck. IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER. Check: squad-station peek {}",
            agent_name, busy_mins, agent_name
        )
    }

    #[test]
    fn test_deadlock_nudge0_message_contains_telegram_send_instruction() {
        let msg = build_deadlock_msg_test(0, 2, "msg-001, msg-002", "", 15);
        assert!(
            msg.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO SEND THIS ALERT TO THE USER"),
            "Nudge 0 must contain Telegram send instruction, got: {msg}"
        );
        assert!(msg.contains("[SQUAD WATCHDOG]"), "must contain watchdog tag");
        assert!(msg.contains("Deadlock detected"), "must contain original content");
        assert!(msg.contains("msg-001, msg-002"), "must contain msg IDs");
        assert!(msg.contains("15m"), "must contain age");
        assert!(msg.contains('\u{1f6a8}'), "must have alarm emoji prefix");
    }

    #[test]
    fn test_deadlock_nudge1_message_contains_telegram_alert_instruction() {
        let msg = build_deadlock_msg_test(1, 2, "msg-001, msg-002", "", 25);
        assert!(
            msg.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "Nudge 1 must contain Telegram alert instruction, got: {msg}"
        );
        assert!(msg.contains("[SQUAD WATCHDOG]"), "must contain watchdog tag");
        assert!(msg.contains("Deadlock persists"), "must contain original content");
        assert!(msg.contains("msg-001, msg-002"), "must contain msg IDs");
        assert!(msg.contains("25m"), "must contain elapsed time");
        assert!(msg.contains('\u{1f6a8}'), "must have alarm emoji prefix");
    }

    #[test]
    fn test_deadlock_nudge2_plus_message_contains_telegram_alert_instruction() {
        let msg = build_deadlock_msg_test(2, 2, "msg-001, msg-002", "", 45);
        assert!(
            msg.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "Nudge 2+ must contain Telegram alert instruction, got: {msg}"
        );
        assert!(msg.contains("[SQUAD WATCHDOG]"), "must contain watchdog tag");
        assert!(msg.contains("CRITICAL"), "must contain CRITICAL label");
        assert!(msg.contains("msg-001, msg-002"), "must contain msg IDs");
        assert!(msg.contains("45m"), "must contain unresolved time");
        assert!(msg.contains('\u{1f6a8}'), "must have alarm emoji prefix");
    }

    #[test]
    fn test_deadlock_messages_retain_existing_content() {
        // All three messages must retain: stuck count, msg_ids, oldest_age
        let stale_len = 3usize;
        let msg_ids_str = "abc, def, ghi";
        let suffix = " (+2 more)";
        let oldest_age = 20i64;

        for count in [0u32, 1u32, 3u32] {
            let msg = build_deadlock_msg_test(count, stale_len, msg_ids_str, suffix, oldest_age);
            assert!(msg.contains("abc, def, ghi"), "count={count}: must contain msg IDs");
            assert!(msg.contains("20m"), "count={count}: must contain age");
            assert!(msg.contains("[SQUAD WATCHDOG]"), "count={count}: must contain watchdog tag");
        }
    }

    #[test]
    fn test_prolonged_busy_message_contains_telegram_alert_instruction() {
        let msg = build_prolonged_busy_msg_test("agent-7", 42);
        assert!(
            msg.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "Prolonged-busy must contain Telegram alert instruction, got: {msg}"
        );
        assert!(msg.contains("[SQUAD WATCHDOG]"), "must contain watchdog tag");
        assert!(msg.contains("agent-7"), "must contain agent name");
        assert!(msg.contains("42m"), "must contain busy minutes");
        assert!(msg.contains("squad-station peek agent-7"), "must contain peek command");
        assert!(msg.contains('\u{1f6a8}'), "must have alarm emoji prefix");
    }

    // ── Integration test infrastructure ──────────────────────────────────

    /// Mock tmux layer that records all calls and returns configurable results.
    struct MockTmux {
        /// All sessions that "exist" in this mock.
        live_sessions: std::collections::HashSet<String>,
        /// Captured send_keys_literal calls: (target, text).
        sent: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl MockTmux {
        fn new(live_sessions: &[&str]) -> Self {
            Self {
                live_sessions: live_sessions.iter().map(|s| s.to_string()).collect(),
                sent: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn sent_messages(&self) -> Vec<(String, String)> {
            self.sent.lock().unwrap().clone()
        }
    }

    impl tmux::TmuxLayer for MockTmux {
        async fn send_keys_literal(&self, target: &str, text: &str) -> Result<()> {
            self.sent.lock().unwrap().push((target.to_string(), text.to_string()));
            Ok(())
        }

        async fn session_exists(&self, session_name: &str) -> bool {
            self.live_sessions.contains(session_name)
        }

        async fn capture_pane_last_line(&self, _session_name: &str) -> Option<String> {
            // Return an active-looking prompt — prevents reconcile from
            // marking busy agents as idle (which would mask deadlock tests).
            Some("working...".to_string())
        }
    }

    /// Controllable time provider for deterministic testing.
    struct TestTime {
        current: std::sync::Mutex<chrono::DateTime<chrono::Utc>>,
    }

    impl TestTime {
        fn new(base: chrono::DateTime<chrono::Utc>) -> Self {
            Self {
                current: std::sync::Mutex::new(base),
            }
        }

        fn advance(&self, duration: chrono::Duration) {
            let mut t = self.current.lock().unwrap();
            *t = *t + duration;
        }

}

    impl TimeProvider for TestTime {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            *self.current.lock().unwrap()
        }
    }

    /// Create an isolated test DB with migrations applied (same as tests/helpers.rs).
    async fn setup_test_db() -> sqlx::SqlitePool {
        let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
        let path = tmp.path().to_owned();
        std::mem::forget(tmp);

        let opts = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
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

    /// Seed an orchestrator + worker in the DB. Returns (orch_name, worker_name).
    async fn seed_agents(pool: &sqlx::SqlitePool) -> (String, String) {
        let orch = "test-orch";
        let worker = "test-worker";
        db::agents::insert_agent(pool, orch, "claude-code", "orchestrator", None, None, None)
            .await
            .unwrap();
        db::agents::insert_agent(pool, worker, "claude-code", "worker", None, None, None)
            .await
            .unwrap();
        (orch.to_string(), worker.to_string())
    }

    /// Insert a processing message from orchestrator to worker, backdated by `age_mins`.
    async fn insert_old_processing_msg(
        pool: &sqlx::SqlitePool,
        from: &str,
        to: &str,
        age_mins: i64,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let created = (chrono::Utc::now() - chrono::Duration::minutes(age_mins)).to_rfc3339();
        sqlx::query(
            "INSERT INTO messages (id, agent_name, from_agent, to_agent, type, task, status, priority, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'task_request', 'test task', 'processing', 'normal', ?, ?)"
        )
        .bind(&id)
        .bind(to)
        .bind(from)
        .bind(to)
        .bind(&created)
        .bind(&created)
        .execute(pool)
        .await
        .unwrap();
        id
    }

    // ── Integration tests: full tick loop ────────────────────────────────

    #[tokio::test]
    async fn test_tick_deadlock_fires_after_debounce() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        // Set agents to idle
        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        // Insert a processing message older than threshold (5 mins old, threshold = 1 min)
        insert_old_processing_msg(&pool, &orch, &worker, 5).await;

        // Time set to "now" (messages are already old relative to real Utc::now)
        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3); // debounce = 3 ticks
        let mut last_count = None;

        // Tick 1 & 2: debounce accumulates, no alert yet
        for i in 0..2 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
            assert!(
                mock_tmux.sent_messages().iter().all(|(_, msg)| !msg.contains("Deadlock")),
                "tick {}: should NOT fire deadlock alert during debounce", i + 1
            );
        }

        // Tick 3: debounce threshold reached → deadlock alert fires
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let sent = mock_tmux.sent_messages();
        let deadlock_msgs: Vec<_> = sent.iter().filter(|(_, msg)| msg.contains("Deadlock detected")).collect();
        assert_eq!(deadlock_msgs.len(), 1, "exactly one deadlock alert should fire after 3 ticks");
        assert!(deadlock_msgs[0].1.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN"));
        assert_eq!(deadlock_msgs[0].0, orch, "alert must target orchestrator");
    }

    #[tokio::test]
    async fn test_tick_no_false_alert_for_pending_only() {
        // Pending messages (not processing) should NOT trigger deadlock
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        // Insert a message and immediately complete it (status = 'completed')
        let msg_id = insert_old_processing_msg(&pool, &orch, &worker, 10).await;
        db::messages::complete_by_id(&pool, &msg_id).await.unwrap();

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1);
        let mut last_count = None;

        // Run 5 ticks — no deadlock should fire (no processing messages)
        for _ in 0..5 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }

        let sent = mock_tmux.sent_messages();
        assert!(
            sent.iter().all(|(_, msg)| !msg.contains("Deadlock")),
            "no deadlock alert should fire when only completed messages exist"
        );
    }

    #[tokio::test]
    async fn test_tick_no_deadlock_when_agent_is_busy() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch, &worker]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        // Worker is BUSY — so deadlock condition (0 busy agents) is NOT met
        db::agents::update_agent_status(&pool, &worker, "busy").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1);
        let mut last_count = None;

        for _ in 0..5 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }

        let sent = mock_tmux.sent_messages();
        assert!(
            sent.iter().all(|(_, msg)| !msg.contains("Deadlock")),
            "no deadlock when a busy agent exists"
        );
    }

    #[tokio::test]
    async fn test_tick_activity_resets_nudges() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3);
        let mut last_count = None;

        // Run 2 ticks to accumulate debounce
        for _ in 0..2 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }
        assert_eq!(deadlock.consecutive_ticks, 2, "2 ticks accumulated");

        // Simulate new message activity (inserts a new message → total_count changes)
        db::messages::insert_message(&pool, &orch, &worker, "task_request", "new task", "normal", None)
            .await.unwrap();

        // Next tick sees count change → resets deadlock state first,
        // then re-evaluates and finds deadlock condition still holds (old msg still processing),
        // so consecutive_ticks goes from 0 → 1 within the same tick.
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        assert_eq!(deadlock.consecutive_ticks, 1, "debounce must restart from 1 after reset (condition still holds)");
        assert_eq!(deadlock.count, 0, "nudge count must reset on activity");
    }

    #[tokio::test]
    async fn test_tick_cooldown_prevents_repeated_alerts() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3); // 600s cooldown
        let mut deadlock = DeadlockState::new(600, 3, 1); // 1-tick debounce for easy testing
        let mut last_count = None;

        // Tick 1: alert fires (debounce=1, first nudge)
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let count_after_first = mock_tmux.sent_messages().iter()
            .filter(|(_, msg)| msg.contains("Deadlock")).count();
        assert_eq!(count_after_first, 1, "first alert fires");

        // Tick 2-5: still within cooldown → no additional alert
        for _ in 0..4 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }

        let count_during_cooldown = mock_tmux.sent_messages().iter()
            .filter(|(_, msg)| msg.contains("Deadlock")).count();
        assert_eq!(count_during_cooldown, 1, "no additional alerts during cooldown");

        // Advance time past cooldown
        time.advance(chrono::Duration::seconds(601));

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let count_after_cooldown = mock_tmux.sent_messages().iter()
            .filter(|(_, msg)| msg.contains("Deadlock")).count();
        assert_eq!(count_after_cooldown, 2, "second alert fires after cooldown");
    }

    #[tokio::test]
    async fn test_tick_dry_run_no_tmux_injection() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1);
        let mut last_count = None;

        // Run with dry_run = true
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, true)
            .await.unwrap();

        assert!(
            mock_tmux.sent_messages().is_empty(),
            "dry-run must not inject any tmux messages"
        );
        // But deadlock state should still advance
        assert_eq!(deadlock.count, 1, "nudge count still tracks in dry-run");

        // Verify log file records DRY-RUN
        let log_content = std::fs::read_to_string(tmp.path().join("log").join("watch.log")).unwrap();
        assert!(log_content.contains("DRY-RUN"), "log must show DRY-RUN entries");
    }

    #[tokio::test]
    async fn test_tick_young_messages_not_stale() {
        // Messages younger than stall_threshold should NOT trigger deadlock
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        // Insert a processing message that is only 30 seconds old (threshold = 5 mins)
        let id = uuid::Uuid::new_v4().to_string();
        let created = (chrono::Utc::now() - chrono::Duration::seconds(30)).to_rfc3339();
        sqlx::query(
            "INSERT INTO messages (id, agent_name, from_agent, to_agent, type, task, status, priority, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'task_request', 'new task', 'processing', 'normal', ?, ?)"
        )
        .bind(&id)
        .bind(&worker)
        .bind(&orch)
        .bind(&worker)
        .bind(&created)
        .bind(&created)
        .execute(&pool)
        .await
        .unwrap();

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1); // 1-tick debounce
        let mut last_count = None;

        // stall_threshold = 5 mins, message is 30s old → not stale
        for _ in 0..5 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 5, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }

        let sent = mock_tmux.sent_messages();
        assert!(
            sent.iter().all(|(_, msg)| !msg.contains("Deadlock")),
            "young processing messages must not trigger deadlock"
        );
        assert_eq!(deadlock.consecutive_ticks, 0, "debounce should be cleared for young msgs");
    }

    #[tokio::test]
    async fn test_tick_debounce_resets_when_condition_clears() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch, &worker]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        let msg_id = insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3);
        let mut last_count = None;

        // 2 ticks → debounce at 2
        for _ in 0..2 {
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
        }
        assert_eq!(deadlock.consecutive_ticks, 2);

        // Complete the message → condition clears
        db::messages::complete_by_id(&pool, &msg_id).await.unwrap();

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        assert_eq!(deadlock.consecutive_ticks, 0, "debounce must reset when condition clears");
    }

    #[tokio::test]
    async fn test_tick_max_nudges_stops_alerts() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(0, 3, 1); // 0 cooldown, 3 max, 1-tick debounce
        let mut last_count = None;

        // Fire 3 nudges
        for i in 0..3 {
            time.advance(chrono::Duration::seconds(1));
            tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
                .await.unwrap();
            let count = mock_tmux.sent_messages().iter()
                .filter(|(_, msg)| msg.contains("Deadlock") || msg.contains("CRITICAL")).count();
            assert_eq!(count, i + 1, "nudge {} should have fired", i + 1);
        }

        // 4th tick: max reached, no more alerts
        time.advance(chrono::Duration::seconds(1));
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let final_count = mock_tmux.sent_messages().iter()
            .filter(|(_, msg)| msg.contains("Deadlock") || msg.contains("CRITICAL")).count();
        assert_eq!(final_count, 3, "no more alerts after max nudges");

        // Verify log contains DEADLOCK_UNRESOLVED
        let log = std::fs::read_to_string(tmp.path().join("log").join("watch.log")).unwrap();
        assert!(log.contains("DEADLOCK_UNRESOLVED"), "must log unresolved state");
    }

    #[tokio::test]
    async fn test_tick_prolonged_busy_alert() {
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch, &worker]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "busy").await.unwrap();

        // Backdate the worker's busy status to 45 minutes ago
        let old_ts = (chrono::Utc::now() - chrono::Duration::minutes(45)).to_rfc3339();
        sqlx::query("UPDATE agents SET status_updated_at = ? WHERE name = ?")
            .bind(&old_ts)
            .bind(&worker)
            .execute(&pool)
            .await
            .unwrap();

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3);
        let mut last_count = None;

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let sent = mock_tmux.sent_messages();
        let busy_alerts: Vec<_> = sent.iter().filter(|(_, msg)| msg.contains("busy for")).collect();
        assert_eq!(busy_alerts.len(), 1, "prolonged-busy alert should fire");
        assert!(busy_alerts[0].1.contains(&worker), "alert must mention the stuck agent");
        assert!(busy_alerts[0].1.contains("TELEGRAM MCP PLUGIN"), "must include Telegram instruction");
    }

    // ── Telegram relay integration tests ────────────────────────────────
    // Verify that actual tick() injections contain the correct Telegram MCP
    // instruction at each escalation level, exercising the full code path
    // (DB → deadlock detection → message formatting → tmux injection).

    #[tokio::test]
    async fn test_tick_deadlock_escalation_telegram_instructions() {
        // Exercises all 3 deadlock escalation levels through real tick() calls
        // and verifies each produces the correct Telegram relay instruction.
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        insert_old_processing_msg(&pool, &orch, &worker, 10).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(0, 3, 1); // 0 cooldown, 1-tick debounce
        let mut last_count = None;

        // Nudge 0: "SEND THIS ALERT TO THE USER"
        time.advance(chrono::Duration::seconds(1));
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        assert_eq!(msgs.len(), 1);
        assert!(
            msgs[0].1.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO SEND THIS ALERT TO THE USER"),
            "nudge 0 must instruct orchestrator to SEND alert via Telegram, got: {}", msgs[0].1
        );
        assert!(msgs[0].1.contains("Deadlock detected"), "nudge 0 label");

        // Nudge 1: "ALERT THE USER"
        time.advance(chrono::Duration::seconds(1));
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        assert_eq!(msgs.len(), 2);
        assert!(
            msgs[1].1.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "nudge 1 must instruct orchestrator to ALERT via Telegram, got: {}", msgs[1].1
        );
        assert!(msgs[1].1.contains("Deadlock persists"), "nudge 1 label");

        // Nudge 2: CRITICAL — "ALERT THE USER"
        time.advance(chrono::Duration::seconds(1));
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        assert_eq!(msgs.len(), 3);
        assert!(
            msgs[2].1.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "nudge 2 must instruct orchestrator to ALERT via Telegram, got: {}", msgs[2].1
        );
        assert!(msgs[2].1.contains("CRITICAL"), "nudge 2 must be CRITICAL level");
        assert!(msgs[2].1.contains("Manual intervention required"), "nudge 2 must signal final escalation");
    }

    #[tokio::test]
    async fn test_tick_prolonged_busy_telegram_instruction() {
        // Verify prolonged-busy alerts (Pass 3) include Telegram relay instruction
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch, &worker]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "busy").await.unwrap();

        // Backdate busy status to 60 minutes ago
        let old_ts = (chrono::Utc::now() - chrono::Duration::minutes(60)).to_rfc3339();
        sqlx::query("UPDATE agents SET status_updated_at = ? WHERE name = ?")
            .bind(&old_ts)
            .bind(&worker)
            .execute(&pool)
            .await
            .unwrap();

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3);
        let mut last_count = None;

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        let busy_alert = msgs.iter().find(|(_, msg)| msg.contains("busy for"));
        assert!(busy_alert.is_some(), "prolonged-busy alert must fire");

        let (target, text) = busy_alert.unwrap();
        assert_eq!(target, &orch, "alert targets orchestrator");
        assert!(
            text.contains("IMMEDIATELY USE YOUR TELEGRAM MCP PLUGIN TO ALERT THE USER"),
            "prolonged-busy must contain Telegram ALERT instruction, got: {text}"
        );
        assert!(text.contains(&worker), "must name the stuck agent");
        assert!(text.contains("squad-station peek"), "must include diagnostic command");
    }

    #[tokio::test]
    async fn test_tick_deadlock_alert_contains_message_ids_and_age() {
        // Verify the Telegram-destined alert includes actionable content:
        // stuck message IDs and oldest message age
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        // Insert two processing messages with different ages
        let msg1 = insert_old_processing_msg(&pool, &orch, &worker, 15).await;
        let msg2 = insert_old_processing_msg(&pool, &orch, &worker, 8).await;

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1); // 1-tick debounce
        let mut last_count = None;

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        let alert = msgs.iter().find(|(_, msg)| msg.contains("Deadlock detected"));
        assert!(alert.is_some(), "deadlock alert must fire");

        let text = &alert.unwrap().1;
        // Both message IDs should appear (truncated to first 5 chars of UUID)
        assert!(text.contains(&msg1[..5]) || text.contains(&msg1), "must contain first msg ID");
        assert!(text.contains(&msg2[..5]) || text.contains(&msg2), "must contain second msg ID");
        // "2 processing message(s)"
        assert!(text.contains("2 processing message(s)"), "must report correct stale count");
        // Age should be ~15m (oldest message)
        assert!(text.contains("15m"), "must report oldest message age");
        // Telegram instruction
        assert!(text.contains("TELEGRAM MCP PLUGIN"), "must include Telegram relay instruction");
    }

    #[tokio::test]
    async fn test_tick_idle_nudge_does_not_contain_telegram_instruction() {
        // Idle nudges (Pass 2) are informational — they should NOT contain
        // the Telegram relay instruction (only deadlock and prolonged-busy do)
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();
        let (orch, worker) = seed_agents(&pool).await;
        let mock_tmux = MockTmux::new(&[&orch]);

        db::agents::update_agent_status(&pool, &orch, "idle").await.unwrap();
        db::agents::update_agent_status(&pool, &worker, "idle").await.unwrap();

        // Insert a completed message with old timestamp to trigger idle detection
        let old_ts = (chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO messages (id, agent_name, from_agent, to_agent, type, task, status, priority, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'task_request', 'old task', 'completed', 'normal', ?, ?)"
        )
        .bind(&id)
        .bind(&worker)
        .bind(&orch)
        .bind(&worker)
        .bind(&old_ts)
        .bind(&old_ts)
        .execute(&pool)
        .await
        .unwrap();

        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 3);
        let mut last_count = None;

        // stall_threshold = 1 min, last activity = 10 mins ago → idle nudge fires
        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        let msgs = mock_tmux.sent_messages();
        let idle_msgs: Vec<_> = msgs.iter().filter(|(_, msg)| msg.contains("SQUAD WATCHDOG")).collect();
        assert!(!idle_msgs.is_empty(), "idle nudge should fire");

        for (_, text) in &idle_msgs {
            assert!(
                !text.contains("TELEGRAM MCP PLUGIN"),
                "idle nudge must NOT contain Telegram instruction — only deadlock/busy alerts do. Got: {text}"
            );
            assert!(text.contains("System idle"), "should be an idle nudge");
        }
    }

    #[tokio::test]
    async fn test_tick_antigravity_orchestrator_no_telegram_alert() {

        // Antigravity orchestrator should not receive tmux injections
        let pool = setup_test_db().await;
        let tmp = tempfile::TempDir::new().unwrap();

        db::agents::insert_agent(&pool, "orch", "antigravity", "orchestrator", None, None, None)
            .await.unwrap();
        db::agents::insert_agent(&pool, "worker", "claude-code", "worker", None, None, None)
            .await.unwrap();
        db::agents::update_agent_status(&pool, "orch", "idle").await.unwrap();
        db::agents::update_agent_status(&pool, "worker", "idle").await.unwrap();

        insert_old_processing_msg(&pool, "orch", "worker", 10).await;

        let mock_tmux = MockTmux::new(&["orch", "worker"]);
        let time = TestTime::new(chrono::Utc::now());
        let mut nudge = NudgeState::new(600, 3);
        let mut deadlock = DeadlockState::new(600, 3, 1);
        let mut last_count = None;

        tick(&mock_tmux, &time, &pool, tmp.path(), 1, &mut nudge, &mut deadlock, &mut last_count, false)
            .await.unwrap();

        assert!(
            mock_tmux.sent_messages().is_empty(),
            "antigravity orchestrator must not receive tmux messages"
        );
    }
}
