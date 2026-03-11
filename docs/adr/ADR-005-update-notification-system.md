# ADR-005: Update Notification System

## Status
Proposed

## Context
sshm-rs is distributed via three channels: a curl install script, `cargo install`, and direct GitHub release downloads. There is currently no mechanism to inform users when a new version is available. The CI pipeline (`.github/workflows/release.yml`) builds multi-platform binaries on tag push and publishes GitHub Releases, but users must manually check for updates.

The project is at v0.1.0 and targets power users (SSH administrators) who expect CLI tools to be low-friction. The solution must not degrade TUI startup time, must respect offline/air-gapped environments, and must work regardless of installation method.

## Decision
Implement a **two-phase approach**: (1) a passive startup version check with TUI notification, and (2) a `sshm-rs update` self-update subcommand.

### Phase 1 -- Passive Version Check (priority: high)

**Mechanism:**
- On TUI startup, spawn a background thread (non-blocking) that calls the GitHub Releases API: `GET https://api.github.com/repos/bit5hift/sshm-rs/releases/latest`
- Compare the `tag_name` field (e.g., `v0.2.0`) against the compiled-in version from `env!("CARGO_PKG_VERSION")`
- Use semantic version comparison via the `semver` crate
- If a newer version exists, set a field on `App` (e.g., `update_available: Option<String>`) that the UI renders

**Rate limiting / caching:**
- Cache the check result in `~/.config/sshm-rs/update-check.json` with fields: `{ "last_checked": "2026-03-11T...", "latest_version": "0.2.0", "current_at_check": "0.1.0" }`
- Skip the API call if the last check was less than **24 hours** ago (read cache, compare timestamp)
- If the API call fails (offline, rate-limited), silently ignore -- never block or error

**TUI notification placement:**
- Show in the **title area** (the subtitle line beneath "SSH Connection Manager"), appending a styled span: `" -- Update available: v0.2.0"` in the accent/warning color
- Alternatively, use the existing **toast system** (`show_toast`) on first appearance, but persist the indicator in the title for the session
- The notification is purely informational; no modal, no blocking

**User opt-out:**
- Respect an environment variable `SSHM_NO_UPDATE_CHECK=1` to disable
- Also add a config key in sshm-rs config dir if a config file is introduced later

**HTTP client:**
- Use `ureq` (synchronous, minimal, no async runtime needed). The background thread makes one blocking call; this avoids pulling in `reqwest` and its heavy dependency tree. `ureq` is ~200KB compiled, pure Rust, and supports TLS via `rustls` (no OpenSSL dependency for this feature)
- Set a 5-second timeout on the request
- Set `User-Agent: sshm-rs/{version}` to comply with GitHub API requirements

### Phase 2 -- Self-Update Subcommand (priority: medium)

**CLI surface:**
```
sshm-rs update              # check + download + replace binary
sshm-rs update --check      # check only, print result
```

**Mechanism:**
- Query the same GitHub Releases API endpoint
- Identify the correct asset by matching the target triple (compiled in via `env!("TARGET")` set in `build.rs`)
- Download the `.tar.gz` / `.zip` asset to a temp directory
- Extract the `sshm-rs` and `sshm-term` binaries
- Replace the running binary using the atomic rename pattern: write to a temp file in the same directory, then `rename()` over the current binary (this is atomic on POSIX)
- On Windows, rename the current binary to `.old` first, then move the new one in place

**Crate recommendation:**
- Do NOT use the `self_update` crate. It is semi-maintained, pulls in `reqwest` + `hyper` + `tokio`, and adds ~5MB to the binary. Its API is also opinionated in ways that conflict with the dual-binary setup (sshm-rs + sshm-term)
- Instead, implement self-update manually using `ureq` (already added for Phase 1) + `flate2`/`tar` (for .tar.gz extraction) + `zip` (for Windows .zip extraction). This keeps the dependency footprint small and gives full control over the dual-binary update

**Security:**
- For v0.1.x: verify download via HTTPS (TLS certificate validation via rustls). GitHub Releases over HTTPS provides transport-level integrity
- Future improvement (post-v1.0): add SHA256 checksums to release assets (generate in CI, upload as `checksums.txt`), verify after download
- Future improvement (post-v1.0): GPG/minisign signature verification. Not worth the complexity at this stage

**cargo install users:**
- When `sshm-rs update` detects it was installed via cargo (heuristic: binary path contains `.cargo/bin`), print a message: `"Installed via cargo. Run: cargo install sshm-rs --force"` instead of self-updating. Overwriting a cargo-managed binary would confuse `cargo install --list`

### Phase 3 -- Version Flag Enhancement (priority: low)

**CLI surface:**
```
sshm-rs --version            # existing: prints "sshm-rs 0.1.0"
sshm-rs version              # new subcommand: prints version + checks for update
```

This is trivially built on top of Phase 1 infrastructure.

### Not recommended at this stage

- **Homebrew/package manager integration**: The project has no Homebrew formula, no AUR package, no snap/flatpak. Creating and maintaining these is significant ongoing effort. Revisit when the project has a stable release cadence and user demand.
- **Auto-update (download + replace without user action)**: Security anti-pattern for CLI tools. Users should explicitly opt into binary replacement.
- **Background daemon / cron-based checking**: Over-engineered for a TUI tool that users run on-demand.

## Alternatives Considered

- **`self_update` crate**: Pros: ready-made, handles GitHub Releases natively. Cons: pulls in reqwest/hyper/tokio (~5MB binary size increase), semi-maintained (last release 2023), does not handle dual-binary updates, opinionated API. Rejected due to dependency weight and lack of dual-binary support.

- **`update-informer` crate**: Pros: lightweight, designed for exactly this use case (check + notify). Cons: limited to checking only (no self-update), uses `ureq` internally anyway, adds an abstraction layer over a simple API call. Marginal value over a direct implementation (~50 lines of code).

- **Check via crates.io API instead of GitHub**: Pros: simpler URL. Cons: only works for `cargo install` users, misses curl/binary users who are the primary audience.

- **Embed update check in the event loop (poll periodically)**: Pros: no startup cost. Cons: unnecessary complexity; a single background thread on startup is simpler and sufficient.

## Consequences

**Positive:**
- Users are informed of updates without manual checking
- Self-update works for the primary install methods (curl script, GitHub release download)
- Minimal dependency footprint (~ureq + flate2/tar)
- No startup latency impact (background thread + 24h cache)
- Graceful degradation (offline, rate-limited, air-gapped environments all work fine)

**Negative:**
- Adds ~3 new dependencies (ureq, flate2, tar) to the build
- Self-update binary replacement is platform-specific code that needs testing on Linux, macOS, and Windows
- The 24-hour cache file adds a small amount of state to manage
- cargo-install users get a degraded experience (notification but no self-update)

**Implementation effort:**
- Phase 1 (version check + notification): ~2-3 days
- Phase 2 (self-update subcommand): ~3-4 days
- Phase 3 (version subcommand): ~0.5 day

## Files to Create/Modify

### New files
- `src/update/mod.rs` -- update checker logic (GitHub API call, version comparison, cache read/write)
- `src/update/self_update.rs` -- binary download, extraction, and replacement logic (Phase 2)

### Modified files
- `Cargo.toml` -- add dependencies: `ureq`, `semver`, `flate2`, `tar` (and `zip` for Windows)
- `src/main.rs` -- add `mod update;`
- `src/cli/mod.rs` -- add `Update` variant to `Commands` enum, wire up handler
- `src/ui/app.rs` -- add `update_available: Option<String>` field to `App`, spawn check thread in `App::new()`
- `src/ui/views/list.rs` -- render update notification in title area (modify `draw_title()`)
- `build.rs` (new) -- set `TARGET` env var for self-update target triple detection
- `.github/workflows/release.yml` -- (Phase 2, future) add checksums.txt generation step

## Date
2026-03-11
