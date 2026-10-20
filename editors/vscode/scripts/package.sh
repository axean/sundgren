#!/usr/bin/env bash
# Packages a platform .vsix with a sundgren binary bundled as bundled/sundgren (Ruff style).
# Checks the extension version matches pyproject.toml and the binary matches the target.
# Run from editors/vscode after `npm ci`. Usage: package.sh <target> <binary> <out.vsix>
set -euo pipefail

target="$1"
binary="$2"
out="$3"

version="$(node -p 'require("./package.json").version')"
expected="$(python3 -c 'import tomllib; print(tomllib.load(open("../../pyproject.toml", "rb"))["project"]["version"])')"
if [[ "$version" != "$expected" ]]; then
  echo "package.json version $version != pyproject.toml version $expected" >&2
  exit 1
fi

case "$target" in
  linux-x64) arch="ELF 64-bit LSB .*x86-64" ;;
  linux-arm64) arch="ELF 64-bit LSB .*aarch64" ;;
  darwin-x64) arch="Mach-O 64-bit .*x86_64" ;;
  darwin-arm64) arch="Mach-O 64-bit .*arm64" ;;
  *)
    echo "unsupported target: $target" >&2
    exit 1
    ;;
esac
description="$(file -b "$binary")"
if ! grep -Eq "$arch" <<<"$description"; then
  echo "$binary is not a $target binary: $description" >&2
  exit 1
fi

rm -rf bundled
mkdir bundled
cp "$binary" bundled/sundgren
chmod 755 bundled/sundgren
npx vsce package --target "$target" --out "$out"

# The binary must be in the .vsix, executable.
if ! unzip -Z "$out" extension/bundled/sundgren | grep -q '^-rwx'; then
  echo "$out: extension/bundled/sundgren is missing or not executable" >&2
  exit 1
fi
