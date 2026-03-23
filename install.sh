#!/usr/bin/env bash
# install.sh — Install sshm-rs from the latest GitHub release.
# Usage: curl -fsSL https://raw.githubusercontent.com/bit5hift/sshm-rs/master/install.sh | bash

set -euo pipefail

REPO="bit5hift/sshm-rs"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

info()  { printf '\033[0;34m[info]\033[0m  %s\n' "$*"; }
ok()    { printf '\033[0;32m[ok]\033[0m    %s\n' "$*"; }
err()   { printf '\033[0;31m[error]\033[0m %s\n' "$*" >&2; }
die()   { err "$*"; exit 1; }

need() {
  command -v "$1" >/dev/null 2>&1 || die "Required tool not found: $1"
}

# ---------------------------------------------------------------------------
# Detect OS
# ---------------------------------------------------------------------------

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux" ;;
    Darwin*) echo "macos" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *) die "Unsupported operating system: $(uname -s)" ;;
  esac
}

# ---------------------------------------------------------------------------
# Detect architecture
# ---------------------------------------------------------------------------

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) die "Unsupported architecture: $(uname -m)" ;;
  esac
}

# ---------------------------------------------------------------------------
# Map (os, arch) to release target triple
# ---------------------------------------------------------------------------

resolve_target() {
  local os="$1"
  local arch="$2"
  case "${os}-${arch}" in
    linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
    linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
    macos-x86_64)   echo "x86_64-apple-darwin" ;;
    macos-aarch64)  echo "aarch64-apple-darwin" ;;
    windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
    *) die "No release available for ${os}/${arch}" ;;
  esac
}

# ---------------------------------------------------------------------------
# Install directory
# ---------------------------------------------------------------------------

install_dir() {
  local os="$1"
  if [ "${os}" = "windows" ]; then
    echo "${USERPROFILE}/.local/bin"
  else
    echo "${HOME}/.local/bin"
  fi
}

# ---------------------------------------------------------------------------
# PATH suggestion helper
# ---------------------------------------------------------------------------

suggest_path() {
  local bin_dir="$1"
  local shell_rc=""

  if [ -n "${ZSH_VERSION:-}" ] || [ "$(basename "${SHELL:-}")" = "zsh" ]; then
    shell_rc="${HOME}/.zshrc"
  else
    shell_rc="${HOME}/.bashrc"
  fi

  if [ -f "${shell_rc}" ] && grep -q "${bin_dir}" "${shell_rc}" 2>/dev/null; then
    return
  fi

  info "${bin_dir} is not in your PATH."
  info "Add it permanently by running:"
  printf '\n    echo '"'"'export PATH="%s:$PATH"'"'"' >> %s\n\n' "${bin_dir}" "${shell_rc}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
  need curl
  need tar

  local os arch target bin_dir version archive_name download_url

  os="$(detect_os)"
  arch="$(detect_arch)"
  target="$(resolve_target "${os}" "${arch}")"

  info "Detected platform: ${os}/${arch} → ${target}"

  info "Fetching latest release information from GitHub..."
  local release_json
  release_json="$(curl -fsSL "${API_URL}")"

  version="$(printf '%s' "${release_json}" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
  [ -n "${version}" ] || die "Could not determine latest release version."
  info "Latest release: ${version}"

  if [ "${os}" = "windows" ]; then
    archive_name="sshm-rs-${version}-${target}.zip"
  else
    archive_name="sshm-rs-${version}-${target}.tar.gz"
  fi

  download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"
  info "Downloading ${archive_name}..."

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "${tmp_dir}"' EXIT

  curl -fsSL --output "${tmp_dir}/${archive_name}" "${download_url}" \
    || die "Download failed: ${download_url}"

  bin_dir="$(install_dir "${os}")"
  mkdir -p "${bin_dir}"

  info "Extracting to ${bin_dir}..."
  if [ "${os}" = "windows" ]; then
    need unzip
    unzip -o -j "${tmp_dir}/${archive_name}" -d "${bin_dir}"
  else
    tar -xzf "${tmp_dir}/${archive_name}" -C "${bin_dir}"
    chmod +x "${bin_dir}/sshm-rs"
  fi

  ok "sshm-rs ${version} installed to ${bin_dir}/"

  # PATH check
  case ":${PATH}:" in
    *":${bin_dir}:"*) ;;
    *) suggest_path "${bin_dir}" ;;
  esac

  ok "Done. Run 'sshm-rs' to get started."
}

main "$@"
