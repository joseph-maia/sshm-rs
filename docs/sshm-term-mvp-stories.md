# sshm-term MVP User Stories

## Overview
sshm-term is a TUI companion app for sshm-rs that provides an integrated SSH terminal + SFTP file browser in a split-panel view. This document contains all MVP user stories grouped by epic.

**MVP Phase 1 Scope**: Connection, Terminal, SFTP browser, file operations, core UX.
**Target**: Single window with 2 panels (terminal left, SFTP browser right), keyboard-driven navigation.

---

## Epic 1: SSH Connection

### US-1: Connect to SSH host with password authentication
**As a** user, **I want** to initiate an SSH connection to a host using password authentication, **so that** I can open an interactive terminal session.

**Acceptance Criteria:**
- [ ] User is prompted to enter a hostname/IP address
- [ ] User is prompted to enter a username
- [ ] User is prompted to enter a password
- [ ] Connection is established using russh with the provided credentials
- [ ] On successful connection, SSH terminal panel is displayed
- [ ] On connection failure, a clear error message is shown (timeout, auth failure, host unreachable)
- [ ] Connection state is tracked (connected / disconnected)

### US-2: Connect to SSH host with public key authentication
**As a** user, **I want** to establish an SSH connection using an RSA/ED25519 public key, **so that** I can use passwordless authentication.

**Acceptance Criteria:**
- [ ] User can specify a path to a private key file (~/.ssh/id_rsa or custom path)
- [ ] Optional: user can specify a passphrase if the key is encrypted
- [ ] russh loads and authenticates using the provided private key
- [ ] On successful connection, SSH terminal panel is displayed
- [ ] On key error (missing file, invalid format, wrong passphrase), a clear error message is shown
- [ ] Connection state reflects the active session

### US-3: Display connection status indicator
**As a** user, **I want** to see a visual indicator of the current connection status, **so that** I can quickly determine if I'm connected to the remote host.

**Acceptance Criteria:**
- [ ] Connection status is displayed in the UI (e.g., title bar or header)
- [ ] Status shows: "Connected to [hostname:port]" when connected
- [ ] Status shows: "Disconnected" when no active connection
- [ ] Status updates in real time as connection state changes
- [ ] Visual indicator is unambiguous (text label sufficient for MVP, emoji/color optional)

---

## Epic 2: Terminal Panel

### US-4: Display interactive SSH terminal in left panel
**As a** user, **I want** to see a live interactive terminal in the left panel, **so that** I can execute commands on the remote host.

**Acceptance Criteria:**
- [ ] Terminal panel occupies the left half of the split-pane window
- [ ] Terminal renders SSH output (stdout/stderr) in real time using vt100 emulation (tui-term crate)
- [ ] User can type commands and send them to the remote shell
- [ ] Terminal responds to keyboard input (arrows, backspace, tab completion)
- [ ] Terminal panel is sized dynamically based on window width
- [ ] Terminal text is scrollable (if content exceeds visible area)
- [ ] Exit command (e.g., `exit` or `Ctrl+D`) properly closes the remote session

### US-5: Handle terminal resize and adapt layout
**As a** user, **I want** the terminal panel to automatically adapt when the window is resized, **so that** I can resize my terminal without breaking the layout.

**Acceptance Criteria:**
- [ ] Window resize event is detected and handled
- [ ] Terminal panel is recalculated and redrawn with new dimensions
- [ ] SFTP panel is also recalculated to maintain 50/50 split (or defined ratio)
- [ ] No layout artifacts or overlapping panels after resize
- [ ] Terminal content is re-rendered at the new size

### US-6: Support ANSI color and formatting in terminal output
**As a** user, **I want** colored output and formatting (bold, underline) from remote commands, **so that** the terminal experience is visually complete.

**Acceptance Criteria:**
- [ ] ANSI color codes (16-color and 256-color) are rendered correctly in the terminal panel
- [ ] ANSI text formatting (bold, underline, reverse) is applied
- [ ] Common terminal applications (ls --color, htop, vim, etc.) display correctly
- [ ] Color palette matches the TUI theme (default Ratatui palette)

---

## Epic 3: SFTP Browser Panel

### US-7: Display SFTP file browser in right panel
**As a** user, **I want** a file browser in the right panel, **so that** I can navigate the remote filesystem without leaving the TUI.

**Acceptance Criteria:**
- [ ] SFTP panel is displayed on the right side when a connection is active
- [ ] SFTP panel shows the current working directory (path displayed in header)
- [ ] File listing displays all files and directories in the current directory
- [ ] Each entry shows: name, type (file/directory), size, permissions, and modification date
- [ ] Directory entries are visually distinguishable from files (e.g., prefix with "/" or different styling)
- [ ] Panel updates when the user navigates to a different directory

### US-8: Navigate directory tree in SFTP browser
**As a** user, **I want** to navigate up and down the directory tree, **so that** I can explore different folders on the remote filesystem.

**Acceptance Criteria:**
- [ ] User can press Enter on a directory to navigate into it
- [ ] User can press Backspace or '`' to navigate to the parent directory
- [ ] Current path is always visible (e.g., in the panel header: "/home/user/documents")
- [ ] Navigation does not break if the directory is inaccessible (show error, stay in current directory)
- [ ] SFTP operations fail gracefully if the user lacks permissions

### US-9: Select files in SFTP browser
**As a** user, **I want** to select a file or directory with arrow keys and highlight it, **so that** I can prepare it for an operation (download, upload, edit).

**Acceptance Criteria:**
- [ ] Arrow Up/Down keys move selection highlight through the file list
- [ ] Selected file/directory is visually highlighted (e.g., inverted colors, thick border)
- [ ] Selection position is preserved when the listing is refreshed
- [ ] Selection index is reset to 0 if the list is empty (e.g., after directory change)

### US-10: Display file metadata in SFTP browser
**As a** user, **I want** to see detailed metadata for each file, **so that** I can make informed decisions about file operations.

**Acceptance Criteria:**
- [ ] File list displays the following columns: Name, Size (human-readable), Permissions (octal), Modified Date
- [ ] Size is displayed in KB/MB/GB format (e.g., "1.5 MB" instead of raw bytes)
- [ ] Permissions are shown in format like "-rw-r--r--" or octal "644"
- [ ] Modified date is shown in a readable format (e.g., "2025-03-05 14:30")
- [ ] Columns are aligned and the layout is clean

---

## Epic 4: File Operations

### US-11: Download file from remote to local
**As a** user, **I want** to download a selected file to my local machine, **so that** I can access remote files locally.

**Acceptance Criteria:**
- [ ] User selects a file in the SFTP browser and presses 'd' to download
- [ ] User is prompted for a local destination directory (default: current working directory of the TUI)
- [ ] File is transferred from remote to local using SFTP
- [ ] Progress is displayed during transfer (percentage complete, e.g., "Downloading: 45%")
- [ ] On completion, a success message is shown with the local path
- [ ] On failure, an error message is shown (insufficient permissions, disk full, network error)
- [ ] User cannot download a directory (error: "Cannot download directory" or offer to skip)

### US-12: Upload file to remote directory
**As a** user, **I want** to upload a local file to the remote filesystem, **so that** I can transfer files to the server.

**Acceptance Criteria:**
- [ ] User presses 'u' in the SFTP browser to upload
- [ ] User is prompted to specify a local file path (or browse local filesystem)
- [ ] File is transferred from local to remote using SFTP (to current SFTP directory)
- [ ] Progress is displayed during transfer (percentage complete)
- [ ] On completion, a success message is shown with the remote path
- [ ] On failure, an error message is shown (file not found, permission denied, disk full)
- [ ] SFTP browser is refreshed after successful upload to show the new file

### US-13: Edit remote file in-place with $EDITOR
**As a** user, **I want** to open a remote file in my local text editor, edit it, and automatically upload the changes, **so that** I can edit remote files without manual upload.

**Acceptance Criteria:**
- [ ] User selects a file in SFTP browser and presses 'e'
- [ ] File is downloaded to a temporary directory
- [ ] Local $EDITOR environment variable is used to open the file (e.g., vim, nano, code)
- [ ] Editor window opens and blocks the TUI
- [ ] On editor exit, the TUI detects changes to the file
- [ ] If the file was modified, the user is prompted: "Upload changes?" (Yes/No)
- [ ] If Yes, modified file is uploaded back to the remote location
- [ ] On failure, error message is shown (can't open editor, upload failed)
- [ ] Temporary file is cleaned up after the operation completes

### US-14: Show transfer progress with visual feedback
**As a** user, **I want** to see the progress of file transfers, **so that** I know how long operations will take.

**Acceptance Criteria:**
- [ ] Download/upload operations display a progress bar
- [ ] Progress bar shows: percentage complete (e.g., "45%"), bytes transferred / total (e.g., "4.5 MB / 10 MB"), and estimated time remaining
- [ ] Progress updates smoothly during the transfer
- [ ] Large files (> 100 MB) show realistic progress over several seconds
- [ ] Transfer can be interrupted with Ctrl+C (cancel and cleanup)
- [ ] After cancellation, a confirmation message is shown

---

## Epic 5: Panel Navigation and UX

### US-15: Switch focus between terminal and SFTP panels
**As a** user, **I want** to switch focus between the terminal and SFTP panels using keyboard shortcuts, **so that** I can operate both panels without the mouse.

**Acceptance Criteria:**
- [ ] Pressing Tab key switches focus from one panel to the other
- [ ] Currently focused panel is visually indicated (e.g., thicker border, different color)
- [ ] All keyboard input is routed to the focused panel (terminal input goes to SSH, arrow keys go to SFTP browser)
- [ ] Tab is repeatable to cycle through panels

### US-16: Toggle SFTP panel visibility
**As a** user, **I want** to show or hide the SFTP panel, **so that** I can maximize the terminal when I don't need the file browser.

**Acceptance Criteria:**
- [ ] User can press a keybind (e.g., 'Ctrl+F' or Ctrl+B') to toggle SFTP panel visibility
- [ ] When hidden, terminal expands to fill the full window width
- [ ] When shown, terminal returns to 50% width (or defined ratio) and SFTP panel takes the other 50%
- [ ] SFTP panel state persists across focus changes (hidden stays hidden, shown stays shown)
- [ ] Title or status bar indicates whether SFTP panel is currently visible

### US-17: Display help/keybindings overlay
**As a** user, **I want** to view a list of all keybindings, **so that** I can discover available actions without memorizing them.

**Acceptance Criteria:**
- [ ] Pressing '?' opens a help overlay showing all keybindings
- [ ] Help overlay displays: [Key] → [Action] in a readable format, grouped by panel (Terminal, SFTP, Global)
- [ ] Help overlay is modal (terminal and SFTP operations are paused while visible)
- [ ] User can press Escape or 'q' to close the help overlay
- [ ] Help overlay does not obstruct the entire window (e.g., scrollable if needed)

### US-18: Show status messages for user feedback
**As a** user, **I want** to receive feedback messages after actions (success, error, warning), **so that** I know if an operation succeeded or failed.

**Acceptance Criteria:**
- [ ] After file operations (download, upload, edit), a status message is displayed
- [ ] Messages appear in a dedicated status bar area (e.g., bottom of screen)
- [ ] Messages show: [Status] [Action] [Details] (e.g., "SUCCESS: File downloaded to /home/user/file.txt")
- [ ] Error messages are clearly marked as errors (different color or prefix)
- [ ] Messages persist for 3-5 seconds or until the next action
- [ ] Messages do not block user input (non-modal)

---

## Epic 6: Core Application Setup

### US-19: Initialize sshm-term application with default configuration
**As a** developer, **I want** a default configuration and initialization, **so that** the app can start up cleanly.

**Acceptance Criteria:**
- [ ] App starts with default settings (window size, terminal config, colors)
- [ ] App initializes Ratatui terminal in raw mode
- [ ] Default local directory is set to the user's home directory or current working directory
- [ ] SFTP panel is shown by default on startup
- [ ] SFTP panel is disabled (no connection active) until a user connects
- [ ] App can be launched with `sshm-term` command (from cargo install or local build)
- [ ] App exits cleanly on Ctrl+C or 'q' key, restoring terminal state

### US-20: Handle disconnection and reconnection scenarios
**As a** user, **I want** the app to handle unexpected disconnections gracefully, **so that** I can understand what happened and potentially reconnect.

**Acceptance Criteria:**
- [ ] If the SSH connection drops unexpectedly, the app detects the failure
- [ ] Terminal panel shows an error message: "Connection lost: [reason]" (e.g., "Connection reset by peer")
- [ ] SFTP panel is disabled and shows: "Not connected"
- [ ] User can initiate a new connection (prompt for host/auth) without restarting the app
- [ ] Existing SFTP browser state is reset on disconnection

### US-21: Handle terminal size constraints and edge cases
**As a** user, **I want** the app to work correctly on very small or very large terminal windows, **so that** I can use it on any terminal size.

**Acceptance Criteria:**
- [ ] App handles terminal size < 80x24 gracefully (e.g., shows a message "Terminal too small, please resize")
- [ ] App handles very large terminal sizes (> 200x100) without layout breaks
- [ ] Minimum viable size is defined (e.g., 80x24), and app prompts user to resize if needed
- [ ] Panel split ratio adapts to available space (e.g., both panels scale proportionally)

---

## Out of Scope for MVP

The following features are explicitly NOT included in Phase 1:

- **Drag-and-drop file operations** (keyboard-only for MVP)
- **File watcher** (auto-sync on file changes)
- **Transfer queue** (single transfers only, sequential)
- **Automatic reconnection** (user must manually reconnect if connection drops)
- **Parallel/concurrent transfers** (single transfer at a time)
- **Directory sync** (cd command synchronization between terminal and SFTP browser)
- **Remote file deletion** (no rm/delete operations via SFTP)
- **Remote file creation** (touch, mkdir via SFTP)
- **File search/filtering** in SFTP browser
- **Multi-select** files for batch operations
- **Bookmarks/favorites** for directories
- **Custom themes** (use default Ratatui theme)
- **SSH key generation** or management UI
- **SSH config integration** (manual host entry only)
- **Port forwarding setup** via TUI
- **Mouse support** (keyboard only)
- **Session recording** or session history
- **Clipboard operations** (e.g., copy hostname to clipboard)

---

## Technical Notes

### Key Dependencies
- **SSH**: russh (async SSH client)
- **SFTP**: russh (includes SFTP support)
- **TUI**: Ratatui (terminal UI framework)
- **Terminal emulation**: tui-term or similar vt100 emulator
- **Async runtime**: Tokio (for SSH and file I/O)

### Design Constraints
- **No external SSH binary** (pure Rust russh library)
- **Single-threaded event loop** (Tokio task-based concurrency)
- **No persistent storage** of credentials (passwords entered per-session, keys from filesystem)
- **Graceful degradation** on small terminal sizes

---

## Acceptance Definition

A story is **DONE** when:
1. All acceptance criteria are met
2. Code is reviewed and approved by mr-reviewer
3. Tests pass (unit + integration tests for critical paths)
4. Manual QA verification confirms the user experience matches the story intent
5. No regressions in other stories or existing sshm-rs features

---

## Story Estimation (Planning Reference)

| Epic | Story Count | Est. Total Effort |
|------|-------------|-------------------|
| SSH Connection | 3 | 8h |
| Terminal Panel | 3 | 12h |
| SFTP Browser | 4 | 12h |
| File Operations | 4 | 16h |
| Panel Navigation | 4 | 6h |
| Core Application | 3 | 8h |
| **TOTAL** | **21 stories** | **~60-70h** |

*Note: Estimates are for reference only. Actual effort may vary based on architecture and test coverage.*
