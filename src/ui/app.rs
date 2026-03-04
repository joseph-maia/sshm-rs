use crate::config::SshHost;
use crate::connectivity::HostStatus;
use crate::history::HistoryManager;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Help,
    DeleteConfirm,
    Info,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddField {
    Name,
    Hostname,
    User,
    Port,
    Password,
    Identity,
    Tags,
}

impl AddField {
    pub fn label(self) -> &'static str {
        match self {
            AddField::Name => "Name",
            AddField::Hostname => "Hostname",
            AddField::User => "User",
            AddField::Port => "Port",
            AddField::Password => "Password",
            AddField::Identity => "IdentityFile",
            AddField::Tags => "Tags",
        }
    }

    pub fn is_secret(self) -> bool {
        matches!(self, AddField::Password)
    }

    pub fn next(self) -> Self {
        match self {
            AddField::Name => AddField::Hostname,
            AddField::Hostname => AddField::User,
            AddField::User => AddField::Port,
            AddField::Port => AddField::Password,
            AddField::Password => AddField::Identity,
            AddField::Identity => AddField::Tags,
            AddField::Tags => AddField::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            AddField::Name => AddField::Tags,
            AddField::Hostname => AddField::Name,
            AddField::User => AddField::Hostname,
            AddField::Port => AddField::User,
            AddField::Password => AddField::Port,
            AddField::Identity => AddField::Password,
            AddField::Tags => AddField::Identity,
        }
    }

    pub const ALL: [AddField; 7] = [
        AddField::Name,
        AddField::Hostname,
        AddField::User,
        AddField::Port,
        AddField::Password,
        AddField::Identity,
        AddField::Tags,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    ByName,
    ByLastUsed,
}

impl SortMode {
    pub fn label(self) -> &'static str {
        match self {
            SortMode::ByName => "Name (A-Z)",
            SortMode::ByLastUsed => "Last Login",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            SortMode::ByName => SortMode::ByLastUsed,
            SortMode::ByLastUsed => SortMode::ByName,
        }
    }
}

pub struct App {
    // Host data
    pub hosts: Vec<SshHost>,
    pub filtered_hosts: Vec<SshHost>,

    // Selection / navigation
    pub selected: usize,
    pub table_offset: usize,

    // Search
    pub search_query: String,
    pub search_mode: bool,

    // View
    pub view_mode: ViewMode,
    pub sort_mode: SortMode,

    // Terminal dimensions
    pub width: u16,
    pub height: u16,

    // Connectivity status
    pub host_status: HashMap<String, HostStatus>,

    // History
    pub history: Option<HistoryManager>,

    // SSH host to connect to (set when user presses Enter)
    pub connect_host: Option<String>,

    // Should quit
    pub should_quit: bool,

    // Delete confirmation target
    pub delete_target: Option<String>,

    // Add form state
    pub add_fields: [String; 7], // name, hostname, user, port, password, identity, tags
    pub add_focused: AddField,
    pub add_error: Option<String>,
    pub config_path: std::path::PathBuf,
}

impl App {
    pub fn new(hosts: Vec<SshHost>, history: Option<HistoryManager>, config_path: std::path::PathBuf) -> Self {
        let mut app = App {
            hosts: Vec::new(),
            filtered_hosts: Vec::new(),
            selected: 0,
            table_offset: 0,
            search_query: String::new(),
            search_mode: false,
            view_mode: ViewMode::List,
            sort_mode: SortMode::ByName,
            width: 80,
            height: 24,
            host_status: HashMap::new(),
            history,
            connect_host: None,
            should_quit: false,
            delete_target: None,
            add_fields: Default::default(),
            add_focused: AddField::Name,
            add_error: None,
            config_path,
        };
        app.hosts = app.sort_hosts(&hosts);
        app.filtered_hosts = app.hosts.clone();
        app
    }

    pub fn reset_add_form(&mut self) {
        self.add_fields = Default::default();
        self.add_fields[3] = "22".to_string(); // default port
        self.add_focused = AddField::Name;
        self.add_error = None;
    }

    pub fn add_field_value(&self, field: AddField) -> &str {
        &self.add_fields[field as usize]
    }

    pub fn reload_hosts(&mut self) {
        if let Ok(hosts) = crate::config::parse_ssh_config(&self.config_path) {
            self.hosts = self.sort_hosts(&hosts);
            self.apply_filter();
        }
    }

    pub fn visible_rows(&self) -> usize {
        // Reserve lines for: header art (5) + search bar (3) + table header (2) + status bar (1) + padding (2)
        let reserved = 13u16;
        if self.height > reserved {
            (self.height - reserved) as usize
        } else {
            3
        }
    }

    pub fn sort_hosts(&self, hosts: &[SshHost]) -> Vec<SshHost> {
        let mut sorted = hosts.to_vec();
        match self.sort_mode {
            SortMode::ByName => {
                sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
            SortMode::ByLastUsed => {
                if let Some(ref history) = self.history {
                    sorted.sort_by(|a, b| {
                        let a_time = history.get(&a.name).map(|h| h.last_connection);
                        let b_time = history.get(&b.name).map(|h| h.last_connection);
                        // Most recent first; hosts without history go to end
                        b_time.cmp(&a_time)
                    });
                } else {
                    sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                }
            }
        }
        sorted
    }

    pub fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_hosts = self.sort_hosts(&self.hosts);
        } else {
            let subqueries: Vec<&str> = self.search_query.split_whitespace().collect();
            let all_hosts = self.hosts.clone();
            let filtered: Vec<SshHost> = all_hosts
                .into_iter()
                .filter(|host| {
                    subqueries.iter().all(|q| {
                        let q = q.to_lowercase();
                        host.name.to_lowercase().contains(&q)
                            || host.hostname.to_lowercase().contains(&q)
                            || host.user.to_lowercase().contains(&q)
                            || host.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    })
                })
                .collect();
            self.filtered_hosts = self.sort_hosts(&filtered);
        }
        // Clamp selected
        if !self.filtered_hosts.is_empty() {
            if self.selected >= self.filtered_hosts.len() {
                self.selected = self.filtered_hosts.len() - 1;
            }
        } else {
            self.selected = 0;
        }
        self.clamp_offset();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.clamp_offset();
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_hosts.is_empty() && self.selected < self.filtered_hosts.len() - 1 {
            self.selected += 1;
            self.clamp_offset();
        }
    }

    fn clamp_offset(&mut self) {
        let visible = self.visible_rows();
        if self.selected < self.table_offset {
            self.table_offset = self.selected;
        } else if self.selected >= self.table_offset + visible {
            self.table_offset = self.selected + 1 - visible;
        }
    }

    pub fn selected_host(&self) -> Option<&SshHost> {
        self.filtered_hosts.get(self.selected)
    }

    pub fn get_status_indicator(&self, host_name: &str) -> (&str, &HostStatus) {
        let status = self
            .host_status
            .get(host_name)
            .unwrap_or(&HostStatus::Unknown);
        let indicator = match status {
            HostStatus::Unknown => "?",
            HostStatus::Connecting => "~",
            HostStatus::Online(_) => "+",
            HostStatus::Offline(_) => "-",
        };
        (indicator, status)
    }

    pub fn format_time_ago(&self, host_name: &str) -> String {
        if let Some(ref history) = self.history {
            if let Some(entry) = history.get(host_name) {
                let now = chrono::Utc::now();
                let duration = now - entry.last_connection;

                if duration.num_seconds() < 60 {
                    let s = duration.num_seconds();
                    return if s <= 1 {
                        "1 second ago".to_string()
                    } else {
                        format!("{s} seconds ago")
                    };
                } else if duration.num_minutes() < 60 {
                    let m = duration.num_minutes();
                    return if m == 1 {
                        "1 minute ago".to_string()
                    } else {
                        format!("{m} minutes ago")
                    };
                } else if duration.num_hours() < 24 {
                    let h = duration.num_hours();
                    return if h == 1 {
                        "1 hour ago".to_string()
                    } else {
                        format!("{h} hours ago")
                    };
                } else if duration.num_days() < 7 {
                    let d = duration.num_days();
                    return if d == 1 {
                        "1 day ago".to_string()
                    } else {
                        format!("{d} days ago")
                    };
                } else if duration.num_weeks() < 4 {
                    let w = duration.num_weeks();
                    return if w == 1 {
                        "1 week ago".to_string()
                    } else {
                        format!("{w} weeks ago")
                    };
                } else if duration.num_days() < 365 {
                    let months = duration.num_days() / 30;
                    return if months <= 1 {
                        "1 month ago".to_string()
                    } else {
                        format!("{months} months ago")
                    };
                } else {
                    let years = duration.num_days() / 365;
                    return if years <= 1 {
                        "1 year ago".to_string()
                    } else {
                        format!("{years} years ago")
                    };
                }
            }
        }
        String::new()
    }
}
