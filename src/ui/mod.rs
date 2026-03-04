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

use app::{App, ViewMode};

pub fn run_tui() -> Result<()> {
    // Load hosts
    let config_path = default_ssh_config_path()?;
    let hosts = parse_ssh_config(&config_path)?;

    // Load history
    let history = HistoryManager::load().ok();

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
    if let Some(host_name) = app.connect_host.take() {
        // Record history
        if let Some(ref mut history) = app.history {
            let _ = history.record_connection(&host_name);
        }
        crate::connectivity::connect_ssh(&host_name, &[], None, false)?;
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
