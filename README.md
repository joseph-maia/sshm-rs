# sshm-rs

A beautiful and performant TUI SSH connection manager written in Rust, with integrated SSH terminal and SFTP browser. Inspired by [sshm (Go)](https://github.com/Gu1llaum-3/sshm) by Gu1llaum-3.

<!-- TODO: add screenshots -->

## Features

### sshm-rs (Main TUI)

- **SSH Config Management** — Parse and browse SSH hosts from `~/.ssh/config` with Include support
- **Interactive Host List** — Fuzzy search, sorting, and multi-select with async connectivity status
- **Host Operations** — Add, edit, delete, favorite, and organize hosts
- **Connection Tracking** — Automatic history recording for frequently used hosts
- **Port Forwarding** — Setup local, remote, and dynamic SSH port forwarding
- **Groups and Tags** — Organize hosts into groups or tag them for quick filtering
- **Command Snippets** — Save and execute pre-configured commands across hosts
- **Multi-Host Operations** — Broadcast commands or delete multiple hosts at once
- **Themes** — Three built-in themes (Tokyo Night, High Contrast, Light) with custom theme support
- **Password Storage** — Secure credential storage using OS keyring
- **Copy to Clipboard** — Quick copy host information for use elsewhere
- **Cross-platform** — Windows, Linux, macOS

### sshm-term (SSH Terminal + SFTP Browser)

- **SSH Terminal** — Full interactive terminal with pseudo-TTY support
- **SFTP Browser** — Integrated file browser for remote filesystem navigation
- **Bidirectional Transfers** — Download and upload files with progress tracking
- **Syntax Highlighting** — Terminal output with proper VT100 escape sequence support
- **Directory Following** — SFTP panel auto-follows terminal working directory (OSC 7)
- **File Editing** — Edit remote files with your local editor
- **Context Menu** — Right-click support for file operations (edit, download, upload, delete, zip)
- **Snippet Access** — Quick access to saved command snippets within the terminal
- **Batch Operations** — Compress and download multiple files as ZIP

## Installation

### Quick Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/bit5hift/sshm-rs/master/install.sh | bash
```

### Quick Install (Windows PowerShell)

```powershell
irm https://raw.githubusercontent.com/bit5hift/sshm-rs/master/install.ps1 | iex
```

### From Source

```bash
cargo install --git https://github.com/bit5hift/sshm-rs
```

### From GitHub Releases

Download the latest binary for your platform from the [Releases page](https://github.com/bit5hift/sshm-rs/releases/latest).

## Usage

### sshm-rs (Main Application)

```bash
# Interactive TUI
sshm-rs

# Direct connection to a host
sshm-rs myhost

# Execute command on remote host
sshm-rs myhost "ls -la /tmp"

# Execute with pseudo-TTY
sshm-rs myhost -t "sudo -s"

# Search mode (focus on search)
sshm-rs --search

# Use alternate SSH config
sshm-rs --config /path/to/config
```

### sshm-rs Subcommands

```bash
# Add a new host interactively
sshm-rs add

# Edit an existing host
sshm-rs edit myhost

# Search hosts and display results
sshm-rs search "database"

# Export hosts to JSON
sshm-rs export --output hosts.json

# Import hosts from JSON
sshm-rs import hosts.json [--skip-duplicates]

# Validate SSH config for warnings
sshm-rs validate

# Generate shell completions
sshm-rs completions bash > sshm-rs.bash
```

### sshm-term (SSH Terminal)

Launched from sshm-rs with `W` key or standalone:

```bash
# SSH terminal with SFTP browser
sshm-term user@host

# Connect to specific port
sshm-term user@host --port 2222

# Use specific key
sshm-term user@host --key ~/.ssh/id_rsa

# Prompt for password
sshm-term user@host --password
```

## Keybindings

### sshm-rs Keybindings

#### Navigation & Connection

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `PgUp` | Page up |
| `PgDn` | Page down |
| `Home` | Jump to first host |
| `End` | Jump to last host |
| `/` | Search hosts |
| `Tab` | Switch focus (search ↔ host list) |
| `Enter` | Connect to selected host |

#### Multi-Select

| Key | Action |
|-----|--------|
| `Space` | Toggle select host, move down |
| `Ctrl+a` | Select all visible hosts |
| `d` | Delete all selected hosts |
| `b` | Broadcast command to selected hosts |
| `Esc` | Clear selection |

#### Host Management

| Key | Action |
|-----|--------|
| `a` | Add new host |
| `e` | Edit selected host |
| `d` | Delete selected host |
| `p` | Set/remove password for host |
| `f` | Toggle favorite |
| `F` | Setup port forwarding |
| `i` | Show host info |
| `s` | Toggle sort mode |
| `y` | Copy host to clipboard |
| `t` | Toggle tag sidebar |
| `r` | Refresh connectivity status |
| `T` | Cycle color theme |
| `S` | Show command snippets |
| `G` | Create new group |
| `g` | Assign host to group |
| `Enter` | Collapse/expand group (on group header) |

#### File Transfer

| Key | Action |
|-----|--------|
| `x` | Quick SFTP session |
| `X` | SCP file transfer |
| `W` | Open with sshm-term (terminal + SFTP) |

#### System

| Key | Action |
|-----|--------|
| `?` / `h` | Show keybindings |
| `q` | Quit |

### sshm-term Keybindings

#### Global

| Key | Action |
|-----|--------|
| `Ctrl+q` | Quit |
| `Ctrl+s` | Switch panel (Terminal ↔ SFTP) |
| `Ctrl+b` | Toggle SFTP panel visibility |
| `Ctrl+f` | Toggle SFTP directory follow mode |
| `Ctrl+p` | Show snippets overlay |

#### Terminal Panel

| Key | Action |
|-----|--------|
| Any key | Send to terminal (when focused) |
| `Ctrl+s` | Switch to SFTP panel |

#### SFTP Panel

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Enter` | Open folder or file |
| `Backspace` | Go to parent directory |
| `/` | Edit current path |
| `e` | Edit file (launches local editor) |
| `d` | Download file |
| `u` | Upload file |
| `a` | Add snippet from file |
| `Right-click` | Context menu (edit, download, open, go up, refresh, zip, etc.) |

#### Context Menu

| Action | Description |
|--------|-------------|
| **Edit** | Edit file with local editor |
| **Download** | Download to local machine |
| **Upload** | Upload file to this location |
| **Open Folder** | Navigate into folder |
| **Go Up** | Navigate to parent directory |
| **Refresh** | Refresh directory listing |
| **Zip** | Compress file/directory |
| **Download as ZIP** | Compress and download |
| **Delete** | Delete remote file/directory |

#### Snippet Overlay

| Key | Action |
|-----|--------|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Tab` / `Right` | Next field in editor |
| `Shift+Tab` / `Left` | Previous field in editor |
| `Enter` | Save and execute snippet |
| `Esc` | Cancel |

## Configuration

### SSH Config Parsing

sshm-rs reads from `~/.ssh/config` (or `%USERPROFILE%\.ssh\config` on Windows) with full support for:

- Standard SSH config keywords (Host, Hostname, User, Port, IdentityFile, ProxyJump, etc.)
- `Include` directive with glob pattern expansion
- Tags via `# Tags: tag1, tag2` comments above Host entries
- Multi-alias hosts (multiple space-separated names per Host block)

Example SSH config:

```
# Tags: production, web
Host webserver web
    Hostname 192.168.1.100
    User ubuntu
    Port 22
    IdentityFile ~/.ssh/web.pem

# Tags: development
Host devenv
    Hostname dev.example.com
    User dev
    ProxyJump bastion

Host bastion
    Hostname jump.example.com
    User ops
```

### sshm Configuration

Configuration files are stored in:
- Linux/macOS: `~/.config/sshm-rs/`
- Windows: `%APPDATA%\sshm-rs\`

#### Snippets

Save command snippets in `snippets.json`:

```json
[
  {
    "name": "Check Disk Space",
    "command": "df -h",
    "description": "Show disk usage"
  },
  {
    "name": "List Processes",
    "command": "ps aux | grep",
    "description": "Find process by name"
  }
]
```

Accessed via `S` in sshm-rs or `Ctrl+P` in sshm-term.

#### Themes

Custom themes can be saved to `theme.json`:

```json
{
  "name": "Custom Dark",
  "bg": [20, 20, 30],
  "fg": [200, 200, 220],
  "primary": [100, 150, 255],
  "green": [100, 200, 100],
  "red": [255, 100, 100],
  "yellow": [255, 200, 50],
  "muted": [150, 150, 170],
  "cyan": [100, 200, 200],
  "purple": [200, 150, 255],
  "orange": [255, 160, 100],
  "selection_bg": [50, 80, 150]
}
```

Cycle through themes in sshm-rs with `T`.

#### History

Connection history is stored in `history.json` and automatically updated each time you connect to a host.

### Credential Storage

Passwords and credentials are stored securely using the OS keyring:
- **Linux** — Uses `secret-service` (GNOME Keyring, KDE Wallet, etc.)
- **macOS** — Uses Keychain
- **Windows** — Uses Credential Manager

Set a password for a host with `p` in the host list, then sshm-rs will use it automatically on future connections.

## Security

### Trust on First Use (TOFU)

sshm-term implements TOFU for SSH host key verification:
- First connection to a new host prompts with the server's key fingerprint
- On acceptance, the fingerprint is saved to `~/.ssh/known_hosts`
- Future connections verify against the saved fingerprint
- If a host key changes, sshm-term alerts you to potential MITM attacks

### Password Handling

- Passwords are stored in the OS keyring, never in plaintext
- Keyring access requires user authentication (password, biometric, etc.)
- Environment variable `SSHM_PASSWORD` can be set for automation (cleared after use)

### SSH Key Management

- Public key authentication is preferred over password authentication
- Supports OpenSSH and PuTTY-format private keys
- SSH agent integration (via `SSH_AUTH_SOCK`)
- Per-host identity file configuration

## CLI Flags

```
USAGE:
    sshm-rs [FLAGS] [OPTIONS] [HOST] [COMMAND]...

FLAGS:
    -h, --help                Print help
    -t, --tty                 Force pseudo-TTY allocation
    -s, --search              Focus on search at startup

OPTIONS:
    -c, --config <FILE>       SSH config file (default: ~/.ssh/config)

SUBCOMMANDS:
    add                       Add new SSH host
    edit <HOST>               Edit SSH host
    search <QUERY>            Search hosts
    export [--output FILE]    Export hosts to JSON
    import <FILE>             Import hosts from JSON
    validate                  Validate SSH config
    completions <SHELL>       Generate shell completions
```

## Development

### Build Requirements

- Rust 1.70+
- OpenSSL development headers (Linux)
- macOS: Xcode command-line tools

### Build

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Troubleshooting

### sshm.exe locked on Windows

If `sshm-rs.exe` is locked after execution:
- Press `Ctrl+C` to terminate the running process
- Close any open SSH connections
- Retry the command

### SFTP unavailable

If sshm-term shows "SFTP unavailable":
- Ensure the remote server has `/usr/lib/sftp-server` or equivalent
- Some restricted shells or environments may not support SFTP
- Check server logs for SFTP subsystem errors

### Host key changed warning

If sshm-term warns "host key changed":
- Verify the server hasn't been compromised
- If you know the host key changed (server maintenance), remove the entry from `~/.ssh/known_hosts`
- Reconnect to accept the new key

## License

MIT
