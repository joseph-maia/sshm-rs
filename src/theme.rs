use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub bg: [u8; 3],
    pub fg: [u8; 3],
    pub primary: [u8; 3],
    pub green: [u8; 3],
    pub red: [u8; 3],
    pub yellow: [u8; 3],
    pub muted: [u8; 3],
    pub cyan: [u8; 3],
    pub purple: [u8; 3],
    pub orange: [u8; 3],
    pub selection_bg: [u8; 3],
}

impl Default for Theme {
    fn default() -> Self {
        Self::tokyo_night()
    }
}

impl Theme {
    #[allow(dead_code)]
    pub fn color(&self, rgb: [u8; 3]) -> Color {
        Color::Rgb(rgb[0], rgb[1], rgb[2])
    }

    /// Tokyo Night — the default palette.
    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".to_string(),
            bg: [0x1a, 0x1b, 0x26],
            fg: [0xc0, 0xca, 0xf5],
            primary: [0x7a, 0xa2, 0xf7],
            green: [0x9e, 0xce, 0x6a],
            red: [0xf7, 0x76, 0x8e],
            yellow: [0xe0, 0xaf, 0x68],
            muted: [0x73, 0x7a, 0xa2],
            cyan: [0x7d, 0xcf, 0xff],
            purple: [0xbb, 0x9a, 0xf7],
            orange: [0xff, 0x9e, 0x64],
            selection_bg: [0x36, 0x4a, 0x82],
        }
    }

    /// Returns the path to `theme.json` in the sshm-rs config directory.
    pub fn config_path() -> PathBuf {
        crate::config::sshm_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("theme.json")
    }

    /// If `theme.json` does not exist, create it with the default theme serialized as
    /// pretty JSON. Returns the path in either case.
    pub fn ensure_config_file() -> anyhow::Result<PathBuf> {
        let path = Self::config_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(&Self::default())?;
            std::fs::write(&path, json)?;
        }
        Ok(path)
    }

    /// Load a theme from `theme.json` (or platform equivalent).
    /// Falls back to `Theme::default()` if the file does not exist or cannot be parsed.
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(theme) = serde_json::from_str::<Theme>(&data) {
                    return theme;
                }
            }
        }
        Self::default()
    }
}
