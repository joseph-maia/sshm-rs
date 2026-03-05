use crate::config::SshHost;
use crate::connectivity::{HostStatus, PingManager};
use crate::favorites::FavoritesManager;
use crate::groups::GroupsManager;
use crate::history::HistoryManager;
use crate::snippets::SnippetManager;
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
    PortForward,
    Broadcast,
    Snippets,
    FileTransfer,
    GroupCreate,
    GroupPicker,
}

#[derive(Debug, Clone)]
pub enum DisplayRow {
    GroupHeader { name: String, host_count: usize, collapsed: bool },
    HostRow(usize),
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

    // Multi-select
    pub selected_hosts: std::collections::HashSet<String>,
    #[allow(dead_code)]
    pub batch_mode: bool,

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

    // Favorites
    pub favorites: FavoritesManager,

    // Port forwarding form state
    pub pf_forward_type: usize,
    pub pf_local_port: String,
    pub pf_remote_host: String,
    pub pf_remote_port: String,
    pub pf_bind_address: String,
    pub pf_focused: usize,
    pub pf_target: Option<String>,
    pub pf_error: Option<String>,
    pub port_forward_args: Option<String>,

    // Tag sidebar
    pub show_sidebar: bool,
    pub sidebar_tags: Vec<String>,
    pub sidebar_selected: usize,
    pub sidebar_active_tag: Option<String>,
    pub sidebar_focused: bool,

    // Config validation warnings
    pub config_warnings: Vec<String>,

    // Theme cycling
    pub theme_index: usize,

    // Command broadcast
    pub broadcast_command: String,
    pub broadcast_error: Option<String>,
    pub pending_broadcast: Option<(Vec<String>, String)>,

    // Snippets
    pub snippet_manager: SnippetManager,
    pub snippet_selected: usize,
    pub snippet_adding: bool,
    pub snippet_fields: [String; 3], // name, command, description
    pub snippet_focused: usize,
    pub snippet_error: Option<String>,

    // SCP/SFTP file transfer state
    pub scp_local_path: String,
    pub scp_remote_path: String,
    pub scp_upload: bool,
    pub scp_focused: usize,
    pub scp_error: Option<String>,
    pub scp_target: Option<String>,

    // Groups
    pub groups: GroupsManager,
    pub display_rows: Vec<DisplayRow>,
    pub group_input: String,
    pub group_picker_items: Vec<String>,
    pub group_picker_selected: usize,
    pub ungrouped_collapsed: bool,
}

impl App {
    pub fn new(hosts: Vec<SshHost>, history: Option<HistoryManager>, config_path: std::path::PathBuf) -> Self {
        let ping_manager = PingManager::new(Duration::from_secs(5));
        let favorites = FavoritesManager::load().unwrap_or_default();
        let config_warnings = crate::config::validate_hosts(&hosts);
        let mut app = App {
            hosts: Vec::new(),
            filtered_hosts: Vec::new(),
            selected: 0,
            table_offset: 0,
            selected_hosts: std::collections::HashSet::new(),
            batch_mode: false,
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
            favorites,
            pf_forward_type: 0,
            pf_local_port: String::new(),
            pf_remote_host: String::new(),
            pf_remote_port: String::new(),
            pf_bind_address: String::new(),
            pf_focused: 0,
            pf_target: None,
            pf_error: None,
            port_forward_args: None,
            show_sidebar: false,
            sidebar_tags: Vec::new(),
            sidebar_selected: 0,
            sidebar_active_tag: None,
            sidebar_focused: false,
            config_warnings,
            theme_index: 0,
            broadcast_command: String::new(),
            broadcast_error: None,
            pending_broadcast: None,
            snippet_manager: SnippetManager::load().unwrap_or_default(),
            snippet_selected: 0,
            snippet_adding: false,
            snippet_fields: Default::default(),
            snippet_focused: 0,
            snippet_error: None,
            scp_local_path: String::new(),
            scp_remote_path: String::new(),
            scp_upload: true,
            scp_focused: 0,
            scp_error: None,
            scp_target: None,
            groups: GroupsManager::load().unwrap_or_default(),
            display_rows: Vec::new(),
            group_input: String::new(),
            group_picker_items: Vec::new(),
            group_picker_selected: 0,
            ungrouped_collapsed: false,
        };
        app.hosts = app.sort_hosts(&hosts);
        app.filtered_hosts = app.hosts.clone();
        app.sidebar_tags = app.build_tag_list();
        app.rebuild_display_rows();
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

    pub fn reset_pf_form(&mut self) {
        self.pf_forward_type = 0;
        self.pf_local_port.clear();
        self.pf_remote_host.clear();
        self.pf_remote_port.clear();
        self.pf_bind_address.clear();
        self.pf_focused = 0;
        self.pf_error = None;
    }

    pub fn prefill_pf_form(&mut self, host_name: &str) {
        self.reset_pf_form();
        if let Some(ref history) = self.history {
            if let Some(pf) = history.get_port_forwarding(host_name) {
                self.pf_forward_type = match pf.forward_type.as_str() {
                    "remote" => 1,
                    "dynamic" => 2,
                    _ => 0, // local
                };
                self.pf_local_port = pf.local_port.clone();
                self.pf_remote_host = pf.remote_host.clone();
                self.pf_remote_port = pf.remote_port.clone();
                self.pf_bind_address = pf.bind_address.clone();
            }
        }
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
            self.sidebar_selected = self.sidebar_tags.len();
        }
    }

    pub fn reload_hosts(&mut self) {
        if let Ok(hosts) = crate::config::parse_ssh_config(&self.config_path) {
            self.config_warnings = crate::config::validate_hosts(&hosts);
            self.hosts = self.sort_hosts(&hosts);
            self.apply_filter();
            self.refresh_sidebar_tags();
        }
    }

    pub fn rebuild_display_rows(&mut self) {
        // If search is active or tag filter is active, flat list without group headers
        if !self.search_query.is_empty() || self.sidebar_active_tag.is_some() {
            self.display_rows = (0..self.filtered_hosts.len())
                .map(DisplayRow::HostRow)
                .collect();
            return;
        }

        let mut rows: Vec<DisplayRow> = Vec::new();
        let ordered_groups = self
            .groups
            .ordered_groups()
            .iter()
            .map(|g| (g.name.clone(), g.collapsed))
            .collect::<Vec<_>>();

        for (group_name, collapsed) in &ordered_groups {
            let group_hosts: Vec<usize> = self
                .filtered_hosts
                .iter()
                .enumerate()
                .filter(|(_, h)| self.groups.get_group_for_host(&h.name) == Some(group_name.as_str()))
                .map(|(i, _)| i)
                .collect();

            if group_hosts.is_empty() {
                continue;
            }

            rows.push(DisplayRow::GroupHeader {
                name: group_name.clone(),
                host_count: group_hosts.len(),
                collapsed: *collapsed,
            });

            if !collapsed {
                for idx in group_hosts {
                    rows.push(DisplayRow::HostRow(idx));
                }
            }
        }

        // Ungrouped hosts
        let ungrouped: Vec<usize> = self
            .filtered_hosts
            .iter()
            .enumerate()
            .filter(|(_, h)| self.groups.get_group_for_host(&h.name).is_none())
            .map(|(i, _)| i)
            .collect();

        if !ungrouped.is_empty() {
            // Only show "Ungrouped" header if there are actual groups defined
            if !self.groups.groups.is_empty() {
                rows.push(DisplayRow::GroupHeader {
                    name: "Ungrouped".to_string(),
                    host_count: ungrouped.len(),
                    collapsed: self.ungrouped_collapsed,
                });
            }
            if !self.ungrouped_collapsed || self.groups.groups.is_empty() {
                for idx in ungrouped {
                    rows.push(DisplayRow::HostRow(idx));
                }
            }
        }

        // If no groups defined at all, just use flat list
        if self.groups.groups.is_empty() {
            self.display_rows = (0..self.filtered_hosts.len())
                .map(DisplayRow::HostRow)
                .collect();
        } else {
            self.display_rows = rows;
        }
    }

    pub fn visible_rows(&self) -> usize {
        // When terminal height < 20, compact title uses 1 line instead of 5
        // Compact: title (1) + search bar (3) + table header (2) + status bar (1) + padding (2) = 9
        // Full:    title (5) + search bar (3) + table header (2) + status bar (1) + padding (2) = 13
        let reserved = if self.height < 20 { 9u16 } else { 13u16 };
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

        // Stable partition: favorites first, preserving sort order within each group
        let mut favs: Vec<SshHost> = Vec::new();
        let mut rest: Vec<SshHost> = Vec::new();
        for host in sorted {
            if self.favorites.is_favorite(&host.name) {
                favs.push(host);
            } else {
                rest.push(host);
            }
        }
        favs.extend(rest);
        favs
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
        self.rebuild_display_rows();
        // Re-clamp after display_rows are built (group headers add rows)
        let row_count = self.display_rows.len();
        if row_count > 0 && self.selected >= row_count {
            self.selected = row_count - 1;
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
        let row_count = if self.display_rows.is_empty() {
            self.filtered_hosts.len()
        } else {
            self.display_rows.len()
        };
        if row_count > 0 && self.selected < row_count - 1 {
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
        if self.display_rows.is_empty() {
            return self.filtered_hosts.get(self.selected);
        }
        match self.display_rows.get(self.selected) {
            Some(DisplayRow::HostRow(idx)) => self.filtered_hosts.get(*idx),
            _ => None,
        }
    }

    pub fn toggle_select(&mut self) {
        if let Some(host) = self.selected_host() {
            let name = host.name.clone();
            if self.selected_hosts.contains(&name) {
                self.selected_hosts.remove(&name);
            } else {
                self.selected_hosts.insert(name);
            }
        }
    }

    pub fn select_all(&mut self) {
        for host in &self.filtered_hosts {
            self.selected_hosts.insert(host.name.clone());
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_hosts.clear();
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_hosts.is_empty()
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
