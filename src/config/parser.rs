// SSH config file parser
// Reference: Go implementation at sshm/internal/config/ssh.go
//
// Parses ~/.ssh/config (with Include support) into SshHost structs.
// Supports: add, update, delete, backup, multi-alias hosts, tags.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use super::paths::{default_ssh_config_path, sshm_backup_dir};

/// Represents a single SSH host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHost {
    pub name: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity: String,
    pub proxy_jump: String,
    pub proxy_command: String,
    pub options: String,
    pub remote_command: String,
    pub request_tty: String,
    pub tags: Vec<String>,
    pub source_file: PathBuf,
    pub line_number: usize,
}

impl SshHost {
    pub fn new(name: String, source_file: PathBuf, line_number: usize) -> Self {
        Self {
            name,
            hostname: String::new(),
            user: String::new(),
            port: "22".to_string(),
            identity: String::new(),
            proxy_jump: String::new(),
            proxy_command: String::new(),
            options: String::new(),
            remote_command: String::new(),
            request_tty: String::new(),
            tags: Vec::new(),
            source_file,
            line_number,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse the default SSH config file (~/.ssh/config) and return all hosts.
pub fn parse_ssh_config_default() -> Result<Vec<SshHost>> {
    let path = default_ssh_config_path()?;
    parse_ssh_config(&path)
}

/// Parse a specific SSH config file and return all hosts (entry point).
pub fn parse_ssh_config(path: &Path) -> Result<Vec<SshHost>> {
    let mut processed = HashSet::new();
    parse_ssh_config_inner(path, &mut processed)
}

/// Recursive parser that tracks already-processed files to avoid circular includes.
fn parse_ssh_config_inner(
    config_path: &Path,
    processed_files: &mut HashSet<PathBuf>,
) -> Result<Vec<SshHost>> {
    let abs_path = fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf());

    // Guard against circular includes
    if processed_files.contains(&abs_path) {
        return Ok(Vec::new());
    }
    processed_files.insert(abs_path.clone());

    // If file does not exist, create it only when it is the main config
    if !config_path.exists() {
        let main_path = default_ssh_config_path().ok();
        let is_main = main_path
            .as_ref()
            .map(|m| {
                fs::canonicalize(m).unwrap_or_else(|_| m.clone())
                    == fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf())
            })
            .unwrap_or(false);

        if is_main {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(config_path, "")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(config_path, fs::Permissions::from_mode(0o600))?;
            }
        }
        return Ok(Vec::new());
    }

    let file = fs::File::open(config_path)?;
    let reader = io::BufReader::new(file);

    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current_host: Option<SshHost> = None;
    let mut alias_names: Vec<String> = Vec::new();
    let mut pending_tags: Vec<String> = Vec::new();
    let mut line_number: usize = 0;

    for line_result in reader.lines() {
        let raw_line = line_result?;
        line_number += 1;
        let line = raw_line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        // Check for tags comment: "# Tags: a, b, c"
        if line.starts_with("# Tags:") {
            let tags_str = line.trim_start_matches("# Tags:").trim();
            if !tags_str.is_empty() {
                for tag in tags_str.split(',') {
                    let tag = tag.trim();
                    if !tag.is_empty() {
                        pending_tags.push(tag.to_string());
                    }
                }
            }
            continue;
        }

        // Skip other comments
        if line.starts_with('#') {
            continue;
        }

        // Split into key + value
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0].to_lowercase();
        let value = parts[1].trim().to_string();

        match key.as_str() {
            "include" => {
                if let Ok(included) =
                    process_include_directive(&value, config_path, processed_files)
                {
                    hosts.extend(included);
                }
            }
            "host" => {
                // Flush previous host
                flush_host(&mut hosts, &mut current_host, &mut alias_names);

                // Parse host names, skip wildcards
                let host_names: Vec<String> = value
                    .split_whitespace()
                    .map(|n| n.trim_matches('"').to_string())
                    .filter(|n| !n.contains('*') && !n.contains('?'))
                    .collect();

                if host_names.is_empty() {
                    current_host = None;
                    pending_tags.clear();
                    continue;
                }

                let mut host =
                    SshHost::new(host_names[0].clone(), abs_path.clone(), line_number);
                host.tags = std::mem::take(&mut pending_tags);

                if host_names.len() > 1 {
                    alias_names = host_names[1..].to_vec();
                }

                current_host = Some(host);
            }
            "hostname" => {
                if let Some(h) = current_host.as_mut() {
                    h.hostname = value;
                }
            }
            "user" => {
                if let Some(h) = current_host.as_mut() {
                    h.user = value;
                }
            }
            "port" => {
                if let Some(h) = current_host.as_mut() {
                    h.port = value;
                }
            }
            "identityfile" => {
                if let Some(h) = current_host.as_mut() {
                    h.identity = value;
                }
            }
            "proxyjump" => {
                if let Some(h) = current_host.as_mut() {
                    h.proxy_jump = value;
                }
            }
            "proxycommand" => {
                if let Some(h) = current_host.as_mut() {
                    // Handle ProxyCommand=value (no space after =)
                    h.proxy_command = value.trim_start_matches('=').to_string();
                }
            }
            "remotecommand" => {
                if let Some(h) = current_host.as_mut() {
                    h.remote_command = value;
                }
            }
            "requesttty" => {
                if let Some(h) = current_host.as_mut() {
                    h.request_tty = value;
                }
            }
            _ => {
                // Store unknown options
                if let Some(h) = current_host.as_mut() {
                    let opt = format!("{} {}", parts[0], value);
                    if h.options.is_empty() {
                        h.options = opt;
                    } else {
                        h.options.push('\n');
                        h.options.push_str(&opt);
                    }
                }
            }
        }
    }

    // Flush the last host
    flush_host(&mut hosts, &mut current_host, &mut alias_names);

    Ok(hosts)
}

/// Flush the current host (and any alias clones) into the hosts vec.
fn flush_host(
    hosts: &mut Vec<SshHost>,
    current_host: &mut Option<SshHost>,
    alias_names: &mut Vec<String>,
) {
    if let Some(host) = current_host.take() {
        // Clone for aliases first so they share the same config
        let aliases: Vec<SshHost> = alias_names
            .drain(..)
            .map(|alias| {
                let mut clone = host.clone();
                clone.name = alias;
                clone
            })
            .collect();

        hosts.push(host);
        hosts.extend(aliases);
    }
    alias_names.clear();
}

// ---------------------------------------------------------------------------
// Include directive
// ---------------------------------------------------------------------------

/// Expand and process an Include directive, returning hosts from matched files.
fn process_include_directive(
    pattern: &str,
    base_config_path: &Path,
    processed_files: &mut HashSet<PathBuf>,
) -> Result<Vec<SshHost>> {
    let expanded = expand_include_pattern(pattern, base_config_path)?;

    let matches = glob::glob(&expanded)
        .with_context(|| format!("Failed to glob pattern: {}", expanded))?;

    let mut all_hosts = Vec::new();
    for entry in matches.flatten() {
        if entry.is_dir() {
            continue;
        }
        if is_non_ssh_config_file(&entry) {
            continue;
        }
        if let Ok(included) = parse_ssh_config_inner(&entry, processed_files) {
            all_hosts.extend(included);
        }
    }
    Ok(all_hosts)
}

/// Expand tilde and make relative patterns absolute.
fn expand_include_pattern(pattern: &str, base_config_path: &Path) -> Result<String> {
    let mut p = pattern.to_string();

    // Expand ~
    if p.starts_with('~') {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home dir"))?;
        p = home.join(&p[1..]).to_string_lossy().to_string();
    }

    // Make relative patterns absolute relative to the base config dir
    let path = Path::new(&p);
    if !path.is_absolute() {
        if let Some(base_dir) = base_config_path.parent() {
            p = base_dir.join(&p).to_string_lossy().to_string();
        }
    }

    // On Windows, glob uses '/' as separator
    #[cfg(target_os = "windows")]
    {
        p = p.replace('\\', "/");
    }

    Ok(p)
}

/// Check whether a file should be excluded from SSH config parsing.
fn is_non_ssh_config_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Skip backup files
    if file_name.ends_with(".backup") {
        return true;
    }

    // Skip common non-config extensions
    let excluded_extensions = [
        ".txt", ".md", ".rst", ".doc", ".docx", ".pdf", ".log", ".tmp", ".bak", ".old", ".orig",
        ".json", ".xml", ".yaml", ".yml", ".toml", ".sh", ".bash", ".zsh", ".fish", ".ps1",
        ".bat", ".cmd", ".py", ".pl", ".rb", ".js", ".php", ".go", ".c", ".cpp", ".jpg", ".jpeg",
        ".png", ".gif", ".bmp", ".svg", ".zip", ".tar", ".gz", ".bz2", ".xz",
    ];
    for ext in &excluded_extensions {
        if file_name.ends_with(ext) {
            return true;
        }
    }

    // Skip hidden files
    if file_name.starts_with('.') {
        return true;
    }

    // Skip readme
    if file_name == "readme" {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// Backup
// ---------------------------------------------------------------------------

/// Create a backup of the config file in the sshm backup directory.
pub fn backup_config(config_path: &Path) -> Result<PathBuf> {
    let backup_dir = sshm_backup_dir()?;
    fs::create_dir_all(&backup_dir)?;

    let file_name = config_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let backup_path = backup_dir.join(format!("{}.backup", file_name));

    fs::copy(config_path, &backup_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&backup_path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(backup_path)
}

// ---------------------------------------------------------------------------
// Add host
// ---------------------------------------------------------------------------

/// Append a new host block to an SSH config file.
pub fn add_host(config_path: &Path, host: &SshHost) -> Result<()> {
    // Backup first if file exists
    if config_path.exists() {
        backup_config(config_path)?;
    }

    // Check for duplicates
    if host_exists_in_file(&host.name, config_path)? {
        bail!("host '{}' already exists", host.name);
    }

    let mut block = String::from("\n");

    // Tags
    if !host.tags.is_empty() {
        block.push_str(&format!("# Tags: {}\n", host.tags.join(", ")));
    }

    block.push_str(&format!("Host {}\n", host.name));
    block.push_str(&format!("    HostName {}\n", host.hostname));

    if !host.user.is_empty() {
        block.push_str(&format!("    User {}\n", host.user));
    }
    if !host.port.is_empty() && host.port != "22" {
        block.push_str(&format!("    Port {}\n", host.port));
    }
    if !host.identity.is_empty() {
        block.push_str(&format!(
            "    IdentityFile {}\n",
            format_config_value(&host.identity)
        ));
    }
    if !host.proxy_jump.is_empty() {
        block.push_str(&format!("    ProxyJump {}\n", host.proxy_jump));
    }
    if !host.proxy_command.is_empty() {
        block.push_str(&format!("    ProxyCommand={}\n", host.proxy_command));
    }
    if !host.remote_command.is_empty() {
        block.push_str(&format!("    RemoteCommand {}\n", host.remote_command));
    }
    if !host.request_tty.is_empty() {
        block.push_str(&format!("    RequestTTY {}\n", host.request_tty));
    }
    if !host.options.is_empty() {
        for opt in host.options.split('\n') {
            let opt = opt.trim();
            if !opt.is_empty() {
                block.push_str(&format!("    {}\n", opt));
            }
        }
    }

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(config_path)?;
    file.write_all(block.as_bytes())?;

    Ok(())
}

/// Quote a value if it contains spaces.
fn format_config_value(value: &str) -> String {
    if value.contains(' ') {
        format!("\"{}\"", value)
    } else {
        value.to_string()
    }
}

/// Check if a host name exists in a specific file (no include traversal).
fn host_exists_in_file(host_name: &str, config_path: &Path) -> Result<bool> {
    if !config_path.exists() {
        return Ok(false);
    }
    let file = fs::File::open(config_path)?;
    let reader = io::BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("host ") {
            let host_part = trimmed[5..].trim();
            for name in host_part.split_whitespace() {
                if name == host_name {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Update host
// ---------------------------------------------------------------------------

/// Update an existing host. Uses source_file and line_number from the host struct
/// to locate the exact block to replace.
pub fn update_host(host: &SshHost) -> Result<()> {
    let config_path = &host.source_file;
    if !config_path.exists() {
        bail!("Config file does not exist: {:?}", config_path);
    }

    backup_config(config_path)?;

    let is_multi = is_multi_host_declaration(&host.name, config_path)?;

    let content = fs::read_to_string(config_path)?;
    let lines: Vec<&str> = content.split('\n').collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut i = 0;
    let mut host_found = false;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        let current_line_number = i + 1; // 1-indexed

        // Check for Tags comment followed by Host line
        if trimmed.starts_with("# Tags:") && i + 1 < lines.len() {
            let next_trimmed = lines[i + 1].trim();
            if next_trimmed.to_lowercase().starts_with("host ") {
                let host_part = next_trimmed[5..].trim();
                let found_names: Vec<&str> = host_part.split_whitespace().collect();
                let target_idx = found_names.iter().position(|&n| n == host.name);
                let next_line_number = i + 2; // 1-indexed

                if target_idx.is_some()
                    && (host.line_number == 0 || next_line_number == host.line_number)
                {
                    host_found = true;

                    if is_multi {
                        handle_update_multi_host(
                            &lines,
                            &mut new_lines,
                            &mut i,
                            &found_names,
                            target_idx.unwrap(),
                            host,
                            true, // has_tags
                        );
                    } else {
                        // Skip tags + host line + body
                        i += 2;
                        skip_host_body(&lines, &mut i);
                        skip_empty_lines(&lines, &mut i);
                        write_host_block(&mut new_lines, host);
                    }
                    continue;
                }
            }
        }

        // Check for Host line without preceding tags
        if trimmed.to_lowercase().starts_with("host ")
            && !trimmed.starts_with('#')
        {
            let host_part = trimmed[5..].trim();
            let found_names: Vec<&str> = host_part.split_whitespace().collect();
            let target_idx = found_names.iter().position(|&n| n == host.name);

            if target_idx.is_some()
                && (host.line_number == 0 || current_line_number == host.line_number)
            {
                host_found = true;

                if is_multi {
                    handle_update_multi_host(
                        &lines,
                        &mut new_lines,
                        &mut i,
                        &found_names,
                        target_idx.unwrap(),
                        host,
                        false,
                    );
                } else {
                    i += 1;
                    skip_host_body(&lines, &mut i);
                    skip_empty_lines(&lines, &mut i);
                    write_host_block(&mut new_lines, host);
                }
                continue;
            }
        }

        new_lines.push(lines[i].to_string());
        i += 1;
    }

    if !host_found {
        bail!("host '{}' not found", host.name);
    }

    fs::write(config_path, new_lines.join("\n"))?;
    Ok(())
}

/// Handle update for a host that is part of a multi-host declaration:
/// remove the old alias from the Host line, keep the rest of the block,
/// then append the updated host as a new separate block.
fn handle_update_multi_host(
    lines: &[&str],
    new_lines: &mut Vec<String>,
    i: &mut usize,
    found_names: &[&str],
    target_idx: usize,
    host: &SshHost,
    has_tags: bool,
) {
    let remaining: Vec<&str> = found_names
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != target_idx)
        .map(|(_, &n)| n)
        .collect();

    if has_tags {
        // Keep the tags line
        new_lines.push(lines[*i].to_string());
    }

    if !remaining.is_empty() {
        new_lines.push(format!("Host {}", remaining.join(" ")));
        // Skip tags(if any) + Host line, copy body
        *i += if has_tags { 2 } else { 1 };
        while *i < lines.len() {
            let t = lines[*i].trim();
            if t.is_empty() || t.to_lowercase().starts_with("host ") {
                break;
            }
            new_lines.push(lines[*i].to_string());
            *i += 1;
        }
    } else {
        // No remaining hosts: skip entire block
        *i += if has_tags { 2 } else { 1 };
        skip_host_body(lines, i);
    }

    // Append the new host as a separate block
    new_lines.push(String::new());
    write_host_block(new_lines, host);
}

// ---------------------------------------------------------------------------
// Delete host
// ---------------------------------------------------------------------------

/// Delete a host from its config file. Uses source_file and line_number from the host struct.
pub fn delete_host(host: &SshHost) -> Result<()> {
    let config_path = &host.source_file;
    if !config_path.exists() {
        bail!("Config file does not exist: {:?}", config_path);
    }

    backup_config(config_path)?;

    let is_multi = is_multi_host_declaration(&host.name, config_path)?;

    let content = fs::read_to_string(config_path)?;
    let lines: Vec<&str> = content.split('\n').collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut i = 0;
    let mut host_found = false;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        let current_line_number = i + 1;

        // Check for Tags + Host
        if trimmed.starts_with("# Tags:") && i + 1 < lines.len() {
            let next_trimmed = lines[i + 1].trim();
            if next_trimmed.to_lowercase().starts_with("host ") {
                let host_part = next_trimmed[5..].trim();
                let found_names: Vec<&str> = host_part.split_whitespace().collect();
                let target_idx = found_names.iter().position(|&n| n == host.name);
                let next_line_number = i + 2;

                if target_idx.is_some()
                    && (host.line_number == 0 || next_line_number == host.line_number)
                {
                    host_found = true;

                    if is_multi {
                        handle_delete_multi_host(
                            &lines,
                            &mut new_lines,
                            &mut i,
                            &found_names,
                            target_idx.unwrap(),
                            true,
                        );
                    } else {
                        i += 2;
                        skip_host_body(&lines, &mut i);
                        skip_empty_lines(&lines, &mut i);
                    }

                    // Copy remaining and break (only delete first match)
                    copy_remaining(&lines, &mut new_lines, &mut i);
                    break;
                }
            }
        }

        // Host line without tags
        if trimmed.to_lowercase().starts_with("host ")
            && !trimmed.starts_with('#')
        {
            let host_part = trimmed[5..].trim();
            let found_names: Vec<&str> = host_part.split_whitespace().collect();
            let target_idx = found_names.iter().position(|&n| n == host.name);

            if target_idx.is_some()
                && (host.line_number == 0 || current_line_number == host.line_number)
            {
                host_found = true;

                if is_multi {
                    handle_delete_multi_host(
                        &lines,
                        &mut new_lines,
                        &mut i,
                        &found_names,
                        target_idx.unwrap(),
                        false,
                    );
                } else {
                    i += 1;
                    skip_host_body(&lines, &mut i);
                    skip_empty_lines(&lines, &mut i);
                }

                copy_remaining(&lines, &mut new_lines, &mut i);
                break;
            }
        }

        new_lines.push(lines[i].to_string());
        i += 1;
    }

    if !host_found {
        bail!("host '{}' not found", host.name);
    }

    fs::write(config_path, new_lines.join("\n"))?;
    Ok(())
}

/// Handle deletion of a host that is part of a multi-host declaration:
/// remove only the target alias from the Host line, keep the block.
fn handle_delete_multi_host(
    lines: &[&str],
    new_lines: &mut Vec<String>,
    i: &mut usize,
    found_names: &[&str],
    target_idx: usize,
    has_tags: bool,
) {
    let remaining: Vec<&str> = found_names
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != target_idx)
        .map(|(_, &n)| n)
        .collect();

    if has_tags {
        new_lines.push(lines[*i].to_string());
    }

    if !remaining.is_empty() {
        new_lines.push(format!("Host {}", remaining.join(" ")));
        *i += if has_tags { 2 } else { 1 };
        while *i < lines.len() {
            let t = lines[*i].trim();
            if t.is_empty() || t.to_lowercase().starts_with("host ") {
                break;
            }
            new_lines.push(lines[*i].to_string());
            *i += 1;
        }
    } else {
        *i += if has_tags { 2 } else { 1 };
        skip_host_body(lines, i);
    }

    skip_empty_lines(lines, i);
}

// ---------------------------------------------------------------------------
// Helpers shared by update / delete
// ---------------------------------------------------------------------------

/// Check if a host name is part of a multi-host declaration in the file.
fn is_multi_host_declaration(host_name: &str, config_path: &Path) -> Result<bool> {
    let content = fs::read_to_string(config_path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("host ") {
            let host_part = trimmed[5..].trim();
            let names: Vec<&str> = host_part.split_whitespace().collect();
            if names.iter().any(|&n| n == host_name) {
                return Ok(names.len() > 1);
            }
        }
    }
    Ok(false)
}

/// Skip lines that belong to a host body (indented config lines).
fn skip_host_body(lines: &[&str], i: &mut usize) {
    while *i < lines.len() {
        let t = lines[*i].trim();
        if t.is_empty() || t.to_lowercase().starts_with("host ") {
            break;
        }
        // Also break on a Tags comment that precedes a Host line
        if t.starts_with("# Tags:") {
            break;
        }
        *i += 1;
    }
}

/// Skip consecutive empty lines.
fn skip_empty_lines(lines: &[&str], i: &mut usize) {
    while *i < lines.len() && lines[*i].trim().is_empty() {
        *i += 1;
    }
}

/// Copy all remaining lines into new_lines.
fn copy_remaining(lines: &[&str], new_lines: &mut Vec<String>, i: &mut usize) {
    while *i < lines.len() {
        new_lines.push(lines[*i].to_string());
        *i += 1;
    }
}

/// Write a complete host block into the output lines.
fn write_host_block(new_lines: &mut Vec<String>, host: &SshHost) {
    // Ensure a blank line separator if previous content exists
    if let Some(last) = new_lines.last() {
        if !last.trim().is_empty() {
            new_lines.push(String::new());
        }
    }

    if !host.tags.is_empty() {
        new_lines.push(format!("# Tags: {}", host.tags.join(", ")));
    }
    new_lines.push(format!("Host {}", host.name));
    new_lines.push(format!("    HostName {}", host.hostname));
    if !host.user.is_empty() {
        new_lines.push(format!("    User {}", host.user));
    }
    if !host.port.is_empty() && host.port != "22" {
        new_lines.push(format!("    Port {}", host.port));
    }
    if !host.identity.is_empty() {
        new_lines.push(format!(
            "    IdentityFile {}",
            format_config_value(&host.identity)
        ));
    }
    if !host.proxy_jump.is_empty() {
        new_lines.push(format!("    ProxyJump {}", host.proxy_jump));
    }
    if !host.proxy_command.is_empty() {
        new_lines.push(format!("    ProxyCommand={}", host.proxy_command));
    }
    if !host.remote_command.is_empty() {
        new_lines.push(format!("    RemoteCommand {}", host.remote_command));
    }
    if !host.request_tty.is_empty() {
        new_lines.push(format!("    RequestTTY {}", host.request_tty));
    }
    if !host.options.is_empty() {
        for opt in host.options.split('\n') {
            let opt = opt.trim();
            if !opt.is_empty() {
                new_lines.push(format!("    {}", opt));
            }
        }
    }
    new_lines.push(String::new());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_config(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_parse_basic_host() {
        let f = temp_config(
            "\
Host myserver
    HostName 192.168.1.1
    User admin
    Port 2222
    IdentityFile ~/.ssh/id_rsa
",
        );
        let hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "myserver");
        assert_eq!(hosts[0].hostname, "192.168.1.1");
        assert_eq!(hosts[0].user, "admin");
        assert_eq!(hosts[0].port, "2222");
        assert_eq!(hosts[0].identity, "~/.ssh/id_rsa");
    }

    #[test]
    fn test_parse_wildcard_skipped() {
        let f = temp_config(
            "\
Host *
    ServerAliveInterval 60

Host real
    HostName example.com
",
        );
        let hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "real");
    }

    #[test]
    fn test_parse_multi_alias_hosts() {
        let f = temp_config(
            "\
Host alias1 alias2
    HostName 10.0.0.1
    User root
",
        );
        let hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "alias1");
        assert_eq!(hosts[1].name, "alias2");
        assert_eq!(hosts[0].hostname, "10.0.0.1");
        assert_eq!(hosts[1].hostname, "10.0.0.1");
    }

    #[test]
    fn test_parse_tags() {
        let f = temp_config(
            "\
# Tags: production, web
Host webserver
    HostName web.example.com
",
        );
        let hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].tags, vec!["production", "web"]);
    }

    #[test]
    fn test_add_host() {
        let f = temp_config("");
        let host = SshHost {
            name: "newhost".into(),
            hostname: "10.0.0.5".into(),
            user: "deploy".into(),
            port: "22".into(),
            identity: String::new(),
            proxy_jump: String::new(),
            proxy_command: String::new(),
            options: String::new(),
            remote_command: String::new(),
            request_tty: String::new(),
            tags: vec!["dev".into()],
            source_file: f.path().to_path_buf(),
            line_number: 0,
        };
        add_host(f.path(), &host).unwrap();
        let hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "newhost");
        assert_eq!(hosts[0].hostname, "10.0.0.5");
        assert_eq!(hosts[0].tags, vec!["dev"]);
    }

    #[test]
    fn test_add_duplicate_host_fails() {
        let f = temp_config(
            "\
Host existing
    HostName 1.2.3.4
",
        );
        let host = SshHost::new("existing".into(), f.path().to_path_buf(), 0);
        assert!(add_host(f.path(), &host).is_err());
    }

    #[test]
    fn test_delete_host() {
        let f = temp_config(
            "\
Host keep
    HostName 1.1.1.1

Host remove
    HostName 2.2.2.2

Host also_keep
    HostName 3.3.3.3
",
        );
        let hosts = parse_ssh_config(f.path()).unwrap();
        let to_delete = hosts.iter().find(|h| h.name == "remove").unwrap();
        delete_host(to_delete).unwrap();

        let remaining = parse_ssh_config(f.path()).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().all(|h| h.name != "remove"));
    }

    #[test]
    fn test_update_host() {
        let f = temp_config(
            "\
Host myhost
    HostName old.example.com
    User olduser
",
        );
        let mut hosts = parse_ssh_config(f.path()).unwrap();
        assert_eq!(hosts.len(), 1);

        hosts[0].hostname = "new.example.com".into();
        hosts[0].user = "newuser".into();
        update_host(&hosts[0]).unwrap();

        let updated = parse_ssh_config(f.path()).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].hostname, "new.example.com");
        assert_eq!(updated[0].user, "newuser");
    }

    #[test]
    fn test_backup_config() {
        let f = temp_config("Host test\n    HostName 1.2.3.4\n");
        let backup_path = backup_config(f.path()).unwrap();
        assert!(backup_path.exists());
        let original = fs::read_to_string(f.path()).unwrap();
        let backup = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(original, backup);
        // Clean up
        let _ = fs::remove_file(backup_path);
    }
}
