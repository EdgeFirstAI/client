#!/usr/bin/env bash
# wait-for-run.sh — block until a workflow's run for a commit succeeds.
#
# Usage:  wait-for-run.sh <workflow-file> <commit-sha>
# Stdout: the successful run's databaseId (nothing else — it is captured)
# Stderr: progress
# Exit:   0 on success, 1 on failure / no run / timeout
#
# Replaces lewagon/wait-on-check-action, which matched on human-readable CHECK
# names. That coupling is silent and brittle: renaming or deleting a job leaves
# the release waiting for a check that will never appear, and the failure only
# surfaces as a timeout. Matching on the workflow FILE and returning the run id
# also gives the caller what actions/download-artifact needs to reach across
# workflow runs, which the check-name approach could not provide.
set -euo pipefail

WORKFLOW=${1:?usage: wait-for-run.sh <workflow-file> <commit-sha>}
SHA=${2:?usage: wait-for-run.sh <workflow-file> <commit-sha>}

INTERVAL=30
MAX_TRIES=120    # 120 x 30s = 60 min overall
APPEAR_TRIES=20  # 20 x 30s = 10 min for a run to show up at all

missing=0
for try in $(seq 1 "$MAX_TRIES"); do
  # A flaky API call must not read as "no run was ever queued", so a query
  # failure is logged and retried without counting against APPEAR_TRIES.
  # A permanently broken query still ends at the overall timeout below.
  if ! run=$(gh run list --repo "$GITHUB_REPOSITORY" --workflow "$WORKFLOW" \
               --commit "$SHA" --limit 1 \
               --json status,conclusion,url,databaseId --jq '.[0] // empty' 2>&1); then
    echo "gh query failed (try $try): $run" >&2
  elif [ -z "$run" ]; then
    missing=$((missing + 1))
    if [ "$missing" -ge "$APPEAR_TRIES" ]; then
      # Almost always a tag pointing at a commit that never landed on main.
      # Diagnose that directly instead of burning the full timeout on a
      # commit that will never build.
      echo "::error::no $WORKFLOW run for $SHA — tag a commit that has landed on main" >&2
      exit 1
    fi
    echo "no $WORKFLOW run for $SHA yet (try $try) — waiting" >&2
  else
    status=$(jq -r .status <<<"$run")
    conclusion=$(jq -r .conclusion <<<"$run")
    url=$(jq -r .url <<<"$run")
    if [ "$status" = "completed" ]; then
      if [ "$conclusion" = "success" ]; then
        echo "$WORKFLOW succeeded: $url" >&2
        jq -r .databaseId <<<"$run"
        exit 0
      fi
      echo "::error::$WORKFLOW concluded '$conclusion' for $SHA — $url" >&2
      exit 1
    fi
    echo "$WORKFLOW is '$status' (try $try) — $url" >&2
  fi
  sleep "$INTERVAL"
done

echo "::error::timed out after $((MAX_TRIES * INTERVAL / 60))m waiting for $WORKFLOW on $SHA" >&2
exit 1
