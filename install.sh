#!/usr/bin/env bash
# Installs the latest goog CLI release into ~/.local/bin (or $GOOG_INSTALL_DIR).
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/SainyTK/goog-cli/main/install.sh | sh
set -euo pipefail

REPO="SainyTK/goog-cli"
INSTALL_DIR="${GOOG_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
  echo "error: $1" >&2
  exit 1
}

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

  if [ -n "${GOOG_VERSION:-}" ]; then
    tag="v$GOOG_VERSION"
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
