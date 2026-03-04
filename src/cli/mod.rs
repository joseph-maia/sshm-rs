// CLI module - command parsing and dispatch
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sshm-rs",
    about = "SSH Manager - A modern SSH connection manager",
    long_about = "SSHM-RS is a modern SSH manager for your terminal.\n\n\
        Main usage:\n  \
        Running 'sshm-rs' (without arguments) opens the interactive TUI.\n  \
        Running 'sshm-rs <host>' connects directly to the specified host.\n  \
        Running 'sshm-rs <host> <command...>' executes a command on the remote host.",
    version
)]
pub struct Cli {
    /// Host to connect to directly
    pub host: Option<String>,

    /// Command (and arguments) to execute on remote host
    #[arg(trailing_var_arg = true)]
    pub command: Vec<String>,

    /// Force pseudo-TTY allocation (useful for interactive remote commands)
    #[arg(short = 't', long = "tty")]
    pub force_tty: bool,

    /// SSH config file to use (default: ~/.ssh/config)
    #[arg(short = 'c', long = "config")]
    pub config_file: Option<String>,

    /// Focus on search input at startup (TUI mode)
    #[arg(short = 's', long = "search")]
    pub search_mode: bool,

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
    /// Search SSH hosts and display results
    Search {
        /// Search query
        query: String,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
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
        Some(Commands::Search { query }) => run_search(&query, cli.config_file.as_deref()),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "sshm-rs", &mut std::io::stdout());
            Ok(())
        }
        None => {
            if let Some(host_name) = cli.host {
                connect_to_host(
                    &host_name,
                    &cli.command,
                    cli.config_file.as_deref(),
                    cli.force_tty,
                )
            } else {
                // Interactive TUI
                crate::ui::run_tui()
            }
        }
    }
}

/// Search hosts and display in a formatted table
fn run_search(query: &str, config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;
    let query_lower = query.to_lowercase();

    let matches: Vec<_> = hosts
        .iter()
        .filter(|h| {
            h.name.to_lowercase().contains(&query_lower)
                || h.hostname.to_lowercase().contains(&query_lower)
                || h.user.to_lowercase().contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        println!("No hosts matching '{query}' found.");
        return Ok(());
    }

    // Calculate column widths
    let name_w = matches
        .iter()
        .map(|h| h.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let host_w = matches
        .iter()
        .map(|h| h.hostname.len())
        .max()
        .unwrap_or(8)
        .max(8);
    let user_w = matches
        .iter()
        .map(|h| h.user.len())
        .max()
        .unwrap_or(4)
        .max(4);

    // Header
    println!(
        "{:<name_w$}  {:<host_w$}  {:<user_w$}  PORT",
        "NAME", "HOSTNAME", "USER",
    );
    println!(
        "{:<name_w$}  {:<host_w$}  {:<user_w$}  ----",
        "----",
        "--------",
        "----",
        name_w = name_w,
        host_w = host_w,
        user_w = user_w,
    );

    for host in &matches {
        let port = if host.port.is_empty() {
            "22"
        } else {
            &host.port
        };
        let user = if host.user.is_empty() {
            "-"
        } else {
            &host.user
        };
        println!(
            "{:<name_w$}  {:<host_w$}  {:<user_w$}  {port}",
            host.name, host.hostname, user,
        );
    }

    println!("\n{} host(s) found.", matches.len());
    Ok(())
}

/// Connect to a host: verify it exists, record history, then exec ssh
fn connect_to_host(
    host_name: &str,
    remote_command: &[String],
    config_file: Option<&str>,
    force_tty: bool,
) -> Result<()> {
    // Check if the host exists in config
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;
    let host_exists = hosts.iter().any(|h| h.name == host_name);

    if !host_exists {
        eprintln!("Error: Host '{}' not found in SSH configuration.", host_name);
        eprintln!("Use 'sshm-rs search <query>' or run 'sshm-rs' to see available hosts.");
        std::process::exit(1);
    }

    // Record connection in history
    match crate::history::HistoryManager::load() {
        Ok(mut hm) => {
            if let Err(e) = hm.record_connection(host_name) {
                eprintln!("Warning: Could not record connection history: {e}");
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not initialize connection history: {e}");
        }
    }

    crate::connectivity::connect_ssh(host_name, remote_command, config_file, force_tty)
}
