#!/usr/bin/env bash
# pre-commit wrapper: runs an extension dev tool (eslint, tsc, prettier) from editors/vscode,
# with file arguments made relative to it. Fails if node_modules is missing or older than
# package-lock.json, since typed linting and type checks need the installed packages.
# Usage: hook.sh <tool> [args...] [-- files...]
set -euo pipefail

dir="editors/vscode"
lock="$dir/node_modules/.package-lock.json"
if [[ ! -f "$lock" ]] || [[ "$dir/package-lock.json" -nt "$lock" ]]; then
  echo "editors/vscode/node_modules is missing or out of date; run: npm ci --prefix editors/vscode" >&2
  exit 1
fi

tool="$1"
shift
args=()
while [[ $# -gt 0 && "$1" != "--" ]]; do
  args+=("$1")
  shift
done
[[ $# -gt 0 ]] && shift
for file in "$@"; do
  args+=("${file#"$dir"/}")
done

cd "$dir"
exec "node_modules/.bin/$tool" "${args[@]}"
