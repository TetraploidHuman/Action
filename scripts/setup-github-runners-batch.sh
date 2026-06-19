#!/usr/bin/env bash
# Register and start additional self-hosted runners (instances START..END).
#
#   ./scripts/setup-github-runners-batch.sh 4 6
set -euo pipefail

START="${1:?start instance id}"
END="${2:?end instance id}"
SCRIPT="$(cd "$(dirname "$0")" && pwd)/setup-github-runner.sh"

for i in $(seq "$START" "$END"); do
    echo "=== registering runner instance $i ==="
    TOKEN=$(gh api repos/TetraploidHuman/Action/actions/runners/registration-token --method POST -q .token)
    "$SCRIPT" "$i" "$TOKEN"
done

"$(dirname "$SCRIPT")/start-github-runners.sh"
