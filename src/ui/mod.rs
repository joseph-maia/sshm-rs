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

    // Create app state
    let mut app = App::new(hosts, history, config_path);

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

    // If the user chose to connect, exec ssh
    if let Some(action) = app.connect_host.take() {
        let pf_args = app.port_forward_args.take();

        if let Some(host_name) = action.strip_prefix("__sftp__:") {
            crate::connectivity::launch_sftp(host_name, None)?;
        } else if let Some(rest) = action.strip_prefix("__scp__:") {
            let parts: Vec<&str> = rest.splitn(4, ':').collect();
            if parts.len() == 4 {
                let host = parts[0];
                let upload = parts[1] == "upload";
                let local = parts[2];
                let remote = parts[3];
                crate::connectivity::launch_scp(host, local, remote, upload, None)?;
            }
        } else if let Some(host_name) = action.strip_prefix("__sshm_term__:") {
            if let Some(ref mut history) = app.history {
                let _ = history.record_connection(host_name);
            }
            crate::connectivity::launch_sshm_term(host_name, None)?;
        } else if let Some(ref pf_arg) = pf_args {
            if let Some(command) = pf_arg.strip_prefix("__snippet__:") {
                if let Some(ref mut history) = app.history {
                    let _ = history.record_connection(&action);
                }
                let args: Vec<String> = command.split_whitespace().map(String::from).collect();
                crate::connectivity::connect_ssh(&action, &args, None, true)?;
            } else {
                crate::connectivity::connect_ssh_with_port_forward(&action, pf_arg, None)?;
            }
        } else {
            if let Some(ref mut history) = app.history {
                let _ = history.record_connection(&action);
            }
            crate::connectivity::connect_ssh(&action, &[], None, false)?;
        }
    }

    // If the user triggered a broadcast, run it now
    if let Some((hosts, command)) = app.pending_broadcast.take() {
        let host_refs: Vec<&str> = hosts.iter().map(String::as_str).collect();
        crate::connectivity::broadcast_command(&host_refs, &command, None)?;
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
