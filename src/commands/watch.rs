use anyhow::{bail, Result};

use crate::{commands::reconcile, config, db, tmux};

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

pub async fn run(
    interval_secs: u64,
    stall_threshold_mins: u64,
    daemon: bool,
    stop: bool,
) -> Result<()> {
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
                .arg(stall_threshold_mins.to_string());
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

    let mut nudge_state = NudgeState::new(600, 3); // 10min cooldown, 3 max nudges
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

    while is_running() {
        if let Err(e) = tick(
            &pool,
            &squad_dir,
            stall_threshold_mins,
            &mut nudge_state,
            &mut last_msg_count,
        )
        .await
        {
            log_watch(&squad_dir, "ERROR", &format!("tick failed: {}", e));
        }

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
    pool: &sqlx::SqlitePool,
    squad_dir: &std::path::Path,
    stall_threshold_mins: u64,
    nudge_state: &mut NudgeState,
    last_msg_count: &mut Option<i64>,
) -> Result<()> {
    // Pass 1: Individual agent reconciliation
    let results = reconcile::reconcile_agents(pool, false).await?;
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
                    let idle_duration = chrono::Utc::now().signed_duration_since(last_ts);
                    let idle_mins = idle_duration.num_minutes();

                    if idle_mins >= stall_threshold_mins as i64 {
                        let now = chrono::Utc::now();
                        if nudge_state.should_nudge(now) {
                            // Find orchestrator and nudge
                            if let Ok(Some(orch)) = db::agents::get_orchestrator(pool).await {
                                if orch.tool != "antigravity"
                                    && tmux::session_exists(&orch.name).await
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
                                    let _ = tmux::send_keys_literal(&orch.name, &msg).await;
                                    log_watch(
                                        squad_dir,
                                        "NUDGE",
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

    // Pass 3: Prolonged busy detection
    for agent in &agents {
        if agent.status == "busy" {
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&agent.status_updated_at) {
                let busy_mins = chrono::Utc::now().signed_duration_since(ts).num_minutes();
                if busy_mins > 30 {
                    log_watch(
                        squad_dir,
                        "WARN",
                        &format!(
                            "agent={} busy_minutes={} reason=prolonged_busy",
                            agent.name, busy_mins
                        ),
                    );
                }
            }
        }
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
}
