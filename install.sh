#!/usr/bin/env sh
set -eu

REPO="SainyTK/goog-cli"
BIN_NAME="goog"
INSTALL_DIR="${GOOG_INSTALL_DIR:-/usr/local/bin}"
VERSION=""

usage() {
  cat <<'USAGE'
Install goog from a Canonical GitHub Release.

Usage:
  install.sh [--version vX.Y.Z] [--install-dir PATH]

Options:
  --version      Install a specific Canonical Release tag.
  --install-dir  Install directory for the goog binary. Defaults to /usr/local/bin.
  -h, --help     Show this help.

Environment:
  GOOG_INSTALL_DIR  Install directory used when --install-dir is not provided.
USAGE
}

fail() {
  printf 'goog installer: %s\n' "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      [ "$#" -ge 2 ] || fail "--version requires a value"
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      [ "$#" -ge 2 ] || fail "--install-dir requires a value"
      INSTALL_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown option: $1"
      ;;
  esac
done

need_cmd curl
need_cmd install
need_cmd tar

case "$(uname -s)" in
  Darwin) os="apple-darwin" ;;
  Linux) os="unknown-linux-gnu" ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    fail "Windows binary releases are not supported yet. Install from source with: cargo install --git https://github.com/SainyTK/goog-cli goog"
    ;;
  *)
    fail "unsupported operating system: $(uname -s)"
    ;;
esac

case "$(uname -m)" in
  arm64|aarch64) arch="aarch64" ;;
  x86_64|amd64) arch="x86_64" ;;
  *)
    fail "unsupported CPU architecture: $(uname -m)"
    ;;
esac

target="${arch}-${os}"

case "$target" in
  aarch64-apple-darwin|x86_64-apple-darwin|x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu) ;;
  *)
    fail "unsupported platform target: ${target}"
    ;;
esac

if [ -z "$VERSION" ]; then
  latest_url="https://api.github.com/repos/${REPO}/releases/latest"
  VERSION="$(curl -fsSL "$latest_url" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  [ -n "$VERSION" ] || fail "could not resolve the latest Canonical Release"
fi

case "$VERSION" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) fail "--version must look like vX.Y.Z" ;;
esac

asset="${BIN_NAME}-${VERSION}-${target}.tar.gz"
base_url="https://github.com/${REPO}/releases/download/${VERSION}"
tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT INT TERM

archive_path="${tmp_dir}/${asset}"
checksum_path="${archive_path}.sha256"

curl -fL "${base_url}/${asset}" -o "$archive_path"
curl -fL "${base_url}/${asset}.sha256" -o "$checksum_path"

expected="$(awk '{print $1}' "$checksum_path")"
[ -n "$expected" ] || fail "checksum file is empty: ${asset}.sha256"

if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$archive_path" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$archive_path" | awk '{print $1}')"
else
  fail "required command not found: sha256sum or shasum"
fi

[ "$actual" = "$expected" ] || fail "checksum verification failed for ${asset}"

tar -xzf "$archive_path" -C "$tmp_dir"
[ -x "${tmp_dir}/${BIN_NAME}" ] || fail "release archive did not contain an executable ${BIN_NAME} binary"

mkdir -p "$INSTALL_DIR"
install -m 0755 "${tmp_dir}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

printf 'goog %s installed to %s/%s\n' "$VERSION" "$INSTALL_DIR" "$BIN_NAME"
