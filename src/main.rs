use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod config;
mod db;
mod tmux;

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
    use cli::Commands::*;
    match cli.command {
        Init { config } => commands::init::run(config, cli.json).await,
        Send { agent, task, priority } => commands::send::run(agent, task, priority, cli.json).await,
        Signal { agent } => commands::signal::run(agent, cli.json).await,
        List { agent, status, limit } => commands::list::run(agent, status, limit, cli.json).await,
        Peek { agent } => commands::peek::run(agent, cli.json).await,
        Register { name, command, role, provider } => {
            commands::register::run(name, command, role, provider, cli.json).await
        }
    }
}
