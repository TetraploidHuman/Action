#!/usr/bin/env bash
# Push from ~/Action. Tries direct GitHub first (bypasses broken FlClash / global proxy).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
PROXY="${ACTION_GIT_PROXY:-http://127.0.0.1:7890}"

echo "git push (repo: $ROOT)"
if git -c http.proxy= -c https.proxy= push "$@"; then
  exit 0
fi

echo "direct push failed; retrying via FlClash proxy $PROXY ..."
export http_proxy="$PROXY" https_proxy="$PROXY" HTTP_PROXY="$PROXY" HTTPS_PROXY="$PROXY"
git -c http.proxy="$PROXY" -c https.proxy="$PROXY" push "$@"
