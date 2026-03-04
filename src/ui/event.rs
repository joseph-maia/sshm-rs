use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

use crate::theme::Theme;
use crate::ui::app::{AddField, App, ViewMode};
use crate::ui::styles;

/// Poll for crossterm events, returning true if the terminal was resized.
pub fn poll_event(app: &mut App) -> anyhow::Result<bool> {
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) => {
                // On Windows, crossterm sends both Press and Release events.
                // Only handle Press to avoid double-processing.
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key);
                }
            }
            Event::Mouse(mouse) => {
                handle_mouse(app, mouse);
            }
            Event::Resize(w, h) => {
                app.width = w;
                app.height = h;
                return Ok(true);
            }
            _ => {}
        }
    }
    Ok(false)
}

fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.view_mode {
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Help => handle_help_key(app, key),
        ViewMode::DeleteConfirm => handle_delete_key(app, key),
        ViewMode::Info => handle_info_key(app, key),
        ViewMode::Add => handle_add_key(app, key),
        ViewMode::Edit => handle_edit_key(app, key),
        ViewMode::Password => handle_password_key(app, key),
        ViewMode::PortForward => handle_port_forward_key(app, key),
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    // Only handle mouse events in List view mode when not in search mode
    if app.view_mode != ViewMode::List {
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.move_up();
        }
        MouseEventKind::ScrollDown => {
            app.move_down();
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Layout: title (5 or 1 lines) + search bar (3 lines) + table border (1 line) + header (1 line)
            let title_height: u16 = if app.height < 20 { 1 } else { 5 };
            let table_top_offset: u16 = title_height + 3 + 1 + 1;
            if mouse.row >= table_top_offset {
                let clicked_index = app.table_offset + (mouse.row - table_top_offset) as usize;
                if clicked_index < app.filtered_hosts.len() {
                    // Double-click detection: same row within 400ms → connect
                    let now = Instant::now();
                    if let (Some(last_time), Some(last_idx)) = (app.last_click_time, app.last_click_index) {
                        if last_idx == clicked_index && now.duration_since(last_time) < Duration::from_millis(400) {
                            app.selected = clicked_index;
                            if let Some(host) = app.selected_host() {
                                app.connect_host = Some(host.name.clone());
                                app.should_quit = true;
                            }
                            app.last_click_time = None;
                            app.last_click_index = None;
                            return;
                        }
                    }
                    app.selected = clicked_index;
                    app.clamp_offset();
                    app.last_click_time = Some(now);
                    app.last_click_index = Some(clicked_index);
                }
            }
        }
        _ => {}
    }
}

fn handle_list_key(app: &mut App, key: KeyEvent) {
    if app.search_mode {
        handle_search_key(app, key);
    } else if app.sidebar_focused {
        handle_sidebar_key(app, key);
    } else {
        handle_table_key(app, key);
    }
}

fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_mode = false;
        }
        KeyCode::Enter | KeyCode::Tab => {
            // Validate search and switch to table navigation
            app.search_mode = false;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
        }
        _ => {}
    }
}

fn handle_table_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Esc => {
            // Clear selection first if active, otherwise clear search query
            if app.has_selection() {
                app.clear_selection();
            } else if !app.search_query.is_empty() {
                app.search_query.clear();
                app.apply_filter();
            }
        }
        KeyCode::Char(' ') => {
            app.toggle_select();
            app.move_down();
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.select_all();
        }
        KeyCode::Char('/') => {
            app.search_mode = true;
        }
        KeyCode::Char('?') | KeyCode::Char('h') => {
            app.view_mode = ViewMode::Help;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
        }
        KeyCode::Home => {
            app.selected = 0;
            app.table_offset = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            if !app.filtered_hosts.is_empty() {
                app.selected = app.filtered_hosts.len() - 1;
                let visible = app.visible_rows();
                if app.filtered_hosts.len() > visible {
                    app.table_offset = app.filtered_hosts.len() - visible;
                }
            }
        }
        KeyCode::PageUp => {
            let page = app.visible_rows();
            if app.selected >= page {
                app.selected -= page;
            } else {
                app.selected = 0;
            }
            app.clamp_offset();
        }
        KeyCode::PageDown => {
            let page = app.visible_rows();
            if !app.filtered_hosts.is_empty() {
                let last = app.filtered_hosts.len() - 1;
                if app.selected + page <= last {
                    app.selected += page;
                } else {
                    app.selected = last;
                }
                app.clamp_offset();
            }
        }
        KeyCode::Enter => {
            if app.has_selection() {
                let count = app.selected_hosts.len();
                app.show_toast(&format!("{count} host{} selected — d: delete | Esc: clear", if count == 1 { "" } else { "s" }));
            } else if let Some(host) = app.selected_host() {
                app.connect_host = Some(host.name.clone());
                app.should_quit = true;
            }
        }
        KeyCode::Char('s') => {
            app.sort_mode = app.sort_mode.toggle();
            app.apply_filter();
        }
        KeyCode::Char('d') => {
            if app.has_selection() {
                // Batch delete: encode count in delete_target as sentinel "__batch__:<count>"
                let count = app.selected_hosts.len();
                app.delete_target = Some(format!("__batch__:{count}"));
                app.view_mode = ViewMode::DeleteConfirm;
            } else if let Some(host) = app.selected_host() {
                app.delete_target = Some(host.name.clone());
                app.view_mode = ViewMode::DeleteConfirm;
            }
        }
        KeyCode::Char('i') => {
            if app.selected_host().is_some() {
                app.view_mode = ViewMode::Info;
            }
        }
        KeyCode::Tab => {
            app.search_mode = true;
        }
        KeyCode::Char('a') => {
            app.reset_add_form();
            app.view_mode = ViewMode::Add;
        }
        KeyCode::Char('p') => {
            if let Some(host) = app.selected_host() {
                app.password_target = Some(host.name.clone());
                app.password_input.clear();
                app.view_mode = ViewMode::Password;
            }
        }
        KeyCode::Char('f') => {
            if let Some(host) = app.selected_host().cloned() {
                let was_favorite = app.favorites.is_favorite(&host.name);
                let _ = app.favorites.toggle(&host.name);
                app.apply_filter();
                if was_favorite {
                    app.show_toast("Removed from favorites");
                } else {
                    app.show_toast("Added to favorites");
                }
            }
        }
        KeyCode::Char('r') => {
            // Refresh connectivity status for all hosts
            app.start_ping();
        }
        KeyCode::Char('F') => {
            if let Some(host) = app.selected_host().cloned() {
                app.pf_target = Some(host.name.clone());
                app.prefill_pf_form(&host.name);
                app.view_mode = ViewMode::PortForward;
            }
        }
        KeyCode::Char('y') => {
            if let Some(host) = app.selected_host() {
                let text = if host.user.is_empty() {
                    host.hostname.clone()
                } else {
                    format!("{}@{}", host.user, host.hostname)
                };
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if clipboard.set_text(&text).is_ok() {
                        app.show_toast(&format!("Copied: {text}"));
                    } else {
                        app.show_toast("Failed to copy to clipboard");
                    }
                } else {
                    app.show_toast("Clipboard not available");
                }
            }
        }
        KeyCode::Char('t') => {
            app.show_sidebar = !app.show_sidebar;
            if !app.show_sidebar {
                app.sidebar_focused = false;
            }
        }
        KeyCode::Char('T') => {
            let themes = Theme::builtin_themes();
            app.theme_index = (app.theme_index + 1) % themes.len();
            let new_theme = themes[app.theme_index].clone();
            let name = new_theme.name.clone();
            styles::set_theme(new_theme);
            app.show_toast(&format!("Theme: {name}"));
        }
        KeyCode::Left => {
            if app.show_sidebar {
                app.sidebar_focused = true;
            }
        }
        KeyCode::Char('e') => {
            if let Some(host) = app.selected_host().cloned() {
                // Pre-populate form fields with current host values
                app.add_fields[0] = host.name.clone();
                app.add_fields[1] = host.hostname.clone();
                app.add_fields[2] = host.user.clone();
                app.add_fields[3] = if host.port.is_empty() {
                    "22".to_string()
                } else {
                    host.port.clone()
                };
                app.add_fields[4] = if crate::credentials::has_password(&host.name) {
                    "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}".to_string()
                } else {
                    String::new()
                };
                app.add_fields[5] = host.identity.clone();
                app.add_fields[6] = host.tags.join(", ");
                app.add_focused = AddField::Name;
                app.add_error = None;
                app.edit_target = Some(host.name.clone());
                app.view_mode = ViewMode::Edit;
            }
        }
        _ => {}
    }
}

fn handle_sidebar_key(app: &mut App, key: KeyEvent) {
    // Total items: "All Hosts" (index 0) + tags (indices 1..=len)
    let total_items = app.sidebar_tags.len() + 1;

    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if app.sidebar_selected + 1 < total_items {
                app.sidebar_selected += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.sidebar_selected > 0 {
                app.sidebar_selected -= 1;
            }
        }
        KeyCode::Enter => {
            if app.sidebar_selected == 0 {
                // "All Hosts" selected -> clear filter
                app.sidebar_active_tag = None;
            } else {
                let tag_index = app.sidebar_selected - 1;
                if let Some(tag) = app.sidebar_tags.get(tag_index).cloned() {
                    if app.sidebar_active_tag.as_deref() == Some(&tag) {
                        // Toggle off: same tag selected again
                        app.sidebar_active_tag = None;
                    } else {
                        app.sidebar_active_tag = Some(tag);
                    }
                }
            }
            app.apply_filter();
        }
        KeyCode::Esc => {
            app.sidebar_active_tag = None;
            app.sidebar_focused = false;
            app.show_sidebar = false;
            app.apply_filter();
        }
        KeyCode::Right | KeyCode::Tab => {
            app.sidebar_focused = false;
        }
        KeyCode::Char('t') => {
            app.show_sidebar = false;
            app.sidebar_focused = false;
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        _ => {}
    }
}

fn handle_help_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Char('h')
        | KeyCode::Enter => {
            app.view_mode = ViewMode::List;
        }
        _ => {}
    }
}

fn handle_delete_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => {
            app.delete_target = None;
            app.view_mode = ViewMode::List;
        }
        KeyCode::Enter | KeyCode::Char('y') => {
            if let Some(ref target) = app.delete_target.clone() {
                if target.starts_with("__batch__:") {
                    // Batch delete all selected hosts
                    let names: Vec<String> = app.selected_hosts.iter().cloned().collect();
                    let count = names.len();
                    for name in &names {
                        if let Some(host) = app.hosts.iter().find(|h| h.name == *name).cloned() {
                            let _ = crate::config::delete_host(&host);
                        }
                    }
                    app.clear_selection();
                    app.reload_hosts();
                    app.show_toast(&format!("Deleted {count} host{}", if count == 1 { "" } else { "s" }));
                } else {
                    if let Some(host) = app.hosts.iter().find(|h| h.name == *target).cloned() {
                        let _ = crate::config::delete_host(&host);
                        app.reload_hosts();
                        app.show_toast("Host deleted");
                    }
                }
            }
            app.delete_target = None;
            app.view_mode = ViewMode::List;
        }
        _ => {}
    }
}

fn handle_info_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.view_mode = ViewMode::List;
        }
        _ => {}
    }
}

fn handle_add_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.view_mode = ViewMode::List;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.add_focused = app.add_focused.next();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.add_focused = app.add_focused.prev();
        }
        KeyCode::Backspace => {
            let idx = app.add_focused as usize;
            app.add_fields[idx].pop();
            app.add_error = None;
        }
        KeyCode::Char(c) => {
            let idx = app.add_focused as usize;
            app.add_fields[idx].push(c);
            app.add_error = None;
        }
        KeyCode::Enter => {
            // Validate and save
            let name = app.add_fields[0].trim().to_string();
            let hostname = app.add_fields[1].trim().to_string();

            if name.is_empty() {
                app.add_error = Some("Name is required".to_string());
                return;
            }
            if name.contains(' ') {
                app.add_error = Some("Name cannot contain spaces".to_string());
                return;
            }
            if hostname.is_empty() {
                app.add_error = Some("Hostname is required".to_string());
                return;
            }

            let password = app.add_fields[4].trim().to_string(); // password field
            let tags: Vec<String> = app.add_fields[6]
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            let host = crate::config::SshHost {
                name: name.clone(),
                hostname,
                user: app.add_fields[2].trim().to_string(),
                port: if app.add_fields[3].trim().is_empty() {
                    "22".to_string()
                } else {
                    app.add_fields[3].trim().to_string()
                },
                identity: app.add_fields[5].trim().to_string(),
                proxy_jump: String::new(),
                proxy_command: String::new(),
                options: String::new(),
                remote_command: String::new(),
                request_tty: String::new(),
                tags,
                source_file: app.config_path.clone(),
                line_number: 0,
            };

            match crate::config::add_host(&app.config_path, &host) {
                Ok(()) => {
                    // Save password to OS credential store if provided
                    if !password.is_empty() {
                        if let Err(e) = crate::credentials::save_password(&name, &password) {
                            app.add_error = Some(format!("Host saved but password failed: {e}"));
                            app.reload_hosts();
                            return;
                        }
                    }
                    app.reload_hosts();
                    app.show_toast("Host added successfully");
                    app.view_mode = ViewMode::List;
                }
                Err(e) => {
                    app.add_error = Some(format!("{e}"));
                }
            }
        }
        _ => {}
    }
}

const PASSWORD_PLACEHOLDER: &str = "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}";

fn handle_edit_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.edit_target = None;
            app.view_mode = ViewMode::List;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.add_focused = app.add_focused.next();
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.add_focused = app.add_focused.prev();
        }
        KeyCode::Backspace => {
            let idx = app.add_focused as usize;
            // If password field still has placeholder, clear it entirely on first edit
            if app.add_focused == AddField::Password
                && app.add_fields[idx] == PASSWORD_PLACEHOLDER
            {
                app.add_fields[idx].clear();
            } else {
                app.add_fields[idx].pop();
            }
            app.add_error = None;
        }
        KeyCode::Char(c) => {
            let idx = app.add_focused as usize;
            // If password field still has placeholder, clear it before typing
            if app.add_focused == AddField::Password
                && app.add_fields[idx] == PASSWORD_PLACEHOLDER
            {
                app.add_fields[idx].clear();
            }
            app.add_fields[idx].push(c);
            app.add_error = None;
        }
        KeyCode::Enter => {
            // Validate and save
            let name = app.add_fields[0].trim().to_string();
            let hostname = app.add_fields[1].trim().to_string();

            if name.is_empty() {
                app.add_error = Some("Name is required".to_string());
                return;
            }
            if name.contains(' ') {
                app.add_error = Some("Name cannot contain spaces".to_string());
                return;
            }
            if hostname.is_empty() {
                app.add_error = Some("Hostname is required".to_string());
                return;
            }

            let original_name = match app.edit_target.clone() {
                Some(n) => n,
                None => {
                    app.add_error = Some("Edit target lost".to_string());
                    return;
                }
            };

            // Check for duplicate name if name changed (but allow keeping the same name)
            if name != original_name {
                let duplicate = app.hosts.iter().any(|h| h.name == name);
                if duplicate {
                    app.add_error = Some(format!("Host '{}' already exists", name));
                    return;
                }
            }

            // Find the original host to get source_file and line_number
            let original_host = app.hosts.iter().find(|h| h.name == original_name).cloned();
            let (source_file, line_number) = match original_host {
                Some(ref h) => (h.source_file.clone(), h.line_number),
                None => {
                    app.add_error = Some("Original host not found".to_string());
                    return;
                }
            };

            let password_field = app.add_fields[4].trim().to_string();
            let tags: Vec<String> = app.add_fields[6]
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            let new_host = crate::config::SshHost {
                name: name.clone(),
                hostname,
                user: app.add_fields[2].trim().to_string(),
                port: if app.add_fields[3].trim().is_empty() {
                    "22".to_string()
                } else {
                    app.add_fields[3].trim().to_string()
                },
                identity: app.add_fields[5].trim().to_string(),
                proxy_jump: original_host
                    .as_ref()
                    .map(|h| h.proxy_jump.clone())
                    .unwrap_or_default(),
                proxy_command: original_host
                    .as_ref()
                    .map(|h| h.proxy_command.clone())
                    .unwrap_or_default(),
                options: original_host
                    .as_ref()
                    .map(|h| h.options.clone())
                    .unwrap_or_default(),
                remote_command: original_host
                    .as_ref()
                    .map(|h| h.remote_command.clone())
                    .unwrap_or_default(),
                request_tty: original_host
                    .as_ref()
                    .map(|h| h.request_tty.clone())
                    .unwrap_or_default(),
                tags,
                source_file,
                line_number,
            };

            // The update_host function uses the host's name field to find the block,
            // but we need to use the original name for the lookup. If the name changed,
            // we need to update under the original name first then the block will have
            // the new name.
            // Since update_host searches by host.name in the file, we need to create
            // a host with the original name for the search, but with new content.
            // Actually, update_host uses the name field to find the host block in the file.
            // If the user changed the name, we still need to find the OLD name in the file.
            // So we temporarily set name = original_name for the lookup, then the written
            // block uses the new name. Let's build a lookup host.
            let mut update_host_obj = new_host.clone();
            // update_host uses host.name to locate the block in the file
            // We need to find the block by the original name
            update_host_obj.name = original_name.clone();

            // First delete the old block, then add the new one
            // Actually, let's use the approach: if name didn't change, just update_host.
            // If name changed, delete old + add new.
            if name == original_name {
                match crate::config::update_host(&new_host) {
                    Ok(()) => {}
                    Err(e) => {
                        app.add_error = Some(format!("{e}"));
                        return;
                    }
                }
            } else {
                // Delete old host block using original name
                match crate::config::delete_host(&update_host_obj) {
                    Ok(()) => {}
                    Err(e) => {
                        app.add_error = Some(format!("Failed to remove old host: {e}"));
                        return;
                    }
                }
                // Add new host with new name
                match crate::config::add_host(&app.config_path, &new_host) {
                    Ok(()) => {}
                    Err(e) => {
                        app.add_error = Some(format!("Failed to add renamed host: {e}"));
                        app.reload_hosts();
                        return;
                    }
                }
            }

            // Handle password changes
            let old_had_password = crate::credentials::has_password(&original_name);
            let password_unchanged = password_field == PASSWORD_PLACEHOLDER;
            let password_cleared = password_field.is_empty();

            if password_unchanged {
                // Password was not touched - if name changed, migrate the credential
                if name != original_name && old_had_password {
                    if let Some(old_pw) = crate::credentials::get_password(&original_name) {
                        let _ = crate::credentials::save_password(&name, &old_pw);
                        let _ = crate::credentials::delete_password(&original_name);
                    }
                }
            } else if password_cleared {
                // User cleared the password field
                if old_had_password {
                    let _ = crate::credentials::delete_password(&original_name);
                    // Also delete under new name if renamed
                    if name != original_name {
                        let _ = crate::credentials::delete_password(&name);
                    }
                }
            } else {
                // User typed a new password
                if let Err(e) = crate::credentials::save_password(&name, &password_field) {
                    app.add_error =
                        Some(format!("Host saved but password failed: {e}"));
                    app.reload_hosts();
                    app.edit_target = None;
                    return;
                }
                // If name changed, clean up old credential
                if name != original_name && old_had_password {
                    let _ = crate::credentials::delete_password(&original_name);
                }
            }

            app.reload_hosts();
            app.edit_target = None;
            app.show_toast("Host updated");
            app.view_mode = ViewMode::List;
        }
        _ => {}
    }
}

fn handle_password_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.password_target = None;
            app.password_input.clear();
            app.view_mode = ViewMode::List;
        }
        KeyCode::Enter => {
            if let Some(ref target) = app.password_target.clone() {
                if !app.password_input.is_empty() {
                    let _ = crate::credentials::save_password(target, &app.password_input);
                    app.show_toast("Password saved");
                }
            }
            app.password_target = None;
            app.password_input.clear();
            app.view_mode = ViewMode::List;
        }
        KeyCode::Delete => {
            if let Some(ref target) = app.password_target.clone() {
                if crate::credentials::has_password(target) {
                    let _ = crate::credentials::delete_password(target);
                    app.show_toast("Password removed");
                }
            }
            app.password_target = None;
            app.password_input.clear();
            app.view_mode = ViewMode::List;
        }
        KeyCode::Backspace => {
            app.password_input.pop();
        }
        KeyCode::Char(c) => {
            app.password_input.push(c);
        }
        _ => {}
    }
}

fn handle_port_forward_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.pf_target = None;
            app.pf_error = None;
            app.view_mode = ViewMode::List;
        }
        KeyCode::Tab | KeyCode::Down => {
            app.pf_focused = (app.pf_focused + 1) % 5;
            app.pf_error = None;
        }
        KeyCode::BackTab | KeyCode::Up => {
            app.pf_focused = if app.pf_focused == 0 { 4 } else { app.pf_focused - 1 };
            app.pf_error = None;
        }
        KeyCode::Enter => {
            // Validate
            if app.pf_local_port.trim().is_empty() {
                app.pf_error = Some("Local port is required".to_string());
                return;
            }

            let forward_type = match app.pf_forward_type {
                1 => "remote",
                2 => "dynamic",
                _ => "local",
            };

            // For local/remote, remote_host and remote_port are required
            if app.pf_forward_type != 2 {
                if app.pf_remote_host.trim().is_empty() {
                    app.pf_error = Some("Remote host is required for local/remote forwarding".to_string());
                    return;
                }
                if app.pf_remote_port.trim().is_empty() {
                    app.pf_error = Some("Remote port is required for local/remote forwarding".to_string());
                    return;
                }
            }

            // Build the SSH port forwarding argument
            let bind = app.pf_bind_address.trim();
            let local_port = app.pf_local_port.trim();
            let remote_host = app.pf_remote_host.trim();
            let remote_port = app.pf_remote_port.trim();

            let pf_arg = match app.pf_forward_type {
                2 => {
                    // Dynamic: -D [bind_address:]local_port
                    if bind.is_empty() {
                        format!("-D {local_port}")
                    } else {
                        format!("-D {bind}:{local_port}")
                    }
                }
                _ => {
                    // Local (-L) or Remote (-R)
                    let flag = if app.pf_forward_type == 1 { "-R" } else { "-L" };
                    if bind.is_empty() {
                        format!("{flag} {local_port}:{remote_host}:{remote_port}")
                    } else {
                        format!("{flag} {bind}:{local_port}:{remote_host}:{remote_port}")
                    }
                }
            };

            // Record in history
            if let Some(ref host_name) = app.pf_target.clone() {
                if let Some(ref mut history) = app.history {
                    let _ = history.record_port_forwarding(
                        host_name,
                        forward_type,
                        local_port,
                        remote_host,
                        remote_port,
                        bind,
                    );
                }

                app.connect_host = Some(host_name.clone());
                app.port_forward_args = Some(pf_arg);
                app.should_quit = true;
            }
        }
        _ => {
            // Field-specific input handling
            match app.pf_focused {
                0 => {
                    // Forward type: Left/Right to cycle, or type l/r/d
                    match key.code {
                        KeyCode::Left => {
                            app.pf_forward_type = if app.pf_forward_type == 0 { 2 } else { app.pf_forward_type - 1 };
                        }
                        KeyCode::Right => {
                            app.pf_forward_type = (app.pf_forward_type + 1) % 3;
                        }
                        KeyCode::Char('l') => app.pf_forward_type = 0,
                        KeyCode::Char('r') => app.pf_forward_type = 1,
                        KeyCode::Char('d') => app.pf_forward_type = 2,
                        _ => {}
                    }
                }
                1 => {
                    // Local port
                    match key.code {
                        KeyCode::Backspace => { app.pf_local_port.pop(); }
                        KeyCode::Char(c) if c.is_ascii_digit() => { app.pf_local_port.push(c); }
                        _ => {}
                    }
                }
                2 => {
                    // Remote host
                    match key.code {
                        KeyCode::Backspace => { app.pf_remote_host.pop(); }
                        KeyCode::Char(c) => { app.pf_remote_host.push(c); }
                        _ => {}
                    }
                }
                3 => {
                    // Remote port
                    match key.code {
                        KeyCode::Backspace => { app.pf_remote_port.pop(); }
                        KeyCode::Char(c) if c.is_ascii_digit() => { app.pf_remote_port.push(c); }
                        _ => {}
                    }
                }
                4 => {
                    // Bind address
                    match key.code {
                        KeyCode::Backspace => { app.pf_bind_address.pop(); }
                        KeyCode::Char(c) => { app.pf_bind_address.push(c); }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
