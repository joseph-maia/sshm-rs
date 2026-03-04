// History module - connection history tracking
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHistory {
    pub last_connection: DateTime<Utc>,
    pub connection_count: u64,
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
        });
        entry.last_connection = Utc::now();
        entry.connection_count += 1;
        self.save()
    }

    pub fn get(&self, host_name: &str) -> Option<&HostHistory> {
        self.entries.get(host_name)
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self)?;
        std::fs::write(&self.file_path, data)?;
        Ok(())
    }
}
