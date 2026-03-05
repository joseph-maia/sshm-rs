use crate::{sftp::SftpBrowser, ssh::{Auth, SshConnection}, terminal::TerminalPanel};
use anyhow::Result;
use ratatui::widgets::ListState;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Terminal,
    Sftp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    Edit,
    Download,
    Open,
    GoUp,
    Refresh,
    Zip,
    DownloadAsZip,
    Delete,
    Upload,
}

#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: String,
    pub action: ContextAction,
}

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub x: u16,
    pub y: u16,
    pub items: Vec<ContextMenuItem>,
    pub selected: usize,
}

pub struct App {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: Auth,
    pub ssh: Option<SshConnection>,
    pub terminal: TerminalPanel,
    pub sftp: SftpBrowser,
    pub focus: PanelFocus,
    pub show_sftp: bool,
    pub should_quit: bool,
    pub status_message: String,
    pub sftp_list_area: Option<ratatui::layout::Rect>,
    pub sftp_breadcrumb_area: Option<ratatui::layout::Rect>,
    pub sftp_editing_path: bool,
    pub sftp_path_input: String,
    pub sftp_list_state: ListState,
    pub sftp_follow_terminal: bool,
    pub pending_edit: Option<String>,
    pub pending_upload: bool,
    pub context_menu: Option<ContextMenu>,
    pub confirm_delete: Option<(String, bool)>, // (remote_path, is_dir)
    pub frame_area: ratatui::layout::Rect,
    pub snippet_overlay: Option<crate::snippets::SnippetOverlay>,
    last_click: Option<(usize, Instant)>,
    last_snippet_click: Option<(usize, Instant)>,
}

impl App {
    pub fn new(host: String, port: u16, user: String, auth: Auth) -> Self {
        Self {
            host,
            port,
            user,
            auth,
            ssh: None,
            terminal: TerminalPanel::new(80, 24),
            sftp: SftpBrowser::new(),
            focus: PanelFocus::Terminal,
            show_sftp: false,
            should_quit: false,
            status_message: String::from("Connecting…"),
            sftp_list_area: None,
            sftp_breadcrumb_area: None,
            sftp_editing_path: false,
            sftp_path_input: String::new(),
            sftp_list_state: ListState::default(),
            sftp_follow_terminal: false,
            pending_edit: None,
            pending_upload: false,
            context_menu: None,
            confirm_delete: None,
            frame_area: ratatui::layout::Rect::default(),
            snippet_overlay: None,
            last_click: None,
            last_snippet_click: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let conn = SshConnection::connect(
            self.host.clone(),
            self.port,
            self.user.clone(),
            self.auth.clone(),
        )
        .await?;
        self.ssh = Some(conn);
        self.status_message = format!("Connected to {}:{}", self.host, self.port);
        Ok(())
    }

    fn build_context_menu_items(is_dotdot: bool, is_dir: bool) -> Vec<ContextMenuItem> {
        if is_dotdot {
            vec![
                ContextMenuItem { label: "Go up (Backspace)".to_string(), action: ContextAction::GoUp },
                ContextMenuItem { label: "Upload here (u)".to_string(), action: ContextAction::Upload },
                ContextMenuItem { label: "Refresh (r)".to_string(), action: ContextAction::Refresh },
            ]
        } else if is_dir {
            vec![
                ContextMenuItem { label: "Open (Enter)".to_string(), action: ContextAction::Open },
                ContextMenuItem { label: "Upload here (u)".to_string(), action: ContextAction::Upload },
                ContextMenuItem { label: "Archive".to_string(), action: ContextAction::Zip },
                ContextMenuItem { label: "Download as Archive".to_string(), action: ContextAction::DownloadAsZip },
                ContextMenuItem { label: "Delete".to_string(), action: ContextAction::Delete },
                ContextMenuItem { label: "Refresh (r)".to_string(), action: ContextAction::Refresh },
            ]
        } else {
            vec![
                ContextMenuItem { label: "Edit (e)".to_string(), action: ContextAction::Edit },
                ContextMenuItem { label: "Download (d)".to_string(), action: ContextAction::Download },
                ContextMenuItem { label: "Delete".to_string(), action: ContextAction::Delete },
                ContextMenuItem { label: "Refresh (r)".to_string(), action: ContextAction::Refresh },
            ]
        }
    }

    fn context_menu_rect(menu: &ContextMenu, frame_area: ratatui::layout::Rect) -> ratatui::layout::Rect {
        let width = menu.items.iter().map(|i| i.label.len()).max().unwrap_or(10) as u16 + 4;
        let height = menu.items.len() as u16 + 2;
        let x = menu.x.min(frame_area.width.saturating_sub(width));
        let y = menu.y.min(frame_area.height.saturating_sub(height));
        ratatui::layout::Rect::new(x, y, width, height)
    }

    async fn execute_context_action(&mut self, action: ContextAction) -> Result<()> {
        match action {
            ContextAction::Edit => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if !entry.is_dir {
                        let remote_path = crate::sftp::posix_join(&self.sftp.current_path, &entry.name);
                        self.pending_edit = Some(remote_path);
                    }
                }
            }
            ContextAction::Download => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if !entry.is_dir {
                        let remote_path = crate::sftp::posix_join(&self.sftp.current_path, &entry.name);
                        let entry_name = entry.name.clone();
                        let download_dir = get_download_dir();
                        match self.sftp.download_to_local(&remote_path, &download_dir).await {
                            Ok((bytes, local_path)) => {
                                self.status_message = format!(
                                    "Downloaded {} ({} bytes) to {}",
                                    entry_name,
                                    bytes,
                                    local_path.display()
                                );
                                let folder = local_path.parent().unwrap_or(&download_dir).to_path_buf();
                                open_folder(&folder);
                            }
                            Err(e) => {
                                self.status_message = format!("Download failed: {e}");
                            }
                        }
                    }
                }
            }
            ContextAction::Open => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if entry.is_dir && entry.name != ".." {
                        let new_path = crate::sftp::posix_join(&self.sftp.current_path, &entry.name);
                        self.sftp.navigate_to(new_path).await?;
                    }
                }
            }
            ContextAction::GoUp => {
                if let Some(parent) = crate::sftp::posix_parent(&self.sftp.current_path) {
                    self.sftp.navigate_to(parent).await?;
                }
            }
            ContextAction::Refresh => {
                self.sftp.list_directory().await?;
            }
            ContextAction::Zip => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if entry.is_dir && entry.name != ".." {
                        let dir_name = entry.name.clone();
                        let parent = self.sftp.current_path.clone();
                        self.status_message = format!("Archiving {}...", dir_name);

                        if let Some(ssh) = &self.ssh {
                            match create_remote_archive(ssh, &parent, &dir_name).await {
                                Ok((_path, archive_name)) => {
                                    self.status_message = format!("Created {}", archive_name);
                                    self.sftp.list_directory().await?;
                                }
                                Err(e) => {
                                    self.status_message = format!("Archive failed: {e}");
                                }
                            }
                        }
                    }
                }
            }
            ContextAction::Delete => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if entry.name != ".." {
                        let remote_path = crate::sftp::posix_join(&self.sftp.current_path, &entry.name);
                        let name = entry.name.clone();
                        self.confirm_delete = Some((remote_path, entry.is_dir));
                        self.status_message = format!("Delete {}? (y/n)", name);
                    }
                }
            }
            ContextAction::DownloadAsZip => {
                if let Some(entry) = self.sftp.entries.get(self.sftp.selected_index) {
                    if entry.is_dir && entry.name != ".." {
                        let dir_name = entry.name.clone();
                        let parent = self.sftp.current_path.clone();
                        self.status_message = format!("Archiving {}...", dir_name);

                        if let Some(ssh) = &self.ssh {
                            match create_remote_archive(ssh, &parent, &dir_name).await {
                                Ok((archive_path, archive_name)) => {
                                    self.status_message = format!("Downloading {}...", archive_name);
                                    let download_dir = get_download_dir();
                                    match self.sftp.download_to_local(&archive_path, &download_dir).await {
                                        Ok((bytes, local_path)) => {
                                            if let Some(ssh2) = &self.ssh {
                                                let rm_cmd = format!("rm -f {}", shell_escape(&archive_path));
                                                let _ = ssh2.exec_command(&rm_cmd).await;
                                            }
                                            self.status_message = format!(
                                                "Downloaded {} ({} bytes)",
                                                archive_name, bytes
                                            );
                                            let folder = local_path.parent().unwrap_or(&download_dir).to_path_buf();
                                            open_folder(&folder);
                                        }
                                        Err(e) => {
                                            if let Some(ssh2) = &self.ssh {
                                                let rm_cmd = format!("rm -f {}", shell_escape(&archive_path));
                                                let _ = ssh2.exec_command(&rm_cmd).await;
                                            }
                                            self.status_message = format!("Download failed: {e}");
                                        }
                                    }
                                    self.sftp.list_directory().await?;
                                }
                                Err(e) => {
                                    self.status_message = format!("Archive failed: {e}");
                                }
                            }
                        }
                    }
                }
            }
            ContextAction::Upload => {
                self.pending_upload = true;
            }
        }
        Ok(())
    }

    pub async fn handle_event(&mut self, event: crate::event::Event) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
        use crate::event::Event;

        // Context menu intercepts all input when visible
        if self.context_menu.is_some() {
            match &event {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Up => {
                            if let Some(ref mut menu) = self.context_menu {
                                if menu.selected > 0 {
                                    menu.selected -= 1;
                                }
                            }
                            return Ok(());
                        }
                        KeyCode::Down => {
                            if let Some(ref mut menu) = self.context_menu {
                                if menu.selected + 1 < menu.items.len() {
                                    menu.selected += 1;
                                }
                            }
                            return Ok(());
                        }
                        KeyCode::Enter => {
                            let action = self.context_menu.as_ref().and_then(|m| {
                                m.items.get(m.selected).map(|i| i.action)
                            });
                            self.context_menu = None;
                            if let Some(action) = action {
                                self.execute_context_action(action).await?;
                            }
                            return Ok(());
                        }
                        KeyCode::Esc => {
                            self.context_menu = None;
                            return Ok(());
                        }
                        _ => {
                            self.context_menu = None;
                            return Ok(());
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let cx = mouse.column;
                            let cy = mouse.row;
                            let action = if let Some(ref menu) = self.context_menu {
                                let rect = Self::context_menu_rect(menu, self.frame_area);
                                if cx >= rect.x && cx < rect.x + rect.width
                                    && cy >= rect.y && cy < rect.y + rect.height
                                {
                                    let item_y = cy.saturating_sub(rect.y + 1);
                                    menu.items.get(item_y as usize).map(|i| i.action)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            self.context_menu = None;
                            if let Some(act) = action {
                                self.execute_context_action(act).await?;
                            }
                            return Ok(());
                        }
                        MouseEventKind::Down(MouseButton::Right) => {
                            self.context_menu = None;
                            // Fall through to handle new right-click below
                        }
                        _ => {
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }

        // Snippet overlay intercepts all input when visible
        if self.snippet_overlay.is_some() {
            match &event {
                Event::Key(key) => {
                    let key = *key;
                    self.handle_snippet_key(key).await?;
                    return Ok(());
                }
                Event::Mouse(mouse) => {
                    let mouse = *mouse;
                    self.handle_snippet_mouse(mouse).await?;
                    return Ok(());
                }
                _ => {}
            }
        }

        // Confirmation dialog intercepts all input when a delete is pending
        if self.confirm_delete.is_some() {
            if let Event::Key(key) = &event {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some((path, is_dir)) = self.confirm_delete.take() {
                            if let Some(ssh) = &self.ssh {
                                let cmd = if is_dir {
                                    format!("rm -rf {}", shell_escape(&path))
                                } else {
                                    format!("rm -f {}", shell_escape(&path))
                                };
                                match ssh.exec_command(&cmd).await {
                                    Ok(_) => {
                                        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                                        self.status_message = format!("Deleted {}", name);
                                        self.sftp.list_directory().await?;
                                    }
                                    Err(e) => {
                                        self.status_message = format!("Delete failed: {e}");
                                    }
                                }
                            }
                        }
                        return Ok(());
                    }
                    _ => {
                        self.confirm_delete = None;
                        self.status_message = "Delete cancelled".to_string();
                        return Ok(());
                    }
                }
            } else {
                return Ok(());
            }
        }

        match event {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                    self.should_quit = true;
                    return Ok(());
                }

                if self.sftp_editing_path {
                    match key.code {
                        KeyCode::Enter => {
                            let path = self.sftp_path_input.clone();
                            self.sftp_editing_path = false;
                            self.sftp.navigate_to(path).await?;
                        }
                        KeyCode::Esc => {
                            self.sftp_editing_path = false;
                        }
                        KeyCode::Char(c) => {
                            self.sftp_path_input.push(c);
                        }
                        KeyCode::Backspace => {
                            self.sftp_path_input.pop();
                        }
                        _ => {}
                    }
                    return Ok(());
                }

                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                    self.focus = match self.focus {
                        PanelFocus::Terminal => PanelFocus::Sftp,
                        PanelFocus::Sftp => PanelFocus::Terminal,
                    };
                    return Ok(());
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
                    self.show_sftp = !self.show_sftp;
                    return Ok(());
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
                    self.sftp_follow_terminal = !self.sftp_follow_terminal;
                    self.status_message = if self.sftp_follow_terminal {
                        "SFTP follow: ON".to_string()
                    } else {
                        "SFTP follow: OFF".to_string()
                    };
                    return Ok(());
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('p') {
                    let snippets = crate::snippets::load_snippets();
                    self.snippet_overlay = Some(crate::snippets::SnippetOverlay::new(snippets));
                    return Ok(());
                }
                match self.focus {
                    PanelFocus::Terminal => {
                        if let Some(ssh) = &mut self.ssh {
                            ssh.send_input(key).await?;
                        }
                    }
                    PanelFocus::Sftp => {
                        if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
                            self.sftp_editing_path = true;
                            self.sftp_path_input =
                                self.sftp.current_path.clone();
                            return Ok(());
                        }
                        if key.code == KeyCode::Char('d') && key.modifiers.is_empty() {
                            self.execute_context_action(ContextAction::Download).await?;
                            return Ok(());
                        }
                        if key.code == KeyCode::Char('e') && key.modifiers.is_empty() {
                            self.execute_context_action(ContextAction::Edit).await?;
                            return Ok(());
                        }
                        if key.code == KeyCode::Char('u') && key.modifiers.is_empty() {
                            self.pending_upload = true;
                            return Ok(());
                        }
                        self.sftp.handle_key(key).await?;
                    }
                }
            }
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let x = mouse.column;
                        let y = mouse.row;

                        if let Some(area) = self.sftp_breadcrumb_area {
                            if x >= area.x
                                && x < area.x + area.width
                                && y >= area.y
                                && y < area.y + area.height
                            {
                                self.sftp_editing_path = true;
                                self.sftp_path_input =
                                    self.sftp.current_path.clone();
                                self.focus = PanelFocus::Sftp;
                                return Ok(());
                            }
                        }

                        if let Some(area) = self.sftp_list_area {
                            if x >= area.x
                                && x < area.x + area.width
                                && y >= area.y
                                && y < area.y + area.height
                            {
                                let inner_y = y.saturating_sub(area.y + 1);
                                let scroll_offset = self.sftp_list_state.offset();
                                let clicked_index = scroll_offset + inner_y as usize;

                                if clicked_index < self.sftp.entries.len() {
                                    self.focus = PanelFocus::Sftp;

                                    let is_double = self.last_click
                                        .map(|(idx, t)| idx == clicked_index && t.elapsed().as_millis() < 400)
                                        .unwrap_or(false);

                                    if is_double {
                                        self.last_click = None;
                                        if let Some(entry) =
                                            self.sftp.entries.get(clicked_index)
                                        {
                                            if entry.name == ".." {
                                                if let Some(parent) =
                                                    crate::sftp::posix_parent(&self.sftp.current_path)
                                                {
                                                    self.sftp
                                                        .navigate_to(parent)
                                                        .await?;
                                                }
                                            } else if entry.is_dir {
                                                let new_path =
                                                    crate::sftp::posix_join(&self.sftp.current_path, &entry.name);
                                                self.sftp.navigate_to(new_path).await?;
                                            }
                                        }
                                    } else {
                                        self.last_click = Some((clicked_index, Instant::now()));
                                        self.sftp.selected_index = clicked_index;
                                    }
                                }
                                return Ok(());
                            }
                        }

                        self.focus = PanelFocus::Terminal;
                    }
                    MouseEventKind::Down(MouseButton::Right) => {
                        let x = mouse.column;
                        let y = mouse.row;

                        if let Some(area) = self.sftp_list_area {
                            if x >= area.x
                                && x < area.x + area.width
                                && y >= area.y
                                && y < area.y + area.height
                            {
                                let inner_y = y.saturating_sub(area.y + 1);
                                let scroll_offset = self.sftp_list_state.offset();
                                let clicked_index = scroll_offset + inner_y as usize;

                                if clicked_index < self.sftp.entries.len() {
                                    self.focus = PanelFocus::Sftp;
                                    self.sftp.selected_index = clicked_index;

                                    let entry = &self.sftp.entries[clicked_index];
                                    let is_dotdot = entry.name == "..";
                                    let is_dir = entry.is_dir;
                                    let items = Self::build_context_menu_items(is_dotdot, is_dir);

                                    self.context_menu = Some(ContextMenu {
                                        x,
                                        y,
                                        items,
                                        selected: 0,
                                    });
                                }
                                return Ok(());
                            }
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if self.focus == PanelFocus::Sftp {
                            self.sftp.selected_index =
                                self.sftp.selected_index.saturating_sub(3);
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if self.focus == PanelFocus::Sftp && !self.sftp.entries.is_empty() {
                            self.sftp.selected_index =
                                (self.sftp.selected_index + 3)
                                    .min(self.sftp.entries.len().saturating_sub(1));
                        }
                    }
                    _ => {}
                }
            }
            Event::SshOutput(bytes) => {
                self.terminal.process_output(&bytes);
                if self.sftp_follow_terminal {
                    if let Some(cwd) = self.terminal.detected_cwd.take() {
                        if cwd != self.sftp.current_path {
                            let _ = self.sftp.navigate_to(cwd).await;
                        }
                    }
                }
            }
            Event::SshEof => {
                self.status_message = "Connection closed.".to_string();
                self.should_quit = true;
            }
            Event::Resize(cols, rows) => {
                let term_cols = if self.show_sftp {
                    (cols as f32 * 0.65) as u16
                } else {
                    cols
                };
                self.terminal.resize(term_cols.saturating_sub(2), rows.saturating_sub(3));

                if let Some(ssh) = &mut self.ssh {
                    ssh.resize_pty(term_cols.saturating_sub(2) as u32, rows.saturating_sub(3) as u32).await?;
                }
            }
            Event::Paste(text) => {
                if let Some(ssh) = &self.ssh {
                    ssh.send_raw_bytes(text.as_bytes()).await?;
                }
            }
            Event::Tick => {}
        }
        Ok(())
    }
    async fn handle_snippet_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let overlay = match self.snippet_overlay.as_mut() {
            Some(o) => o,
            None => return Ok(()),
        };

        match overlay.mode {
            crate::snippets::SnippetMode::Browse => {
                match key.code {
                    KeyCode::Esc => {
                        self.snippet_overlay = None;
                    }
                    KeyCode::Up => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.move_up();
                        }
                    }
                    KeyCode::Down => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.move_down();
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(overlay) = self.snippet_overlay.as_ref() {
                            if let Some(snippet) = overlay.selected_snippet() {
                                let cmd = format!("{}\r", snippet.command);
                                let name = snippet.name.clone();
                                if let Some(ssh) = &self.ssh {
                                    ssh.send_raw_bytes(cmd.as_bytes()).await?;
                                }
                                self.snippet_overlay = None;
                                self.status_message = format!("Executed: {}", name);
                            }
                        }
                    }
                    KeyCode::Char('a') if key.modifiers.is_empty() => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.mode = crate::snippets::SnippetMode::Add;
                            o.form = Some(crate::snippets::AddForm::new());
                        }
                    }
                    KeyCode::Char('e') if key.modifiers.is_empty() => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(&idx) = o.filtered_indices.get(o.selected_index) {
                                let s = &o.snippets[idx];
                                let form = crate::snippets::AddForm::from_snippet(
                                    idx,
                                    &s.name,
                                    &s.command,
                                    &s.description,
                                );
                                o.form = Some(form);
                                o.mode = crate::snippets::SnippetMode::Edit;
                            }
                        }
                    }
                    KeyCode::Char('d') if key.modifiers.is_empty() => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if o.selected_snippet().is_some() {
                                o.mode = crate::snippets::SnippetMode::ConfirmDelete;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.search_input.pop();
                            o.update_filter();
                        }
                    }
                    KeyCode::Char(c)
                        if key.modifiers.is_empty()
                            || key.modifiers == KeyModifiers::SHIFT =>
                    {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.search_input.push(c);
                            o.update_filter();
                        }
                    }
                    _ => {}
                }
            }
            crate::snippets::SnippetMode::Add | crate::snippets::SnippetMode::Edit => {
                match key.code {
                    KeyCode::Esc => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.form = None;
                            o.mode = crate::snippets::SnippetMode::Browse;
                        }
                    }
                    KeyCode::Tab => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(form) = &mut o.form {
                                form.active_field = form.active_field.next();
                            }
                        }
                    }
                    KeyCode::BackTab => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(form) = &mut o.form {
                                form.active_field = form.active_field.prev();
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(form) = o.form.take() {
                                if !form.name.is_empty() && !form.command.is_empty() {
                                    let desc = form.description.clone();
                                    if let Some(edit_idx) = form.editing_index {
                                        if let Some(s) = o.snippets.get_mut(edit_idx) {
                                            s.name = form.name.clone();
                                            s.command = form.command.clone();
                                            s.description = desc;
                                        }
                                        self.status_message =
                                            format!("Updated: {}", form.name);
                                    } else {
                                        let new_snippet = crate::snippets::Snippet {
                                            name: form.name.clone(),
                                            command: form.command.clone(),
                                            description: desc,
                                        };
                                        o.snippets.push(new_snippet);
                                        self.status_message =
                                            format!("Added: {}", form.name);
                                    }
                                    crate::snippets::save_snippets(&o.snippets);
                                    o.update_filter();
                                }
                                o.mode = crate::snippets::SnippetMode::Browse;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(form) = &mut o.form {
                                form.active_field_mut().pop();
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(form) = &mut o.form {
                                form.active_field_mut().push(c);
                            }
                        }
                    }
                    _ => {}
                }
            }
            crate::snippets::SnippetMode::ConfirmDelete => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            if let Some(&idx) = o.filtered_indices.get(o.selected_index) {
                                let name = o.snippets[idx].name.clone();
                                o.snippets.remove(idx);
                                crate::snippets::save_snippets(&o.snippets);
                                o.update_filter();
                                self.status_message = format!("Deleted: {}", name);
                            }
                            o.mode = crate::snippets::SnippetMode::Browse;
                        }
                    }
                    _ => {
                        if let Some(o) = self.snippet_overlay.as_mut() {
                            o.mode = crate::snippets::SnippetMode::Browse;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_snippet_mouse(
        &mut self,
        mouse: crossterm::event::MouseEvent,
    ) -> Result<()> {
        use crossterm::event::{MouseButton, MouseEventKind};

        let overlay = match self.snippet_overlay.as_mut() {
            Some(o) => o,
            None => return Ok(()),
        };

        if overlay.mode != crate::snippets::SnippetMode::Browse {
            return Ok(());
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let x = mouse.column;
                let y = mouse.row;

                if let Some(list_area) = overlay.list_area {
                    if x >= list_area.x
                        && x < list_area.x + list_area.width
                        && y >= list_area.y
                        && y < list_area.y + list_area.height
                    {
                        let inner_y = y.saturating_sub(list_area.y) as usize;
                        let clicked_index = overlay.scroll_offset + inner_y / 3;

                        if clicked_index < overlay.filtered_indices.len() {
                            let is_double = self
                                .last_snippet_click
                                .map(|(idx, t)| {
                                    idx == clicked_index && t.elapsed().as_millis() < 400
                                })
                                .unwrap_or(false);

                            if is_double {
                                self.last_snippet_click = None;
                                if let Some(o) = self.snippet_overlay.as_mut() {
                                    o.selected_index = clicked_index;
                                }
                                if let Some(overlay) = self.snippet_overlay.as_ref() {
                                    if let Some(snippet) = overlay.selected_snippet() {
                                        let cmd = format!("{}\r", snippet.command);
                                        let name = snippet.name.clone();
                                        if let Some(ssh) = &self.ssh {
                                            ssh.send_raw_bytes(cmd.as_bytes()).await?;
                                        }
                                        self.snippet_overlay = None;
                                        self.status_message = format!("Executed: {}", name);
                                    }
                                }
                            } else {
                                self.last_snippet_click =
                                    Some((clicked_index, Instant::now()));
                                if let Some(o) = self.snippet_overlay.as_mut() {
                                    o.selected_index = clicked_index;
                                }
                            }
                        }
                        return Ok(());
                    }
                }

                // Click outside overlay = close
                let close = if let Some(overlay_area) = overlay.overlay_area {
                    !(x >= overlay_area.x
                        && x < overlay_area.x + overlay_area.width
                        && y >= overlay_area.y
                        && y < overlay_area.y + overlay_area.height)
                } else {
                    false
                };
                if close {
                    self.snippet_overlay = None;
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                let x = mouse.column;
                let y = mouse.row;

                if let Some(list_area) = overlay.list_area {
                    if x >= list_area.x
                        && x < list_area.x + list_area.width
                        && y >= list_area.y
                        && y < list_area.y + list_area.height
                    {
                        let inner_y = y.saturating_sub(list_area.y) as usize;
                        let clicked_index = overlay.scroll_offset + inner_y / 3;

                        if clicked_index < overlay.filtered_indices.len() {
                            overlay.selected_index = clicked_index;
                            if let Some(&idx) = overlay.filtered_indices.get(clicked_index) {
                                let s = &overlay.snippets[idx];
                                let form = crate::snippets::AddForm::from_snippet(
                                    idx,
                                    &s.name,
                                    &s.command,
                                    &s.description,
                                );
                                overlay.form = Some(form);
                                overlay.mode = crate::snippets::SnippetMode::Edit;
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                overlay.move_up();
            }
            MouseEventKind::ScrollDown => {
                overlay.move_down();
            }
            _ => {}
        }
        Ok(())
    }
}

fn open_folder(path: &std::path::Path) {
    if cfg!(windows) {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    } else if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("open").arg(path).spawn();
    } else {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

fn get_download_dir() -> std::path::PathBuf {
    let base = if cfg!(windows) {
        std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
    } else {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    };
    std::path::PathBuf::from(base).join("Downloads")
}

/// Simple shell escaping for remote command arguments.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Create an archive of a remote directory. Tries `zip` first, falls back to `tar.gz`.
/// Returns (archive_remote_path, archive_filename).
async fn create_remote_archive(
    ssh: &crate::ssh::SshConnection,
    parent_dir: &str,
    dir_name: &str,
) -> Result<(String, String)> {
    let has_zip = ssh.exec_command("command -v zip").await.is_ok();

    let (archive_name, cmd) = if has_zip {
        let name = format!("{}.zip", dir_name);
        let c = format!(
            "cd {} && zip -r {} {}",
            shell_escape(parent_dir),
            shell_escape(&name),
            shell_escape(dir_name)
        );
        (name, c)
    } else {
        let name = format!("{}.tar.gz", dir_name);
        let c = format!(
            "cd {} && tar -czf {} {}",
            shell_escape(parent_dir),
            shell_escape(&name),
            shell_escape(dir_name)
        );
        (name, c)
    };

    let archive_path = crate::sftp::posix_join(parent_dir, &archive_name);
    ssh.exec_command(&cmd).await?;

    // Verify the archive was actually created
    let check = format!("test -f {} && echo OK", shell_escape(&archive_path));
    let check_output = ssh.exec_command(&check).await
        .map_err(|_| anyhow::anyhow!("Archive was not created at {}", archive_path))?;
    if !check_output.trim().contains("OK") {
        anyhow::bail!("Archive was not created at {}", archive_path);
    }

    Ok((archive_path, archive_name))
}
