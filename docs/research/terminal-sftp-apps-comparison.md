# Terminal + SFTP Browser Applications: Comprehensive Comparison

> **Date**: 2026-03-05
> **Purpose**: Research existing terminal + SFTP browser solutions to inform sshm-term design decisions.

---

## Summary Comparison Matrix

| Application | Platform | Architecture | SFTP Integration | Terminal Quality | License | Approx. Memory | Single Window |
|---|---|---|---|---|---|---|---|
| **MobaXterm** | Windows | Native (C++) | Auto sidebar on SSH connect | Good (xterm) | Freemium | ~100-150 MB | Yes |
| **Termius** | Win/Mac/Linux/iOS/Android | Electron | Separate SFTP tab/panel | Good (true-color) | Freemium | ~200-300 MB | Yes |
| **SecureCRT + SecureFX** | Win/Mac/Linux | Native (C++) | Shared sessions, SFTP tab | Excellent | Commercial ($119+) | ~80-120 MB | Integrated pair |
| **PuTTY + WinSCP** | Windows | Native (C) | Separate apps, linked launch | Basic (PuTTY) | Free/OSS | ~30 MB combined | No (two apps) |
| **Royal TSX** | macOS | Native (Swift/ObjC) | Plugin-based SFTP panel | Good (iTerm2 plugin) | Freemium | ~120-180 MB | Yes |
| **Remmina** | Linux | Native (GTK+3) | Separate SFTP protocol tab | Basic | Free/OSS | ~60-100 MB | Yes |
| **Electerm** | Win/Mac/Linux | Electron | Side-by-side SFTP + terminal | Good (xterm.js) | Free/OSS | ~200-350 MB | Yes |
| **WindTerm** | Win/Mac/Linux | Native (C/C++) | Integrated SFTP + local FM | Good | Free (source avail.) | ~70 MB | Yes |
| **Tabby** | Win/Mac/Linux | Electron | SFTP panel per SSH session | Good (xterm.js) | Free/OSS | ~250-400 MB | Yes |
| **TUI tools** (mc, ranger, nnn, yazi) | Any (over SSH) | Native (C/Python/Rust) | Run remotely over SSH | N/A (run inside terminal) | Free/OSS | ~5-30 MB | N/A |

### Quick Verdict

| Criterion | Best-in-class |
|---|---|
| UX integration (terminal + SFTP) | MobaXterm, WindTerm |
| Cross-platform | Termius, WindTerm, Electerm |
| Performance / memory | WindTerm, PuTTY+WinSCP |
| Feature richness | MobaXterm, SecureCRT+SecureFX |
| Open source | Electerm, Tabby, Remmina |
| Mobile support | Termius (only one with iOS/Android) |

---

## Detailed Application Profiles

### 1. MobaXterm (Windows)

**Architecture**: Native Win32 application written in C++. Ships as a single portable `.exe` or installer. Embeds Cygwin/MSYS tools for Unix compatibility on Windows. Lightweight despite the massive feature set.

**SFTP Integration**: The defining feature. When you open an SSH session, a graphical SFTP browser **automatically appears in the left sidebar**, sharing the same authenticated connection. No extra authentication step, no separate window. Users can:
- Browse the remote filesystem in a tree view
- Drag-and-drop files between local machine and remote server
- Double-click to edit remote files locally (auto-upload on save)
- Right-click context menus for rename, delete, chmod, etc.
- Auto-fallback from SFTP to SCP if SFTP subsystem is unavailable

**Terminal Quality**: xterm-compatible terminal emulation. Supports 256-color, Unicode. No GPU acceleration or ligature support. Adequate for most server administration work.

**Key Features**:
- Multi-tab sessions with session management sidebar
- Embedded X11 server (Xorg) for remote GUI apps
- Built-in RDP, VNC, XDMCP, FTP, serial port support
- Macro recording and replay
- Split-pane terminal views
- Persistent home directory (Cygwin-based)
- Portable mode (single .exe, no install needed)
- SSH tunneling GUI

**Performance**: Starts in ~2-3 seconds. Memory usage ~100-150 MB with several sessions open. Efficient native code. SFTP browser is responsive even with large directories.

**License**: Free Home Edition (limited to 12 sessions, 2 SSH tunnels, 4 macros). Professional Edition ~$69/user.

**What Makes MobaXterm's UX Exceptional**:
1. **Zero-friction SFTP**: The sidebar appears automatically -- no menu navigation, no separate connection
2. **Shared authentication**: SFTP reuses the SSH session, no re-entering passwords
3. **Spatial consistency**: The file browser is always in the same place (left sidebar), building muscle memory
4. **Overlay mode**: The sidebar can overlay the terminal without resizing it, or be pinned as a persistent panel
5. **Context-aware**: The SFTP browser follows the terminal's `cd` commands (optional)
6. **Edit-in-place**: Double-click a file, it opens in a local editor; save triggers auto-upload

---

### 2. Termius (Cross-platform)

**Architecture**: Built with Electron (Chromium + Node.js). Available on Windows, macOS, Linux, iOS, and Android. Uses a centralized cloud service for syncing connections, keys, and snippets across devices.

**SFTP Integration**: SFTP is available as a separate tab or panel alongside terminal tabs. Recent updates allow opening multiple SFTP sessions simultaneously. Features include:
- Drag-and-drop between SFTP panels
- In-app file editor for remote files
- File preview on mobile (tap to preview)

**Terminal Quality**: Good. True-color support, customizable themes. Relies on xterm.js under the hood (Electron). No GPU acceleration. Ligature support is limited.

**Key Features**:
- Cloud sync of connections, keys, snippets, and SFTP bookmarks
- Team sharing and access control (Business/Enterprise tiers)
- Port forwarding GUI
- Snippet library (reusable command templates)
- Autocomplete for hosts and commands
- Multi-platform including mobile (unique differentiator)

**Performance**: Electron overhead means ~200-300 MB RAM baseline. Startup is 3-5 seconds. SFTP transfers can be slow for large folder operations per user reports. SSH connection itself is fast.

**License**: Free tier (limited features, no SFTP, no sync). Pro ~$10/month. Team ~$8/user/month. Business tier available.

---

### 3. SecureCRT + SecureFX (Commercial)

**Architecture**: Native C++ application by VanDyke Software. SecureCRT (terminal) and SecureFX (file transfer) are separate applications that can be installed as an integrated package sharing a common session database.

**SFTP Integration**: When installed together, SecureCRT and SecureFX share sessions, host keys, and credentials. Key integration points:
- Open an SFTP tab within SecureCRT to run SFTP commands without re-authenticating
- Launch SecureFX from SecureCRT toolbar with one click (shared session)
- Drag files from Windows Explorer to the SFTP tab
- Tab completion for SFTP commands
- SecureFX provides a full dual-pane (local/remote) file browser

**Terminal Quality**: Excellent. One of the best terminal emulators available. Supports VT100/VT220/ANSI/xterm/Wyse, true-color (24-bit), Unicode, and extensive character set support. Used in enterprise environments for decades.

**Key Features**:
- Scripting support (Python, VBScript, JScript)
- Session management with folders and credential linking
- TFTP server built-in
- Button bar for custom commands
- Keyword highlighting
- Multi-session launch and tiled sessions
- Protocols: SSH2, SSH1, Telnet, Serial, SFTP, SCP, FTP, FTPS, HTTP, HTTPS (WebDAV, S3)

**Performance**: Lightweight native apps. ~80-120 MB combined memory. Fast startup (~1-2 seconds). Rock-solid stability -- the enterprise standard.

**License**: SecureCRT ~$99. SecureFX ~$69. Bundle ~$119. Per-user perpetual license + optional maintenance.

---

### 4. PuTTY + WinSCP (Windows)

**Architecture**: Two separate native Windows applications. PuTTY is a minimalist SSH terminal written in C. WinSCP is a graphical SFTP/SCP client written in C++ (Delphi origins). They can be linked but remain fundamentally separate programs.

**SFTP Integration**: Not truly integrated. WinSCP can:
- Import PuTTY saved sessions
- Launch PuTTY from within WinSCP (Commands > Open in PuTTY)
- Share site settings and host keys with PuTTY
- However, each maintains its own SSH connection (no shared session)

**Terminal Quality**: PuTTY's terminal is basic but functional. Supports VT102/xterm emulation, 256-color. No true-color (24-bit), no ligatures, no GPU acceleration. Has not evolved significantly in years.

**Key Features**:
- PuTTY: SSH, Telnet, Serial, raw TCP. Agent forwarding (Pageant). X11 forwarding.
- WinSCP: Dual-pane file manager (Commander style) or Explorer style. Built-in text editor. Synchronize directories. Scripting/automation (.NET assembly). Keepalive. Queue transfers.
- Both: Free, open source, extremely well-tested and trusted.

**Performance**: Minimal resource usage. PuTTY uses ~15-20 MB. WinSCP ~30-50 MB. Near-instant startup. The gold standard for lightweight.

**License**: Both free and open source (MIT-style / GPL).

---

### 5. Royal TSX (macOS)

**Architecture**: Native macOS application built with Swift/Objective-C. Plugin-based architecture where each protocol (SSH, RDP, VNC, SFTP) is handled by a separate plugin.

**SFTP Integration**: File Transfer Plugin provides SFTP/FTP/SCP browsing. Runs as a connection type within the same window as terminal sessions. However, SSH terminal and SFTP are separate connection entries -- no auto-sidebar like MobaXterm. Users must manually create both an SSH and an SFTP connection for the same host.

**Terminal Quality**: Uses an iTerm2-based terminal plugin. Good quality with true-color support and modern terminal features inherited from iTerm2.

**Key Features**:
- Plugin ecosystem (Terminal, File Transfer, RDP, VNC, web, etc.)
- Credential management with 1Password, LastPass, KeePass integration
- Shared credentials via folder inheritance
- Dynamic folder and credential linking
- Royal Server integration for enterprise gateway access
- Secure Gateway (SSH tunneling) support

**Performance**: ~120-180 MB. Startup ~2-3 seconds. Native macOS performance. Stable.

**License**: Free (limited to 10 connections). Individual license available. Site licenses for enterprise.

---

### 6. Remmina (Linux)

**Architecture**: Native Linux application written in C using GTK+3. Part of the GNOME ecosystem. Focused primarily on remote desktop protocols (RDP, VNC) with SSH/SFTP as secondary features.

**SFTP Integration**: SFTP is treated as a separate protocol/connection type. You create an SFTP connection (selecting "Secure File Transfer" protocol) that opens a file browser panel. It is not linked to SSH terminal sessions -- separate connection, separate authentication.

**Terminal Quality**: Basic SSH terminal. Functional but not a primary focus. Limited customization compared to dedicated terminal emulators.

**Key Features**:
- Multi-protocol: RDP, VNC, SSH, SFTP, SPICE, NX, XDMCP
- Connection profiles with groups
- Session recording
- Quick connect bar
- Tabbed interface for multiple connections
- Floating or embedded windows

**Performance**: Lightweight GTK application. ~60-100 MB. Fast startup on Linux. SFTP browsing is functional but basic.

**License**: Free and open source (GPL v2).

---

### 7. Electerm (Cross-platform, Electron)

**Architecture**: Electron-based (Chromium + Node.js). Open source. Uses xterm.js for terminal emulation. Available on Windows, macOS, and Linux.

**SFTP Integration**: One of the better Electron implementations. The SFTP file browser sits **side-by-side with the terminal** in a split view. When connected via SSH, you see the terminal on one side and the remote file browser on the other. Features include:
- Dual-pane file browser (local + remote side by side)
- Double-click to edit remote files
- Zmodem (rz/sz) and Trzsz support
- Drag and drop transfers

**Terminal Quality**: xterm.js based. True-color support, Unicode, customizable themes. No GPU acceleration. Performance can lag with heavy output.

**Key Features**:
- Bookmark sync to GitHub/Gitee gist (private)
- AI assistant integration (DeepSeek, OpenAI) for command suggestions
- Global hotkey to toggle visibility (quake-style dropdown)
- SSH tunnel support
- Serial port, Telnet, RDP, VNC, SPICE support
- Multi-language UI
- Global and per-session proxy settings

**Performance**: Electron overhead: ~200-350 MB RAM. Startup 3-5 seconds. The author updates frequently. Can encounter bugs. Not the most polished but actively maintained.

**License**: Free and open source (MIT).

---

### 8. WindTerm (Cross-platform, Free)

**Architecture**: Native C/C++ application. Cross-platform (Windows, macOS, Linux). Notably performant -- the developer focuses heavily on speed and memory efficiency. Uses custom rendering, not Electron or web-based.

**SFTP Integration**: Integrated SFTP client alongside the terminal. Also includes a **local file manager** for local file operations. Features:
- File browser for remote SFTP operations (upload, download, delete, rename)
- SCP support alongside SFTP
- Integrated within the same window as terminal sessions

**Terminal Quality**: Good. Supports SSH, Telnet, Raw TCP, Serial, Shell, and Tmux protocols. Custom terminal rendering with reasonable performance.

**Key Features**:
- Dynamic memory compression (reduces working memory by 20-90%)
- Auto-completion, auto-login (password, pubkey, keyboard-interactive, GSSAPI)
- SSH auto-execution
- X11 forwarding, local/remote/dynamic port forwarding
- Split-pane / tiled layout
- Session management
- Tmux integration
- Portable mode

**Performance**: Outstanding. ~70 MB memory usage. Fast startup. Dynamic memory compression is a unique feature that actively reduces RAM as sessions accumulate. The performance-per-feature ratio is best-in-class.

**Key Limitation**: Relatively new project. Community and documentation are still developing. UI is functional but not as polished as MobaXterm or SecureCRT. Source code is available but the project license has restrictions (not fully OSS).

**License**: Free for personal and commercial use. Source code available on GitHub but with a custom license (not OSI-approved open source). Apache 2.0 for some components.

---

### 9. Tabby (Cross-platform, Electron)

**Architecture**: Electron-based. Uses xterm.js for terminal rendering. Highly configurable via YAML config and a settings GUI. Plugin architecture for extensibility.

**SFTP Integration**: SFTP panel available per SSH session. When connected via SSH, users can open an SFTP file browser for that session. Features:
- Double-click to open files with local editor
- Drag and drop for upload/download
- Right-click context menu (delete, rename, chmod)
- Directory bookmarks
- Transfer queue management

**Terminal Quality**: Good. xterm.js with true-color, Unicode, customizable fonts and themes. Plugin system allows extending terminal capabilities. No GPU acceleration (Electron limitation).

**Key Features**:
- Highly customizable (themes, hotkeys, plugins)
- Profile-based session management (SSH, serial, local shell)
- SSH key authentication, port forwarding, jump hosts
- Plugin ecosystem (community plugins)
- Zmodem support
- Split panes and tab management
- Config sync (via cloud or file)

**Performance**: Heavy Electron footprint: ~250-400 MB RAM. Slow startup (4-8 seconds). Some users report font rendering issues on Windows. The trade-off for customizability is resource consumption.

**License**: Free and open source (MIT).

---

### 10. TUI-Based Solutions

There is **no existing TUI application that combines a terminal multiplexer with an integrated SFTP file browser** in the way MobaXterm does for GUI. This is a significant gap in the ecosystem. However, related tools exist:

**Midnight Commander (mc)**:
- Dual-pane TUI file manager with built-in SFTP/FTP/SMB support
- Can browse remote filesystems over SFTP as if local
- Has a built-in text editor and viewer
- Does NOT include a terminal emulator (runs inside one)
- Written in C, ~10-15 MB memory

**ranger / nnn / yazi / lf**:
- TUI file managers that can run over SSH
- ranger: Python-based, Vim keybindings, three-column preview layout
- nnn: C-based, ultra-lightweight (~5 MB), supports SFTP via FUSE
- yazi: Rust-based, modern, fast, async I/O, image preview
- lf: Go-based, ranger-inspired, minimal
- None of these integrate a terminal session + SFTP in a single TUI

**tmux / zellij**:
- Terminal multiplexers that can split panes
- One could run `mc` or `yazi` in one pane and a shell in another
- This is a manual composition, not an integrated experience
- No shared authentication, no drag-and-drop, no auto-sync

**Key Insight**: The TUI space has excellent file managers and excellent terminal multiplexers, but **nobody has combined them into a single integrated SSH + SFTP TUI tool**. This is exactly the niche sshm-term could fill.

---

## Architecture Comparison

| App | Language | Rendering | SFTP Library | Terminal Lib |
|---|---|---|---|---|
| MobaXterm | C++ | Win32 GDI | Custom (libssh2?) | Custom xterm emu |
| Termius | JS/TS | Electron/Chromium | Node SSH2 | xterm.js |
| SecureCRT | C++ | Native per-platform | Custom | Custom VT emu |
| PuTTY | C | Win32 GDI | N/A (WinSCP separate) | Custom |
| WinSCP | C++ (Delphi) | VCL/WinAPI | Custom (based on PuTTY code) | N/A |
| Royal TSX | Swift/ObjC | Cocoa | Plugin-based | iTerm2 plugin |
| Remmina | C | GTK+3 | libssh | VTE |
| Electerm | JS/TS | Electron/Chromium | ssh2 (npm) | xterm.js |
| WindTerm | C/C++ | Custom cross-platform | Custom | Custom |
| Tabby | JS/TS | Electron/Chromium | ssh2 (npm) | xterm.js |

---

## SFTP Integration Patterns

Three distinct patterns emerge across these applications:

### Pattern A: Auto-Sidebar (MobaXterm, WindTerm)
- SFTP browser **automatically appears** when SSH connects
- Shares the authenticated SSH session (no re-auth)
- Always visible in a consistent location (sidebar)
- Lowest friction, highest usability

### Pattern B: Separate Tab/Panel (Termius, Tabby, Electerm, SecureCRT)
- SFTP available as a tab or panel you explicitly open
- May or may not share the SSH session
- More flexibility in layout but requires user action to activate
- Medium friction

### Pattern C: Separate Application (PuTTY+WinSCP, SecureCRT+SecureFX, Remmina)
- Terminal and file transfer are distinct programs
- May share session database/credentials but not live connections
- Highest friction, most disjointed UX

**Conclusion**: Pattern A is clearly superior for workflow efficiency. It eliminates the cognitive overhead of "how do I transfer a file?" and makes SFTP feel like a natural extension of the terminal session.

---

## Lessons for sshm-term

### Must-Have Features

1. **Auto-sidebar SFTP browser** (Pattern A)
   - When an SSH session connects, the SFTP file browser must appear automatically in a side panel
   - Must reuse the same SSH connection (no re-authentication, no second TCP connection)
   - Must be dismissible/toggleable with a single keybind

2. **Shared authentication**
   - SFTP must piggyback on the SSH session's authentication -- zero additional prompts
   - This is technically achievable by opening an SFTP subsystem channel on the existing SSH connection

3. **Basic file operations**
   - Browse directories (tree or flat list)
   - Upload/download files (with progress indication)
   - Delete, rename, chmod
   - View file sizes and permissions
   - Navigate with keyboard (Vim-style keybindings)

4. **Edit-in-place**
   - Select a remote file, open it in `$EDITOR` locally, auto-upload on save
   - This is one of MobaXterm's most-loved features

5. **Responsive file listing**
   - Async directory loading -- never block the terminal while listing files
   - Handle large directories gracefully (lazy loading / pagination)

### Nice-to-Have Features

1. **Directory sync with terminal `cd`**
   - Optionally track the shell's working directory and update the SFTP browser to match
   - MobaXterm offers this; it creates a seamless feeling of "same filesystem"
   - Implementation: parse OSC 7 escape sequences or poll `pwd`

2. **Drag-and-drop equivalent for TUI**
   - Since TUI cannot do literal drag-and-drop, provide quick keybinds:
     - `y` to yank (copy path), `p` to paste/upload, etc.
   - Consider integration with system clipboard for paths

3. **Transfer queue / progress**
   - Show ongoing transfers in a status bar or dedicated panel
   - Support queuing multiple transfers

4. **Bookmarks / favorites**
   - Quick-access bookmarks for frequently used remote directories
   - Per-connection bookmarks stored in sshm's connection config

5. **File preview**
   - Syntax-highlighted preview of text files in a panel (like ranger)
   - File size, permissions, modification date in the listing

6. **Local file browser pane**
   - Dual-pane mode (local + remote) for transfer operations
   - Lower priority since most TUI users are comfortable with shell commands

### Differentiators: What Would Make sshm-term Unique

1. **First integrated SSH + SFTP TUI tool**
   - No existing TUI tool combines terminal multiplexing with an SFTP browser
   - This is a genuinely unoccupied niche
   - Users currently must compose tmux + mc/ranger manually

2. **Native Rust performance**
   - WindTerm proves that native code crushes Electron on memory (~70 MB vs ~300 MB)
   - A Rust TUI should target <50 MB for the full application
   - Instant startup (<1 second) vs Electron's 4-8 seconds

3. **Connection manager + terminal + SFTP in one TUI**
   - sshm already manages connections; adding terminal + SFTP creates a complete workflow
   - The "open connection -> land in terminal with SFTP sidebar" flow would be unique in TUI

4. **Cross-platform without Electron**
   - WindTerm is the closest competitor (native, cross-platform, free) but is not open source
   - A Rust-based MIT/Apache solution would attract the open-source community

### What Makes MobaXterm's UX Great (and How to Replicate in TUI)

| MobaXterm UX Element | TUI Equivalent |
|---|---|
| Auto-opening SFTP sidebar | Auto-show right panel on SSH connect (toggleable) |
| Drag-and-drop files | `y`/`p` keybinds, or `:upload`/`:download` commands |
| Double-click to edit | `e` key to edit-in-place (download to tmp, open in $EDITOR, upload on save) |
| Right-click context menu | `d` delete, `r` rename, `c` chmod, `m` mkdir -- single-key actions |
| Sidebar overlay mode | Panel can overlay terminal (like tmux popup) or split (like tmux pane) |
| Follow terminal `cd` | Parse OSC 7 or shell integration escape sequences |
| Session list in sidebar | Already exists in sshm's connection manager |
| Persistent home dir | Not applicable (TUI runs on user's machine) |

### Technical Recommendations

1. **SSH channel multiplexing**: Use a single SSH connection with multiple channels -- one for the PTY shell, one for the SFTP subsystem. The `russh` or `ssh2` crate should support this. This is how MobaXterm and SecureCRT achieve zero-friction SFTP.

2. **Async SFTP operations**: All file listing and transfer operations must be async (tokio-based). Never block the terminal rendering loop.

3. **Panel architecture**: Design the TUI layout as composable panels from the start:
   - Left: connection list (existing sshm)
   - Center: terminal emulator
   - Right: SFTP file browser (auto-shown on connect)
   - Bottom: status bar with transfer progress

4. **Progressive disclosure**: Start with the terminal full-width. Show SFTP panel only when the user wants it (keybind toggle) or auto-show on connect (configurable). Don't overwhelm the screen on small terminals.

5. **Vim-native keybindings**: The TUI advantage over GUI is keyboard speed. Every SFTP operation should be reachable in 1-2 keystrokes. Use hjkl navigation, `/` for search, `g`/`G` for top/bottom.

---

## References

- [MobaXterm Features](https://mobaxterm.mobatek.net/features.html)
- [Termius Reviews - G2](https://www.g2.com/products/termius/reviews)
- [SecureFX + SecureCRT Integration](https://www.vandyke.com/products/securefx/securecrt_securefx_integration.html)
- [SecureCRT SSH File Transfer](https://www.vandyke.com/products/securecrt/ssh_file_transfer.html)
- [WinSCP PuTTY Integration](https://winscp.net/eng/docs/integration_putty)
- [PuTTY vs WinSCP Comparison](https://techreviewadvisor.com/putty-vs-winscp/)
- [Royal TSX Features](https://royalapps.com/ts/mac/features)
- [Remmina Features](https://remmina.org/remmina-features/)
- [Electerm GitHub](https://github.com/electerm/electerm)
- [WindTerm GitHub](https://github.com/kingToolbox/WindTerm)
- [Tabby GitHub](https://github.com/Eugeny/tabby)
- [SSH Terminal Tools Comparison](https://armbasedsolutions.com/blog-detail/comparison-of-six-popular-ssh-terminal-tools)
- [Best SFTP Clients 2025](https://sftptogo.com/blog/best-sftp-clients-of-2025-secure-fast-file-transfers/)
- [TUI File Managers - It's FOSS](https://itsfoss.gitlab.io/post/14-best-command-line-file-managers-for-linux-in-2026/)
- [Tauri vs Electron Performance](https://www.gethopp.app/blog/tauri-vs-electron)
