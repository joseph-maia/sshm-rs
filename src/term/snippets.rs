use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub name: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetMode {
    Browse,
    Add,
    Edit,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddFormField {
    Name,
    Command,
    Description,
}

impl AddFormField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Command,
            Self::Command => Self::Description,
            Self::Description => Self::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::Description,
            Self::Command => Self::Name,
            Self::Description => Self::Command,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddForm {
    pub name: String,
    pub command: String,
    pub description: String,
    pub active_field: AddFormField,
    pub editing_index: Option<usize>,
}

impl AddForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            description: String::new(),
            active_field: AddFormField::Name,
            editing_index: None,
        }
    }

    pub fn from_snippet(index: usize, name: &str, command: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            description: description.to_string(),
            active_field: AddFormField::Name,
            editing_index: Some(index),
        }
    }

    pub fn active_field_mut(&mut self) -> &mut String {
        match self.active_field {
            AddFormField::Name => &mut self.name,
            AddFormField::Command => &mut self.command,
            AddFormField::Description => &mut self.description,
        }
    }
}

pub struct SnippetOverlay {
    pub mode: SnippetMode,
    pub search_input: String,
    pub selected_index: usize,
    pub filtered_indices: Vec<usize>,
    pub snippets: Vec<Snippet>,
    pub form: Option<AddForm>,
    pub scroll_offset: usize,
    pub overlay_area: Option<ratatui::layout::Rect>,
    pub list_area: Option<ratatui::layout::Rect>,
}

impl SnippetOverlay {
    pub fn new(snippets: Vec<Snippet>) -> Self {
        let filtered_indices: Vec<usize> = (0..snippets.len()).collect();
        Self {
            mode: SnippetMode::Browse,
            search_input: String::new(),
            selected_index: 0,
            filtered_indices,
            snippets,
            form: None,
            scroll_offset: 0,
            overlay_area: None,
            list_area: None,
        }
    }

    pub fn update_filter(&mut self) {
        let query = self.search_input.to_lowercase();
        if query.is_empty() {
            self.filtered_indices = (0..self.snippets.len()).collect();
        } else {
            self.filtered_indices = self
                .snippets
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.name.to_lowercase().contains(&query)
                        || s.command.to_lowercase().contains(&query)
                        || s.description.to_lowercase().contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn selected_snippet(&self) -> Option<&Snippet> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&i| self.snippets.get(i))
    }

    pub fn move_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index =
                (self.selected_index + 1).min(self.filtered_indices.len() - 1);
        }
    }
}

pub fn load_snippets() -> Vec<Snippet> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sshm-rs");
    let path = config_dir.join("snippets.json");
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(snippets) = serde_json::from_str::<Vec<Snippet>>(&data) {
                return snippets;
            }
        }
    }
    Vec::new()
}

pub fn save_snippets(snippets: &[Snippet]) {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sshm-rs");
    let _ = std::fs::create_dir_all(&config_dir);
    let path = config_dir.join("snippets.json");
    if let Ok(data) = serde_json::to_string_pretty(snippets) {
        let _ = std::fs::write(path, data);
    }
}
