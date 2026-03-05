use anyhow::Result;
use crossterm::event::KeyEvent;
use russh::client::{self, Msg};
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key};
use russh::{Channel, ChannelReadHalf, ChannelWriteHalf};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Auth {
    Password(String),
    PublicKey(PathBuf),
    Agent,
}

struct SshHandler {
    host: String,
    port: u16,
}

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        use russh::keys::known_hosts::learn_known_hosts;
        use russh::keys::{Error as KeyError, check_known_hosts};

        match check_known_hosts(&self.host, self.port, server_public_key) {
            Ok(true) => Ok(true),
            Ok(false) => {
                learn_known_hosts(&self.host, self.port, server_public_key)
                    .map_err(|e| anyhow::anyhow!("Failed to write to known_hosts: {}", e))?;
                Ok(true)
            }
            Err(KeyError::KeyChanged { line }) => {
                anyhow::bail!(
                    "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED for {} (line {})!\n\
                     This could indicate a man-in-the-middle attack.\n\
                     If this is expected, remove the old entry from ~/.ssh/known_hosts.",
                    self.host,
                    line
                );
            }
            Err(e) => Err(anyhow::anyhow!("Failed to check known_hosts: {}", e)),
        }
    }
}

pub struct SshConnection {
    handle: client::Handle<SshHandler>,
    shell_writer: Option<ChannelWriteHalf<Msg>>,
}

impl SshConnection {
    pub async fn connect(host: String, port: u16, user: String, auth: Auth) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let handler = SshHandler {
            host: host.clone(),
            port,
        };

        let mut handle = client::connect(config, (host.as_str(), port), handler).await?;

        match auth {
            Auth::Password(pw) => {
                let auth_result = handle.authenticate_password(&user, pw).await?;
                if !auth_result.success() {
                    anyhow::bail!("Password authentication failed");
                }
            }
            Auth::PublicKey(key_path) => {
                let key_pair = load_secret_key(&key_path, None)
                    .or_else(|_| {
                        let prompt = format!("Passphrase for {}: ", key_path.display());
                        let passphrase = rpassword::prompt_password(prompt).ok();
                        load_secret_key(&key_path, passphrase.as_deref())
                    })?;
                let hash_alg = handle.best_supported_rsa_hash().await?.flatten();
                let auth_result = handle
                    .authenticate_publickey(
                        &user,
                        PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash_alg),
                    )
                    .await?;
                if !auth_result.success() {
                    anyhow::bail!("Public key authentication failed");
                }
            }
            Auth::Agent => {
                let home = dirs::home_dir().unwrap_or_default();
                let key_paths = vec![
                    home.join(".ssh/id_ed25519"),
                    home.join(".ssh/id_rsa"),
                ];
                let mut authenticated = false;
                for path in key_paths {
                    if path.exists() {
                        let key_result = load_secret_key(&path, None)
                            .or_else(|_| {
                                let prompt = format!("Passphrase for {}: ", path.display());
                                let passphrase = rpassword::prompt_password(prompt).ok();
                                load_secret_key(&path, passphrase.as_deref())
                            });
                        if let Ok(key) = key_result {
                            let hash_alg = handle.best_supported_rsa_hash().await?.flatten();
                            if let Ok(result) = handle
                                .authenticate_publickey(
                                    &user,
                                    PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
                                )
                                .await
                            {
                                if result.success() {
                                    authenticated = true;
                                    break;
                                }
                            }
                        }
                    }
                }
                if !authenticated {
                    anyhow::bail!("No suitable authentication method found");
                }
            }
        }

        Ok(Self {
            handle,
            shell_writer: None,
        })
    }

    /// Open a shell channel and return the read half for use in a reader task.
    /// The write half is stored internally and used by `send_input` and `resize_pty`.
    pub async fn open_shell_channel(
        &mut self,
        cols: u32,
        rows: u32,
    ) -> Result<ChannelReadHalf> {
        let channel: Channel<Msg> = self.handle.channel_open_session().await?;
        channel
            .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
            .await?;
        channel.request_shell(false).await?;
        let (reader, writer) = channel.split();
        self.shell_writer = Some(writer);
        Ok(reader)
    }

    pub async fn send_input(&mut self, key: KeyEvent) -> Result<()> {
        let bytes = key_to_bytes(&key);
        if !bytes.is_empty() {
            if let Some(writer) = &self.shell_writer {
                writer.data(&bytes[..]).await?;
            }
        }
        Ok(())
    }

    pub async fn send_raw_bytes(&self, bytes: &[u8]) -> Result<()> {
        if let Some(writer) = &self.shell_writer {
            writer.data(bytes).await?;
        }
        Ok(())
    }

    /// Inject shell configuration to emit OSC 7 (CWD reporting) on every prompt.
    /// Works for bash and zsh. The leading space prevents history recording when
    /// HISTCONTROL=ignorespace or ignoreboth is set.
    pub async fn inject_osc7_prompt(&self) -> Result<()> {
        let setup = concat!(
            " if [ -n \"$BASH_VERSION\" ]; then",
            " PROMPT_COMMAND=\"${PROMPT_COMMAND:+$PROMPT_COMMAND;}\"",
            "'printf \"\\033]7;file://%s%s\\033\\\\\" \"$(hostname)\" \"$(pwd)\"'",
            "; elif [ -n \"$ZSH_VERSION\" ]; then",
            " _sshm_precmd() { printf \"\\033]7;file://%s%s\\033\\\\\" \"$(hostname)\" \"$(pwd)\"; }",
            "; precmd_functions+=(_sshm_precmd); fi\r",
        );
        self.send_raw_bytes(setup.as_bytes()).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.send_raw_bytes(b" clear\r").await?;
        Ok(())
    }

    pub async fn resize_pty(&mut self, cols: u32, rows: u32) -> Result<()> {
        if let Some(writer) = &self.shell_writer {
            writer.window_change(cols, rows, 0, 0).await?;
        }
        Ok(())
    }

    pub async fn open_sftp_channel(&mut self) -> Result<russh_sftp::client::SftpSession> {
        let channel = self.handle.channel_open_session().await?;
        channel.request_subsystem(false, "sftp").await?;
        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
        Ok(sftp)
    }

    /// Execute a single command on the remote server and return its stdout output.
    pub async fn exec_command(&self, command: &str) -> Result<String> {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let mut output = Vec::new();
            let mut stderr_output = Vec::new();
            let mut exit_code: Option<u32> = None;

            loop {
                match channel.wait().await {
                    Some(russh::ChannelMsg::Data { data }) => {
                        output.extend_from_slice(&data);
                    }
                    Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                        stderr_output.extend_from_slice(&data);
                    }
                    Some(russh::ChannelMsg::ExitStatus { exit_status }) => {
                        exit_code = Some(exit_status);
                    }
                    Some(russh::ChannelMsg::Eof) => {
                        // Eof means no more data, but ExitStatus comes after Eof.
                        // Do NOT break here — wait for Close.
                    }
                    Some(russh::ChannelMsg::Close) | None => {
                        break;
                    }
                    _ => {}
                }
            }

            if let Some(code) = exit_code {
                if code != 0 {
                    let err = String::from_utf8_lossy(&stderr_output).to_string();
                    let msg = if err.trim().is_empty() {
                        format!("Command exited with status {}", code)
                    } else {
                        err.trim().to_string()
                    };
                    anyhow::bail!("{}", msg);
                }
            }

            Ok::<String, anyhow::Error>(String::from_utf8_lossy(&output).to_string())
        })
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after 30s"))??;

        Ok(result)
    }
}

fn key_to_bytes(key: &KeyEvent) -> Vec<u8> {
    use crossterm::event::{KeyCode, KeyModifiers};

    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let lower = c.to_ascii_lowercase();
                match lower {
                    'a'..='z' => vec![lower as u8 - b'a' + 1],
                    '[' => vec![0x1b],
                    '\\' => vec![0x1c],
                    ']' => vec![0x1d],
                    '^' => vec![0x1e],
                    '_' => vec![0x1f],
                    '@' => vec![0x00],
                    _ => vec![],
                }
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
        _ => vec![],
    }
}
