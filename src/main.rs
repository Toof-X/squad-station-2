use anyhow::Result;
use clap::Parser;

// Re-use the library crate's modules so the binary and integration tests share the same code.
use squad_station::{cli, commands};

#[tokio::main]
async fn main() {
    // SAFE-04: Reset SIGPIPE to default behavior before any I/O
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = cli::Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

async fn run(cli: cli::Cli) -> Result<()> {
    match cli.command {
        None => {
            use std::io::IsTerminal;
            use std::path::PathBuf;
            if cli.tui && std::io::stdin().is_terminal() {
                let has_config = std::path::Path::new("squad.yml").exists();
                let action = commands::welcome::run_welcome_tui(has_config).await?;
                match action {
                    Some(commands::welcome::WelcomeAction::LaunchInit) => {
                        commands::init::run(PathBuf::from("squad.yml"), false, true).await?;
                    }
                    Some(commands::welcome::WelcomeAction::LaunchDashboard) => {
                        commands::ui::run().await?;
                    }
                    _ => {}
                }
            } else {
                commands::welcome::print_welcome();
            }
            Ok(())
        }
        Some(cmd) => {
            use cli::Commands::*;
            match cmd {
                Init { config, tui } => commands::init::run(config, cli.json, tui).await,
                Send {
                    agent,
                    body,
                    priority,
                    thread,
                } => commands::send::run(agent, body, priority, cli.json, thread).await,
                Signal { agent } => commands::signal::run(agent, cli.json).await,
                Notify { body, agent } => commands::notify::run(body, agent, cli.json).await,
                List {
                    agent,
                    status,
                    limit,
                } => commands::list::run(agent, status, limit, cli.json).await,
                Peek { agent } => commands::peek::run(agent, cli.json).await,
                Register { name, role, tool } => {
                    commands::register::run(name, role, tool, cli.json).await
                }
                Clone { agent } => commands::clone::run(agent, cli.json).await,
                Agents => commands::agents::run(cli.json).await,
                Fleet => commands::fleet::run(cli.json).await,
                Context { inject } => commands::context::run(inject).await,
                Status => commands::status::run(cli.json).await,
                Ui => commands::ui::run().await,
                Monitor => commands::monitor::run().await,
                Open => commands::open::run().await,
                View => commands::view::run(cli.json).await,
                Close { config } => commands::close::run(config, cli.json).await,
                Reset {
                    config,
                    no_relaunch,
                } => commands::reset::run(config, no_relaunch, cli.json).await,
                Reconcile { dry_run } => commands::reconcile::run(dry_run, cli.json).await,
                Freeze => commands::freeze::run_freeze(cli.json).await,
                Unfreeze => commands::freeze::run_unfreeze(cli.json).await,
                Watch {
                    interval,
                    stall_threshold,
                    daemon,
                    stop,
                } => commands::watch::run(interval, stall_threshold, daemon, stop).await,
                Clean { config, yes, all } => commands::clean::run(config, yes, all, cli.json).await,
            }
        }
    }
}
