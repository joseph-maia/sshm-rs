#![allow(dead_code)]
// Credentials module - secure password storage via OS keychain
// On Windows: Windows Credential Manager (DPAPI encrypted)
// On macOS: Keychain
// On Linux: Secret Service (GNOME Keyring / KWallet)

use anyhow::{Context, Result};

const SERVICE_NAME: &str = "sshm-rs";

/// Save a password for an SSH host in the OS credential store.
pub fn save_password(host_name: &str, password: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_name)
        .context("Failed to create keyring entry")?;
    entry
        .set_password(password)
        .context("Failed to save password to credential store")?;
    Ok(())
}

/// Retrieve a saved password for an SSH host.
/// Returns None if no password is stored.
pub fn get_password(host_name: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_name).ok()?;
    entry.get_password().ok()
}

/// Delete a saved password for an SSH host.
pub fn delete_password(host_name: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_name)
        .context("Failed to create keyring entry")?;
    // Ignore "not found" errors
    let _ = entry.delete_credential();
    Ok(())
}

/// Check if a password exists for an SSH host.
pub fn has_password(host_name: &str) -> bool {
    get_password(host_name).is_some()
}
