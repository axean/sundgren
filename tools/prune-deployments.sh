#!/usr/bin/env bash
# Delete the deployment records of an environment for one git ref, except the KEEP newest.
#
# Usage: prune-deployments.sh ENVIRONMENT REF [KEEP]   (KEEP defaults to 0: delete all)
#   An empty REF means every ref.
# GitHub only deletes inactive deployments, so each is marked inactive first. Needs GH_TOKEN with
# deployments: write, and GITHUB_REPOSITORY (owner/name).
set -euo pipefail
environment=$1 ref=$2 keep=${3:-0}
repo=repos/$GITHUB_REPOSITORY
# Newest first, as the API returns them.
ids=$(gh api -X GET "$repo/deployments" -f environment="$environment" ${ref:+-f ref="$ref"} \
  -f per_page=100 --jq '.[].id' | tail -n +$((keep + 1)))
for id in $ids; do
  gh api -X POST "$repo/deployments/$id/statuses" -f state=inactive >/dev/null
  gh api -X DELETE "$repo/deployments/$id" >/dev/null
  echo "deleted $environment deployment $id (${ref:-any ref})"
done
