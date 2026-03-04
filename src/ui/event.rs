use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::ui::app::{App, ViewMode};

/// Poll for crossterm events, returning true if the terminal was resized.
pub fn poll_event(app: &mut App) -> anyhow::Result<bool> {
    if event::poll(Duration::from_millis(100))? {
        match event::read()? {
            Event::Key(key) => handle_key(app, key),
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
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
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
        KeyCode::End => {
            if !app.filtered_hosts.is_empty() {
                app.selected = app.filtered_hosts.len() - 1;
                let visible = app.visible_rows();
                if app.filtered_hosts.len() > visible {
                    app.table_offset = app.filtered_hosts.len() - visible;
                }
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
        // 'a' and 'e' are placeholders
        KeyCode::Char('a') | KeyCode::Char('e') => {
            // TODO: add/edit forms
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
            // TODO: actually delete from config when parser is ready
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
