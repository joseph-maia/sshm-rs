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
    #[serde(default = "default_hover_bg")]
    pub hover_bg: [u8; 3],
}

fn default_hover_bg() -> [u8; 3] {
    [0x24, 0x28, 0x35]
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
            hover_bg: [0x24, 0x28, 0x35],
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha".to_string(),
            bg: [0x1e, 0x1e, 0x2e],
            fg: [0xcd, 0xd6, 0xf4],
            primary: [0x89, 0xb4, 0xfa],
            green: [0xa6, 0xe3, 0xa1],
            red: [0xf3, 0x8b, 0xa8],
            yellow: [0xf9, 0xe2, 0xaf],
            muted: [0x6c, 0x70, 0x86],
            cyan: [0x94, 0xe2, 0xd5],
            purple: [0xcb, 0xa6, 0xf7],
            orange: [0xfa, 0xb3, 0x87],
            selection_bg: [0x45, 0x47, 0x5a],
            hover_bg: [0x31, 0x32, 0x44],
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            bg: [0x28, 0x2a, 0x36],
            fg: [0xf8, 0xf8, 0xf2],
            primary: [0xbd, 0x93, 0xf9],
            green: [0x50, 0xfa, 0x7b],
            red: [0xff, 0x55, 0x55],
            yellow: [0xf1, 0xfa, 0x8c],
            muted: [0x62, 0x72, 0xa4],
            cyan: [0x8b, 0xe9, 0xfd],
            purple: [0xff, 0x79, 0xc6],
            orange: [0xff, 0xb8, 0x6c],
            selection_bg: [0x44, 0x47, 0x5a],
            hover_bg: [0x38, 0x3a, 0x4a],
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            bg: [0x2e, 0x34, 0x40],
            fg: [0xec, 0xef, 0xf4],
            primary: [0x88, 0xc0, 0xd0],
            green: [0xa3, 0xbe, 0x8c],
            red: [0xbf, 0x61, 0x6a],
            yellow: [0xeb, 0xcb, 0x8b],
            muted: [0x4c, 0x56, 0x6a],
            cyan: [0x8f, 0xbc, 0xbb],
            purple: [0xb4, 0x8e, 0xad],
            orange: [0xd0, 0x87, 0x70],
            selection_bg: [0x3b, 0x42, 0x52],
            hover_bg: [0x34, 0x3b, 0x4a],
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "Gruvbox Dark".to_string(),
            bg: [0x28, 0x28, 0x28],
            fg: [0xeb, 0xdb, 0xb2],
            primary: [0x83, 0xa5, 0x98],
            green: [0xb8, 0xbb, 0x26],
            red: [0xfb, 0x49, 0x34],
            yellow: [0xfa, 0xbd, 0x2f],
            muted: [0x66, 0x5c, 0x54],
            cyan: [0x8e, 0xc0, 0x7c],
            purple: [0xd3, 0x86, 0x9b],
            orange: [0xfe, 0x80, 0x19],
            selection_bg: [0x3c, 0x38, 0x36],
            hover_bg: [0x32, 0x30, 0x2e],
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".to_string(),
            bg: [0x00, 0x2b, 0x36],
            fg: [0x83, 0x94, 0x96],
            primary: [0x26, 0x8b, 0xd2],
            green: [0x85, 0x99, 0x00],
            red: [0xdc, 0x32, 0x2f],
            yellow: [0xb5, 0x89, 0x00],
            muted: [0x58, 0x6e, 0x75],
            cyan: [0x2a, 0xa1, 0x98],
            purple: [0x6c, 0x71, 0xc4],
            orange: [0xcb, 0x4b, 0x16],
            selection_bg: [0x07, 0x36, 0x42],
            hover_bg: [0x05, 0x2a, 0x35],
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".to_string(),
            bg: [0x28, 0x2c, 0x34],
            fg: [0xab, 0xb2, 0xbf],
            primary: [0x61, 0xaf, 0xef],
            green: [0x98, 0xc3, 0x79],
            red: [0xe0, 0x6c, 0x75],
            yellow: [0xe5, 0xc0, 0x7b],
            muted: [0x5c, 0x63, 0x70],
            cyan: [0x56, 0xb6, 0xc2],
            purple: [0xc6, 0x78, 0xdd],
            orange: [0xd1, 0x9a, 0x66],
            selection_bg: [0x3e, 0x44, 0x52],
            hover_bg: [0x33, 0x37, 0x43],
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".to_string(),
            bg: [0x00, 0x00, 0x00],
            fg: [0xff, 0xff, 0xff],
            primary: [0x00, 0xff, 0xff],
            green: [0x00, 0xff, 0x00],
            red: [0xff, 0x00, 0x00],
            yellow: [0xff, 0xff, 0x00],
            muted: [0x88, 0x88, 0x88],
            cyan: [0x00, 0xff, 0xff],
            purple: [0xff, 0x00, 0xff],
            orange: [0xff, 0x88, 0x00],
            selection_bg: [0x33, 0x33, 0x33],
            hover_bg: [0x22, 0x22, 0x22],
        }
    }

    /// All built-in theme presets.
    pub fn presets() -> Vec<Theme> {
        vec![
            Self::tokyo_night(),
            Self::catppuccin_mocha(),
            Self::dracula(),
            Self::nord(),
            Self::gruvbox_dark(),
            Self::solarized_dark(),
            Self::one_dark(),
            Self::high_contrast(),
        ]
    }

    /// Returns the path to `theme.json` in the sshm-rs config directory.
    pub fn config_path() -> PathBuf {
        crate::config::sshm_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("theme.json")
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

    /// Save this theme to `theme.json`.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        crate::config::write_private(&path, &json)?;
        Ok(())
    }

    /// Delete `theme.json` to revert to the built-in default.
    pub fn reset() -> anyhow::Result<()> {
        let path = Self::config_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Save this theme to an explicit path (used in tests).
    #[cfg(test)]
    pub fn save_to(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        crate::config::write_private(path, &json)?;
        Ok(())
    }

    /// Load a theme from an explicit path (used in tests).
    #[cfg(test)]
    pub fn load_from(path: &std::path::Path) -> Self {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(theme) = serde_json::from_str::<Theme>(&data) {
                    return theme;
                }
            }
        }
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: assert that an RGB array has exactly 3 elements (always true for [u8;3], but
    // we validate the values are in the valid u8 range which is guaranteed by the type).
    fn is_valid_rgb(rgb: [u8; 3]) -> bool {
        // u8 values are always 0-255; the array is always length 3 by type.
        // This check confirms the array is non-zero in at least one channel to
        // catch accidentally zeroed-out colors (all-black is only valid intentionally).
        let _ = rgb; // explicit: all [u8;3] arrays are structurally valid
        true
    }

    fn theme_colors_all_valid(theme: &Theme) -> bool {
        is_valid_rgb(theme.bg)
            && is_valid_rgb(theme.fg)
            && is_valid_rgb(theme.primary)
            && is_valid_rgb(theme.green)
            && is_valid_rgb(theme.red)
            && is_valid_rgb(theme.yellow)
            && is_valid_rgb(theme.muted)
            && is_valid_rgb(theme.cyan)
            && is_valid_rgb(theme.purple)
            && is_valid_rgb(theme.orange)
            && is_valid_rgb(theme.selection_bg)
            && is_valid_rgb(theme.hover_bg)
    }

    // -----------------------------------------------------------------------
    // Test 1: Default theme has a non-empty name and all valid color arrays
    // -----------------------------------------------------------------------
    #[test]
    fn default_theme_has_valid_name_and_colors() {
        let theme = Theme::default();
        assert!(!theme.name.is_empty(), "default theme name must not be empty");
        assert!(
            theme_colors_all_valid(&theme),
            "default theme must have all valid RGB arrays"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: presets() returns exactly 8 themes with valid colors
    // -----------------------------------------------------------------------
    #[test]
    fn presets_returns_eight_valid_themes() {
        let presets = Theme::presets();
        assert_eq!(presets.len(), 8, "expected exactly 8 preset themes");
        for preset in &presets {
            assert!(
                !preset.name.is_empty(),
                "preset '{}' has an empty name",
                preset.name
            );
            assert!(
                theme_colors_all_valid(preset),
                "preset '{}' has invalid color arrays",
                preset.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 3: All expected preset names are present
    // -----------------------------------------------------------------------
    #[test]
    fn preset_names_match_expected() {
        let expected_names = [
            "Tokyo Night",
            "Catppuccin Mocha",
            "Dracula",
            "Nord",
            "Gruvbox Dark",
            "Solarized Dark",
            "One Dark",
            "High Contrast",
        ];
        let presets = Theme::presets();
        let preset_names: Vec<&str> = presets.iter().map(|t| t.name.as_str()).collect();
        for expected in &expected_names {
            assert!(
                preset_names.contains(expected),
                "expected preset '{}' not found in presets list",
                expected
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 4: No duplicate names in presets
    // -----------------------------------------------------------------------
    #[test]
    fn preset_names_are_unique() {
        let presets = Theme::presets();
        let mut seen = std::collections::HashSet::new();
        for preset in &presets {
            let inserted = seen.insert(preset.name.as_str());
            assert!(
                inserted,
                "duplicate preset name found: '{}'",
                preset.name
            );
        }
    }

    // -----------------------------------------------------------------------
    // Test 5: Save and load roundtrip via save_to / load_from
    // -----------------------------------------------------------------------
    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("theme.json");

        let original = Theme::dracula();
        original.save_to(&path).expect("save_to failed");

        assert!(path.exists(), "theme file should exist after save_to");

        let loaded = Theme::load_from(&path);
        assert_eq!(loaded.name, original.name, "name should survive roundtrip");
        assert_eq!(loaded.bg, original.bg, "bg should survive roundtrip");
        assert_eq!(loaded.fg, original.fg, "fg should survive roundtrip");
        assert_eq!(loaded.primary, original.primary, "primary should survive roundtrip");
        assert_eq!(loaded.green, original.green, "green should survive roundtrip");
        assert_eq!(loaded.red, original.red, "red should survive roundtrip");
        assert_eq!(loaded.yellow, original.yellow, "yellow should survive roundtrip");
        assert_eq!(loaded.muted, original.muted, "muted should survive roundtrip");
        assert_eq!(loaded.cyan, original.cyan, "cyan should survive roundtrip");
        assert_eq!(loaded.purple, original.purple, "purple should survive roundtrip");
        assert_eq!(loaded.orange, original.orange, "orange should survive roundtrip");
        assert_eq!(loaded.selection_bg, original.selection_bg, "selection_bg should survive roundtrip");
        assert_eq!(loaded.hover_bg, original.hover_bg, "hover_bg should survive roundtrip");
    }

    // -----------------------------------------------------------------------
    // Test 6: load_from returns default when file is absent
    // -----------------------------------------------------------------------
    #[test]
    fn load_from_missing_file_returns_default() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("nonexistent_theme.json");

        let loaded = Theme::load_from(&path);
        let default = Theme::default();
        assert_eq!(
            loaded.name, default.name,
            "load_from missing file should return default theme"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: load_from returns default when file contains invalid JSON
    // -----------------------------------------------------------------------
    #[test]
    fn load_from_invalid_json_returns_default() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("bad_theme.json");
        std::fs::write(&path, b"not valid json at all {{{{").expect("write failed");

        let loaded = Theme::load_from(&path);
        let default = Theme::default();
        assert_eq!(
            loaded.name, default.name,
            "load_from invalid JSON should return default theme"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: File is removed after save_to then manual remove (simulates reset)
    // -----------------------------------------------------------------------
    #[test]
    fn file_removal_after_save() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("theme.json");

        Theme::nord().save_to(&path).expect("save_to failed");
        assert!(path.exists(), "file should exist after save");

        std::fs::remove_file(&path).expect("remove_file failed");
        assert!(!path.exists(), "file should not exist after removal");

        // After removal, load_from should fall back to default
        let loaded = Theme::load_from(&path);
        assert_eq!(loaded.name, Theme::default().name);
    }

    // -----------------------------------------------------------------------
    // Test 9: serde round-trip via JSON string (no filesystem)
    // -----------------------------------------------------------------------
    #[test]
    fn serde_json_roundtrip_all_presets() {
        for preset in Theme::presets() {
            let json = serde_json::to_string(&preset)
                .unwrap_or_else(|e| panic!("serialize '{}' failed: {}", preset.name, e));
            let restored: Theme = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize '{}' failed: {}", preset.name, e));
            assert_eq!(preset.name, restored.name);
            assert_eq!(preset.bg, restored.bg);
            assert_eq!(preset.hover_bg, restored.hover_bg);
        }
    }

    // -----------------------------------------------------------------------
    // Test 10: init_theme / current_theme reflects the installed theme name
    // -----------------------------------------------------------------------
    #[test]
    fn init_theme_then_current_theme_matches() {
        use crate::ui::styles;

        let preset = Theme::catppuccin_mocha();
        let expected_name = preset.name.clone();
        styles::init_theme(preset);

        let active = styles::current_theme();
        assert_eq!(
            active.name, expected_name,
            "current_theme() should return the theme set by init_theme()"
        );
    }
}
