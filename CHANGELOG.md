# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Responsive title**
  - ASCII art title hidden when terminal height < 20 lines
  - Compact one-line "sshm-rs" title shown instead
  - Table gains 4 extra visible rows in small terminals

- **Favorites / Pinned hosts** (`f` key in TUI)
  - Toggle favorite on any host with `f` key
  - Favorites pinned to top of list with gold star indicator
  - Persisted in `favorites.json` in sshm config directory
  - Works with all sort modes (favorites always on top)

- **Edit Host form** (`e` key in TUI)
  - Pre-populated fields from existing host data
  - Password credential migration on host rename
  - Shared form renderer with Add form

- **Live connectivity status**
  - PingManager wired into TUI (was built but unused)
  - Hosts show real-time TCP connectivity: Online (green ●), Offline (red ●), Connecting (◌)
  - `r` key to refresh all host statuses

- **Fuzzy search** (nucleo-matcher)
  - Replaces substring matching with fuzzy scoring
  - Results ranked by match quality
  - Prefix filters: `tag:`, `user:`, `host:`

- **Mouse support**
  - Click to select host
  - Scroll wheel navigation
  - Double-click to connect
  - Visual scrollbar (▲/▼) when list overflows

- **Toast/flash messages**
  - Feedback after operations: "Host added", "Host deleted", "Password saved", etc.
  - Auto-dismiss after 3 seconds

- **Shell completions**
  - `sshm-rs completions bash/zsh/fish/powershell`

- **SSH password auto-connect** via ssh2 crate
  - Direct SSH connection using saved credentials (no SSH_ASKPASS)
  - Graceful fallback to system `ssh` when ssh2 negotiation fails
  - Password management overlay (`p` key): set, update, or delete passwords

- **UI polish**
  - Rounded borders on all blocks and overlays
  - Unicode status indicators (● ○ ◌)
  - Extended Tokyo Night palette: cyan (hostnames), purple (tags), orange
  - PageUp/PageDown/G navigation
  - Improved MUTED color contrast (WCAG AA compliant)
  - Empty fields hidden in Info overlay
  - `Esc` no longer quits app (only `q` quits)

### Fixed

- Keyring credentials not persisting (missing `windows-native` feature)
- Name with spaces creating multiple SSH host aliases (validation added)

## [0.1.0-beta.1] - 2026-03-04

First beta release. Rust rewrite inspired by [sshm (Go)](https://github.com/Gu1llaum-3/sshm).

### Added

- **TUI interactive** : Full terminal UI built with Ratatui + Crossterm
  - Scrollable host table with columns: Status, Name, User, Hostname, Port, Tags
  - Dynamic column width based on terminal size
  - Multi-word search filtering (intersection) with `/` to focus
  - Sort toggle (`s`): by name (A-Z) or by last login
  - Host info overlay (`i`) with full host details
  - Delete confirmation dialog (`d`) with backup before deletion
  - Help overlay (`?`) showing all keybindings
  - Tokyo Night color theme

- **SSH config parser** : Full `~/.ssh/config` parser
  - `Include` directive support with glob patterns and tilde expansion
  - Multi-alias host handling (`Host alias1 alias2` creates separate entries)
  - Wildcard host skipping (`Host *`)
  - `# Tags:` comment parsing for host categorization
  - Source file and line number tracking for precise edits
  - Add / Update / Delete host operations
  - Automatic config backup before modifications
  - 9 unit tests covering parsing, CRUD, and backup

- **CLI commands**
  - `sshm-rs` : Launch interactive TUI
  - `sshm-rs <host>` : Direct SSH connection
  - `sshm-rs <host> <command...>` : Remote command execution
  - `sshm-rs search <query>` : Search hosts with formatted table output
  - `sshm-rs add` / `sshm-rs edit <host>` : Host management (stubs)
  - `--tty/-t` : Force pseudo-TTY allocation
  - `--config/-c <path>` : Custom SSH config file
  - `--search/-s` : Focus search at TUI startup

- **Add Host form** (`a` key in TUI)
  - Fields: Name, Hostname, User, Port, Password, IdentityFile, Tags
  - Input validation (name and hostname required)
  - Duplicate host detection
  - Tab/Arrow navigation between fields

- **Secure password storage** via OS credential manager
  - Windows: Windows Credential Manager (DPAPI encrypted)
  - macOS: Keychain (via `keyring` crate)
  - Linux: Secret Service / GNOME Keyring
  - Passwords never stored on disk
  - Auto-retrieval on connect via `SSH_ASKPASS` mechanism
  - Password field masked with `*****` in TUI
  - Password status shown in host info overlay

- **Async connectivity check**
  - `PingManager` with threaded concurrency (one thread per host)
  - TCP connect with 5s timeout + DNS resolution
  - `HostStatus` enum: Unknown, Connecting, Online (with latency), Offline (with error)
  - Thread-safe result storage via `Arc<RwLock<HashMap>>`

- **Connection history**
  - JSON persistence in OS config directory
  - Connection count and last connection timestamp
  - Port forwarding configuration history
  - Sort by last used in TUI
  - Time-ago display ("2 hours ago", "3 days ago")
  - Cleanup method for stale entries

- **Cross-platform support**
  - Windows, macOS, Linux
  - Platform-aware config paths (`%APPDATA%`, `XDG_CONFIG_HOME`, etc.)
  - Separate file permission handling per OS

### Fixed

- Double keypress on Windows (filter `KeyEventKind::Press` only)

### Technical

- Built with Rust 2021 edition
- Dependencies: ratatui, crossterm, clap, ssh2, keyring, serde, chrono, regex, glob, anyhow
- ~3000 lines of Rust code
- 9 unit tests

[0.1.0-beta.1]: https://github.com/joseph-maia/sshm-rs/releases/tag/v0.1.0-beta.1
