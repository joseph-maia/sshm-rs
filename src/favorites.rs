use anyhow::Result;
use std::path::PathBuf;

pub struct FavoritesManager {
    favorites: Vec<String>,
    file_path: PathBuf,
}

impl Default for FavoritesManager {
    fn default() -> Self {
        let file_path = crate::config::sshm_config_dir()
            .map(|dir| dir.join("favorites.json"))
            .unwrap_or_else(|_| PathBuf::from("favorites.json"));
        Self {
            favorites: Vec::new(),
            file_path,
        }
    }
}

impl FavoritesManager {
    pub fn load() -> Result<Self> {
        let dir = crate::config::sshm_config_dir()?;
        let file_path = dir.join("favorites.json");

        if !file_path.exists() {
            return Ok(Self {
                favorites: Vec::new(),
                file_path,
            });
        }

        let content = std::fs::read_to_string(&file_path)?;
        let favorites: Vec<String> = serde_json::from_str(&content)?;

        Ok(Self {
            favorites,
            file_path,
        })
    }

    pub fn toggle(&mut self, host_name: &str) -> Result<()> {
        if let Some(pos) = self.favorites.iter().position(|n| n == host_name) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(host_name.to_string());
        }
        self.save()
    }

    pub fn is_favorite(&self, host_name: &str) -> bool {
        self.favorites.iter().any(|n| n == host_name)
    }

    #[allow(dead_code)]
    pub fn favorites(&self) -> &[String] {
        &self.favorites
    }

    fn save(&self) -> Result<()> {
        // Ensure the config directory exists
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.favorites)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }
}
