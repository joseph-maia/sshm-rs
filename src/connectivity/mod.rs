#![allow(dead_code)]
// Connectivity module - SSH ping and connection management
use anyhow::Result;
use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// SSH host connectivity status
#[derive(Debug, Clone, PartialEq)]
pub enum HostStatus {
    Unknown,
    Connecting,
    Online(Duration),
    Offline(Option<String>),
}

impl std::fmt::Display for HostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostStatus::Unknown => write!(f, "?"),
            HostStatus::Connecting => write!(f, "..."),
            HostStatus::Online(d) => write!(f, "Online ({:.0}ms)", d.as_secs_f64() * 1000.0),
            HostStatus::Offline(_) => write!(f, "Offline"),
        }
    }
}

/// Result of pinging a single host
#[derive(Debug, Clone)]
pub struct PingResult {
    pub host_name: String,
    pub status: HostStatus,
}

/// Manages concurrent SSH connectivity checks for multiple hosts.
/// Uses std::thread (not tokio) to keep things simple.
/// The TUI polls `get_status()` for live updates.
pub struct PingManager {
    results: Arc<RwLock<HashMap<String, HostStatus>>>,
    timeout: Duration,
}

impl PingManager {
    /// Create a new PingManager with the given per-host timeout.
    pub fn new(timeout: Duration) -> Self {
        Self {
            results: Arc::new(RwLock::new(HashMap::new())),
            timeout,
        }
    }

    /// Get the current status for a host.
    pub fn get_status(&self, host_name: &str) -> HostStatus {
        self.results
            .read()
            .ok()
            .and_then(|r| r.get(host_name).cloned())
            .unwrap_or(HostStatus::Unknown)
    }

    /// Get a snapshot of all results.
    pub fn get_all_statuses(&self) -> HashMap<String, HostStatus> {
        self.results
            .read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Ping all hosts concurrently, returning a channel that receives results
    /// as they complete. Each host is pinged in its own thread.
    pub fn start_ping_all(
        &self,
        hosts: Vec<(String, String, String)>, // (name, hostname, port)
    ) -> mpsc::Receiver<PingResult> {
        let (tx, rx) = mpsc::channel();
        let timeout = self.timeout;
        let results = Arc::clone(&self.results);

        // Mark all hosts as Connecting
        if let Ok(mut map) = results.write() {
            for (name, _, _) in &hosts {
                map.insert(name.clone(), HostStatus::Connecting);
            }
        }

        for (name, hostname, port) in hosts {
            let tx = tx.clone();
            let results = Arc::clone(&results);
            let timeout = timeout;

            thread::spawn(move || {
                let result = ping_host_tcp(&name, &hostname, &port, timeout);

                // Store in shared map
                if let Ok(mut map) = results.write() {
                    map.insert(name.clone(), result.status.clone());
                }

                // Send via channel (ignore error if receiver dropped)
                let _ = tx.send(result);
            });
        }

        rx
    }
}

/// Ping a single host via TCP connect with timeout.
/// If the hostname resolves and TCP connects, we consider it online.
/// This mirrors the Go implementation: TCP connect first, then treat a
/// successful connection as "online" even without SSH handshake auth.
fn ping_host_tcp(name: &str, hostname: &str, port: &str, timeout: Duration) -> PingResult {
    let port = if port.is_empty() { "22" } else { port };
    let addr_str = format!("{}:{}", hostname, port);

    let start = Instant::now();

    // Resolve + connect
    let resolved = addr_str.to_socket_addrs();
    let addrs = match resolved {
        Ok(a) => a.collect::<Vec<_>>(),
        Err(e) => {
            return PingResult {
                host_name: name.to_string(),
                status: HostStatus::Offline(Some(e.to_string())),
            };
        }
    };

    if addrs.is_empty() {
        return PingResult {
            host_name: name.to_string(),
            status: HostStatus::Offline(Some("could not resolve host".to_string())),
        };
    }

    // Try each resolved address
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => {
                let duration = start.elapsed();
                drop(stream);
                return PingResult {
                    host_name: name.to_string(),
                    status: HostStatus::Online(duration),
                };
            }
            Err(_) => continue,
        }
    }

    let duration = start.elapsed();
    PingResult {
        host_name: name.to_string(),
        status: HostStatus::Offline(Some(format!("connection timed out after {duration:.1?}"))),
    }
}

/// Connect to an SSH host.
/// If a saved password exists in credential store, uses ssh2 for direct auth.
/// Otherwise, falls back to the system `ssh` command (key-based auth).
pub fn connect_ssh(
    host: &str,
    remote_command: &[String],
    config_file: Option<&str>,
    force_tty: bool,
) -> Result<()> {
    // Check for saved password
    if let Some(password) = crate::credentials::get_password(host) {
        // Resolve host details from SSH config
        let config_path = match config_file {
            Some(p) => std::path::PathBuf::from(p),
            None => crate::config::default_ssh_config_path()?,
        };
        let hosts = crate::config::parse_ssh_config(&config_path)?;

        if let Some(host_info) = hosts.iter().find(|h| h.name == host) {
            let hostname = if host_info.hostname.is_empty() {
                &host_info.name
            } else {
                &host_info.hostname
            };
            let port = if host_info.port.is_empty() { "22" } else { &host_info.port };
            let user = if host_info.user.is_empty() {
                whoami::username()
            } else {
                host_info.user.clone()
            };

            let remote_cmd = if !remote_command.is_empty() {
                Some(remote_command.join(" "))
            } else {
                None
            };

            match connect_ssh2_interactive(hostname, port, &user, &password, remote_cmd.as_deref()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    eprintln!("ssh2 connection failed: {e}");
                    eprintln!("Falling back to system ssh...");
                }
            }
        }
    }

    // Fallback: system ssh command (key-based or interactive password prompt)
    connect_ssh_system(host, remote_command, config_file, force_tty)
}

/// Direct SSH connection via ssh2 crate with password auth + interactive shell.
fn connect_ssh2_interactive(
    hostname: &str,
    port: &str,
    user: &str,
    password: &str,
    remote_command: Option<&str>,
) -> Result<()> {
    use ssh2::Session;

    let addr = format!("{hostname}:{port}");
    println!("Connecting to {user}@{hostname}:{port} (saved password)...");

    // TCP connect
    let tcp = TcpStream::connect_timeout(
        &addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Cannot resolve {addr}"))?,
        Duration::from_secs(10),
    )?;

    // SSH session
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    // Verify host key before sending credentials
    let port_num: u16 = port.parse().unwrap_or(22);
    if let Some((host_key, key_type)) = session.host_key() {
        let known_hosts_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".ssh")
            .join("known_hosts");

        let mut known_hosts = session.known_hosts()?;
        if known_hosts_path.exists() {
            let _ = known_hosts.read_file(&known_hosts_path, ssh2::KnownHostFileKind::OpenSSH);
        }

        let check = known_hosts.check_port(hostname, port_num, host_key);
        match check {
            ssh2::CheckResult::Match => {}
            ssh2::CheckResult::Mismatch => {
                return Err(anyhow::anyhow!(
                    "HOST KEY CHANGED for {}:{}! Possible MITM attack. Connection refused.\n\
                     Remove the old key from ~/.ssh/known_hosts to connect.",
                    hostname,
                    port
                ));
            }
            ssh2::CheckResult::NotFound | ssh2::CheckResult::Failure => {
                // TOFU: log fingerprint before accepting
                let fingerprint: String = host_key
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<_>>()
                    .join(":");
                eprintln!(
                    "New host key accepted ({}) for {}:{}",
                    fingerprint, hostname, port_num
                );
                let host_entry = if port_num == 22 {
                    hostname.to_string()
                } else {
                    format!("[{}]:{}", hostname, port_num)
                };
                known_hosts.add(&host_entry, host_key, "", key_type.into())?;
                if let Some(parent) = known_hosts_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                known_hosts.write_file(&known_hosts_path, ssh2::KnownHostFileKind::OpenSSH)?;
            }
        }
    } else {
        return Err(anyhow::anyhow!("Server did not provide a host key"));
    }

    session.userauth_password(user, password)?;

    if !session.authenticated() {
        anyhow::bail!("Authentication failed for {user}@{hostname}");
    }

    // Open channel
    let mut channel = session.channel_session()?;

    // Request PTY
    channel.request_pty("xterm-256color", None, None)?;

    if let Some(cmd) = remote_command {
        channel.exec(cmd)?;
    } else {
        channel.shell()?;
    }

    // Set non-blocking for the channel
    session.set_blocking(false);

    // Enable raw mode for local terminal
    crossterm::terminal::enable_raw_mode()?;

    // Run the I/O loop, capturing the result so we can always restore the terminal
    let loop_result = ssh_io_loop(&mut session, &mut channel);

    // Always restore terminal, even if the loop errored
    let _ = crossterm::terminal::disable_raw_mode();

    // Propagate any error from the I/O loop
    loop_result?;

    // Get exit status (session must be blocking for wait_close)
    session.set_blocking(true);
    channel.wait_close()?;
    let _exit_code = channel.exit_status().unwrap_or(0);

    Ok(())
}

/// I/O loop for the SSH interactive session.
/// Forwards stdin (via crossterm events) to the SSH channel and channel output to stdout/stderr.
/// Returns Ok(()) when the channel closes, or Err on a fatal I/O error.
fn ssh_io_loop(session: &mut ssh2::Session, channel: &mut ssh2::Channel) -> Result<()> {
    use std::io::{Read, Write};

    let mut stdout = std::io::stdout();
    let mut buf = [0u8; 4096];

    loop {
        // Read from SSH channel -> stdout
        match channel.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                stdout.write_all(&buf[..n])?;
                stdout.flush()?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                if channel.eof() {
                    break;
                }
                return Err(e.into());
            }
        }

        // Read stderr from channel
        match channel.stderr().read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                std::io::stderr().write_all(&buf[..n])?;
                std::io::stderr().flush()?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }

        // Read from stdin -> SSH channel (non-blocking via crossterm events)
        if crossterm::event::poll(Duration::from_millis(10))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    let bytes = key_event_to_bytes(&key);
                    if !bytes.is_empty() {
                        session.set_blocking(true);
                        channel.write_all(&bytes)?;
                        channel.flush()?;
                        session.set_blocking(false);
                    }
                }
            }
        }

        // Check if channel closed
        if channel.eof() {
            break;
        }

        thread::sleep(Duration::from_millis(5));
    }

    Ok(())
}

/// Convert a crossterm key event to raw bytes for the SSH channel.
fn key_event_to_bytes(key: &crossterm::event::KeyEvent) -> Vec<u8> {
    use crossterm::event::{KeyCode, KeyModifiers};

    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+A = 0x01, Ctrl+C = 0x03, etc.
                let ctrl_byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                vec![ctrl_byte]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
        _ => vec![],
    }
}

/// Validate a port-forwarding argument string.
/// Each whitespace-separated token must be a -L/-R/-D flag (optionally with its
/// spec joined, e.g. "-L8080:host:80") or a bare port-forward spec containing
/// only digits, colons, dots, and brackets. Any other token (e.g. "-o", "--option")
/// is rejected to prevent SSH flag injection.
fn validate_pf_arg(arg: &str) -> bool {
    let parts: Vec<&str> = arg.split_whitespace().collect();
    parts.iter().all(|p| {
        p.starts_with("-L")
            || p.starts_with("-R")
            || p.starts_with("-D")
            || p.chars()
                .all(|c| c.is_ascii_digit() || c == ':' || c == '.' || c == '[' || c == ']')
    })
}

/// Connect to an SSH host with port forwarding arguments.
pub fn connect_ssh_with_port_forward(
    host: &str,
    pf_arg: &str,
    config_file: Option<&str>,
) -> Result<()> {
    if !validate_pf_arg(pf_arg) {
        anyhow::bail!(
            "Invalid port-forwarding argument: {:?}. Only -L/-R/-D flags and port specs are allowed.",
            pf_arg
        );
    }

    let mut cmd = std::process::Command::new("ssh");

    if let Some(cfg) = config_file {
        cmd.args(["-F", cfg]);
    }

    for part in pf_arg.split_whitespace() {
        cmd.arg(part);
    }

    cmd.arg(host);

    println!("Connecting to {host} with port forwarding ({pf_arg})...");

    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    cmd.status()?;
    Ok(())
}

/// Run a command on multiple SSH hosts sequentially.
pub fn broadcast_command(hosts: &[&str], command: &str, config_file: Option<&str>) -> Result<()> {
    use std::io::Write;

    println!();
    println!("=== COMMAND BROADCAST ===");
    println!("Command: {command}");
    println!("Hosts:   {}", hosts.join(", "));
    println!("=========================================");
    println!();

    for &host in hosts {
        println!("--- [{host}] ---");

        let mut cmd = std::process::Command::new("ssh");

        if let Some(cfg) = config_file {
            cmd.args(["-F", cfg]);
        }

        cmd.args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10"]);
        cmd.arg(host);
        cmd.arg(command);

        cmd.stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        match cmd.status() {
            Ok(status) => {
                let code = status.code().unwrap_or(-1);
                if code != 0 {
                    println!("[{host}] exited with code {code}");
                }
            }
            Err(e) => {
                eprintln!("[{host}] failed to execute: {e}");
            }
        }

        println!();
    }

    println!("=========================================");
    println!("Broadcast complete. Press Enter to return to sshm-rs...");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(())
}

/// Launch the sshm-term companion app for integrated terminal + SFTP.
/// Check if a program name can be found on PATH.
fn which_exists(program: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(program);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

pub fn launch_sshm_term(host: &str, config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;

    let mut cmd_args: Vec<String> = Vec::new();

    if let Some(host_info) = hosts.iter().find(|h| h.name == host) {
        let hostname = if host_info.hostname.is_empty() {
            &host_info.name
        } else {
            &host_info.hostname
        };
        let user = if host_info.user.is_empty() {
            whoami::username()
        } else {
            host_info.user.clone()
        };
        let port = if host_info.port.is_empty() || host_info.port == "22" {
            None
        } else {
            Some(host_info.port.as_str())
        };

        cmd_args.push(format!("{}@{}", user, hostname));

        if let Some(p) = port {
            cmd_args.push("-p".to_string());
            cmd_args.push(p.to_string());
        }

        if !host_info.identity.is_empty() {
            cmd_args.push("-i".to_string());
            cmd_args.push(host_info.identity.clone());
        }

        if let Some(password) = crate::credentials::get_password(host) {
            // Pass password via env var (not CLI arg, which is visible in ps)
            cmd_args.push("--password".to_string());

            let current_exe = std::env::current_exe()?;
            let exe_dir = current_exe.parent().unwrap_or(std::path::Path::new("."));

            let sshm_term_path = if cfg!(windows) {
                exe_dir.join("sshm-term.exe")
            } else {
                exe_dir.join("sshm-term")
            };

            let program = if sshm_term_path.exists() {
                sshm_term_path.to_string_lossy().to_string()
            } else {
                // Check if sshm-term is on PATH
                let fallback = "sshm-term".to_string();
                if which_exists(&fallback) {
                    fallback
                } else {
                    anyhow::bail!(
                        "sshm-term binary not found. Install it with: cargo install --git https://github.com/bit5hift/sshm-rs --bin sshm-term"
                    );
                }
            };

            let mut cmd = std::process::Command::new(&program);
            cmd.args(&cmd_args);
            cmd.env("SSHM_PASSWORD", &password);

            cmd.stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit());

            let status = cmd.status()?;

            if !status.success() {
                eprintln!("sshm-term exited with status: {}", status);
            }

            return Ok(());
        }
    } else {
        cmd_args.push(host.to_string());
    }

    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap_or(std::path::Path::new("."));

    let sshm_term_path = if cfg!(windows) {
        exe_dir.join("sshm-term.exe")
    } else {
        exe_dir.join("sshm-term")
    };

    let program = if sshm_term_path.exists() {
        sshm_term_path.to_string_lossy().to_string()
    } else {
        let fallback = "sshm-term".to_string();
        if which_exists(&fallback) {
            fallback
        } else {
            anyhow::bail!(
                "sshm-term binary not found. Install it with: cargo install --git https://github.com/bit5hift/sshm-rs --bin sshm-term"
            );
        }
    };

    let mut cmd = std::process::Command::new(&program);
    cmd.args(&cmd_args);

    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status()?;

    if !status.success() {
        eprintln!("sshm-term exited with status: {}", status);
    }

    Ok(())
}

/// Fallback: connect using the system `ssh` command (for key-based auth).
fn connect_ssh_system(
    host: &str,
    remote_command: &[String],
    config_file: Option<&str>,
    force_tty: bool,
) -> Result<()> {
    let mut cmd = std::process::Command::new("ssh");

    if let Some(cfg) = config_file {
        cmd.args(["-F", cfg]);
    }

    if force_tty {
        cmd.arg("-t");
    }

    cmd.arg(host);

    if !remote_command.is_empty() {
        cmd.args(remote_command);
    } else {
        println!("Connecting to {host}...");
    }

    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    cmd.status()?;
    Ok(())
}
