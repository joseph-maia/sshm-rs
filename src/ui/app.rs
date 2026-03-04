use crate::config::SshHost;
use crate::connectivity::{HostStatus, PingManager};
use crate::history::HistoryManager;
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Help,
    DeleteConfirm,
    Info,
    Add,
    Edit,
    Password,
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
    pub ping_manager: PingManager,

    // History
    pub history: Option<HistoryManager>,

    // SSH host to connect to (set when user presses Enter)
    pub connect_host: Option<String>,

    // Should quit
    pub should_quit: bool,

    // Delete confirmation target
    pub delete_target: Option<String>,

    // Add/Edit form state
    pub add_fields: [String; 7], // name, hostname, user, port, password, identity, tags
    pub add_focused: AddField,
    pub add_error: Option<String>,
    pub config_path: std::path::PathBuf,

    // Edit mode: original host name being edited
    pub edit_target: Option<String>,

    // Password overlay state
    pub password_input: String,
    pub password_target: Option<String>,

    // Toast/flash message
    pub toast_message: Option<String>,
    pub toast_expires: Option<Instant>,

    // Double-click detection
    pub last_click_time: Option<Instant>,
    pub last_click_index: Option<usize>,

    // Tag sidebar
    pub show_sidebar: bool,
    pub sidebar_tags: Vec<String>,
    pub sidebar_selected: usize,
    pub sidebar_active_tag: Option<String>,
    pub sidebar_focused: bool,
}

impl App {
    pub fn new(hosts: Vec<SshHost>, history: Option<HistoryManager>, config_path: std::path::PathBuf) -> Self {
        let ping_manager = PingManager::new(Duration::from_secs(5));
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
            ping_manager,
            history,
            connect_host: None,
            should_quit: false,
            delete_target: None,
            add_fields: Default::default(),
            add_focused: AddField::Name,
            add_error: None,
            config_path,
            edit_target: None,
            password_input: String::new(),
            password_target: None,
            toast_message: None,
            toast_expires: None,
            last_click_time: None,
            last_click_index: None,
            show_sidebar: false,
            sidebar_tags: Vec::new(),
            sidebar_selected: 0,
            sidebar_active_tag: None,
            sidebar_focused: false,
        };
        app.hosts = app.sort_hosts(&hosts);
        app.filtered_hosts = app.hosts.clone();
        app.sidebar_tags = app.build_tag_list();
        app.start_ping();
        app
    }

    /// Build the host tuples and start pinging all hosts via the PingManager.
    pub fn start_ping(&self) {
        let hosts_data: Vec<(String, String, String)> = self
            .hosts
            .iter()
            .map(|h| {
                let hostname = if h.hostname.is_empty() {
                    h.name.clone()
                } else {
                    h.hostname.clone()
                };
                let port = if h.port.is_empty() {
                    "22".to_string()
                } else {
                    h.port.clone()
                };
                (h.name.clone(), hostname, port)
            })
            .collect();
        let _ = self.ping_manager.start_ping_all(hosts_data);
    }

    pub fn show_toast(&mut self, msg: &str) {
        self.toast_message = Some(msg.to_string());
        self.toast_expires = Some(Instant::now() + Duration::from_secs(3));
    }

    pub fn check_toast(&mut self) {
        if let Some(expires) = self.toast_expires {
            if Instant::now() >= expires {
                self.toast_message = None;
                self.toast_expires = None;
            }
        }
    }

    pub fn reset_add_form(&mut self) {
        self.add_fields = Default::default();
        self.add_fields[3] = "22".to_string(); // default port
        self.add_focused = AddField::Name;
        self.add_error = None;
    }

    #[allow(dead_code)]
    pub fn add_field_value(&self, field: AddField) -> &str {
        &self.add_fields[field as usize]
    }

    pub fn build_tag_list(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .hosts
            .iter()
            .flat_map(|h| h.tags.iter().cloned())
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        tags.sort();
        tags
    }

    pub fn refresh_sidebar_tags(&mut self) {
        self.sidebar_tags = self.build_tag_list();
        if self.sidebar_selected >= self.sidebar_tags.len() + 1 {
            self.sidebar_selected = self.sidebar_tags.len(); // last tag index (0 = All Hosts)
        }
    }

    pub fn reload_hosts(&mut self) {
        if let Ok(hosts) = crate::config::parse_ssh_config(&self.config_path) {
            self.hosts = self.sort_hosts(&hosts);
            self.apply_filter();
            self.refresh_sidebar_tags();
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
            let mut matcher = Matcher::new(Config::DEFAULT);

            // Score each host: all subqueries must match for the host to be included.
            // The total score is the sum of per-word best scores.
            let mut scored: Vec<(SshHost, u32)> = self
                .hosts
                .iter()
                .filter_map(|host| {
                    let mut total_score: u32 = 0;

                    for &q in &subqueries {
                        // Check for prefix filters
                        let (field_filter, needle) = parse_query_prefix(q);
                        let haystack = build_haystack(host, field_filter);

                        let pattern = Pattern::new(
                            needle,
                            CaseMatching::Ignore,
                            Normalization::Smart,
                            AtomKind::Fuzzy,
                        );
                        let mut haystack_buf = Vec::new();
                        let score = pattern.score(
                            nucleo_matcher::Utf32Str::new(&haystack, &mut haystack_buf),
                            &mut matcher,
                        );
                        match score {
                            Some(s) => total_score = total_score.saturating_add(s),
                            None => return None, // this word didn't match at all
                        }
                    }

                    Some((host.clone(), total_score))
                })
                .collect();

            // Sort by score descending (best match first), then by name for ties
            scored.sort_by(|a, b| {
                b.1.cmp(&a.1)
                    .then_with(|| a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
            });

            self.filtered_hosts = scored.into_iter().map(|(host, _)| host).collect();
        }
        // Apply sidebar tag filter
        if let Some(ref tag) = self.sidebar_active_tag {
            self.filtered_hosts.retain(|h| h.tags.iter().any(|t| t == tag));
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

    pub fn clamp_offset(&mut self) {
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

    pub fn get_status_indicator(&self, host_name: &str) -> (&'static str, HostStatus) {
        let status = self.ping_manager.get_status(host_name);
        let indicator = match status {
            HostStatus::Unknown => "\u{25CB}",
            HostStatus::Connecting => "\u{25CC}",
            HostStatus::Online(_) => "\u{25CF}",
            HostStatus::Offline(_) => "\u{25CF}",
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

// ---------------------------------------------------------------------------
// Fuzzy-search helpers
// ---------------------------------------------------------------------------

/// Which host field(s) to search in.
#[derive(Clone, Copy)]
enum FieldFilter {
    All,
    Tag,
    User,
    Host,
}

/// Parse a query token like `tag:web` into `(FieldFilter::Tag, "web")`.
/// If no recognised prefix is found, returns `(FieldFilter::All, <original>)`.
fn parse_query_prefix(token: &str) -> (FieldFilter, &str) {
    if let Some(rest) = token.strip_prefix("tag:") {
        (FieldFilter::Tag, rest)
    } else if let Some(rest) = token.strip_prefix("user:") {
        (FieldFilter::User, rest)
    } else if let Some(rest) = token.strip_prefix("host:") {
        (FieldFilter::Host, rest)
    } else {
        (FieldFilter::All, token)
    }
}

/// Build a haystack string for fuzzy matching from the relevant host fields.
fn build_haystack(host: &SshHost, filter: FieldFilter) -> String {
    match filter {
        FieldFilter::Tag => host.tags.join(" "),
        FieldFilter::User => host.user.clone(),
        FieldFilter::Host => host.hostname.clone(),
        FieldFilter::All => {
            let tags_str = host.tags.join(" ");
            format!(
                "{} {} {} {} {}",
                host.name, host.hostname, host.user, host.port, tags_str
            )
        }
    }
}
