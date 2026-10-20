#!/usr/bin/env bash
# Update a gh-pages checkout and push it.
#
# Usage: gh-pages.sh CHECKOUT DEST [SITE]
#   DEST "." is the live site: everything at the root except the previews (pr-preview/ and
#   main/) is replaced by SITE.
#   DEST main or pr-preview/pr-N is a preview: replaced by SITE, or removed without SITE.
set -euo pipefail
checkout=$1 dest=$2 site=${3:-}
cd "$checkout"
if [[ $dest == . ]]; then
  find . -mindepth 1 -maxdepth 1 ! -name .git ! -name pr-preview ! -name main -exec rm -rf {} +
else
  rm -rf "$dest"
fi
if [[ -n $site ]]; then
  mkdir -p "$dest"
  cp -r "$site"/. "$dest"
fi
touch .nojekyll  # serve Sphinx's _static/ as is
git add -A
if git diff --cached --quiet; then
  echo "gh-pages: $dest unchanged"
  exit 0
fi
git -c user.name="github-actions[bot]" \
    -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
    commit -q -m "$( [[ -n $site ]] && echo Update || echo Remove ) $dest"
# Other previews may push at the same time; they touch other folders, so a rebase resolves it.
for _ in 1 2 3 4 5; do
  git pull -q --rebase origin gh-pages && git push -q origin HEAD:gh-pages && exit 0
  sleep 5
done
echo "gh-pages: push failed" >&2
exit 1
