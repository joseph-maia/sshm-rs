#![allow(dead_code)]
// History module - connection history tracking with port forwarding support
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Port forwarding configuration stored in history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardConfig {
    /// "local", "remote", or "dynamic"
    pub forward_type: String,
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
    #[serde(default)]
    pub bind_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHistory {
    pub last_connection: DateTime<Utc>,
    pub connection_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_forwarding: Option<PortForwardConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HistoryManager {
    pub entries: HashMap<String, HostHistory>,
    #[serde(skip)]
    file_path: PathBuf,
}

impl HistoryManager {
    pub fn load() -> Result<Self> {
        let path = crate::config::sshm_config_dir()?.join("history.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut manager: HistoryManager = serde_json::from_str(&data)?;
            manager.file_path = path;
            Ok(manager)
        } else {
            Ok(HistoryManager {
                entries: HashMap::new(),
                file_path: path,
            })
        }
    }

    pub fn record_connection(&mut self, host_name: &str) -> Result<()> {
        let entry = self.entries.entry(host_name.to_string()).or_insert(HostHistory {
            last_connection: Utc::now(),
            connection_count: 0,
            port_forwarding: None,
        });
        entry.last_connection = Utc::now();
        entry.connection_count += 1;
        self.save()
    }

    /// Record a connection with port forwarding info
    pub fn record_port_forwarding(
        &mut self,
        host_name: &str,
        forward_type: &str,
        local_port: &str,
        remote_host: &str,
        remote_port: &str,
        bind_address: &str,
    ) -> Result<()> {
        let pf = PortForwardConfig {
            forward_type: forward_type.to_string(),
            local_port: local_port.to_string(),
            remote_host: remote_host.to_string(),
            remote_port: remote_port.to_string(),
            bind_address: bind_address.to_string(),
        };

        let entry = self.entries.entry(host_name.to_string()).or_insert(HostHistory {
            last_connection: Utc::now(),
            connection_count: 0,
            port_forwarding: None,
        });
        entry.last_connection = Utc::now();
        entry.connection_count += 1;
        entry.port_forwarding = Some(pf);
        self.save()
    }

    /// Get the last used port forwarding config for a host
    pub fn get_port_forwarding(&self, host_name: &str) -> Option<&PortForwardConfig> {
        self.entries
            .get(host_name)
            .and_then(|h| h.port_forwarding.as_ref())
    }

    pub fn get(&self, host_name: &str) -> Option<&HostHistory> {
        self.entries.get(host_name)
    }

    /// Get all connections sorted by last connection time (most recent first)
    pub fn get_all_sorted(&self) -> Vec<(&String, &HostHistory)> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| b.1.last_connection.cmp(&a.1.last_connection));
        entries
    }

    /// Remove entries for hosts that no longer exist in the config
    pub fn cleanup(&mut self, current_hosts: &[String]) -> Result<()> {
        let host_set: std::collections::HashSet<&String> = current_hosts.iter().collect();
        self.entries.retain(|name, _| host_set.contains(name));
        self.save()
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self)?;
        crate::config::write_private(&self.file_path, &data)?;
        Ok(())
    }
}
