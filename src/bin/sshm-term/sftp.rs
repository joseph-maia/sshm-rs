use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub size: u64,
    pub permissions: u32,
    pub is_dir: bool,
    #[allow(dead_code)]
    pub modified: u64,
}

/// Join two POSIX path components (never uses backslash).
pub fn posix_join(base: &str, child: &str) -> String {
    if base == "/" {
        format!("/{child}")
    } else {
        format!("{base}/{child}")
    }
}

/// Get the parent of a POSIX path.
pub fn posix_parent(path: &str) -> Option<String> {
    if path == "/" {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => Some("/".to_string()),
        Some(pos) => Some(trimmed[..pos].to_string()),
        None => None,
    }
}

pub struct SftpBrowser {
    pub current_path: String,
    pub entries: Vec<SftpEntry>,
    pub selected_index: usize,
    pub show_hidden: bool,
    sftp: Option<russh_sftp::client::SftpSession>,
    pub loading: bool,
    pub error: Option<String>,
}

impl SftpBrowser {
    pub fn new() -> Self {
        Self {
            current_path: "/".to_string(),
            entries: Vec::new(),
            selected_index: 0,
            show_hidden: false,
            sftp: None,
            loading: false,
            error: None,
        }
    }

    pub fn set_session(&mut self, sftp: russh_sftp::client::SftpSession) {
        self.sftp = Some(sftp);
    }

    pub async fn list_directory(&mut self) -> Result<()> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SFTP session"))?;
        self.loading = true;
        self.error = None;

        match sftp.read_dir(&self.current_path).await {
            Ok(dir_entries) => {
                let mut entries: Vec<SftpEntry> = Vec::new();
                for entry in dir_entries {
                    let name = entry.file_name();
                    if !self.show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let metadata = entry.metadata();
                    entries.push(SftpEntry {
                        name,
                        size: metadata.size.unwrap_or(0),
                        permissions: metadata.permissions.unwrap_or(0),
                        is_dir: metadata.is_dir(),
                        modified: metadata.mtime.unwrap_or(0) as u64,
                    });
                }

                entries.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });

                if self.current_path != "/" {
                    entries.insert(
                        0,
                        SftpEntry {
                            name: "..".to_string(),
                            size: 0,
                            permissions: 0o755,
                            is_dir: true,
                            modified: 0,
                        },
                    );
                }

                self.entries = entries;
                self.selected_index = 0;
                self.loading = false;
            }
            Err(e) => {
                self.error = Some(e.to_string());
                self.loading = false;
            }
        }

        Ok(())
    }

    pub async fn navigate_to(&mut self, path: String) -> Result<()> {
        let old_path = std::mem::replace(&mut self.current_path, path);
        if let Err(e) = self.list_directory().await {
            self.current_path = old_path;
            return Err(e);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn download_file(&self, remote: &str, local: &std::path::Path) -> Result<u64> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SFTP session"))?;

        let mut remote_file = sftp.open(remote).await?;
        let mut data = Vec::new();
        remote_file.read_to_end(&mut data).await?;

        let bytes_written = data.len() as u64;
        tokio::fs::write(local, &data).await?;

        Ok(bytes_written)
    }

    pub async fn upload_file(&self, local: &std::path::Path, remote: &str) -> Result<u64> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SFTP session"))?;

        let data = tokio::fs::read(local).await?;
        let bytes = data.len() as u64;

        let mut remote_file = sftp.create(remote).await?;
        tokio::io::AsyncWriteExt::write_all(&mut remote_file, &data).await?;

        Ok(bytes)
    }

    pub async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.entries.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1).min(self.entries.len().saturating_sub(1));
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                if let Some(entry) = self.entries.get(self.selected_index) {
                    if entry.name == ".." {
                        if let Some(parent) = posix_parent(&self.current_path) {
                            self.navigate_to(parent).await?;
                        }
                    } else if entry.is_dir {
                        let new_path = posix_join(&self.current_path, &entry.name);
                        self.navigate_to(new_path).await?;
                    }
                }
            }
            KeyCode::Backspace | KeyCode::Char('h') => {
                if let Some(parent) = posix_parent(&self.current_path) {
                    self.navigate_to(parent).await?;
                }
            }
            KeyCode::Char('.') => {
                self.show_hidden = !self.show_hidden;
                self.list_directory().await?;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.list_directory().await?;
            }
            KeyCode::Char('G') => {
                if !self.entries.is_empty() {
                    self.selected_index = self.entries.len() - 1;
                }
            }
            KeyCode::Char('g') => {
                self.selected_index = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Download a remote file to a temp path, open it in $VISUAL/$EDITOR, and
    /// re-upload it if the file was modified.  Terminal save/restore is handled
    /// by the caller (main.rs) before this method is invoked.
    pub async fn edit_file(&self, remote_path: &str) -> Result<String> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SFTP session"))?;

        let filename = remote_path.rsplit('/').next().unwrap_or("file");

        let temp_dir = std::env::temp_dir().join("sshm-term");
        tokio::fs::create_dir_all(&temp_dir).await?;
        let random: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let safe_name = format!("{}.{:x}", filename, random);
        let local_path = temp_dir.join(safe_name);

        // Download
        let mut remote_file = sftp.open(remote_path).await?;
        let mut data = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut remote_file, &mut data).await?;
        tokio::fs::write(&local_path, &data).await?;

        let mtime_before = tokio::fs::metadata(&local_path).await?.modified()?;

        let (program, extra_args) = detect_editor();

        let status = tokio::process::Command::new(&program)
            .args(&extra_args)
            .arg(&local_path)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await;

        match status {
            Ok(s) if s.success() => {
                let mtime_after = tokio::fs::metadata(&local_path).await?.modified()?;

                if mtime_after != mtime_before {
                    let new_data = tokio::fs::read(&local_path).await?;
                    let len = new_data.len();
                    let mut remote_out = sftp.create(remote_path).await?;
                    tokio::io::AsyncWriteExt::write_all(&mut remote_out, &new_data).await?;
                    let _ = tokio::fs::remove_file(&local_path).await;
                    Ok(format!("Uploaded {} ({} bytes)", filename, len))
                } else {
                    let _ = tokio::fs::remove_file(&local_path).await;
                    Ok(format!("{} not modified", filename))
                }
            }
            Ok(s) => {
                let _ = tokio::fs::remove_file(&local_path).await;
                Ok(format!("Editor exited with code {}", s))
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&local_path).await;
                Err(anyhow::anyhow!("Failed to launch editor '{}': {}", program, e))
            }
        }
    }

    pub fn format_size(bytes: u64) -> String {
        if bytes < 1024 {
            return format!("{bytes} B");
        }
        if bytes < 1024 * 1024 {
            return format!("{:.1} KB", bytes as f64 / 1024.0);
        }
        if bytes < 1024 * 1024 * 1024 {
            return format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0));
        }
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }

    pub fn format_permissions(mode: u32) -> String {
        let mut s = String::with_capacity(9);
        let flags = [
            (0o400, 'r'),
            (0o200, 'w'),
            (0o100, 'x'),
            (0o040, 'r'),
            (0o020, 'w'),
            (0o010, 'x'),
            (0o004, 'r'),
            (0o002, 'w'),
            (0o001, 'x'),
        ];
        for (flag, ch) in flags {
            s.push(if mode & flag != 0 { ch } else { '-' });
        }
        s
    }

    /// Download a remote file/directory to a local destination.
    /// Returns the number of bytes downloaded and the local path used.
    pub async fn download_to_local(
        &self,
        remote_path: &str,
        local_dir: &std::path::Path,
    ) -> Result<(u64, std::path::PathBuf)> {
        let sftp = self
            .sftp
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No SFTP session"))?;

        let filename = remote_path.rsplit('/').next().unwrap_or("file");
        let local_path = local_dir.join(filename);

        let mut remote_file = sftp.open(remote_path).await?;
        let mut data = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut remote_file, &mut data).await?;

        let bytes = data.len() as u64;
        tokio::fs::write(&local_path, &data).await?;

        Ok((bytes, local_path))
    }
}

/// Detect the best available editor. Priority:
/// 1. $VISUAL  2. $EDITOR  3. `code --wait` if VS Code is on PATH  4. notepad/vi
fn detect_editor() -> (String, Vec<String>) {
    // Check env vars first
    if let Ok(editor) = std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")) {
        let parts: Vec<&str> = editor.split_whitespace().collect();
        if let Some((cmd, args)) = parts.split_first() {
            return (
                cmd.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
            );
        }
    }

    // Try VS Code
    if cfg!(windows) {
        if which_exists("code.cmd") {
            return ("cmd".to_string(), vec!["/c".to_string(), "code".to_string(), "--wait".to_string()]);
        }
    } else if which_exists("code") {
        return ("code".to_string(), vec!["--wait".to_string()]);
    }

    // Fallback
    if cfg!(windows) {
        ("notepad".to_string(), vec![])
    } else {
        ("vi".to_string(), vec![])
    }
}

fn which_exists(cmd: &str) -> bool {
    std::process::Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
