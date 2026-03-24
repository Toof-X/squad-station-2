use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Message routing and orchestration for AI agent squads
#[derive(Parser, Debug)]
#[command(name = "squad-station", version, about)]
pub struct Cli {
    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    /// Launch interactive welcome TUI (auto-launched by installer in TTY environments)
    #[arg(long)]
    pub tui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize squad from a config file
    Init {
        /// Path to squad config file
        #[arg(default_value = crate::config::DEFAULT_CONFIG_FILE)]
        config: PathBuf,
        /// Launch interactive TUI wizard instead of using existing squad.yml
        #[arg(long)]
        tui: bool,
    },
    /// Send a task to an agent
    Send {
        /// Agent name
        agent: String,
        /// Task body to send
        #[arg(long)]
        body: String,
        /// Task priority
        #[arg(long, value_enum, default_value = "normal")]
        priority: Priority,
        /// Thread ID to group related messages (omit to start a new thread)
        #[arg(long)]
        thread: Option<String>,
    },
    /// Signal agent completion
    Signal {
        /// Agent name or tmux pane ID (e.g. %3). Omit to auto-detect from $TMUX_PANE.
        agent: Option<String>,
    },
    /// Send a mid-task notification to orchestrator (agent needs input)
    Notify {
        /// Message to send
        #[arg(long)]
        body: String,
        /// Source agent name. Omit to auto-detect from tmux session name.
        #[arg(long)]
        agent: Option<String>,
    },
    /// List messages
    List {
        /// Filter by agent name
        #[arg(long)]
        agent: Option<String>,
        /// Filter by status (processing, completed)
        #[arg(long)]
        status: Option<String>,
        /// Maximum number of messages to show
        #[arg(long, default_value = "20")]
        limit: u32,
    },
    /// Peek at an agent's next pending task
    Peek {
        /// Agent name
        agent: String,
    },
    /// Register an agent at runtime
    Register {
        /// Agent name
        name: String,
        /// Agent role
        #[arg(long, default_value = "worker")]
        role: String,
        /// Agent tool label (e.g. claude-code, gemini)
        #[arg(long, default_value = "unknown")]
        tool: String, // CONF-04: renamed from provider
    },
    /// Clone an existing agent with auto-incremented name
    Clone {
        /// Source agent name to clone
        agent: String,
    },
    /// List agents with reconciled status
    Agents,
    /// Show fleet status — pending tasks, busy duration, alignment per agent
    Fleet,
    /// Generate orchestrator context file
    Context {
        /// Output context to stdout for SessionStart hook injection (orchestrator only)
        #[arg(long)]
        inject: bool,
    },
    /// Show project and agent status summary
    Status,
    /// Launch interactive TUI dashboard
    Ui,
    /// Interactive monitor — live agent pane output viewer
    Monitor,
    /// Attach to the monitor tmux session (tiled agent panes)
    Open,
    /// Open tmux tiled view of all live agent sessions
    View,
    /// Kill all tmux sessions and remove database (graceful teardown)
    Close {
        /// Path to squad config file
        #[arg(default_value = crate::config::DEFAULT_CONFIG_FILE)]
        config: PathBuf,
    },
    /// Kill all sessions and delete database, then relaunch
    Reset {
        /// Path to squad config file
        #[arg(default_value = crate::config::DEFAULT_CONFIG_FILE)]
        config: PathBuf,
        /// Skip relaunching sessions after reset
        #[arg(long)]
        no_relaunch: bool,
    },
    /// Detect and fix stuck agents (busy in DB but idle in tmux)
    Reconcile {
        /// Show what would be fixed without changing the database
        #[arg(long)]
        dry_run: bool,
    },
    /// Freeze all agents — block orchestrator from sending tasks (user takes control)
    Freeze,
    /// Unfreeze all agents — allow orchestrator to send tasks again
    Unfreeze,
    /// Watchdog daemon — auto-detect and fix stuck agents
    Watch {
        /// Poll interval in seconds
        #[arg(long, default_value = "30")]
        interval: u64,
        /// Minutes of system-wide idle before nudging orchestrator
        #[arg(long, default_value = "5")]
        stall_threshold: u64,
        /// Fork to background and write PID to .squad/watch.pid
        #[arg(long)]
        daemon: bool,
        /// Stop a running watchdog daemon
        #[arg(long)]
        stop: bool,
        /// Log stall detections without injecting into tmux or running reconciliation
        #[arg(long)]
        dry_run: bool,
        /// Show watchdog daemon status (PID, uptime, stall state)
        #[arg(long)]
        status: bool,
        /// Alert cooldown in seconds (minimum gap between repeated alerts for same condition)
        #[arg(long, default_value = "600")]
        cooldown: u64,
        /// Number of consecutive poll cycles a stall must persist before alerting
        #[arg(long, default_value = "3")]
        debounce: u32,
    },
    /// Kill all squad tmux sessions and delete the database
    Clean {
        /// Path to squad config file
        #[arg(default_value = crate::config::DEFAULT_CONFIG_FILE)]
        config: PathBuf,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
        /// Also delete .squad/log/ directory (by default, logs are preserved for post-mortem)
        #[arg(long)]
        all: bool,
    },
    /// Launch browser visualization (requires --features browser)
    Browser {
        /// Port to bind to (default: 3000, fallback to random if taken)
        #[arg(long)]
        port: Option<u16>,
        /// Skip auto-opening browser
        #[arg(long)]
        no_open: bool,
        /// Run server in background and return immediately
        #[arg(long)]
        detach: bool,
    },
}

/// Task priority level
#[derive(clap::ValueEnum, Clone, Debug, serde::Serialize)]
pub enum Priority {
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Urgent => write!(f, "urgent"),
        }
    }
}
