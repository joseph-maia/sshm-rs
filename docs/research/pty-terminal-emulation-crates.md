# Research: Rust Crates for PTY Management and Terminal Emulation in Ratatui

**Date:** 2026-03-05
**Context:** Embedding a live SSH terminal (left panel) alongside an SFTP file browser (right panel) inside a ratatui TUI app (`sshm-term`).

---

## Table of Contents

1. [PTY Crates](#1-pty-crates)
2. [Terminal Emulation / VT Parsing Crates](#2-terminal-emulation--vt-parsing-crates)
3. [Existing Examples of PTY-in-TUI](#3-existing-examples-of-pty-in-tui)
4. [SSH2/Russh Channel Approach vs PTY + System SSH](#4-ssh2russh-channel-approach-vs-pty--system-ssh)
5. [Performance Analysis](#5-performance-analysis)
6. [Recommendation](#6-recommendation)

---

## 1. PTY Crates

### 1.1 portable-pty (wezterm)

| Field | Value |
|---|---|
| **Version** | 0.9.0 (Feb 2025) |
| **Downloads** | ~912K/month |
| **Dependents** | 201 crates |
| **Platforms** | Windows (ConPTY), Linux, macOS |
| **Async** | No (synchronous API, blocking reads) |
| **License** | MIT |

**API:** Trait-based design (`PtySystem` trait) allows runtime selection between PTY backends. On Windows, supports both ConPTY and legacy WinPTY. Provides `MasterPty` and `SlavePty` abstractions. Spawn processes with `CommandBuilder`.

**Strengths:**
- Battle-tested in WezTerm (a full terminal emulator)
- Cross-platform with runtime backend selection
- Large ecosystem adoption (201 dependents)
- Handles Windows ConPTY complexities

**Weaknesses:**
- Synchronous only -- requires wrapping in `spawn_blocking` for async contexts
- Heavy dependency tree (pulls in parts of wezterm ecosystem)
- Some dependencies flagged as "obsolete" or "outdated"
- No native tokio integration

**Windows notes:** ConPTY support works on Windows 10 1809+. Can spawn `ssh.exe` as a child process and capture output. VT100 escape sequences in output must be parsed by the caller.

---

### 1.2 rust-pty

| Field | Value |
|---|---|
| **Version** | 0.1.0 (Jan 2026) |
| **Downloads** | Moderate (ranked #865 in Async category) |
| **Platforms** | Windows (ConPTY), Linux, macOS, BSD |
| **Async** | Yes, first-class Tokio support |
| **License** | MIT/Apache-2.0 |

**API:** Unified async interface using `tokio::io::AsyncRead` and `AsyncWrite`. Uses `rustix` for Unix PTY allocation and `windows-sys` for ConPTY on Windows. Includes window size management and signal handling.

**Strengths:**
- Native async/Tokio -- no `spawn_blocking` needed
- Cross-platform (Unix + Windows ConPTY)
- Modern dependency stack (rustix, windows-sys, tokio 1.49)
- Clean, focused API

**Weaknesses:**
- Very new (v0.1.0, unstable)
- Limited ecosystem adoption
- Less battle-tested than portable-pty
- Sparse documentation

---

### 1.3 pseudoterminal

| Field | Value |
|---|---|
| **Version** | 0.2.1 (Dec 2024) |
| **Platforms** | Windows (ConPTY), Unix |
| **Async** | Incomplete ("not implemented yet") |
| **License** | MIT |

**API:** Extends `std::process::Command` via `CommandExt` trait with `spawn_terminal()`. Returns handle with separate `terminal_in`/`terminal_out` streams.

**Strengths:**
- Ergonomic API extending std Command
- Cross-platform

**Weaknesses:**
- Async support not implemented
- Early stage (29 commits, 16 stars)
- Last commit Dec 2024 -- potentially stale

---

### 1.4 pty-process

| Field | Value |
|---|---|
| **Version** | 0.5.3 (Jul 2025) |
| **Downloads** | ~70K/month |
| **Dependents** | 24 crates |
| **Platforms** | Unix only (Linux, macOS) |
| **Async** | Yes (optional `async` feature, Tokio) |
| **License** | MIT |

**API:** Wraps `tokio::process::Command` or `std::process::Command`. Allocates PTY and spawns processes attached to it. PTY implements `AsyncRead`/`AsyncWrite`.

**Strengths:**
- Clean async Tokio integration
- Well-maintained (regular releases through 2025)
- Good download numbers

**Weaknesses:**
- **Unix only -- no Windows support**
- Not suitable for our cross-platform requirement

---

### 1.5 nix::pty

Unix-only low-level bindings to POSIX PTY functions (`openpty`, `forkpty`). Not a standalone crate but part of the `nix` crate. No Windows support. Too low-level for direct use -- better to use a higher-level wrapper.

### 1.6 conpty / winpty

Windows-only solutions. `winpty-rs` (v1.0.4) wraps both WinPTY and ConPTY backends. Useful only as a Windows-specific implementation detail, not as a cross-platform solution.

### PTY Crate Summary

| Crate | Cross-platform | Async | Maturity | Downloads/mo |
|---|---|---|---|---|
| **portable-pty** | Yes | No | High | 912K |
| **rust-pty** | Yes | Yes (Tokio) | Low | Low |
| **pseudoterminal** | Yes | No (WIP) | Low | Low |
| **pty-process** | Unix only | Yes | Medium | 70K |

---

## 2. Terminal Emulation / VT Parsing Crates

### 2.1 vte (alacritty)

| Field | Value |
|---|---|
| **Version** | 0.15.0 (Feb 2025) |
| **Downloads** | ~4.25M/month |
| **Dependents** | 1,486 crates |
| **License** | Apache-2.0/MIT |

**What it does:** Low-level state machine parser implementing Paul Williams' ANSI parser. Parses bytes into actions (Print, Execute, CSI dispatch, OSC dispatch, etc.) via the `Perform` trait. Does NOT maintain a screen buffer.

**Strengths:**
- Extremely fast (powers Alacritty, the fastest terminal emulator)
- Proven at scale (4M+ downloads/month)
- UTF-8 aware
- Used by both Alacritty and Zellij

**Weaknesses:**
- Low-level: you must implement `Perform` and build your own screen buffer
- No 2D grid, no color state, no cursor tracking out of the box
- Significant implementation effort to go from vte to a renderable screen

**Best for:** Building a custom terminal emulator from scratch where you need maximum control and performance.

---

### 2.2 vt100

| Field | Value |
|---|---|
| **Version** | 0.16.2 (Jul 2025) |
| **Downloads** | ~1M/month |
| **Dependents** | 182 crates |
| **License** | MIT |

**What it does:** High-level terminal emulator library that parses VT100/xterm byte streams into an in-memory 2D character grid with colors, cursor position, and terminal state. Uses `vte` internally for parsing.

**API:** `Parser::new(rows, cols)` creates a virtual terminal. Feed bytes with `parser.process(bytes)`. Query the screen with `parser.screen()` which returns a `Screen` with:
- `cell(row, col)` -- character + foreground/background colors + attributes
- `cursor_position()` -- (row, col)
- `title()` -- window title set by escape sequences
- `alternate_screen()` -- whether alternate screen is active
- `scrollback()` -- scrollback buffer content

**Strengths:**
- Complete screen buffer with colors and attributes
- Handles alternate screen, cursor movement, scrolling, line wrapping
- Used by `tui-term` (the ratatui pseudoterminal widget)
- Used by `ratatui-testlib` for testing
- Good performance (~1M downloads/month)
- 256-color and true color support

**Weaknesses:**
- Mouse event parsing not directly exposed (handled internally)
- No direct ratatui integration (need to convert Screen to ratatui widgets)
- Performance may be lower than raw vte for extreme throughput

**Best for:** Our use case. Provides exactly the "bytes in, 2D grid out" abstraction we need.

---

### 2.3 termwiz (wezterm)

| Field | Value |
|---|---|
| **Version** | 0.23.3 (Mar 2025) |
| **Downloads** | ~1M/month |
| **Dependents** | 154 crates |
| **License** | MIT |

**What it does:** Full terminal toolkit from WezTerm. Includes escape sequence parser, surface model (2D grid with True Color, Hyperlinks, Sixel/iTerm graphics), widget system, line editor, and terminal I/O abstraction.

**Strengths:**
- Most complete terminal emulation library in the Rust ecosystem
- True Color, Hyperlinks, Sixel graphics support
- Both Unix and Windows 10+ support
- Can encode and decode escape sequences

**Weaknesses:**
- Very heavy: ~447K SLoC, 15-22MB dependencies
- "Subject to fairly wild sweeping changes" (per maintainer)
- Its own widget system competes with ratatui (not designed for ratatui integration)
- Overkill for embedding inside ratatui -- we only need parsing, not the full TUI stack
- Tight coupling with wezterm internals

**Best for:** Building a standalone terminal emulator. NOT recommended for embedding inside ratatui.

---

### 2.4 ansi-to-tui (ratatui official)

| Field | Value |
|---|---|
| **Version** | 8.0.1 (Jan 2026) |
| **Downloads** | ~339K/month |
| **Dependents** | 154 crates |
| **License** | MIT |

**What it does:** Converts ANSI-escaped byte strings into `ratatui::text::Text` with proper `Style` (colors, modifiers). Supports 3/4-bit, 8-bit (256), and 24-bit True Color.

**Strengths:**
- Official ratatui ecosystem crate
- Direct conversion to ratatui types
- Fast (uses SIMD via simdutf8, nom parser)
- Handles malformed sequences gracefully

**Weaknesses:**
- Only handles SGR (color/style) sequences
- Does NOT handle cursor movement, screen clearing, alternate screen, scrolling
- Cannot build a full terminal emulator -- only colorizes static text
- No 2D grid or screen buffer

**Best for:** Displaying colored command output in a ratatui Paragraph widget. NOT sufficient for a full terminal emulator.

---

### 2.5 strip-ansi-escapes

Simple utility to strip ANSI escape sequences from text. Not useful for terminal emulation.

### VT Parser Summary

| Crate | Screen Buffer | Colors | Ratatui Integration | Complexity | Use Case |
|---|---|---|---|---|---|
| **vte** | No | No | None | Build everything | Custom emulator |
| **vt100** | Yes (2D grid) | Yes (true color) | Via tui-term | Feed bytes, get grid | Embedded terminal |
| **termwiz** | Yes (Surface) | Yes (true color+) | Poor (own widget system) | Full toolkit | Standalone emulator |
| **ansi-to-tui** | No | Yes (true color) | Native | Simple conversion | Colored text display |

---

## 3. Existing Examples of PTY-in-TUI

### 3.1 tui-term (THE key crate for our use case)

| Field | Value |
|---|---|
| **Version** | 0.3.2 (Mar 2026) |
| **Repo** | https://github.com/a-kenji/tui-term |
| **Stars** | 207 |
| **Status** | Active development, work in progress |
| **Backend** | vt100 (only supported backend) |
| **License** | MIT |

**What it does:** A `PseudoTerminal` widget for ratatui that renders a vt100 `Screen` as a ratatui widget. This is exactly the bridge between PTY output and ratatui rendering.

**Architecture:**
1. Spawn process via PTY (portable-pty or similar)
2. Read PTY output bytes
3. Feed bytes into `vt100::Parser`
4. Pass `parser.screen()` to `tui_term::widget::PseudoTerminal`
5. Render in ratatui `Frame::render_widget()`

**Examples provided:**
- Channel-based communication pattern
- RWLock-based shared state pattern
- Controller-based lifecycle management (experimental)

**Keyboard input:** Raw keycodes can be passed to the PTY process. The widget intercepts configurable key combinations (like Ctrl-b in tmux).

**Strengths:**
- Purpose-built for our exact use case
- Active development with recent release (Mar 2026)
- Born from official ratatui discussion (#540)
- Uses vt100 for correct terminal emulation

**Weaknesses:**
- Still "work in progress"
- Limited to vt100 backend
- Experimental controller feature
- May have undiscovered edge cases

---

### 3.2 Ratterm

**Repo:** https://github.com/hastur-dev/ratterm

A split-terminal TUI with PTY terminal + code editor, built with ratatui + crossterm. Uses **portable-pty** for PTY management. Supports Windows (ConPTY) and Unix. Demonstrates the exact architecture we need:
- Multiple terminal tabs
- Grid layouts
- Scrollback history
- 256-color and true color
- ANSI/VT100 escape sequence parsing

**Key takeaway:** Ratterm proves the portable-pty + vt100 + tui-term + ratatui stack works in production.

---

### 3.3 Zellij Architecture

Zellij is a terminal multiplexer in Rust that embeds multiple terminal emulators. Its architecture:

- **PTY Thread:** Manages PTY file descriptors in a `HashMap<u32, RawFd>`. Uses `async_std` for non-blocking reads. Spawns shell processes and creates PTY slaves.
- **VT Parser:** Uses the `vte` crate (low-level). Implements custom `Perform` trait to build a `Grid` data structure.
- **Screen Thread:** Maintains `TerminalPane` objects, each with a `Grid`. Renders grid content back into ANSI sequences for the host terminal.
- **Communication:** Bounded crossbeam channels with backpressure (50-message buffer) between PTY and Screen threads.

**Key takeaway:** Zellij uses vte (low-level) + custom Grid, which is much more work than vt100 (which does this for you). Zellij's approach makes sense for a multiplexer but is overkill for our embedded terminal.

---

### 3.4 WezTerm Architecture

WezTerm uses its own termwiz crate for everything. Tightly coupled, not designed for embedding in other TUI frameworks.

---

## 4. SSH2/Russh Channel Approach vs PTY + System SSH

### Approach A: PTY + System SSH

```
[sshm-term] --spawn--> [PTY] --exec--> [ssh user@host]
                  |
            read/write PTY fd
                  |
            [vt100 parser] --> [tui-term widget] --> [ratatui]
```

**How it works:**
1. Use portable-pty to create a PTY pair
2. Spawn `ssh user@host` as a child process attached to the PTY
3. Read PTY master fd for output, write to it for input
4. Feed output bytes through vt100 parser
5. Render with tui-term widget

**Pros:**
- Simple architecture
- ssh handles all protocol details (auth, key exchange, keepalive)
- User's SSH config (~/.ssh/config) is respected automatically
- Agent forwarding, ProxyJump, etc. work out of the box
- Proven approach (every terminal emulator does this)

**Cons:**
- Requires `ssh` binary on the system (OpenSSH on Windows must be installed)
- Two separate connections needed: one for terminal, one for SFTP
- Hard to share authentication state between terminal and SFTP
- Windows ConPTY can have quirks with some SSH implementations
- Process management overhead (monitoring child process lifecycle)

---

### Approach B: Russh Channel (Pure Rust SSH)

```
[sshm-term] --russh--> [SSH Session] --channel_session()--> [Shell Channel]
                              |                                    |
                              |                              request_pty()
                              |                              shell()
                              |                                    |
                              |                          read/write channel
                              |                                    |
                              +--channel_session()--> [SFTP Subsystem]
                                                           |
                                                     russh-sftp
```

**How it works:**
1. Use `russh` to establish SSH connection (async, Tokio-native)
2. Open a channel with `channel_session()`
3. Request PTY: `channel.request_pty(false, "xterm-256color", cols, rows, ...)`
4. Start shell: `channel.request_shell(false)`
5. Read channel output (raw bytes with escape sequences)
6. Feed through vt100 parser, render with tui-term
7. Open second channel for SFTP via `russh-sftp`

**Pros:**
- Single SSH connection for both terminal AND SFTP
- No external `ssh` binary dependency
- Pure Rust, full control over SSH protocol
- Async-native (Tokio)
- Can share authentication between terminal and SFTP channels
- Better for Windows (no ConPTY needed for the SSH connection itself)
- Can implement connection pooling, reconnection logic
- Programmatic access to SSH features (port forwarding, etc.)

**Cons:**
- More complex implementation
- Must handle SSH authentication ourselves (password, key, agent)
- User's ~/.ssh/config is NOT automatically respected (must parse or ignore)
- russh does not expose stdout/stderr separation easily (needs async-ssh2-russh wrapper)
- SSH agent forwarding requires additional implementation
- ProxyJump/bastion host requires manual implementation

---

### Approach C: Hybrid (Recommended)

Use **russh** for the SSH connection but handle it similarly to Approach B:

```
[sshm-term]
    |
    +-- russh Session (single TCP connection)
          |
          +-- Channel 1: PTY shell (terminal panel)
          |     |
          |     +-- vt100 parser --> tui-term --> ratatui (left panel)
          |
          +-- Channel 2: SFTP subsystem
                |
                +-- russh-sftp --> file browser (right panel)
```

**Why hybrid is best for sshm-term:**
- We already need SSH connection management (sshm-rs is an SSH connection manager)
- Single connection = single authentication = better UX
- russh is async/Tokio (matches ratatui's async event loop patterns)
- russh-sftp is the companion crate for SFTP (same connection)
- No dependency on system `ssh` binary
- Works identically on Windows and Unix (no PTY needed on the local side)

---

### Russh Details

| Field | Value |
|---|---|
| **Version** | 0.57.1 (Feb 2026) |
| **Downloads** | ~44K/week |
| **Dependents** | 145 crates |
| **Async** | Yes (Tokio-native) |
| **License** | Apache-2.0 |
| **Notable users** | Warpgate, Yazi, Motor OS |

**Key capabilities:**
- SSH transport, authentication (password, publickey, keyboard-interactive)
- Channel multiplexing (multiple channels over one TCP connection)
- PTY request on channels
- Shell and exec request
- SFTP subsystem via `russh-sftp`
- AsyncRead/AsyncWrite on channels

**Comparison with ssh2:**

| Feature | ssh2 (libssh2) | russh |
|---|---|---|
| Async | No (blocking, C FFI) | Yes (Tokio-native) |
| Dependencies | libssh2 C library | Pure Rust |
| Maintenance | Sporadic | Active (Feb 2026) |
| SFTP | ssh2::Sftp (blocking) | russh-sftp (async) |
| Performance | Good (C impl) | Good (native Rust) |
| Build complexity | Requires C toolchain | Pure cargo build |
| Windows | Works but FFI issues | Native, clean |

**Verdict:** russh is clearly superior to ssh2 for our use case. It is async, pure Rust, actively maintained, and has a companion SFTP crate.

---

## 5. Performance Analysis

### VT Parsing Performance

**Zellij benchmarks** (from their blog post on optimization):
- Parsing 2M lines through VT parser: went from 19.2s to 5.3s after optimization
- Key optimizations: bounded channel backpressure, preallocated vectors, cached char widths, selective rendering (only changed lines)
- Final performance matched tmux (5.6s for same workload)

**vte (alacritty) performance:**
- Alacritty is consistently the fastest terminal emulator in vtebench benchmarks
- The vte parser is the core of this performance
- vt100 uses vte internally, so inherits its parsing speed

**Throughput considerations for our use case:**
- SSH terminal output is network-bound, not parser-bound
- Even a slow SSH connection maxes out at ~10-100 Mbps
- vt100 parser can handle orders of magnitude more than network throughput
- Bottleneck will be network latency, not parsing

### Ratatui Rendering Performance

- Ratatui achieves sub-millisecond rendering with double-buffered diff algorithm
- Only changed cells are written to the terminal
- 80x24 terminal = 1,920 cells. Even full redraw is trivial.
- At 30fps (typical TUI refresh rate), rendering overhead is negligible
- The wgpu backend can do 800+ fps for complex layouts

**Practical assessment:** Performance is NOT a concern for our use case. An 80x24 terminal embedded in ratatui, fed by a network SSH connection, will never hit parser or renderer limits.

---

## 6. Recommendation

### Recommended Stack

```
russh (SSH connection)
  +-- Channel 1: PTY shell --> vt100::Parser --> tui-term::PseudoTerminal --> ratatui
  +-- Channel 2: SFTP subsystem --> russh-sftp --> SFTP file browser widget --> ratatui
```

### Component Selection

| Component | Choice | Rationale |
|---|---|---|
| **SSH library** | **russh** (0.57.1) | Async/Tokio, pure Rust, active, SFTP companion crate, single connection for both terminal + SFTP |
| **VT parser** | **vt100** (0.16.2) | Complete screen buffer, used by tui-term, handles all escape sequences, 1M downloads/month |
| **Ratatui widget** | **tui-term** (0.3.2) | Purpose-built PseudoTerminal widget, active development, bridges vt100 to ratatui |
| **SFTP** | **russh-sftp** | Companion to russh, async, same SSH session |
| **Local PTY** | **Not needed** | russh provides the PTY channel directly -- no local PTY required |

### Why NOT the alternatives

| Rejected | Reason |
|---|---|
| portable-pty + system ssh | Requires ssh binary, two connections, no async, Windows ConPTY quirks |
| ssh2 (libssh2) | Blocking C FFI, poor Windows build experience, less maintained |
| termwiz | Overkill, own widget system conflicts with ratatui, unstable API |
| vte (raw) | Too low-level, requires building screen buffer from scratch |
| ansi-to-tui | Only handles SGR, no cursor/scrolling/alternate screen |
| pty-process | Unix only |

### Architecture Overview

```
                    +-------------------+
                    |    sshm-term      |
                    |   (ratatui app)   |
                    +-------------------+
                    |   Event Loop      |
                    | (crossterm events |
                    |  + SSH channel    |
                    |  async select)    |
                    +--------+----------+
                             |
              +--------------+--------------+
              |                             |
    +---------v----------+     +------------v-----------+
    | Left Panel:        |     | Right Panel:           |
    | Terminal            |     | SFTP Browser           |
    |                    |     |                        |
    | tui-term widget    |     | Custom ratatui widget  |
    | (PseudoTerminal)   |     | (directory listing)    |
    +---------+----------+     +------------+-----------+
              |                             |
    +---------v----------+     +------------v-----------+
    | vt100::Parser      |     | russh-sftp             |
    | (screen buffer)    |     | (async SFTP ops)       |
    +---------+----------+     +------------+-----------+
              |                             |
              +--------------+--------------+
                             |
                    +--------v----------+
                    | russh::Session    |
                    | (single SSH conn) |
                    +--------+----------+
                             |
                    +--------v----------+
                    | Channel 1: shell  |
                    | Channel 2: sftp   |
                    +-------------------+
                             |
                         [Network]
                             |
                    +-------------------+
                    |  Remote SSH Host  |
                    +-------------------+
```

### Data Flow: Terminal Input

```
Keyboard event (crossterm)
  --> sshm-term event handler
    --> if terminal panel focused:
      --> convert crossterm::KeyEvent to bytes
        --> write bytes to russh shell channel
          --> remote shell receives input
            --> remote shell produces output
              --> russh channel read (async)
                --> vt100::Parser::process(bytes)
                  --> tui-term renders updated screen
```

### Key Implementation Notes

1. **No local PTY needed.** The russh channel with `request_pty()` creates a remote PTY. The bytes we read from the channel already contain VT100 escape sequences.

2. **Window resize.** When the ratatui terminal panel resizes, call `channel.window_change(cols, rows, 0, 0)` to inform the remote PTY.

3. **Key translation.** Crossterm `KeyEvent` must be converted to the byte sequences that a terminal would send (e.g., arrow keys become `\x1b[A`, `\x1b[B`, etc.). The `terminput` or similar crate may help, or implement manually.

4. **Concurrency.** Use `tokio::select!` to multiplex between crossterm events (stdin), russh channel reads (SSH output), and SFTP operations.

5. **Connection lifecycle.** Handle SSH disconnection gracefully. Show connection status in the UI. Implement reconnection logic.

### Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| tui-term is "work in progress" | Medium | Pin version, contribute fixes upstream, fallback to manual vt100-to-ratatui conversion |
| russh PTY channel edge cases | Medium | Test with various remote shells (bash, zsh, fish), test with ncurses apps (vim, htop) |
| Key translation gaps | Low | Build comprehensive keymap, test with interactive programs |
| Performance with large output | Low | Network is the bottleneck, not parsing. Add flow control if needed |
| Windows compatibility | Low | russh is pure Rust, no ConPTY needed. Only crossterm for local terminal |

---

## Sources

### PTY Crates
- [portable-pty on lib.rs](https://lib.rs/crates/portable-pty)
- [portable-pty docs](https://docs.rs/portable-pty)
- [rust-pty on lib.rs](https://lib.rs/crates/rust-pty)
- [pseudoterminal on GitHub](https://github.com/michaelvanstraten/pseudoterminal)
- [pty-process on lib.rs](https://lib.rs/crates/pty-process)

### VT Parsing Crates
- [vt100 on lib.rs](https://lib.rs/crates/vt100)
- [vt100 on GitHub](https://github.com/doy/vt100-rust)
- [vte on GitHub](https://github.com/alacritty/vte)
- [termwiz on lib.rs](https://lib.rs/crates/termwiz)
- [ansi-to-tui on GitHub](https://github.com/ratatui/ansi-to-tui)

### PTY-in-TUI Examples
- [tui-term on GitHub](https://github.com/a-kenji/tui-term)
- [Ratterm on GitHub](https://github.com/hastur-dev/ratterm)
- [Ratatui Pseudoterminal Widget Discussion #540](https://github.com/ratatui/ratatui/discussions/540)
- [Zellij DeepWiki Architecture](https://deepwiki.com/zellij-org/zellij)
- [Zellij Performance Blog Post](https://poor.dev/blog/performance/)

### SSH Libraries
- [russh on GitHub](https://github.com/Eugeny/russh)
- [russh on lib.rs](https://lib.rs/crates/russh)
- [ssh2 Channel docs](https://docs.rs/ssh2/latest/ssh2/struct.Channel.html)
- [async-ssh2-russh on GitHub](https://github.com/hydro-project/async-ssh2-russh)

### Performance
- [Ratatui Rendering Discussion #579](https://github.com/ratatui/ratatui/discussions/579)
- [Ratatui CPU Usage Issue #1338](https://github.com/ratatui/ratatui/issues/1338)
- [Alacritty vtebench](https://github.com/alacritty/vtebench)
