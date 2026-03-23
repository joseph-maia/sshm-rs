use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

use super::event::{Event, TransferState, TransferUpdate};

const CHUNK_SIZE: usize = 256 * 1024; // 256 KB
const PROGRESS_THROTTLE_MS: u128 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

pub struct TransferInfo {
    pub id: u64,
    pub filename: String,
    pub direction: TransferDirection,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub started_at: Instant,
    pub status: TransferStatus,
    pub cancel_token: CancellationToken,
}

pub struct TransferManager {
    transfers: HashMap<u64, TransferInfo>,
    next_id: u64,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            transfers: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn start_transfer(
        &mut self,
        filename: String,
        total_bytes: u64,
        direction: TransferDirection,
        cancel_token: CancellationToken,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.transfers.insert(
            id,
            TransferInfo {
                id,
                filename,
                direction,
                total_bytes,
                transferred_bytes: 0,
                started_at: Instant::now(),
                status: TransferStatus::InProgress,
                cancel_token,
            },
        );
        id
    }

    pub fn update_progress(&mut self, id: u64, bytes_transferred: u64) {
        if let Some(info) = self.transfers.get_mut(&id) {
            info.transferred_bytes = bytes_transferred;
        }
    }

    pub fn complete_transfer(&mut self, id: u64, total_bytes: u64) {
        if let Some(info) = self.transfers.get_mut(&id) {
            info.transferred_bytes = total_bytes;
            info.total_bytes = total_bytes;
            info.status = TransferStatus::Completed;
        }
    }

    pub fn fail_transfer(&mut self, id: u64) {
        if let Some(info) = self.transfers.get_mut(&id) {
            info.status = TransferStatus::Failed;
        }
    }

    pub fn cancel_transfer(&mut self, id: u64) {
        if let Some(info) = self.transfers.get(&id) {
            info.cancel_token.cancel();
        }
        if let Some(info) = self.transfers.get_mut(&id) {
            info.status = TransferStatus::Cancelled;
        }
    }

    #[allow(dead_code)]
    pub fn remove_transfer(&mut self, id: u64) {
        self.transfers.remove(&id);
    }

    pub fn active_transfers(&self) -> Vec<&TransferInfo> {
        let mut active: Vec<&TransferInfo> = self
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
            .collect();
        active.sort_by_key(|t| t.id);
        active
    }

    pub fn has_active(&self) -> bool {
        self.transfers
            .values()
            .any(|t| t.status == TransferStatus::InProgress)
    }

    pub fn active_count(&self) -> usize {
        self.transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
            .count()
    }

    /// Remove transfers that are no longer active (completed, failed, or cancelled).
    /// Called periodically from the event loop.
    pub fn prune_finished(&mut self) {
        self.transfers
            .retain(|_, t| t.status == TransferStatus::InProgress);
    }

    pub fn cancel_all_active(&mut self) {
        let ids: Vec<u64> = self
            .transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
            .map(|t| t.id)
            .collect();
        for id in ids {
            self.cancel_transfer(id);
        }
    }

    pub fn speed_bytes_per_sec(info: &TransferInfo) -> f64 {
        let elapsed = info.started_at.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0.0;
        }
        info.transferred_bytes as f64 / elapsed
    }

    pub fn eta_secs(info: &TransferInfo) -> Option<u64> {
        if info.total_bytes == 0 {
            return None;
        }
        let speed = Self::speed_bytes_per_sec(info);
        if speed < 1.0 {
            return None;
        }
        let remaining = info.total_bytes.saturating_sub(info.transferred_bytes);
        Some((remaining as f64 / speed) as u64)
    }

    pub fn format_speed(bytes_per_sec: f64) -> String {
        if bytes_per_sec < 1024.0 {
            format!("{:.0} B/s", bytes_per_sec)
        } else if bytes_per_sec < 1024.0 * 1024.0 {
            format!("{:.1} KB/s", bytes_per_sec / 1024.0)
        } else {
            format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
        }
    }

    pub fn get(&self, id: u64) -> Option<&TransferInfo> {
        self.transfers.get(&id)
    }
}

/// Spawn a background download task that deletes the remote file on completion.
/// Used for DownloadAsZip where a temporary archive is created on the server.
pub async fn spawn_download_and_cleanup(
    sftp: Arc<russh_sftp::client::SftpSession>,
    remote_path: String,
    local_path: PathBuf,
    id: u64,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) {
    let remote_clone = remote_path.clone();
    let sftp_clone = sftp.clone();
    let result = chunked_download(sftp, remote_path, local_path, id, tx, cancel).await;
    // Clean up remote archive regardless of download outcome
    let _ = sftp_clone.remove_file(remote_clone).await;
    let _ = result;
}

/// Spawn a background download task. The SFTP session is wrapped in Arc so it
/// can be shared without borrowing SftpBrowser.
pub fn spawn_download(
    sftp: Arc<russh_sftp::client::SftpSession>,
    remote_path: String,
    local_path: PathBuf,
    id: u64,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        let result =
            chunked_download(sftp, remote_path, local_path.clone(), id, tx.clone(), cancel).await;
        if let Err(e) = result {
            let _ = tx.send(Event::TransferProgress(TransferUpdate {
                id,
                state: TransferState::Failed {
                    error: e.to_string(),
                },
            }));
        }
    });
}

/// Spawn a background upload task.
pub fn spawn_upload(
    sftp: Arc<russh_sftp::client::SftpSession>,
    local_path: PathBuf,
    remote_path: String,
    id: u64,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        let result =
            chunked_upload(sftp, local_path, remote_path, id, tx.clone(), cancel).await;
        if let Err(e) = result {
            let _ = tx.send(Event::TransferProgress(TransferUpdate {
                id,
                state: TransferState::Failed {
                    error: e.to_string(),
                },
            }));
        }
    });
}

async fn chunked_download(
    sftp: Arc<russh_sftp::client::SftpSession>,
    remote: String,
    local: PathBuf,
    id: u64,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut remote_file = sftp.open(&remote).await?;
    let mut local_file = tokio::fs::File::create(&local).await?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut total: u64 = 0;
    let mut last_progress = Instant::now();

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                drop(local_file);
                let _ = tokio::fs::remove_file(&local).await;
                let _ = tx.send(Event::TransferProgress(TransferUpdate {
                    id,
                    state: TransferState::Failed { error: "Cancelled".into() },
                }));
                return Ok(());
            }
            result = remote_file.read(&mut buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(e) = local_file.write_all(&buf[..n]).await {
                            drop(local_file);
                            let _ = tokio::fs::remove_file(&local).await;
                            return Err(e.into());
                        }
                        total += n as u64;
                        if last_progress.elapsed().as_millis() >= PROGRESS_THROTTLE_MS {
                            last_progress = Instant::now();
                            let _ = tx.send(Event::TransferProgress(TransferUpdate {
                                id,
                                state: TransferState::Progress { bytes_transferred: total },
                            }));
                        }
                    }
                    Err(e) => {
                        drop(local_file);
                        let _ = tokio::fs::remove_file(&local).await;
                        return Err(e.into());
                    }
                }
            }
        }
    }

    local_file.flush().await?;
    let _ = tx.send(Event::TransferProgress(TransferUpdate {
        id,
        state: TransferState::Completed { total_bytes: total },
    }));
    Ok(())
}

async fn chunked_upload(
    sftp: Arc<russh_sftp::client::SftpSession>,
    local: PathBuf,
    remote: String,
    id: u64,
    tx: UnboundedSender<Event>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut local_file = tokio::fs::File::open(&local).await?;
    let mut remote_file = sftp.create(&remote).await?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut total: u64 = 0;
    let mut last_progress = Instant::now();

    loop {
        let n = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                drop(remote_file);
                let _ = sftp.remove_file(&remote).await;
                let _ = tx.send(Event::TransferProgress(TransferUpdate {
                    id,
                    state: TransferState::Failed { error: "Cancelled".into() },
                }));
                return Ok(());
            }
            result = local_file.read(&mut buf) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        drop(remote_file);
                        let _ = sftp.remove_file(&remote).await;
                        return Err(e.into());
                    }
                }
            }
        };

        remote_file.write_all(&buf[..n]).await?;
        total += n as u64;

        if last_progress.elapsed().as_millis() >= PROGRESS_THROTTLE_MS {
            last_progress = Instant::now();
            let _ = tx.send(Event::TransferProgress(TransferUpdate {
                id,
                state: TransferState::Progress { bytes_transferred: total },
            }));
        }
    }

    remote_file.flush().await?;
    let _ = tx.send(Event::TransferProgress(TransferUpdate {
        id,
        state: TransferState::Completed { total_bytes: total },
    }));
    Ok(())
}
