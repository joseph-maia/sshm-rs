# ADR-006: Merge sshm-term into sshm-rs as a mono-binary

## Status
Proposed

## Context

`sshm-rs` currently ships two binaries: the main TUI (`sshm-rs`) and a companion SSH terminal + SFTP browser (`sshm-term`). When a user presses Enter on a host in the TUI, `sshm-rs` exits the TUI, then spawns `sshm-term` as a child process via `std::process::Command` (see `connectivity/mod.rs:611-623`).

This architecture causes a critical adoption blocker: corporate IT security (SI) policies block the spawning of a second binary from within an approved application. The second binary is treated as an untrusted child process.

Additionally, the current `sshm-term` binary uses `#[path = "..."]` directives to re-include source files from the main crate (`config/mod.rs`, `theme.rs`, `ui/styles.rs`), which is fragile and causes duplicate compilation.

## Decision

Merge `sshm-term` into the `sshm-rs` crate as a library module (`src/term/`) and expose it as the `term` subcommand. The main binary remains `sshm-rs`. The `sshm-term` binary target is removed entirely.

### Target Architecture

```
sshm-rs (single binary)
  |
  +-- No args          -> TUI (current behavior)
  +-- term <args>      -> SSH terminal + SFTP browser (current sshm-term)
  +-- <host>           -> Direct SSH connect (current behavior)
  +-- search|export|.. -> Other subcommands (unchanged)
```

### New Module Structure

```
src/
  main.rs              (unchanged)
  cli/mod.rs           (add Term subcommand, add dispatch)
  connectivity/mod.rs  (replace launch_sshm_term with in-process call)
  term/                (NEW - moved from src/bin/sshm-term/)
    mod.rs             (public entry point: pub fn run_term(...) and pub async fn async_main(...))
    app.rs             (moved as-is)
    event.rs           (moved as-is)
    sftp.rs            (moved as-is)
    snippets.rs        (moved as-is)
    ssh.rs             (moved as-is)
    terminal.rs        (moved as-is)
    transfer.rs        (moved as-is)
    ui.rs              (moved as-is, fix styles import)
```

### Detailed Migration Plan

#### Step 1: Create `src/term/` module (move files)

Move all files from `src/bin/sshm-term/` to `src/term/`:

| Source | Destination | Changes needed |
|--------|-------------|---------------|
| `src/bin/sshm-term/app.rs` | `src/term/app.rs` | None (crate:: refs now resolve to main crate) |
| `src/bin/sshm-term/event.rs` | `src/term/event.rs` | None |
| `src/bin/sshm-term/sftp.rs` | `src/term/sftp.rs` | Change `crate::ssh::` to `crate::term::ssh::` |
| `src/bin/sshm-term/snippets.rs` | `src/term/snippets.rs` | None |
| `src/bin/sshm-term/ssh.rs` | `src/term/ssh.rs` | None |
| `src/bin/sshm-term/terminal.rs` | `src/term/terminal.rs` | None |
| `src/bin/sshm-term/transfer.rs` | `src/term/transfer.rs` | Change `crate::event::` to `crate::term::event::` |
| `src/bin/sshm-term/ui.rs` | `src/term/ui.rs` | Change `crate::term_styles` to `crate::term::styles`, fix other crate:: refs |
| `src/bin/sshm-term/main.rs` | `src/term/mod.rs` | Major refactor (see below) |

#### Step 2: Create `src/term/mod.rs` (refactored from main.rs)

The new `mod.rs` replaces the old `main.rs`. Key changes:

1. Declare submodules: `pub mod app; mod event; pub mod sftp; ...`
2. Remove `#[path = "..."]` hacks -- the module now lives inside the main crate and can use `crate::config`, `crate::theme`, `crate::ui::styles` directly
3. Remove `mod config; mod theme; mod term_styles;` -- these are now accessible via `crate::`
4. Remove `fn main()` and `Args` struct (CLI parsing moves to `cli/mod.rs`)
5. Create a `styles` submodule that re-exports `crate::ui::styles` (to minimize changes in `term/ui.rs`)
6. Export two public functions:

```rust
/// Called from CLI: `sshm-rs term user@host -p 22`
pub fn run_term(host: String, port: u16, user: Option<String>,
                key: Option<PathBuf>, password: bool) -> Result<()>

/// Called in-process from TUI via connectivity module
pub fn run_term_for_host(host: String, port: u16, user: String,
                         auth: ssh::Auth, password: Option<String>) -> Result<()>
```

Both functions handle the env-password safety dance, create a tokio runtime, set up the terminal, and call the existing `run()` async function.

#### Step 3: Update `src/term/*.rs` internal references

All `crate::` references in the moved files currently resolve within the `sshm-term` binary crate. After the move, `crate::` resolves to the `sshm-rs` library crate. Update pattern:

| Old reference | New reference |
|---------------|---------------|
| `crate::sftp::` | `crate::term::sftp::` |
| `crate::ssh::` | `crate::term::ssh::` |
| `crate::event::` | `crate::term::event::` |
| `crate::transfer::` | `crate::term::transfer::` |
| `crate::snippets::` | `crate::term::snippets::` |
| `crate::terminal::` | `crate::term::terminal::` |
| `crate::app::` | `crate::term::app::` |
| `crate::term_styles` | `crate::term::styles` |
| `crate::config` | `crate::config` (unchanged -- now resolves correctly) |
| `crate::theme` | `crate::theme` (unchanged -- now resolves correctly) |

**Simplification**: Use `use super::` or local `use` statements within `src/term/` to keep changes minimal. Add to the top of `src/term/mod.rs`:

```rust
// Re-export styles from main crate for term UI
pub(crate) mod styles {
    pub use crate::ui::styles::*;
}
```

Then in each file under `src/term/`, replace `crate::` with `super::` for sibling module references. For example in `src/term/app.rs`:

```rust
// Before (in src/bin/sshm-term/app.rs):
use crate::{sftp::SftpBrowser, ssh::{Auth, SshConnection}, ...};

// After (in src/term/app.rs):
use super::{sftp::SftpBrowser, ssh::{Auth, SshConnection}, ...};
```

#### Step 4: Update `src/cli/mod.rs`

Add `Term` variant to the `Commands` enum:

```rust
#[derive(Subcommand)]
pub enum Commands {
    // ... existing variants ...

    /// Open SSH terminal with integrated SFTP browser
    Term {
        /// Remote host (user@host or host)
        host: String,
        /// Remote port
        #[arg(short, long, default_value_t = 22)]
        port: u16,
        /// SSH user (overrides user@host syntax)
        #[arg(short, long)]
        user: Option<String>,
        /// Path to private key file
        #[arg(short = 'i', long)]
        key: Option<std::path::PathBuf>,
        /// Prompt for password authentication
        #[arg(long)]
        password: bool,
    },
}
```

Add dispatch in `run()`:

```rust
Some(Commands::Term { host, port, user, key, password }) => {
    crate::term::run_term(host, port, user, key, password)
}
```

#### Step 5: Update `src/connectivity/mod.rs`

Replace `launch_sshm_term()` (lines 546-667). The new function no longer spawns a process; it calls `crate::term::run_term_for_host()` directly:

```rust
pub fn launch_sshm_term(host: &str, config_file: Option<&str>) -> Result<()> {
    let config_path = match config_file {
        Some(p) => std::path::PathBuf::from(p),
        None => crate::config::default_ssh_config_path()?,
    };
    let hosts = crate::config::parse_ssh_config(&config_path)?;

    let (hostname, user, port, identity, password) = if let Some(host_info) =
        hosts.iter().find(|h| h.name == host)
    {
        let hostname = if host_info.hostname.is_empty() {
            host_info.name.clone()
        } else {
            host_info.hostname.clone()
        };
        let user = if host_info.user.is_empty() {
            whoami::username()
        } else {
            host_info.user.clone()
        };
        let port: u16 = host_info.port.parse().unwrap_or(22);
        let identity = if host_info.identity.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(&host_info.identity))
        };
        let password = crate::credentials::get_password(host);
        (hostname, user, port, identity, password)
    } else {
        (host.to_string(), whoami::username(), 22, None, None)
    };

    let auth = if let Some(key_path) = identity {
        crate::term::ssh::Auth::PublicKey(key_path)
    } else if password.is_some() {
        crate::term::ssh::Auth::Password(password.clone().unwrap())
    } else {
        crate::term::ssh::Auth::AutoDetect
    };

    crate::term::run_term_for_host(
        format!("{}@{}", user, hostname),
        port,
        user,
        auth,
        password,
    )
}
```

This eliminates all `std::process::Command` usage, binary path lookups, and the `which_exists` helper (if unused elsewhere).

#### Step 6: Register the module in `src/main.rs`

Add `mod term;` to the module declarations:

```rust
mod cli;
mod config;
mod connectivity;
mod credentials;
mod favorites;
mod groups;
mod history;
mod snippets;
mod term;       // <-- NEW
mod theme;
mod ui;
mod update;
```

#### Step 7: Update `Cargo.toml`

Remove the `sshm-term` binary target:

```toml
# REMOVE these lines:
# [[bin]]
# name = "sshm-term"
# path = "src/bin/sshm-term/main.rs"
```

Update the dependency comment:

```toml
# SSH terminal + SFTP browser (integrated term subcommand)
```

#### Step 8: Delete `src/bin/sshm-term/`

Remove the entire directory after the move is verified to compile.

#### Step 9: Update CI (`release.yml`)

In the Package steps, remove references to `sshm-term`:

**Linux/macOS step** (lines 66-73):
```yaml
- name: Package (Linux / macOS)
  if: matrix.os != 'windows-latest'
  shell: bash
  run: |
    VERSION=${GITHUB_REF_NAME}
    TARGET=${{ matrix.target }}
    STAGING="sshm-rs-${VERSION}-${TARGET}"
    mkdir -p "${STAGING}"
    cp "target/${TARGET}/release/sshm-rs" "${STAGING}/"
    tar -czf "${STAGING}.tar.gz" -C "${STAGING}" sshm-rs
    echo "ASSET=${STAGING}.tar.gz" >> "$GITHUB_ENV"
```

**Windows step** (lines 75-86):
```yaml
- name: Package (Windows)
  if: matrix.os == 'windows-latest'
  shell: pwsh
  run: |
    $version  = "${{ github.ref_name }}"
    $target   = "${{ matrix.target }}"
    $staging  = "sshm-rs-${version}-${target}"
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Copy-Item "target\$target\release\sshm-rs.exe" "$staging\"
    Compress-Archive -Path "$staging\*" -DestinationPath "$staging.zip"
    "ASSET=$staging.zip" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
```

#### Step 10: Update install scripts

**`install.sh`** (line 149): Remove `sshm-term` from chmod:
```bash
chmod +x "${bin_dir}/sshm-rs"
```

Remove comment mentioning `sshm-term` from line 2.

**`install.ps1`** (line 51): Remove `sshm-term.exe` from extracted file filter:
```powershell
$extractedFiles = Get-ChildItem -Path $tmpDir -Recurse -Include "sshm-rs.exe"
```

Remove `sshm-term` from comment on line 1.

#### Step 11: Update `src/ui/mod.rs` (TUI exit handler)

The exit handler at line 57 currently sets `connect_host` to `"__sshm_term__:<host>"` and quits the TUI, then calls `connectivity::launch_sshm_term()` after terminal restore. This flow remains identical -- only the implementation of `launch_sshm_term` changes (Step 5). No changes needed in `ui/mod.rs` or `ui/event.rs`.

## Alternatives considered

### Alternative A: Symlink/hardlink trick (`sshm-rs` detects `argv[0]` = `sshm-term`)
- Pros: Zero CLI changes, single binary on disk
- Cons: Fragile (depends on filesystem support, breaks on Windows, confusing UX), does not solve the process-spawning problem since the TUI still needs to exec the same binary

### Alternative B: Feature-flag the term code behind `--features term`
- Pros: Smaller binary for users who do not need the terminal
- Cons: Adds build complexity, does not align with the "single binary" requirement, CI must still build with the feature enabled

### Alternative C: Embed sshm-term as a separate workspace crate, linked statically
- Pros: Clean crate boundaries, separate compilation units
- Cons: Significantly more complex Cargo.toml setup (workspace), still requires careful public API design, over-engineered for the current codebase size (~9 files)

### Alternative D: Keep two binaries, bundle sshm-term as a resource extracted at runtime
- Pros: No code restructuring
- Cons: Does not solve the SI blocking problem (still spawns a separate process), adds extraction complexity, increases binary size with embedded binary

## Consequences

### Positive
- Single binary eliminates the SI/corporate IT blocker entirely
- Removes the fragile `#[path = "..."]` source inclusion hack
- `config`, `theme`, and `styles` are shared naturally via `crate::`
- Simpler install scripts, CI pipelines, and user experience
- No more "sshm-term binary not found" error paths
- In-process call is faster than process spawn (no exec overhead)

### Negative
- The `sshm-rs` binary size increases (all term dependencies are always linked)
- `term` module introduces `tokio` runtime creation in a currently sync-only binary -- but this is isolated to `term::run_term()` and does not affect the main TUI path
- Internal module references require a one-time bulk rename (`crate::X` -> `super::X` or `crate::term::X`)
- Risk of name collision between `src/snippets/` (main crate) and `src/term/snippets.rs` (term module) -- mitigated by the module namespace

### Migration risk assessment
- **Low risk**: File moves are mechanical; the Rust compiler will catch every broken reference
- **Medium risk**: The `SSHM_PASSWORD` env-var handling currently uses `unsafe { remove_var }` before tokio starts. In the mono-binary, when called from the TUI path, the tokio runtime does not exist yet either (the TUI is sync). When called from CLI (`sshm-rs term ...`), `main()` is still single-threaded at parse time. The safety invariant is preserved in both paths.
- **Low risk**: The `term` subcommand creates its own tokio runtime via `Runtime::new()?.block_on()`. This is safe as long as it is not called from within an existing tokio context (which it is not -- the main TUI is synchronous).

## Date
2026-03-23
