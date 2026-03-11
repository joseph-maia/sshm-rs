# Changelog

All notable changes to this project will be documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-11

### Added

- **Themes** (`T` key in TUI)
  - 8 built-in presets with live picker
  - `sshm-rs theme list` and `sshm-rs theme set <name>` CLI commands
  - Custom themes via `~/.config/sshm-rs/theme.json`
  - Thread-safe dynamic switching at runtime

- **TUI redesign**
  - Modern header with version and connection stats
  - Fixed-width columns with overflow truncation
  - Hover highlight on mouse movement
  - Hash-based tag colors for visual grouping

- **sshm-term companion app**: Split-panel SSH terminal + SFTP file browser
  - SSH connection via `russh` with password, public key, and auto-detect authentication
  - Known hosts verification (TOFU) with MITM detection via `~/.ssh/known_hosts`
  - Live interactive terminal panel using `vt100` + `tui-term`
  - SFTP file browser with directory navigation, file metadata, and permissions display
  - Panel toggle (`Ctrl+B`) and focus switching (`Ctrl+S`)
  - Mouse support: click, double-click to enter directory, scroll wheel, right-click context menu
  - File operations: download (`d`), edit-in-place (`e`) via `$VISUAL`/`$EDITOR`/VS Code
  - Directory operations: archive (zip or tar.gz), download as archive
  - Delete files/directories with confirmation prompt
  - Editable path bar (click or `/` key) for direct path navigation
  - Follow terminal directory: SFTP auto-syncs with shell `cd` via OSC 7 detection
  - Remote command execution via SSH exec channel with proper exit code handling
  - Downloads open containing folder automatically
  - CLI interface: `sshm-term user@host [-p port] [-i key]`
  - Launch from sshm-rs TUI via `Enter` key

- **Keybinding refactor**
  - `Enter`: connect via sshm-term
  - `Shift+Enter`: connect via classic system SSH

- **Update notifications**
  - Background GitHub release check on startup
  - Non-blocking toast shown when a new version is available

- **Port forwarding TUI** (`F` key)
  - Overlay form for Local, Remote, and Dynamic SSH forwarding
  - Auto-prefill from last used configuration
  - Per-type field validation (required fields enforced)

- **Favorites / Pinned hosts** (`f` key)
  - Toggle favorite; pinned to top with gold star indicator
  - Persisted in `favorites.json` in sshm config directory

- **Edit Host form** (`e` key)
  - Pre-populated fields from existing host data
  - Password credential migration on host rename

- **Multi-select and batch operations** (`Space` / `Ctrl+A`)
  - Space to toggle selection; Ctrl+A to select all visible hosts
  - Batch delete with confirmation dialog
  - Command broadcast (`b` key): run a command on all selected hosts sequentially

- **Command snippets** (`S` key)
  - Save, manage, and execute frequently-used SSH commands on selected host
  - Persisted to `~/.config/sshm-rs/snippets.json`

- **SFTP/SCP file transfer** (`x` / `X` keys)
  - `x`: quick interactive SFTP session
  - `X`: SCP form with upload/download toggle and local/remote paths

- **Fuzzy search** via `nucleo-matcher`
  - Results ranked by match quality
  - Prefix filters: `tag:`, `user:`, `host:`

- **Tag filtering sidebar** (`t` key)
  - Toggle panel showing unique tags from all hosts
  - Click or Enter to filter; Left/Right arrows to switch focus

- **Live connectivity status**
  - Real-time TCP reachability: Online (green ●), Offline (red ●), Connecting (◌)
  - `r` key to refresh all statuses

- **Mouse support**
  - Click to select, double-click to connect, scroll wheel navigation
  - Visual scrollbar (▲/▼) when list overflows

- **Clipboard copy** (`y` key)
  - Copies `user@hostname` to system clipboard via `arboard`
  - Toast feedback on success or failure

- **Export/Import CLI commands**
  - `sshm-rs export [-o file.json]`: export all hosts to JSON
  - `sshm-rs import <file.json> [--skip-duplicates]`: import from JSON

- **Config validation** (`sshm-rs validate` + TUI warnings)
  - Detects duplicate names, empty hostnames, invalid ports, missing identity files
  - Warning count in status bar; per-host warnings in Info overlay

- **Shell completions**: `sshm-rs completions bash|zsh|fish|powershell`

- **SSH password auto-connect** via `ssh2`
  - Direct connection using saved credentials; graceful fallback to system `ssh`
  - Password management overlay (`p` key): set, update, or delete

- **Responsive title**
  - ASCII art hidden when terminal height < 20 lines; compact title shown instead

- **Toast/flash messages**
  - Auto-dismissing feedback after host add, delete, password save, and other operations

- **UI polish**
  - Rounded borders on all blocks and overlays
  - PageUp/PageDown/G navigation
  - `Esc` no longer quits (only `q` quits)

- **Landing page**

- **MIT License**

### Fixed

- Port-forward validation to prevent misconfigured forwarding rules
- Config and credential files created with `0o600` permissions
- Environment variable handling corrected for SSH subprocess invocation
- Clipboard paste fix in sshm-term terminal panel
- Keyring credentials not persisting (missing `windows-native` feature)
- Host names with spaces creating multiple SSH config aliases
- **sshm-term**: SSH exec channel exit code ignored (Eof before ExitStatus)
- **sshm-term**: Temp file path made unpredictable to prevent symlink attacks
- **sshm-term**: Unicode filename truncation panic on multi-byte characters
- **sshm-term**: SFTP navigation broken by Windows-style backslash paths
- **sshm-term**: Scroll offset not accounted for in click selection
- **sshm-term**: VS Code launch failure on Windows
- **sshm-term**: `navigate_to` leaving stale state on directory listing failure
- **sshm-term**: Context menu hit-test using incorrect frame size

## [0.1.0-beta.1] - 2026-03-04

First beta release.

### Added

- **TUI interactive**: Full terminal UI built with Ratatui + Crossterm
  - Scrollable host table: Status, Name, User, Hostname, Port, Tags
  - Dynamic column widths, multi-word search, sort by name or last login
  - Host info overlay (`i`), delete confirmation (`d`), help overlay (`?`)
  - Tokyo Night color theme

- **SSH config parser**: Full `~/.ssh/config` parser
  - `Include` directive with glob patterns and tilde expansion
  - Multi-alias host handling, wildcard host skipping
  - `# Tags:` comment parsing for host categorization
  - Source file and line number tracking for precise edits
  - Add, update, and delete host operations with automatic backup

- **CLI commands**
  - `sshm-rs`: launch TUI
  - `sshm-rs <host>`: direct SSH connection
  - `sshm-rs <host> <command...>`: remote command execution
  - `sshm-rs search <query>`: search with formatted output
  - `--tty/-t`, `--config/-c`, `--search/-s` flags

- **Add Host form** (`a` key): Name, Hostname, User, Port, Password, IdentityFile, Tags

- **Secure password storage** via OS credential manager
  - Windows Credential Manager, macOS Keychain, Linux Secret Service
  - Passwords never stored on disk; auto-retrieved via `SSH_ASKPASS`

- **Async connectivity check**: `PingManager` with per-host threads, 5s timeout, latency tracking

- **Connection history**: JSON persistence, last-used sort, time-ago display

- **Cross-platform support**: Windows, macOS, Linux with platform-aware config paths

### Fixed

- Double keypress on Windows (filter `KeyEventKind::Press` only)

[0.1.0]: https://github.com/bit5hift/sshm-rs/releases/tag/v0.1.0
[0.1.0-beta.1]: https://github.com/bit5hift/sshm-rs/releases/tag/v0.1.0-beta.1
