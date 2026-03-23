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
    /// Export hosts to JSON file
    Export {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import hosts from JSON file
    Import {
        /// Input file path
        file: String,
        /// Skip duplicate hosts instead of erroring
        #[arg(long)]
        skip_duplicates: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },
    /// Validate the SSH config file and report warnings
    Validate {
        /// SSH config file to validate (default: ~/.ssh/config)
        #[arg(short = 'c', long = "config")]
        config_file: Option<String>,
    },
    /// Manage the color theme
    Theme {
        #[command(subcommand)]
        action: Option<ThemeCommands>,
    },
    /// Download the latest version of sshm-rs
    Update,
    /// Open SSH terminal with integrated SFTP browser
    Term {
        /// Remote host (user@host or host)
        host: String,
        /// Remote port
        #[arg(short, long, default_value_t = 22)]
        port: u16,
        /// SSH user (overrides user@host syntax)
        #[arg(short, long)]
        user: Option<String>,
        /// Path to private key file
        #[arg(short = 'i', long)]
        key: Option<std::path::PathBuf>,
        /// Prompt for password authentication
        #[arg(long)]
        password: bool,
    },
}

#[derive(Subcommand)]
pub enum ThemeCommands {
    /// List all available theme presets
    List,
    /// Apply a theme preset by name and save it
    Set {
        /// Name of the theme preset to apply
        name: String,
    },
    /// Remove theme.json to revert to the built-in default
    Reset,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.subcommand {
        Some(Commands::Add) => {
            println!("Use the TUI to add hosts: run `sshm-rs` then press `a`");
            Ok(())
        }
        Some(Commands::Edit { host: _ }) => {
            println!("Use the TUI to edit hosts: run `sshm-rs` then press `e`");
            Ok(())
        }
        Some(Commands::Search { query }) => run_search(&query, cli.config_file.as_deref()),
        Some(Commands::Export { output }) => run_export(output.as_deref(), cli.config_file.as_deref()),
        Some(Commands::Import { file, skip_duplicates }) => run_import(&file, skip_duplicates, cli.config_file.as_deref()),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "sshm-rs", &mut std::io::stdout());
            Ok(())
        }
        Some(Commands::Validate { config_file }) => {
            run_validate(config_file.as_deref().or(cli.config_file.as_deref()))
        }
        Some(Commands::Theme { action }) => match action {
            None | Some(ThemeCommands::List) => run_theme_list(),
            Some(ThemeCommands::Set { name }) => run_theme_set(&name),
            Some(ThemeCommands::Reset) => run_theme_reset(),
        },
        Some(Commands::Update) => {
            println!(
                "Check https://github.com/bit5hift/sshm-rs/releases for the latest version.\nTo update: cargo install --git https://github.com/bit5hift/sshm-rs --force"
            );
            Ok(())
        }
        Some(Commands::Term { host, port, user, key, password }) => {
            crate::term::run_term(host, port, user, key, password)
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

/// Export all hosts to JSON
fn run_export(output: Option<&str>, config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;

    let export_data: Vec<serde_json::Value> = hosts
        .iter()
        .map(|h| {
            serde_json::json!({
                "name": h.name,
                "hostname": h.hostname,
                "user": h.user,
                "port": h.port,
                "identity": h.identity,
                "proxy_jump": h.proxy_jump,
                "tags": h.tags,
            })
        })
        .collect();

    let json = serde_json::to_string_pretty(&export_data)?;

    match output {
        Some(path) => {
            std::fs::write(path, &json)?;
            eprintln!("Exported {} hosts to {}", hosts.len(), path);
        }
        None => {
            println!("{json}");
        }
    }
    Ok(())
}

/// Import hosts from a JSON file
fn run_import(file: &str, skip_duplicates: bool, config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };

    let data = std::fs::read_to_string(file)?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&data)?;

    let mut imported = 0;
    let mut skipped = 0;

    for entry in &entries {
        let name = entry["name"].as_str().unwrap_or("").to_string();
        let hostname = entry["hostname"].as_str().unwrap_or("").to_string();

        if name.is_empty() || hostname.is_empty() {
            eprintln!("Skipping entry with missing name or hostname");
            skipped += 1;
            continue;
        }

        let tags: Vec<String> = entry["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let host = crate::config::SshHost {
            name: name.clone(),
            hostname,
            user: entry["user"].as_str().unwrap_or("").to_string(),
            port: entry["port"].as_str().unwrap_or("22").to_string(),
            identity: entry["identity"].as_str().unwrap_or("").to_string(),
            proxy_jump: entry["proxy_jump"].as_str().unwrap_or("").to_string(),
            proxy_command: String::new(),
            options: String::new(),
            remote_command: String::new(),
            request_tty: String::new(),
            tags,
            source_file: config_path.clone(),
            line_number: 0,
        };

        match crate::config::add_host(&config_path, &host) {
            Ok(()) => {
                imported += 1;
            }
            Err(e) => {
                if skip_duplicates && e.to_string().contains("already exists") {
                    skipped += 1;
                } else {
                    return Err(e);
                }
            }
        }
    }

    eprintln!("Imported {imported} hosts ({skipped} skipped)");
    Ok(())
}

/// Validate the SSH config file and print any warnings.
fn run_validate(config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;
    let warnings = crate::config::validate_hosts(&hosts);

    if warnings.is_empty() {
        println!("No warnings found in SSH config.");
        Ok(())
    } else {
        for w in &warnings {
            eprintln!("Warning: {}", w);
        }
        eprintln!("\n{} warning(s) found.", warnings.len());
        std::process::exit(1);
    }
}

/// Open the theme config file in the user's preferred editor.
fn run_theme_list() -> Result<()> {
    println!("Available theme presets:");
    for preset in crate::theme::Theme::presets() {
        println!("  {}", preset.name);
    }
    Ok(())
}

fn run_theme_set(name: &str) -> Result<()> {
    let presets = crate::theme::Theme::presets();
    let name_lower = name.to_lowercase();
    let matched = presets
        .into_iter()
        .find(|p| p.name.to_lowercase() == name_lower);
    match matched {
        Some(theme) => {
            theme.save()?;
            println!("Theme '{}' applied. Restart sshm-rs to see the change.", theme.name);
            Ok(())
        }
        None => {
            eprintln!("Unknown theme: '{}'. Run 'sshm-rs theme list' to see available presets.", name);
            std::process::exit(1);
        }
    }
}

fn run_theme_reset() -> Result<()> {
    crate::theme::Theme::reset()?;
    println!("Theme reset to default (Tokyo Night). Restart sshm-rs to apply.");
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
