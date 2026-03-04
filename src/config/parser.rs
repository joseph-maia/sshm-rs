// SSH config file parser - to be implemented by agent
// Reference: C:\Users\josep\Documents\workspace\sshm\internal\config\ssh.go

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a single SSH host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHost {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity: String,
    pub proxy_jump: String,
    pub proxy_command: String,
    pub options: String,
    pub remote_command: String,
    pub request_tty: String,
    pub tags: Vec<String>,
    pub source_file: PathBuf,
    pub line_number: usize,
}

impl SshHost {
    pub fn new(name: String, source_file: PathBuf, line_number: usize) -> Self {
        Self {
            name,
            hostname: String::new(),
            user: String::new(),
            port: String::new(),
            identity: String::new(),
            proxy_jump: String::new(),
            proxy_command: String::new(),
            options: String::new(),
            remote_command: String::new(),
            request_tty: String::new(),
            tags: Vec::new(),
            source_file,
            line_number,
        }
    }
}

/// Parse the SSH config file and return all hosts
pub fn parse_ssh_config(path: &Path) -> Result<Vec<SshHost>> {
    // TODO: implement - parse SSH config with Include support
    let _ = path;
    Ok(Vec::new())
}

/// Add a new host to the SSH config
pub fn add_host(path: &Path, host: &SshHost) -> Result<()> {
    let _ = (path, host);
    todo!("implement add_host")
}

/// Update an existing host in the SSH config
pub fn update_host(host: &SshHost) -> Result<()> {
    let _ = host;
    todo!("implement update_host")
}

/// Delete a host from the SSH config
pub fn delete_host(host: &SshHost) -> Result<()> {
    let _ = host;
    todo!("implement delete_host")
}

/// Backup the SSH config file
pub fn backup_config(path: &Path) -> Result<PathBuf> {
    let _ = path;
    todo!("implement backup_config")
}
