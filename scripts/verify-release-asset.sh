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

tar -C "$staging" -xzf "$asset"
binary="$staging/goog"
test -x "$binary"

actual_version="$("$binary" --version)"
if [ "$actual_version" != "goog $version" ]; then
  echo "packaged binary reported '$actual_version', expected 'goog $version'" >&2
  exit 1
fi

"$binary" version --json > "$staging/version.json"
python3 - "$staging/version.json" "$version" "$tag" "$commit" "$release_channel" "$target" <<'PY'
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
