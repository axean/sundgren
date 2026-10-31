#!/usr/bin/env bash
# Update the gh-pages branch and push it: one environment's site, and environments.json, the index
# of every environment that the site's Development menu reads (tools/environments.py).
#
# Usage: gh-pages.sh CHECKOUT DEST [SITE]
#   DEST "." is the live site: everything at the root except the previews (pr-preview/ and
#   main/) is replaced by SITE.
#   DEST main or pr-preview/pr-N is a preview: replaced by SITE, or removed without SITE.
# With SITE, DEST/environment.json describes it, from the SUNDGREN_ENV_* variables.
set -euo pipefail
checkout=$1 dest=$2 site=${3:-}
environments="$(cd "$(dirname "$0")" && pwd)/environments.py"
cd "$checkout"

update() {
  if [[ $dest == . ]]; then
    find . -mindepth 1 -maxdepth 1 ! -name .git ! -name pr-preview ! -name main -exec rm -rf {} +
  else
    rm -rf "$dest"
  fi
  if [[ -n $site ]]; then
    mkdir -p "$dest"
    cp -r "$site"/. "$dest"
    python3 "$environments" write "$dest"
  fi
  python3 "$environments" index
  touch .nojekyll  # serve Sphinx's _static/ as is
  git add -A
}

# Other previews may push at the same time. Each try starts again from the newest gh-pages, so
# the index lists every environment.
for _ in 1 2 3 4 5; do
  git fetch -q origin gh-pages
  git reset -q --hard origin/gh-pages
  git clean -qfd
  update
  if git diff --cached --quiet; then
    echo "gh-pages: $dest unchanged"
    exit 0
  fi
  git -c user.name="github-actions[bot]" \
      -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
      commit -q -m "$( [[ -n $site ]] && echo Update || echo Remove ) $dest"
  if git push -q origin HEAD:gh-pages; then
    exit 0
  fi
  sleep 5
done
echo "gh-pages: push failed" >&2
exit 1
