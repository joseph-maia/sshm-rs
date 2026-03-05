# sshm-rs

A beautiful TUI SSH connection manager written in Rust.

Inspired by [sshm (Go)](https://github.com/Gu1llaum-3/sshm) by Gu1llaum-3.

## Installation

### Quick install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/bit5hift/sshm-rs/master/install.sh | bash
```

### From source

```bash
cargo install --git https://github.com/bit5hift/sshm-rs
```

## Features

- Interactive TUI to browse, search, and connect to SSH hosts
- SSH config parsing (`~/.ssh/config`) with Include support
- Real-time connectivity status (async ping)
- Add / Edit / Delete SSH hosts
- Connection history tracking
- Port forwarding setup (Local, Remote, Dynamic)
- Cross-platform (Windows, Linux, macOS)

## Usage

```bash
sshm-rs              # Interactive TUI
sshm-rs <host>       # Direct SSH connection
sshm-rs add          # Add a new host
sshm-rs edit <host>  # Edit a host
sshm-rs search <q>   # Search hosts
```

## Keybindings

| Key | Action |
|-----|--------|
| `Enter` | Connect to selected host |
| `/` | Search |
| `a` | Add new host |
| `e` | Edit selected host |
| `d` | Delete selected host |
| `i` | Show host info |
| `s` | Toggle sort mode |
| `?` | Help |
| `q` | Quit |

## License

MIT
