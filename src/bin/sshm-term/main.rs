mod app;
mod event;
mod sftp;
mod snippets;
mod ssh;
mod terminal;
mod ui;

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let (user, host) = parse_target(&args.host, &args.user);

    let auth = if let Some(key_path) = args.key {
        Auth::PublicKey(key_path)
    } else if args.password {
        // Check env var first (set by sshm-rs launcher), then prompt interactively
        let pw = if let Ok(env_pw) = std::env::var("SSHM_PASSWORD") {
            std::env::remove_var("SSHM_PASSWORD");
            env_pw
        } else {
            rpassword::prompt_password(format!("{}@{}'s password: ", user, host))
                .map_err(|e| anyhow::anyhow!("Failed to read password: {}", e))?
        };
        Auth::Password(pw)
    } else {
        Auth::Agent
    };

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
    let mut app = App::new(host.clone(), port, user.clone(), auth);

    let (mut events, event_tx) = event::EventLoop::new();

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

            let selected_file = open_file_dialog();

            enable_raw_mode()?;
            execute!(
                term.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;
            term.clear()?;

            if let Some(local_path) = selected_file {
                let filename = local_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");
                let remote_path = sftp::posix_join(&app.sftp.current_path, filename);

                app.status_message = format!("Uploading {}...", filename);

                match app.sftp.upload_file(&local_path, &remote_path).await {
                    Ok(bytes) => {
                        app.status_message = format!("Uploaded {} ({} bytes)", filename, bytes);
                        let _ = app.sftp.list_directory().await;
                    }
                    Err(e) => {
                        app.status_message = format!("Upload failed: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}

fn open_file_dialog() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Title = 'Select file to upload'; if ($f.ShowDialog() -eq 'OK') { $f.FileName }"#,
            ])
            .output()
            .ok()?;
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(std::path::PathBuf::from(path)) }
    } else if cfg!(target_os = "macos") {
        let output = std::process::Command::new("osascript")
            .args(["-e", r#"POSIX path of (choose file with prompt "Select file to upload")"#])
            .output()
            .ok()?;
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() { None } else { Some(std::path::PathBuf::from(path)) }
    } else {
        let output = std::process::Command::new("zenity")
            .args(["--file-selection", "--title=Select file to upload"])
            .output()
            .or_else(|_| {
                std::process::Command::new("kdialog")
                    .args(["--getopenfilename", ".", "All Files (*)"])
                    .output()
            })
            .ok()?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() { None } else { Some(std::path::PathBuf::from(path)) }
        } else {
            println!("Enter local file path to upload (or press Enter to cancel):");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok()?;
            let path = input.trim().to_string();
            if path.is_empty() {
                None
            } else {
                let p = std::path::PathBuf::from(&path);
                if p.exists() {
                    Some(p)
                } else {
                    eprintln!("File not found: {}", path);
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    None
                }
            }
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
            .unwrap_or_else(|| whoami::username());
        (user, host_arg.to_string())
    }
}
