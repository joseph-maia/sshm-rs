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

- **Port forwarding TUI** (`F` key in TUI)
  - Interactive overlay form to configure SSH port forwarding (-L, -R, -D)
  - Support for Local, Remote, and Dynamic forwarding types
  - Fields: forward type selector, local port, remote host, remote port, bind address
  - Auto-prefill from last used configuration (persisted in history)
  - Validation: required fields enforced per forwarding type
  - Dynamic mode auto-disables remote host/port fields
  - Connects via system `ssh` with proper -L/-R/-D arguments

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

- **Clipboard copy** (`y` key in TUI)
  - Copies `user@hostname` (or just hostname if no user) to system clipboard
  - Cross-platform via `arboard` crate
  - Toast feedback on copy success/failure

- **Export/Import CLI commands**
  - `sshm-rs export [-o file.json]`: Export all hosts to JSON (stdout or file)
  - `sshm-rs import <file.json> [--skip-duplicates]`: Import hosts from JSON
  - Clean export format (name, hostname, user, port, identity, proxy_jump, tags)
  - Duplicate detection with optional skip on import

- **Tag filtering sidebar** (`t` key in TUI)
  - Toggle sidebar panel with unique tags extracted from all hosts
  - Click/Enter to filter host list by tag (toggle behavior)
  - "All Hosts" option to clear tag filter
  - Left/Right arrow keys to switch focus between sidebar and table
  - Visual indicators: active tag (bold blue), selected (highlight), tags (purple)
  - Sidebar state integrates with existing search/sort filters

- **Toast/flash messages**
  - Feedback after operations: "Host added", "Host deleted", "Password saved", etc.
  - Auto-dismiss after 3 seconds

- **Shell completions**
  - `sshm-rs completions bash/zsh/fish/powershell`

- **SSH password auto-connect** via ssh2 crate
  - Direct SSH connection using saved credentials (no SSH_ASKPASS)
  - Graceful fallback to system `ssh` when ssh2 negotiation fails
  - Password management overlay (`p` key): set, update, or delete passwords

- **Config validation** (`sshm-rs validate` CLI + TUI warnings)
  - Detects duplicate host names, empty hostnames, invalid ports, missing identity files
  - CLI command: `sshm-rs validate [-c config_file]`
  - TUI: warning count indicator in status bar, per-host warnings in Info overlay
  - 6 unit tests for validation rules

- **Configurable color themes** (`T` key in TUI)
  - 3 built-in presets: Tokyo Night (default), High Contrast, Light
  - Custom themes via `~/.config/sshm-rs/theme.json`
  - Live theme cycling with `T` key
  - OnceLock + RwLock pattern for thread-safe dynamic theme switching

- **Multi-select and batch operations** (`Space` / `Ctrl+a`)
  - Space to toggle individual host selection
  - Ctrl+a to select all visible hosts
  - Batch delete with confirmation dialog
  - Selected hosts highlighted with cyan checkmark
  - Esc to clear selection

- **Command broadcast** (`b` key with active selection)
  - Execute a command on all selected hosts sequentially
  - Output prefixed by hostname separator
  - Interactive overlay to type and confirm command

- **Command snippets** (`S` key in TUI)
  - Save and manage frequently-used SSH commands
  - Add/delete/execute snippets on selected host
  - Persisted to `~/.config/sshm-rs/snippets.json`
  - Snippet list with name, command preview, and description

- **SFTP/SCP file transfer** (`x` / `X` keys in TUI)
  - `x`: Quick SFTP interactive session to selected host
  - `X`: SCP form with upload/download toggle, local/remote paths
  - Uses system sftp/scp commands with SSH config support

- **sshm-term companion app**: Split-panel SSH terminal + SFTP file browser (MobaXterm-like)
  - SSH connection via `russh` with password, public key, and auto-detect authentication
  - Known hosts verification (TOFU) with MITM detection via `~/.ssh/known_hosts`
  - Live interactive terminal panel using `vt100` + `tui-term` widget
  - SFTP file browser panel with directory navigation, file metadata, permissions display
  - Async event loop with `tokio` for concurrent terminal I/O and SFTP operations
  - Panel toggle (`Ctrl+B`) and focus switching (`Ctrl+S`) between terminal and SFTP
  - Mouse support: click to select, double-click to enter directory, scroll wheel
  - Right-click context menu with contextual actions per entry type
  - File operations: download (`d`), edit-in-place (`e`) via `$VISUAL`/`$EDITOR`/VS Code
  - Directory operations: archive (zip or tar.gz with auto-detection), download as archive
  - Delete files/directories with confirmation prompt (red warning bar)
  - Editable path bar (click or `/` key) for direct path navigation
  - Follow terminal directory: SFTP auto-syncs with shell `cd` via OSC 7 detection
  - Smart editor detection: `$VISUAL` → `$EDITOR` → VS Code (`code --wait`) → fallback
  - Remote command execution via SSH exec channel with proper exit code handling
  - Downloads open containing folder automatically (explorer/open/xdg-open)
  - CLI interface: `sshm-term user@host [-p port] [-i key]`
  - Launch from sshm-rs TUI via `W` key

- **Research documents for sshm-term architecture**
  - PTY crates comparison and selection rationale
  - SFTP architecture and async patterns
  - Competitive analysis of SSH clients and file transfer tools

- **ADR-003: sshm-term companion app architecture decision**
  - Documents the design decisions, tradeoffs, and technology choices

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
- **sshm-term**: SSH exec channel exit code ignored (Eof received before ExitStatus)
- **sshm-term**: SFTP navigation with backslashes on Windows→Linux paths
- **sshm-term**: Scroll offset not accounted for in SFTP click selection
- **sshm-term**: VS Code editor launch failure on Windows (use `cmd /c code`)
- **sshm-term**: Unicode filename truncation panic on multi-byte characters
- **sshm-term**: `navigate_to` leaves stale state on directory listing failure
- **sshm-term**: Context menu hit-test using dummy area instead of real frame size
- **sshm-term**: Predictable temp file path in edit_file (symlink attack vector)

### Known Limitations (beta)

- `--password` CLI argument visible in process listings and shell history
- `Auth::Agent` tries local key files only, does not connect to ssh-agent/pageant
- Encrypted private keys (passphrase-protected) silently fail without prompting
- `exec_command` has no timeout — may hang on broken connections
- `Ctrl` key combinations beyond `a-z` not fully mapped

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
