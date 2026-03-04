// CLI module - command parsing and dispatch
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sshm-rs", about = "A beautiful TUI SSH connection manager")]
pub struct Cli {
    /// Host to connect to directly
    pub host: Option<String>,

    /// Command to execute on remote host
    pub command: Option<String>,

    #[command(subcommand)]
    pub subcommand: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new SSH host
    Add,
    /// Edit an existing SSH host
    Edit {
        /// Host name to edit
        host: String,
    },
    /// Search SSH hosts
    Search {
        /// Search query
        query: String,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.subcommand {
        Some(Commands::Add) => {
            println!("Add host - TUI form");
            Ok(())
        }
        Some(Commands::Edit { host }) => {
            println!("Edit host: {host}");
            Ok(())
        }
        Some(Commands::Search { query }) => {
            // CLI search mode
            let config_path = crate::config::default_ssh_config_path()?;
            let hosts = crate::config::parse_ssh_config(&config_path)?;
            let query_lower = query.to_lowercase();
            for host in hosts.iter().filter(|h| h.name.to_lowercase().contains(&query_lower)) {
                println!("{} -> {}@{}:{}", host.name, host.user, host.hostname, host.port);
            }
            Ok(())
        }
        None => {
            if let Some(host_name) = cli.host {
                // Direct connection
                crate::connectivity::connect_ssh(&host_name, cli.command.as_deref())
            } else {
                // Interactive TUI
                crate::ui::run_tui()
            }
        }
    }
}
