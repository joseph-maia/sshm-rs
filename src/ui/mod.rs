mod app;
mod event;
pub mod styles;
mod views;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

use crate::config::{default_ssh_config_path, parse_ssh_config};
use crate::history::HistoryManager;
use crate::theme::Theme;

use app::{App, ViewMode};

pub fn run_tui() -> Result<()> {
    // Load hosts
    let config_path = default_ssh_config_path()?;
    let hosts = parse_ssh_config(&config_path)?;

    // Load history
    let history = HistoryManager::load().ok();

    // Load and initialise the active theme (reads theme.json if present)
    styles::init_theme(Theme::load());

    // Create app state once — persists across connection retry iterations
    let mut app = App::new(hosts, history, config_path);

    loop {
        // Reset quit flag so the TUI re-enters cleanly after a failed connection
        app.should_quit = false;

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main event loop
        let result = run_loop(&mut terminal, &mut app);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        terminal.show_cursor()?;

        result?;

        // If the user chose to connect, attempt the connection
        if let Some(action) = app.connect_host.take() {
            let pf_args = app.port_forward_args.take();

            let conn_result = if let Some(host_name) = action.strip_prefix("__sshm_term__:") {
                crate::connectivity::launch_sshm_term(host_name, None).map(|_| Some(host_name.to_string()))
            } else if let Some(ref pf_arg) = pf_args {
                if let Some(command) = pf_arg.strip_prefix("__snippet__:") {
                    let args: Vec<String> = command.split_whitespace().map(String::from).collect();
                    crate::connectivity::connect_ssh(&action, &args, None, true).map(|_| Some(action.clone()))
                } else {
                    crate::connectivity::connect_ssh_with_port_forward(&action, pf_arg, None).map(|_| None)
                }
            } else {
                crate::connectivity::connect_ssh(&action, &[], None, false).map(|_| Some(action.clone()))
            };

            match conn_result {
                Ok(Some(host_name)) => {
                    // Record history only on success
                    if let Some(ref mut history) = app.history {
                        let _ = history.record_connection(&host_name);
                    }
                    break;
                }
                Ok(None) => break,
                Err(e) => {
                    // Terminal is now in normal mode — safe to prompt for password
                    eprintln!("Connection failed: {e}");
                    if let Some(host_name) = action.strip_prefix("__sshm_term__:") {
                        if let Ok(new_password) = rpassword::prompt_password("Password: ") {
                            if !new_password.is_empty() {
                                let _ = crate::credentials::save_password(host_name, &new_password);
                            }
                        }
                    }
                    app.show_toast_error(&format!("Connection failed: {e}"));
                    continue;
                }
            }
        } else if let Some((hosts, command)) = app.pending_broadcast.take() {
            // User triggered a broadcast — run it now and exit
            let host_refs: Vec<&str> = hosts.iter().map(String::as_str).collect();
            crate::connectivity::broadcast_command(&host_refs, &command, None)?;
            break;
        } else {
            // User quit without connecting
            break;
        }
    }

    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    // Get initial terminal size
    let size = terminal.size()?;
    app.width = size.width;
    app.height = size.height;

    loop {
        // Clear expired toast messages
        app.check_toast();

        // Promote background update check result when ready
        app.poll_update_check();

        // Draw
        terminal.draw(|f| {
            match app.view_mode {
                ViewMode::Help => {
                    // Draw list as background, then help overlay
                    views::list::draw(f, app);
                    views::help::draw(f, f.area());
                }
                _ => {
                    views::list::draw(f, app);
                }
            }
        })?;

        // Handle events
        event::poll_event(app)?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
