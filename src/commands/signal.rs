use owo_colors::OwoColorize;
use std::io::IsTerminal;

use crate::{config, db, tmux};

/// Append a structured log line to `.squad/log/signal.log`.
/// Best-effort: silently ignores write failures. Must never cause signal to fail.
fn log_signal(project_root: &std::path::Path, level: &str, agent: &str, msg: &str) {
    let log_dir = project_root.join(".squad").join("log");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = log_dir.join("signal.log");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        use std::io::Write;
        let _ = writeln!(
            f,
            "{} {:<5} agent={} {}",
            chrono::Utc::now().to_rfc3339(),
            level,
            agent,
            msg
        );
        // Log rotation: truncate to last 500 lines when file exceeds 1MB
        if let Ok(meta) = std::fs::metadata(&log_file) {
            if meta.len() > 1_048_576 {
                rotate_log(&log_file);
            }
        }
    }
}

/// Truncate log file to last 500 lines.
fn rotate_log(path: &std::path::Path) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() > 500 {
            let tail = &lines[lines.len() - 500..];
            let _ = std::fs::write(path, tail.join("\n") + "\n");
        }
    }
}

/// Output empty JSON for Gemini CLI hook compatibility when exiting early.
/// Gemini CLI requires valid JSON on stdout from hooks — empty output = hook failure.
fn emit_empty_json_if_hook() {
    if !std::io::stdout().is_terminal() {
        print!("{{}}");
    }
}

pub async fn run(agent: Option<String>, json: bool) -> anyhow::Result<()> {
    // GUARD 1: No explicit agent name provided -- silent exit 0 (HOOK-03)
    // The hook command passes the session name explicitly via $SQUAD_AGENT_NAME.
    // If no name is provided (e.g. outside tmux, in CI), we silently exit.
    let agent: String = match agent {
        Some(name) => name,
        None => {
            emit_empty_json_if_hook();
            return Ok(());
        }
    };

    // GUARD 2: Config/DB connection -- warning to stderr + exit 0 on failure
    // Per locked decision: real errors go to stderr but NEVER fail the provider (exit 0 always).
    let config_path = std::path::Path::new(crate::config::DEFAULT_CONFIG_FILE);
    let config = match config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("squad-station: warning: {e}");
            emit_empty_json_if_hook();
            return Ok(());
        }
    };
    let db_path = match config::resolve_db_path(&config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("squad-station: warning: {e}");
            emit_empty_json_if_hook();
            return Ok(());
        }
    };

    // Resolve project root for logging (DB is at <root>/.squad/station.db)
    let project_root = db_path
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    let pool = match db::connect(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("squad-station: warning: DB connection failed: {e}");
            log_signal(
                &project_root,
                "GUARD",
                &agent,
                "reason=db_connection_failed",
            );
            emit_empty_json_if_hook();
            return Ok(());
        }
    };

    // GUARD 3: Agent not registered -- silent exit 0 in hook context (HOOK-03),
    // but print a message when running interactively so manual usage isn't confusing.
    // Uses match instead of ? to maintain exit-0 contract (never fail the provider).
    let agent_record = match db::agents::get_agent(&pool, &agent).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            if std::io::stdout().is_terminal() {
                println!("Agent not found: {} (ignored)", agent);
            }
            log_signal(&project_root, "GUARD", &agent, "reason=agent_not_found");
            emit_empty_json_if_hook();
            return Ok(());
        }
        Err(e) => {
            eprintln!("squad-station: warning: get_agent failed: {e}");
            log_signal(&project_root, "GUARD", &agent, "reason=get_agent_error");
            emit_empty_json_if_hook();
            return Ok(());
        }
    };

    // GUARD 4: Orchestrator self-signal -- silent exit 0 (HOOK-01)
    // Prevents infinite loop where the orchestrator's AfterAgent hook signals itself.
    if agent_record.role == "orchestrator" {
        log_signal(
            &project_root,
            "GUARD",
            &agent,
            "reason=orchestrator_self_signal",
        );
        emit_empty_json_if_hook();
        return Ok(());
    }

    // --- Signal flow: use current_task FK for targeted completion ---

    let (rows, task_id): (u64, Option<String>) = if let Some(ref ct_id) = agent_record.current_task
    {
        // Primary path: complete the specific task pointed to by current_task
        let r = db::messages::complete_by_id(&pool, ct_id).await?;
        if r > 0 {
            log_signal(
                &project_root,
                "OK",
                &agent,
                &format!("task={} method=current_task", ct_id),
            );
            (r, Some(ct_id.clone()))
        } else {
            // current_task exists but message is already completed (duplicate signal)
            log_signal(
                &project_root,
                "OK",
                &agent,
                &format!("task={} rows=0 reason=already_completed_or_missing", ct_id),
            );
            (0, None)
        }
    } else {
        // current_task is NULL — fallback to FIFO (edge case: task sent but current_task race)
        let fifo_rows = db::messages::update_status(&pool, &agent).await?;
        if fifo_rows > 0 {
            // Retrieve the task that FIFO just completed
            let tid = db::messages::last_completed_id(&pool, &agent).await?;
            log_signal(
                &project_root,
                "WARN",
                &agent,
                &format!(
                    "task={} method=fifo_fallback reason=current_task_null",
                    tid.as_deref().unwrap_or("unknown")
                ),
            );
            (fifo_rows, tid)
        } else {
            log_signal(
                &project_root,
                "OK",
                &agent,
                "task=none reason=no_current_task_no_processing",
            );
            (0, None)
        }
    };

    // Find orchestrator and notify (only on actual state change).
    // GUARD 5: Skip notification when agent is frozen (user is in control).
    let orchestrator_notified = if rows > 0 && agent_record.status != "frozen" {
        let orchestrator = db::agents::get_orchestrator(&pool).await?;
        if let Some(orch) = orchestrator {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            let notification = format!(
                "[SQUAD SIGNAL] Agent '{}' completed task {}. Read output: tmux capture-pane -t {} -p | Next: squad-station status",
                agent, task_id_str, agent
            );
            if orch.tool == "antigravity" {
                // DB-only orchestrator: polls DB for completions, no push notification needed.
                log_signal(
                    &project_root,
                    "OK",
                    &agent,
                    &format!("task={} notified=false reason=antigravity", task_id_str),
                );
                false
            } else if tmux::session_exists(&orch.name).await {
                // Only notify if orchestrator tmux session is running.
                match tmux::send_keys_literal(&orch.name, &notification).await {
                    Ok(()) => {
                        log_signal(
                            &project_root,
                            "OK",
                            &agent,
                            &format!("task={} notified=true orch={}", task_id_str, orch.name),
                        );
                        true
                    }
                    Err(e) => {
                        log_signal(
                            &project_root,
                            "WARN",
                            &agent,
                            &format!(
                                "task={} notified=false reason=send_keys_failed err={}",
                                task_id_str, e
                            ),
                        );
                        false
                    }
                }
            } else {
                log_signal(
                    &project_root,
                    "WARN",
                    &agent,
                    &format!(
                        "task={} notified=false reason=orch_session_missing orch={}",
                        task_id_str, orch.name
                    ),
                );
                false
            }
        } else {
            // No orchestrator registered — signal is persisted in DB only.
            false
        }
    } else {
        false
    };

    // After successful signal, check remaining tasks and update agent status accordingly.
    if rows > 0 {
        let remaining = db::messages::count_processing(&pool, &agent).await?;
        if remaining > 0 {
            // Still has processing tasks — update current_task to next task, stay busy
            let next = db::messages::peek_message(&pool, &agent).await?;
            if let Some(next_msg) = next {
                db::agents::set_current_task(&pool, &agent, &next_msg.id).await?;
            }
            // Agent remains busy — don't change status
        } else {
            // No remaining tasks — clear current_task and set idle
            db::agents::clear_current_task(&pool, &agent).await?;
            db::agents::update_agent_status(&pool, &agent, "idle").await?;
        }
    }

    // Output result
    if json {
        let out = serde_json::json!({
            "signaled": true,
            "agent": agent,
            "task_id": task_id,
            "orchestrator_notified": orchestrator_notified,
        });
        println!("{}", serde_json::to_string(&out)?);
    } else if std::io::stdout().is_terminal() {
        // Interactive terminal: human-readable output
        if rows > 0 {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            println!(
                "{} Signaled completion for {} (task_id={})",
                "✓".green(),
                agent,
                task_id_str
            );
        } else {
            println!(
                "{} Signal acknowledged (no pending task for {})",
                "✓".green(),
                agent
            );
        }
    } else {
        // Hook context (non-TTY): output empty JSON for Gemini CLI compatibility.
        // Gemini CLI requires valid JSON on stdout from hooks.
        // Human-readable info goes to stderr for logging.
        if rows > 0 {
            let task_id_str = task_id.as_deref().unwrap_or("unknown");
            eprintln!(
                "Signaled completion for {} (task_id={})",
                agent, task_id_str
            );
        } else {
            eprintln!("Signal acknowledged (no pending task for {})", agent);
        }
        print!("{{}}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotate_log_truncates_to_500_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log_file = tmp.path().join("test.log");

        // Write 1000 lines
        let mut content = String::new();
        for i in 0..1000 {
            content.push_str(&format!("line {}\n", i));
        }
        std::fs::write(&log_file, &content).unwrap();

        rotate_log(&log_file);

        let result = std::fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 500);
        assert!(lines[0].contains("line 500"));
        assert!(lines[499].contains("line 999"));
    }

    #[test]
    fn test_log_signal_creates_directory_and_writes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_root = tmp.path();

        log_signal(project_root, "OK", "test-agent", "task=abc123 rows=1");

        let log_file = project_root.join(".squad").join("log").join("signal.log");
        assert!(log_file.exists(), "Log file must be created");

        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("OK"));
        assert!(content.contains("agent=test-agent"));
        assert!(content.contains("task=abc123 rows=1"));
    }

    #[test]
    fn test_log_signal_appends_not_overwrites() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_root = tmp.path();

        log_signal(project_root, "OK", "agent-a", "first");
        log_signal(project_root, "WARN", "agent-b", "second");

        let log_file = project_root.join(".squad").join("log").join("signal.log");
        let content = std::fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("agent-a"));
        assert!(lines[1].contains("agent-b"));
    }
}
