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

/// Connect to an SSH host using the system ssh command.
/// If a saved password exists, uses SSH_ASKPASS mechanism to pass it.
pub fn connect_ssh(
    host: &str,
    remote_command: &[String],
    config_file: Option<&str>,
    force_tty: bool,
) -> Result<()> {
    let saved_password = crate::credentials::get_password(host);

    // If we have a saved password, create a temp askpass helper
    let _askpass_guard = if let Some(ref password) = saved_password {
        Some(AskpassHelper::create(password)?)
    } else {
        None
    };

    let mut cmd = std::process::Command::new("ssh");

    if let Some(cfg) = config_file {
        cmd.args(["-F", cfg]);
    }

    if force_tty {
        cmd.arg("-t");
    }

    // If we have a password, set up SSH_ASKPASS environment
    if let Some(ref guard) = _askpass_guard {
        cmd.env("SSH_ASKPASS", &guard.script_path);
        cmd.env("SSH_ASKPASS_REQUIRE", "force");
        cmd.env("DISPLAY", ":0");
    }

    cmd.arg(host);

    if !remote_command.is_empty() {
        cmd.args(remote_command);
    } else {
        let has_pw = if saved_password.is_some() { " (using saved password)" } else { "" };
        println!("Connecting to {host}...{has_pw}");
    }

    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Temporary askpass helper script that outputs the password.
/// The script is deleted when the guard is dropped.
struct AskpassHelper {
    script_path: std::path::PathBuf,
}

impl AskpassHelper {
    fn create(password: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("sshm_askpass.cmd");

        // Write a .cmd script that echoes the password via an env var
        // The password is passed as env var SSHM_PASS to avoid disk exposure
        let script_content = "@echo off\necho %SSHM_PASS%\n";
        std::fs::write(&script_path, script_content)?;

        // Set the env var with the actual password for this process
        std::env::set_var("SSHM_PASS", password);

        Ok(Self { script_path })
    }
}

impl Drop for AskpassHelper {
    fn drop(&mut self) {
        // Clean up the script file
        let _ = std::fs::remove_file(&self.script_path);
        // Clear the env var
        std::env::remove_var("SSHM_PASS");
    }
}
