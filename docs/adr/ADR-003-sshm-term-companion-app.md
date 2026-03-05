# ADR-003: sshm-term Companion App Architecture

## Status

Accepted

## Date

2026-03-05

## Context

sshm-rs is a TUI SSH connection manager that handles connection cataloging, search, and launching SSH sessions via the system's `ssh` binary. Users have expressed interest in a more integrated experience -- a split-panel view combining an SSH terminal (left) with an SFTP file browser (right), similar to MobaXterm but implemented as a pure Rust TUI application.

Building this within sshm-rs itself would conflate two distinct responsibilities: connection management and interactive terminal/file-transfer sessions. The terminal companion has fundamentally different dependencies (async SSH library, terminal emulation, SFTP protocol) and a different runtime model (long-lived interactive session vs. short-lived launcher).

Research identified the following ecosystem components:

- **russh 0.57.1**: Pure-Rust async SSH library built on Tokio, supporting channel multiplexing over a single TCP connection.
- **russh-sftp 2.1.1**: SFTP subsystem companion to russh, also async.
- **vt100 0.16.2**: VT100/xterm terminal emulator providing a complete 2D screen buffer with color and attribute tracking.
- **tui-term 0.3.2**: Ratatui widget that renders a `vt100::Screen` as a `PseudoTerminal` widget.
- **tui-tree-widget 0.24.0**: Ratatui tree view widget suitable for hierarchical file browser display.

No existing TUI tool combines an embedded SSH terminal with an SFTP file browser, making this an unoccupied niche.

## Decision

We make the following architectural decisions:

### 1. Separate binary crate in the Cargo workspace

`sshm-term` will be a new binary crate added to the sshm-rs Cargo workspace. The workspace layout becomes:

```
sshm-rs/
  Cargo.toml          # workspace root
  sshm-rs/            # existing connection manager (moved into sub-crate)
  sshm-term/          # new companion: terminal + SFTP browser
  sshm-core/          # (future) shared types: connection config, credential resolution
```

The two binaries are independently installable (`cargo install sshm-rs`, `cargo install sshm-term`) but share workspace-level dependency versions and, eventually, a `sshm-core` library crate for connection configuration and credential resolution.

### 2. russh over ssh2 for SSH connectivity

`sshm-term` will use **russh** (pure Rust, async, Tokio-native) instead of the libssh2-based `ssh2` crate currently used by sshm-rs for connectivity checks.

Rationale -- russh provides channel multiplexing: a single TCP connection can carry both a shell channel (for the terminal) and an SFTP subsystem channel (for the file browser) simultaneously. This is the killer feature. The `ssh2` crate binds to the C libssh2 library, requires system-level build dependencies, and does not expose clean async channel multiplexing.

### 3. Tokio async runtime

russh requires Tokio. The sshm-term binary will use `#[tokio::main]` as its entry point. The ratatui event loop will run in a dedicated task, communicating with SSH I/O tasks via `tokio::sync::mpsc` channels.

### 4. Terminal emulation: vt100 + tui-term (no local PTY)

The remote shell's output will be parsed by `vt100` into a 2D screen buffer. The `tui-term` crate's `PseudoTerminal` widget renders this buffer as a ratatui widget. No local PTY (pseudo-terminal) is needed -- russh requests a remote PTY directly from the SSH server, and all terminal processing happens in-process via the vt100 state machine.

Data flow:

```
[SSH server] --bytes--> [russh channel] --bytes--> [vt100::Parser] --> [Screen buffer]
[ratatui render loop] --reads--> [Screen buffer] --renders--> [PseudoTerminal widget]
[User keypress] --bytes--> [russh channel] --bytes--> [SSH server]
```

### 5. SFTP browser: tui-tree-widget

The SFTP file browser will use `tui-tree-widget` to render remote directory listings as a collapsible tree. Directory contents are fetched lazily on expand via the russh-sftp channel. Common operations (download, upload, delete, rename, mkdir) will be exposed through contextual key bindings.

### 6. Single SSH connection, two channels

One TCP connection to the remote host carries:

- **Channel 1**: Interactive shell session with PTY allocation (terminal panel).
- **Channel 2**: SFTP subsystem (file browser panel).

This avoids double authentication, reduces latency, and is the standard SSH channel multiplexing model defined in RFC 4254.

### 7. Panel architecture: toggleable split view

The default layout is a horizontal split: terminal (left, ~70% width) and SFTP browser (right, ~30% width). The SFTP panel can be toggled on/off (full-screen terminal mode). A keybinding (e.g., `F2` or `Ctrl-B`) switches focus between panels. The focused panel receives keyboard input; the unfocused panel remains visible but inert.

## Alternatives Considered

### SSH library: ssh2 (libssh2 binding)

- **Pros**: Already used by sshm-rs for connectivity checks; mature C library; well-documented.
- **Cons**: Requires C toolchain for compilation; synchronous API (async wrapper is unofficial and incomplete); no clean channel multiplexing -- would require two separate TCP connections for shell and SFTP. Cross-compilation is harder due to native dependency.
- **Verdict**: Rejected. Channel multiplexing and pure-Rust async are decisive advantages for russh.

### Terminal emulation: portable-pty (local PTY)

- **Pros**: Full terminal emulation delegated to the OS; battle-tested approach.
- **Cons**: Unnecessary complexity -- russh already provides the remote PTY. A local PTY would mean piping SSH channel bytes through an OS-level pseudo-terminal just to get them back as bytes, adding latency and platform-specific code (Windows ConPTY vs. Unix PTY). The vt100 crate provides the same screen-buffer abstraction without OS involvement.
- **Verdict**: Rejected. Local PTY adds a layer with no benefit when the SSH library handles remote PTY directly.

### Embedding inside sshm-rs as a feature flag

- **Pros**: Single binary; simpler distribution; shared code without a library crate.
- **Cons**: Pulls heavy async dependencies (Tokio, russh, vt100) into the connection manager even when unused; bloats compile time and binary size; conflates two distinct user workflows (managing connections vs. using them interactively); feature-flag complexity grows over time.
- **Verdict**: Rejected. Separation of concerns is cleaner as distinct crates in a shared workspace.

### Using thrussh instead of russh

- **Pros**: Original library that russh forked from.
- **Cons**: thrussh is unmaintained (last release 2021); russh is the actively maintained successor with more features and fixes.
- **Verdict**: Rejected. russh is the living fork.

### Web-based approach (e.g., xterm.js in a webview)

- **Pros**: Mature terminal emulation; rich ecosystem.
- **Cons**: Completely breaks the TUI philosophy of the project; adds a webview dependency; not a Rust-native solution.
- **Verdict**: Rejected. Out of scope for a terminal-native tool.

## Consequences

### Positive

- **Single connection, dual function**: Users get terminal and file browser over one SSH connection with one authentication prompt.
- **Pure Rust stack**: No C dependencies in sshm-term (unlike ssh2). Easier cross-compilation, no system library requirements.
- **Clean separation**: sshm-rs remains a lightweight launcher; sshm-term is the power-user interactive tool. They can evolve independently.
- **Unoccupied niche**: No existing TUI tool offers this combination, giving sshm-term a unique value proposition.
- **Workspace synergy**: Shared dependency versions, CI pipeline, and (future) shared configuration types via sshm-core.

### Negative

- **Two SSH libraries in the workspace**: sshm-rs currently uses `ssh2` for connectivity checks while sshm-term will use `russh`. This divergence is acceptable short-term but should be resolved by migrating sshm-rs to russh in a future ADR.
- **Tokio dependency**: Adds a heavyweight async runtime to the workspace. sshm-rs itself does not need Tokio today, but the workspace-level dependency is isolated to sshm-term.
- **vt100 fidelity**: The vt100 crate may not handle every terminal escape sequence perfectly (e.g., some 256-color/truecolor edge cases). Users with exotic shell configurations may notice rendering differences compared to a native terminal emulator.
- **New dependency surface**: Six new crates (russh, russh-sftp, russh-keys, vt100, tui-term, tui-tree-widget) increase the supply-chain attack surface and maintenance burden.

### Neutral

- **Cargo workspace migration**: The current single-crate project must be restructured into a workspace. This is a one-time migration with well-understood mechanics but touches the root `Cargo.toml` and directory layout.
- **Shared configuration**: Eventually, connection definitions should be shared between sshm-rs and sshm-term via a `sshm-core` crate. This is deferred to a future ADR but is a known follow-up.
