use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub name: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnippetManager {
    pub snippets: Vec<Snippet>,
    #[serde(skip)]
    file_path: PathBuf,
}

impl Default for SnippetManager {
    fn default() -> Self {
        let file_path = crate::config::sshm_config_dir()
            .map(|dir| dir.join("snippets.json"))
            .unwrap_or_else(|_| PathBuf::from("snippets.json"));
        Self {
            snippets: Vec::new(),
            file_path,
        }
    }
}

impl SnippetManager {
    pub fn load() -> Result<Self> {
        let dir = crate::config::sshm_config_dir()?;
        let file_path = dir.join("snippets.json");

        if !file_path.exists() {
            return Ok(Self {
                snippets: Vec::new(),
                file_path,
            });
        }

        let content = std::fs::read_to_string(&file_path)?;
        let snippets: Vec<Snippet> = serde_json::from_str(&content)?;

        Ok(Self { snippets, file_path })
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.snippets)?;
        crate::config::write_private(&self.file_path, &json)?;
        Ok(())
    }

    pub fn add(&mut self, snippet: Snippet) {
        self.snippets.push(snippet);
        let _ = self.save();
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.snippets.len() {
            self.snippets.remove(index);
            let _ = self.save();
        }
    }
}
