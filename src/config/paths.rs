// Platform-aware path helpers for SSH and SSHM config directories
use anyhow::Result;
use std::path::PathBuf;

/// Returns ~/.ssh/config path
pub fn default_ssh_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    Ok(home.join(".ssh").join("config"))
}

/// Returns the sshm-rs config directory (platform-aware)
pub fn sshm_config_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata).join("sshm-rs"));
        }
    }

    let config = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Cannot find config dir"))?;
    Ok(config.join("sshm-rs"))
}

/// Returns the sshm-rs backup directory
pub fn sshm_backup_dir() -> Result<PathBuf> {
    Ok(sshm_config_dir()?.join("backups"))
}

/// Returns the ~/.ssh directory
pub fn ssh_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    Ok(home.join(".ssh"))
}
