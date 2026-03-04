use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::ui::app::{App, ViewMode};

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
        ViewMode::Password => handle_password_key(app, key),
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
        KeyCode::Char('e') => {
            // TODO: edit form
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
