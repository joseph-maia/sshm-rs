// Connectivity module - SSH ping and connection
use anyhow::Result;
use std::net::TcpStream;
use std::time::Duration;

/// SSH host connectivity status
#[derive(Debug, Clone, PartialEq)]
pub enum HostStatus {
    Unknown,
    Connecting,
    Online,
    Offline,
}

impl std::fmt::Display for HostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostStatus::Unknown => write!(f, "?"),
            HostStatus::Connecting => write!(f, "..."),
            HostStatus::Online => write!(f, "Online"),
            HostStatus::Offline => write!(f, "Offline"),
        }
    }
}

/// Check if an SSH host is reachable via TCP
pub fn ping_host(hostname: &str, port: &str) -> HostStatus {
    let port = if port.is_empty() { "22" } else { port };
    let addr = format!("{hostname}:{port}");
    match TcpStream::connect_timeout(
        &addr.parse().unwrap_or_else(|_| ([0, 0, 0, 0], 22).into()),
        Duration::from_secs(5),
    ) {
        Ok(_) => HostStatus::Online,
        Err(_) => HostStatus::Offline,
    }
}

/// Connect to an SSH host using the system ssh command
pub fn connect_ssh(host: &str, command: Option<&str>) -> Result<()> {
    let mut cmd = std::process::Command::new("ssh");
    cmd.arg(host);
    if let Some(remote_cmd) = command {
        cmd.arg(remote_cmd);
    }
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}
