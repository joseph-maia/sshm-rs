use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::Duration;

use crate::ui::app::{AddField, App, ViewMode};

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
            // Layout: title (5 lines) + search bar (3 lines) + table border (1 line) + header (1 line) = 10 lines offset
            let table_top_offset: u16 = 10;
            if mouse.row >= table_top_offset {
                let clicked_index = app.table_offset + (mouse.row - table_top_offset) as usize;
                if clicked_index < app.filtered_hosts.len() {
                    app.selected = clicked_index;
                    app.clamp_offset();
                }
            }
        }
        _ => {}
    }
}

fn handle_list_key(app: &mut App, key: KeyEvent) {
    if app.search_mode {
        handle_search_key(app, key);
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
            // Clear search query if one is active, otherwise do nothing
            if !app.search_query.is_empty() {
                app.search_query.clear();
                app.apply_filter();
            }
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
            if let Some(host) = app.selected_host() {
                app.connect_host = Some(host.name.clone());
                app.should_quit = true;
            }
        }
        KeyCode::Char('s') => {
            app.sort_mode = app.sort_mode.toggle();
            app.apply_filter();
        }
        KeyCode::Char('d') => {
            if let Some(host) = app.selected_host() {
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
        KeyCode::Char('r') => {
            // Refresh connectivity status for all hosts
            app.start_ping();
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
            if let Some(ref target) = app.delete_target {
                if let Some(host) = app.hosts.iter().find(|h| h.name == *target).cloned() {
                    let _ = crate::config::delete_host(&host);
                    app.reload_hosts();
                    app.show_toast("Host deleted");
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
