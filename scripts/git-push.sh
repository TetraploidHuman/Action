#!/usr/bin/env bash
# Push from ~/Action via FlClash (default mixed-port 7890).
set -euo pipefail
PROXY="${ACTION_GIT_PROXY:-http://127.0.0.1:7890}"
export http_proxy="$PROXY" https_proxy="$PROXY" HTTP_PROXY="$PROXY" HTTPS_PROXY="$PROXY"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "git push via proxy $PROXY (repo: $ROOT)"
git push "$@"
