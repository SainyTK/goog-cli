#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 5 ]; then
  echo "usage: $0 ASSET TAG COMMIT RELEASE_CHANNEL TARGET" >&2
  exit 2
fi

asset="$1"
tag="$2"
commit="$3"
release_channel="$4"
target="$5"
version="${tag#v}"
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

# Git Bash's GNU tar cannot read zip, and no single zip tool is present in
# every environment that runs this script, so take the first one available.
# The extractor receives an absolute path because that is the form MSYS
# argument conversion handles reliably.
extract_zip() {
  archive="$1"
  destination="$2"
  case "$archive" in
    /*) ;;
    *) archive="${PWD}/${archive}" ;;
  esac

  if command -v unzip >/dev/null 2>&1; then
    unzip -q "$archive" -d "$destination"
  elif command -v 7z >/dev/null 2>&1; then
    (cd "$destination" && 7z x -bso0 -bsp0 "$archive")
  elif [ -x /c/Windows/System32/tar.exe ]; then
    # Windows ships bsdtar, which reads zip.
    /c/Windows/System32/tar.exe -C "$destination" -xf "$archive"
  else
    echo "no zip extractor found; install unzip or 7z" >&2
    exit 1
  fi
}

case "$asset" in
  *.zip)
    extract_zip "$asset" "$staging"
    binary="$staging/goog.exe"
    ;;
  *)
    tar -C "$staging" -xzf "$asset"
    binary="$staging/goog"
    ;;
esac
test -x "$binary"

actual_version="$("$binary" --version)"
if [ "$actual_version" != "goog $version" ]; then
  echo "packaged binary reported '$actual_version', expected 'goog $version'" >&2
  exit 1
fi

"$binary" version --json > "$staging/version.json"

# The Windows runner image is not guaranteed to expose python3, and the
# Microsoft Store python3.exe alias is on PATH but exits non-zero, so probe by
# running the interpreter rather than by looking it up.
python_bin="python3"
if ! "$python_bin" -c "" >/dev/null 2>&1; then
  python_bin="python"
fi

"$python_bin" - "$staging/version.json" "$version" "$tag" "$commit" "$release_channel" "$target" <<'PY'
import json
import sys

path, version, tag, commit, release_channel, target = sys.argv[1:]
with open(path, encoding="utf-8") as version_file:
    actual = json.load(version_file)

expected = {
    "semanticVersion": version,
    "displayVersion": version,
    "gitCommit": commit,
    "dirty": False,
    "distanceFromTag": 0,
    "sourceTag": tag,
    "releaseChannel": release_channel,
    "target": target,
}
if actual != expected:
    raise SystemExit(
        "packaged binary provenance mismatch:\n"
        f"expected: {json.dumps(expected, sort_keys=True)}\n"
        f"actual:   {json.dumps(actual, sort_keys=True)}"
    )
PY

"$binary" docs --help > /dev/null
"$binary" docs image insert --help > /dev/null
"$binary" drive ls --help > /dev/null
