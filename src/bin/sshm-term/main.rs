mod app;
mod event;
mod sftp;
mod snippets;
mod ssh;
mod terminal;
mod transfer;
mod ui;

#[allow(unused_imports, dead_code)]
#[path = "../../config/mod.rs"]
mod config;
#[allow(dead_code)]
#[path = "../../theme.rs"]
mod theme;
#[path = "../../ui/styles.rs"]
pub mod term_styles;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use russh::ChannelMsg;
use ssh::Auth;
use std::{io, path::PathBuf};

#[derive(Parser, Debug)]
#[command(name = "sshm-term", about = "SSH terminal and SFTP browser")]
struct Args {
    /// Remote host (user@host or host)
    host: String,

    /// Remote port
    #[arg(short, long, default_value_t = 22)]
    port: u16,

    /// SSH user (overrides user@host syntax)
    #[arg(short, long)]
    user: Option<String>,

    /// Path to private key file
    #[arg(short = 'i', long)]
    key: Option<PathBuf>,

    /// Prompt for password authentication
    #[arg(long)]
    password: bool,
}

fn main() -> Result<()> {
    // SAFETY: Read and clear SSHM_PASSWORD before the tokio runtime starts any
    // worker threads. std::env::remove_var is unsound when called concurrently
    // with other env reads; doing it here, before tokio::Runtime::new(), ensures
    // no other threads exist yet.
    let env_password = std::env::var("SSHM_PASSWORD").ok();
    if env_password.is_some() {
        // SAFETY: No threads are running at this point.
        unsafe { std::env::remove_var("SSHM_PASSWORD") };
    }

    tokio::runtime::Runtime::new()?.block_on(async_main(env_password))
}

async fn async_main(env_password: Option<String>) -> Result<()> {
    let args = Args::parse();

    let (user, host) = parse_target(&args.host, &args.user);

    let auth = if let Some(key_path) = args.key {
        Auth::PublicKey(key_path)
    } else if args.password {
        // Use password from env (already read and cleared before runtime started),
        // or prompt interactively if it was not set.
        let pw = if let Some(pw) = env_password {
            pw
        } else {
            rpassword::prompt_password(format!("{}@{}'s password: ", user, host))
                .map_err(|e| anyhow::anyhow!("Failed to read password: {}", e))?
        };
        Auth::Password(pw)
    } else {
        Auth::AutoDetect
    };

    let theme = theme::Theme::load();
    term_styles::init_theme(theme);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let result = run(&mut term, host, args.port, user, auth).await;

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture, DisableBracketedPaste)?;
    term.show_cursor()?;

    result
}

async fn run(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    host: String,
    port: u16,
    user: String,
    auth: Auth,
) -> Result<()> {
    let (mut events, event_tx) = event::EventLoop::new();

    let mut app = App::new(host.clone(), port, user.clone(), auth, event_tx.clone());

    app.connect().await?;

    let size = term.size()?;

    let shell_reader = if let Some(ssh) = &mut app.ssh {
        let reader = ssh
            .open_shell_channel(size.width as u32, size.height as u32)
            .await?;
        Some(reader)
    } else {
        None
    };

    if let Some(ssh) = &mut app.ssh {
        match ssh.open_sftp_channel().await {
            Ok(sftp_session) => {
                app.sftp.set_session(sftp_session);
                app.sftp.list_directory().await.ok();
                app.show_sftp = true;
            }
            Err(e) => {
                app.status_message = format!("SFTP unavailable: {e}");
            }
        }
    }
    if let Some(ssh) = &app.ssh {
        app.sftp.load_name_cache(ssh).await;
    }

    if let Some(mut reader) = shell_reader {
        let ssh_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                match reader.wait().await {
                    Some(ChannelMsg::Data { data }) => {
                        if ssh_tx
                            .send(event::Event::SshOutput(data.to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        if ssh_tx
                            .send(event::Event::SshOutput(data.to_vec()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                        let _ = ssh_tx.send(event::Event::SshEof);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    // Inject OSC 7 prompt reporting so the SFTP follow-terminal feature works out of the box
    if let Some(ssh) = &app.ssh {
        let _ = ssh.inject_osc7_prompt().await;
    }

    app.terminal
        .resize(size.width.saturating_sub(2), size.height.saturating_sub(3));

    while !app.should_quit {
        term.draw(|f| ui::draw(f, &mut app))?;
        if let Some(ev) = events.next().await {
            app.handle_event(ev).await?;
        }

        if let Some(remote_path) = app.pending_edit.take() {
            disable_raw_mode()?;
            execute!(
                term.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                DisableBracketedPaste
            )?;

            let result = app.sftp.edit_file(&remote_path).await;

            enable_raw_mode()?;
            execute!(
                term.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;
            term.clear()?;

            match result {
                Ok(msg) => app.status_message = msg,
                Err(e) => app.status_message = format!("Edit error: {e}"),
            }

            let _ = app.sftp.list_directory().await;
        }

        if app.pending_upload {
            app.pending_upload = false;

            disable_raw_mode()?;
            execute!(
                term.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                DisableBracketedPaste
            )?;

            let selected_files = open_file_dialog_multi();

            enable_raw_mode()?;
            execute!(
                term.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;
            term.clear()?;

            if selected_files.is_empty() {
                // user cancelled — nothing to do
            } else if app.sftp.session_arc().is_none() {
                app.status_message = "No SFTP session".to_string();
            } else {
                let total_files = selected_files.len();
                let mut queued = 0usize;
                for local_path in selected_files {
                    if app.transfers.active_count() >= 3 {
                        app.status_message = format!(
                            "Queued {} of {} files — max 3 concurrent transfers",
                            queued, total_files
                        );
                        break;
                    }
                    let filename = local_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string();
                    let remote_path = sftp::posix_join(&app.sftp.current_path, &filename);
                    let total_bytes = std::fs::metadata(&local_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if let Some(sftp_arc) = app.sftp.session_arc() {
                        let cancel = tokio_util::sync::CancellationToken::new();
                        let id = app.transfers.start_transfer(
                            filename.clone(),
                            total_bytes,
                            transfer::TransferDirection::Upload,
                            cancel.clone(),
                        );
                        transfer::spawn_upload(
                            sftp_arc,
                            local_path,
                            remote_path,
                            id,
                            event_tx.clone(),
                            cancel,
                        );
                        queued += 1;
                    }
                }
                if app.transfers.active_count() < 3 {
                    app.status_message = if queued == 1 {
                        "Uploading 1 file...".to_string()
                    } else {
                        format!("Uploading {} files...", queued)
                    };
                }
            }
        }
    }

    Ok(())
}

fn open_file_dialog_multi() -> Vec<std::path::PathBuf> {
    if cfg!(windows) {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Title = 'Select files to upload'; $f.Multiselect = $true; if ($f.ShowDialog() -eq 'OK') { $f.FileNames -join "`n" }"#,
            ])
            .output()
            .ok();
        match output {
            Some(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .map(std::path::PathBuf::from)
                .collect(),
            None => vec![],
        }
    } else if cfg!(target_os = "macos") {
        let output = std::process::Command::new("osascript")
            .args(["-e", r#"set fs to (choose file with prompt "Select files to upload" with multiple selections allowed)
set out to ""
repeat with f in fs
  set out to out & POSIX path of f & linefeed
end repeat
out"#])
            .output()
            .ok();
        match output {
            Some(o) => String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .map(std::path::PathBuf::from)
                .collect(),
            None => vec![],
        }
    } else {
        // Try zenity with --multiple, then kdialog, then stdin fallback
        let zenity = std::process::Command::new("zenity")
            .args(["--file-selection", "--title=Select files to upload", "--multiple", "--separator=\n"])
            .output();
        let kdialog = || {
            std::process::Command::new("kdialog")
                .args(["--getopenfilenames", ".", "All Files (*)", "--separator", "\n"])
                .output()
        };

        let result = zenity
            .ok()
            .filter(|o| o.status.success())
            .or_else(|| kdialog().ok().filter(|o| o.status.success()));

        if let Some(output) = result {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .map(std::path::PathBuf::from)
                .collect()
        } else {
            println!("Enter local file paths to upload, one per line (empty line to finish):");
            let mut paths = vec![];
            loop {
                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_err() {
                    break;
                }
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    break;
                }
                let p = std::path::PathBuf::from(&trimmed);
                if p.exists() {
                    paths.push(p);
                } else {
                    eprintln!("File not found: {}", trimmed);
                }
            }
            paths
        }
    }
}

fn parse_target(host_arg: &str, user_override: &Option<String>) -> (String, String) {
    if let Some(at) = host_arg.find('@') {
        let user = host_arg[..at].to_string();
        let host = host_arg[at + 1..].to_string();
        (user_override.clone().unwrap_or(user), host)
    } else {
        let user = user_override
            .clone()
            .unwrap_or_else(whoami::username);
        (user, host_arg.to_string())
    }
}
