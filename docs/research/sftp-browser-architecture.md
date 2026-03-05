# SFTP File Browser Architecture Research

**Date:** 2026-03-05
**Context:** Research for `sshm-term` right panel -- SFTP file browser alongside SSH terminal
**Status:** Research complete, pending ADR

---

## 1. SFTP Crate Comparison

### 1.1 ssh2 (libssh2 binding) -- already in Cargo.toml

| Attribute | Detail |
|---|---|
| **Version** | 0.9.5 (Feb 2025) |
| **Downloads** | ~203k/month |
| **Maintainers** | Alex Crichton, Wez Furlong, Matteo Bigoi, Gabriel Smith + 70 contributors |
| **Type** | FFI binding to libssh2 (C library) |
| **Sync/Async** | Synchronous (blocking). Non-blocking mode available but requires manual EAGAIN retry loops. |
| **Windows** | Fully supported. CI tests on Linux/Windows/macOS. `vendored-openssl` feature for static linking. |
| **License** | MIT / Apache-2.0 |

**SFTP Operations (via `Session::sftp()`):**

| Operation | Method | Notes |
|---|---|---|
| List directory | `readdir()` | Returns `Vec<(PathBuf, FileStat)>`, filters `.` and `..` |
| File stat | `stat()`, `lstat()` | lstat for symlinks |
| Open file | `open()`, `open_mode()` | Read mode helper / custom flags+permissions |
| Create file | `create()` | Write-only with truncation |
| Make directory | `mkdir()` | With mode parameter |
| Remove directory | `rmdir()` | |
| Delete file | `unlink()` | |
| Rename | `rename()` | Flags: Overwrite, Native, Atomic |
| Symlink | `symlink()`, `readlink()` | |
| Resolve path | `realpath()` | |
| Set metadata | `setstat()` | |

**Authentication methods:**
- `userauth_password()` -- password
- `userauth_pubkey_file()` / `userauth_pubkey_memory()` -- public key
- `userauth_agent()` -- SSH agent (including Pageant on Windows)
- `userauth_keyboard_interactive()` -- keyboard-interactive (2FA)
- `userauth_hostbased_file()` -- host-based

**Connection multiplexing:**
- SSH protocol supports multiple channels per connection
- `Session::channel_session()` for shell, `Session::sftp()` for SFTP subsystem
- **Both can coexist on the same Session** BUT: a blocking read on one channel blocks ALL other channels on the same Session
- Non-blocking mode is possible but requires manual `EAGAIN` retry loops -- fragile and error-prone
- **Verdict: multiplexing is technically possible but impractical for concurrent shell+SFTP with ssh2's synchronous API**

**Performance:**
- SFTP transfer speed depends heavily on buffer size. 65KB buffers match SCP speed; 4KB buffers are 3.4x slower
- Use `BufReader`/`BufWriter` with 32-65KB capacity + `std::io::copy()`
- Large directory listings (29k+ files) have caused segfaults in libssh2 historically
- `readdir()` returns all entries at once -- no streaming/pagination API

**Keepalive:** `set_keepalive()` and `keepalive_send()` available

**Strengths:**
- Already a dependency in our project
- Battle-tested, very widely used (193 dependent crates)
- Full SFTP operation coverage
- Excellent Windows support
- Strong auth method coverage

**Weaknesses:**
- Synchronous -- blocking one channel blocks all channels on the session
- C dependency (libssh2) -- compile complexity, potential segfaults
- Non-blocking mode is cumbersome
- No native async support (would need `spawn_blocking` wrappers)

---

### 1.2 russh + russh-sftp (Pure Rust, async)

| Attribute | Detail |
|---|---|
| **Version** | russh 0.57.1 (Feb 2026), russh-sftp 2.1.1 (Apr 2025) |
| **Downloads** | russh ~194k/month, russh-sftp ~95k/month |
| **Maintainer** | Eugene (Eugeny -- author of Tabby terminal) + 93 contributors |
| **Type** | Pure Rust SSH2 implementation (fork of thrussh) |
| **Sync/Async** | Fully async (tokio) |
| **Windows** | Supported (Pageant agent support included). No explicit known blockers. |
| **License** | Apache-2.0 |

**SFTP operations (via russh-sftp):**
- SFTPv3 compliant (most widely deployed version)
- Extensions: `limits@openssh.com`, `hardlink@openssh.com`, `fsync@openssh.com`, `statvfs@openssh.com`
- Async I/O for file operations
- Client and server support

**Authentication:** Password, public key, keyboard-interactive, OpenSSH certificates, none

**Connection multiplexing:**
- SSH protocol channels are first-class citizens in russh
- Multiple channels (shell + SFTP) on one connection are natively supported
- Async architecture means channels do NOT block each other
- **Verdict: True concurrent shell+SFTP on a single connection is fully supported**

**Key advantages:**
- Pure Rust -- no C dependencies, no segfault risk, better cross-compilation
- Truly async -- non-blocking concurrent channels
- Modern, actively maintained (backed by Eugeny/Tabby)
- Growing ecosystem (used by VS Code russh fork from Microsoft)

**Key concerns:**
- Would require adding tokio as a dependency (currently the project is sync/std::thread)
- Larger API surface, more complex client setup
- russh-sftp last updated Apr 2025 (10 months ago) -- not stale but not fresh either
- 4 security advisories in russh history (resolved)

---

### 1.3 openssh-sftp-client

| Attribute | Detail |
|---|---|
| **Type** | Pure Rust SFTP v3 client, generic over transport |
| **API style** | Modeled after `std::fs` and `tokio::fs` |
| **Flexibility** | Can be paired with ANY SSH implementation |
| **Async** | Yes (tokio) |

**Pros:**
- Very ergonomic API (feels like local filesystem)
- Decoupled from SSH implementation -- could use russh, ssh2, or system openssh
- Pure Rust

**Cons:**
- Requires a separate SSH implementation to provide the SFTP subsystem channel
- Lower adoption than ssh2 or russh-sftp
- Additional complexity from abstraction layers

**Verdict:** Interesting for flexibility, but adds indirection. Better to use russh-sftp which is tightly integrated.

---

### 1.4 remotefs + remotefs-ssh

| Attribute | Detail |
|---|---|
| **Version** | remotefs-ssh 0.7.2 (Jan 2026) |
| **Downloads** | ~334/month |
| **Author** | Christian Visintin (veeso -- termscp author) |
| **Backend** | libssh2 (default) or libssh |

**Pros:**
- Protocol-agnostic `RemoteFs` trait -- same API for SFTP/FTP/S3/WebDAV
- Proven in production (termscp)
- Higher-level abstraction, less boilerplate
- SSH config file parsing built-in

**Cons:**
- Very low adoption (334 downloads/month)
- Still depends on libssh2 under the hood -- same C dependency issues
- Synchronous
- Extra abstraction layer we may not need (we only do SFTP)
- `SftpFs` is only `Sync` with libssh2 backend

**Verdict:** Over-engineered for our single-protocol use case. Good concept but we would not benefit from the multi-protocol abstraction.

---

### 1.5 thrussh (predecessor of russh)

**Status: Deprecated.** Russh is the active fork. Do not use thrussh.

---

### Summary Table

| Crate | Async | Pure Rust | Windows | Multiplexing | Downloads | Recommendation |
|---|---|---|---|---|---|---|
| ssh2 | No | No (libssh2) | Yes | Impractical | 203k/mo | Keep for connectivity checks |
| russh + russh-sftp | Yes | Yes | Yes | Native | 194k/mo | **Best for sshm-term** |
| openssh-sftp-client | Yes | Yes | Partial | N/A | Low | Skip |
| remotefs-ssh | No | No (libssh2) | Yes | No | 0.3k/mo | Skip |
| thrussh | Yes | Yes | Unknown | Unknown | Deprecated | Skip |

---

## 2. SSH Connection Multiplexing

### The Critical Question

Can we use ONE SSH connection for both the terminal shell AND SFTP file browser?

### Answer: YES -- with russh

The SSH protocol (RFC 4254) supports multiple channels on a single connection. This is how MobaXterm works -- since version 8.4, MobaXterm's SSH-browser reuses the existing terminal connection via SSH multiplexing rather than opening a new connection.

**Benefits of single-connection multiplexing:**
- No double authentication (critical for OTP/2FA)
- Fewer network resources
- SFTP browser stays alive as long as the terminal connection
- Consistent with user expectations (one host = one connection)

**Implementation by crate:**

| Crate | Single-connection multiplexing | Practical for shell+SFTP? |
|---|---|---|
| **ssh2** | Channels exist but blocking one blocks all | No -- synchronous mutex makes it impractical |
| **russh** | Multiple async channels on one connection | **Yes -- designed for this** |

**With russh:**
1. Establish one `russh::client::Handle`
2. Open channel A: request PTY + shell (for terminal)
3. Open channel B: request SFTP subsystem (for file browser)
4. Both channels operate concurrently via tokio tasks
5. Single authentication, single TCP connection

**This is the strongest argument for choosing russh over ssh2.**

---

## 3. File Browser UI Patterns in ratatui

### 3.1 Existing Widgets

| Widget | Version | Downloads | Style | Notes |
|---|---|---|---|---|
| **tui-tree-widget** | 0.24.0 (Jan 2026) | 45k/mo | Tree view | Best option. Ratatui 0.30 compatible. Used in 56 crates. |
| **ratatui-explorer** | 0.2.1 (May 2025) | 1.8k/mo | Flat list | Local filesystem only. Vim keybindings. Not suited for remote. |
| **tui-file-explorer** | - | Low | Flat list | Self-contained, keyboard-driven. Local only. |

### 3.2 Recommended Approach: Custom widget with tui-tree-widget

Use `tui-tree-widget` for the tree structure, wrapping it in a custom SFTP browser component.

**UI Elements:**
- **Breadcrumb path** at top: `/home/user/project/src/` with clickable segments
- **Tree view** for directory listing with expand/collapse
- **File icons** via Unicode: folder, file types (code, text, image, archive, etc.)
- **Columns**: Name | Size (human-readable) | Date | Permissions
- **Status bar** at bottom: selected count, current operation, connection status

**File Icon Mapping (Unicode):**
```
Directory:  (or folder emoji)
Rust:       (or generic code icon)
Config:
Text:
Image:
Archive:
Symlink:
Default:
```

**Size Formatting:** `humansize` pattern -- 1.2 KB, 45.3 MB, 2.1 GB

**Permissions Display:** Unix-style `rwxr-xr-x` or octal `755`

**Date Formatting:** Relative for recent (2h ago, yesterday), absolute for older (2026-01-15)

### 3.3 Keyboard Navigation

| Key | Action |
|---|---|
| j/k or arrows | Navigate up/down |
| Enter or l | Enter directory / expand tree node |
| Backspace or h | Go to parent directory |
| Space | Toggle selection |
| / | Filter/search within current directory |
| Tab | Switch focus between terminal and file browser |
| u | Upload file(s) |
| d | Download file(s) |
| e | Edit file (download + editor + re-upload) |
| D | Delete file(s) with confirmation |
| r | Rename |
| n | New directory |
| R | Refresh listing |
| . | Toggle hidden files |

---

## 4. File Transfer Patterns

### 4.1 Progress Tracking

Use ratatui's `Gauge` widget for transfer progress:
- Show filename, transferred/total bytes, speed (MB/s), ETA
- For multiple files: overall progress bar + current file bar
- Use tokio channels to send progress updates from transfer task to UI

**Architecture:**
```
[Transfer Task (tokio::spawn)]
    |
    | ---> mpsc::Sender<TransferProgress>
    |
[UI Event Loop]
    |
    | <--- mpsc::Receiver<TransferProgress>
    |
    v
[Gauge Widget renders progress]
```

### 4.2 Buffer Strategy

Based on ssh2-rs benchmarks and libssh2 documentation:
- **Read buffer:** 65536 bytes (65KB) -- optimal for SFTP
- **Write buffer:** 65536 bytes -- matches SCP performance
- Use `BufReader`/`BufWriter` wrappers
- Never use buffers smaller than 32KB for SFTP (causes round-trip per call)

### 4.3 Resume Support

SFTP v3 supports `SSH_FXP_OPEN` with `SSH_FXF_APPEND` flag. For resume:
1. Check remote file size with `stat()`
2. Open with offset seek
3. Continue transfer from offset
4. Verify integrity (optional: checksum if available)

### 4.4 Recursive Directory Transfer

```
fn transfer_directory_recursive(sftp, local_path, remote_path, progress_tx):
    1. List local directory entries
    2. For each entry:
       - If directory: mkdir remote + recurse
       - If file: transfer with progress reporting
    3. Report completion to progress channel
```

### 4.5 Conflict Resolution

When target file exists, present dialog:
- **Overwrite** -- replace target
- **Skip** -- skip this file
- **Rename** -- append suffix (e.g., `file (1).txt`)
- **Overwrite All / Skip All** -- apply to remaining files

### 4.6 Background Transfers

- Transfers run in tokio tasks, not blocking the UI
- Transfer queue for multiple operations
- Cancel support via `tokio::select!` with cancellation token
- Transfer history/log panel

---

## 5. Edit-in-Editor Integration

### 5.1 Workflow

```
1. User presses 'e' on a remote file
2. Download file to temp directory: %TEMP%/sshm-rs/<host>/<path>/filename
3. Record file modification time (mtime)
4. Launch editor:
   - $EDITOR if set (vim, nano, etc.)
   - `code --wait <file>` for VS Code
   - Configurable in sshm-rs settings
5. Wait for editor process to exit
6. Compare mtime: if changed, re-upload to remote
7. Clean up temp file (or keep for cache)
```

### 5.2 Editor Detection

```
Priority:
1. sshm-rs config setting (user preference)
2. $VISUAL environment variable
3. $EDITOR environment variable
4. Platform default: notepad (Windows), vi (Unix)
```

### 5.3 File Watching (Advanced -- Post-MVP)

For editors that detach (like VS Code without `--wait`):
- Use the `notify` crate (cross-platform filesystem watcher)
- Watch the temp file for modifications
- Auto-upload on save (debounced)
- `notify` handles safe-save patterns (write to temp + rename) correctly
- Stop watching when user closes the file browser or navigates away

### 5.4 How termscp Does It

termscp implements the same pattern:
1. Download to temp directory
2. Open in configured text editor
3. Check mtime after editor exits
4. Re-upload if modified
5. No file watcher -- they acknowledge this limitation

termscp also has a separate file watcher feature (`<T>` key) that monitors local directories and auto-syncs changes to remote. We could implement something similar as a post-MVP feature.

---

## 6. Performance Considerations

### 6.1 Large Directory Listings

**Problem:** `readdir()` returns ALL entries at once. Directories with 10k+ files could cause:
- Memory spikes (all FileStat entries in memory)
- UI freeze during listing
- Historical libssh2 segfaults with 29k+ entries

**Mitigations:**
- Run `readdir()` in a background task, stream results to UI
- Virtual scrolling in the tree widget (only render visible rows)
- Lazy loading: expand subdirectories on demand, not eagerly
- Consider a maximum display limit with "load more" prompt (e.g., 5000 files)
- Cache directory listings with TTL (30-60 seconds)

### 6.2 File Transfer

- 65KB buffer size is optimal (matches SCP performance)
- For files > 100MB, show transfer speed and ETA
- Memory usage: only buffer-sized chunks in memory, no full file loading
- Parallel transfers: support 2-3 concurrent transfers (configurable)

### 6.3 SSH Keepalive

- Set keepalive interval (default: 30 seconds)
- Handle keepalive failures gracefully (show "reconnecting..." status)
- With russh: `keepalive_interval` configuration option

### 6.4 Reconnection

- Detect connection drop (channel EOF, timeout, error)
- Auto-reconnect with exponential backoff
- Re-authenticate using cached credentials
- Restore file browser to previous path after reconnection
- Show connection status indicator in UI

---

## 7. Recommendation

### 7.1 Best SFTP Crate: russh + russh-sftp

**Primary reasons:**
1. **Single-connection multiplexing** -- the killer feature. One SSH connection serves both terminal and SFTP, avoiding double authentication and matching MobaXterm's behavior.
2. **Pure Rust** -- no C dependency (libssh2), no segfault risk, simpler cross-compilation.
3. **Async-native** -- concurrent shell I/O and SFTP operations without blocking each other. Essential for a responsive TUI.
4. **Active maintenance** -- maintained by the author of Tabby terminal, 93 contributors, ~194k downloads/month.
5. **Modern auth** -- supports OpenSSH certificates in addition to standard methods.

**What about ssh2 (already in Cargo.toml)?**
Keep ssh2 for the existing connectivity check feature (`PingManager`). It is simple, works well for TCP+handshake checks, and does not need async. The SFTP browser is a separate concern and should use russh.

### 7.2 Migration Path

The project currently uses ssh2 for two things:
1. **Connectivity checks** (`PingManager`) -- keep on ssh2, it works fine
2. **Interactive shell** (`connect_ssh2_interactive`) -- this should migrate to russh when building sshm-term, as it needs to share the connection with SFTP

### 7.3 New Dependencies Required

| Crate | Purpose | Justification |
|---|---|---|
| `russh` | SSH client | Pure Rust async SSH, enables connection multiplexing |
| `russh-sftp` | SFTP subsystem | SFTPv3 client for file operations |
| `tokio` | Async runtime | Required by russh; also needed for async TUI event loop |
| `tui-tree-widget` | Tree view widget | Mature (45k/mo), ratatui 0.30 compatible, used in 56 crates |
| `notify` | File watcher | For edit-in-editor auto-sync (post-MVP) |

### 7.4 MVP Feature Set

**Phase 1 -- Core Browser:**
- [ ] Single SSH connection with shell channel + SFTP channel (russh)
- [ ] Right panel: directory listing with tree view
- [ ] Navigate directories (enter, parent, breadcrumb)
- [ ] File metadata display (size, date, permissions)
- [ ] Toggle hidden files
- [ ] Filter/search within directory

**Phase 2 -- File Operations:**
- [ ] Download file (single file, with progress bar)
- [ ] Upload file (single file, with progress bar)
- [ ] Delete file/directory with confirmation
- [ ] Rename file/directory
- [ ] Create new directory

**Phase 3 -- Edit & Advanced:**
- [ ] Edit-in-editor (download, edit, re-upload)
- [ ] Recursive directory download/upload
- [ ] Multiple file selection
- [ ] Conflict resolution dialogs
- [ ] Background transfer queue

**Phase 4 -- Polish:**
- [ ] File watcher for auto-sync
- [ ] Transfer resume
- [ ] Connection reconnection
- [ ] Parallel transfers
- [ ] Drag-and-drop (if terminal supports it)

### 7.5 Architecture Diagram

```
+------------------------------------------------------------------+
|                        sshm-term                                  |
|                                                                   |
|  +---------------------------+  +------------------------------+  |
|  |   Left Panel: Terminal    |  |  Right Panel: SFTP Browser   |  |
|  |                           |  |                              |  |
|  |  +---------+              |  |  +-------------------------+ |  |
|  |  | VT100   |  stdin/out   |  |  | Breadcrumb Path Bar     | |  |
|  |  | Parser  |  pty I/O     |  |  +-------------------------+ |  |
|  |  +---------+              |  |  | Tree View (tui-tree)    | |  |
|  |       |                   |  |  |  > src/                 | |  |
|  |       v                   |  |  |    main.rs   1.2KB      | |  |
|  |  +---------+              |  |  |    lib.rs    3.4KB      | |  |
|  |  | Channel |              |  |  |  > docs/               | |  |
|  |  | (shell) |              |  |  +-------------------------+ |  |
|  |  +----+----+              |  |  | Status / Progress Bar   | |  |
|  |       |                   |  |  +------------+------------+ |  |
|  +-------|-------------------+  +---------------|------------+  |
|          |                                      |               |
|          +------------------+-------------------+               |
|                             |                                   |
|                    +--------v--------+                          |
|                    |  russh::client  |                          |
|                    |    Handle       |                          |
|                    |                 |                          |
|                    | Channel A: PTY  |                          |
|                    | Channel B: SFTP |                          |
|                    +--------+--------+                          |
|                             |                                   |
+-----------------------------+-----------------------------------+
                              |
                     Single TCP Connection
                              |
                     +--------v--------+
                     |   SSH Server    |
                     +-----------------+
```

### 7.6 Key Architecture Decisions

1. **Single connection, two channels** -- russh enables true SSH multiplexing
2. **Async runtime (tokio)** -- required for russh, also benefits UI responsiveness
3. **Component-based UI** -- terminal panel and SFTP panel as independent ratatui components communicating via channels
4. **Background transfers** -- file operations run in tokio tasks, progress reported via mpsc channels
5. **Lazy directory loading** -- only fetch directory contents when expanded
6. **Keep ssh2 for ping** -- no need to migrate the connectivity checker

---

## Sources

- [ssh2 Session docs](https://docs.rs/ssh2/latest/ssh2/struct.Session.html)
- [ssh2 Sftp docs](https://docs.rs/ssh2/latest/ssh2/struct.Sftp.html)
- [ssh2 on lib.rs](https://lib.rs/crates/ssh2)
- [russh on GitHub](https://github.com/Eugeny/russh)
- [russh on lib.rs](https://lib.rs/crates/russh)
- [russh-sftp docs](https://docs.rs/russh-sftp/latest/russh_sftp/)
- [russh-sftp on lib.rs](https://lib.rs/crates/russh-sftp)
- [openssh-sftp-client on lib.rs](https://lib.rs/crates/openssh-sftp-client)
- [remotefs-ssh on lib.rs](https://lib.rs/crates/remotefs-ssh)
- [remotefs docs](https://docs.rs/remotefs/latest/remotefs/index.html)
- [A journey into File Transfer Protocols in Rust (veeso blog)](https://blog.veeso.dev/blog/en/a-journey-into-file-transfer-protocols-in-rust/)
- [termscp File Operations (DeepWiki)](https://deepwiki.com/veeso/termscp/4.2-file-operations)
- [termscp on GitHub](https://github.com/veeso/termscp)
- [tui-tree-widget on GitHub](https://github.com/EdJoPaTo/tui-rs-tree-widget)
- [tui-tree-widget on lib.rs](https://lib.rs/crates/tui-tree-widget)
- [ratatui-explorer on lib.rs](https://lib.rs/crates/ratatui-explorer)
- [Ratatui Async Tutorial](https://ratatui.rs/tutorials/counter-async-app/)
- [ssh2-rs SFTP performance issue #206](https://github.com/alexcrichton/ssh2-rs/issues/206)
- [libssh2 SFTP performance discussion](https://github.com/libssh2/libssh2/issues/90)
- [MobaXterm 8.4 SSH multiplexing](https://blog.mobatek.net/post/mobaxterm-new-release-8.4/)
- [MobaXterm features](https://mobaxterm.mobatek.net/features.html)
- [notify crate on GitHub](https://github.com/notify-rs/notify)
- [russh multiple sessions discussion](https://users.rust-lang.org/t/running-multiple-ssh-client-sessions-using-russh/123513)
- [portable-pty on lib.rs](https://lib.rs/crates/portable-pty)
