#!/usr/bin/env bash
# Installs a Canonical Release of the goog CLI into ~/.local/bin (or $GOOG_INSTALL_DIR).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh -s -- --version v0.1.0
set -euo pipefail

REPO="SainyTK/goog-cli"
INSTALL_DIR="${GOOG_INSTALL_DIR:-$HOME/.local/bin}"
REQUESTED_VERSION="${GOOG_VERSION:-}"

fail() {
  echo "error: $1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      [ $# -ge 2 ] || fail "--version requires a value"
      REQUESTED_VERSION="$2"
      shift 2
      ;;
    --version=*)
      REQUESTED_VERSION="${1#--version=}"
      shift
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) platform="apple-darwin" ;;
    Linux) platform="unknown-linux-gnu" ;;
    *) fail "unsupported OS: $os" ;;
  esac

  case "$arch" in
    arm64 | aarch64) cpu="aarch64" ;;
    x86_64 | amd64) cpu="x86_64" ;;
    *) fail "unsupported architecture: $arch" ;;
  esac

  echo "${cpu}-${platform}"
}

main() {
  command -v curl >/dev/null 2>&1 || fail "curl is required"
  command -v tar >/dev/null 2>&1 || fail "tar is required"

  target="$(detect_target)"

  if [ -n "$REQUESTED_VERSION" ]; then
    case "$REQUESTED_VERSION" in
      v*) tag="$REQUESTED_VERSION" ;;
      *) tag="v$REQUESTED_VERSION" ;;
    esac
  else
    api_url="https://api.github.com/repos/$REPO/releases/latest"
    tag="$(curl -fsSL "$api_url" | grep -m1 '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')"
    [ -n "$tag" ] || fail "could not determine latest release tag"
  fi
  version="${tag#v}"

  archive="goog-${version}-${target}.tar.gz"
  download_url="https://github.com/$REPO/releases/download/${tag}/${archive}"

  workdir="$(mktemp -d)"
  trap 'rm -rf "$workdir"' EXIT

  echo "Downloading goog ${version} for ${target}..."
  curl -fsSL "$download_url" -o "$workdir/$archive" \
    || fail "failed to download $download_url (no release for your platform?)"
  curl -fsSL "$download_url.sha256" -o "$workdir/$archive.sha256" \
    || fail "failed to download checksum for $archive"

  echo "Verifying checksum..."
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$workdir" && sha256sum -c "$archive.sha256") || fail "checksum verification failed for $archive"
  else
    (cd "$workdir" && shasum -a 256 -c "$archive.sha256") || fail "checksum verification failed for $archive"
  fi

  tar -xzf "$workdir/$archive" -C "$workdir"

  mkdir -p "$INSTALL_DIR"
  install -m 755 "$workdir/goog" "$INSTALL_DIR/goog"

  echo "Installed goog to $INSTALL_DIR/goog"

  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      echo ""
      echo "Add $INSTALL_DIR to your PATH to use goog directly, e.g.:"
      echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc"
      ;;
  esac

  "$INSTALL_DIR/goog" --version
}

main "$@"
