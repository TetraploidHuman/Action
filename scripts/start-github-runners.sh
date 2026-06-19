#!/usr/bin/env bash
# Start all local GitHub Actions runner instances (background).
set -euo pipefail

ROOT="${RUNNER_ROOT:-$HOME/桌面/Runner}"
LOG_DIR="$ROOT/logs"
mkdir -p "$LOG_DIR"

start_one() {
    local dir="$1"
    local name
    name="$(basename "$dir")"
    if pgrep -f "$dir/bin/Runner.Listener run" >/dev/null 2>&1; then
        echo "already running: $name"
        return 0
    fi
    if [[ ! -x "$dir/run-nixos.sh" && ! -x "$dir/run-helper.sh" ]]; then
        echo "skip (not a runner dir): $dir"
        return 0
    fi
    local launcher="$dir/run-nixos.sh"
    [[ -x "$launcher" ]] || launcher="$dir/run-helper.sh"
    nohup "$launcher" >>"$LOG_DIR/${name}.log" 2>&1 &
    echo "started $name (pid $!, log $LOG_DIR/${name}.log)"
}

# Instance 1 (legacy path without numeric suffix).
start_one "$ROOT/runner"

for dir in "$ROOT"/runner-[0-9]*; do
    [[ -d "$dir" ]] || continue
    start_one "$dir"
done

sleep 1
gh api repos/TetraploidHuman/Action/actions/runners --jq '.runners[] | "\(.name) \(.status) busy=\(.busy)"' 2>/dev/null \
    || echo "run 'gh api repos/TetraploidHuman/Action/actions/runners' to verify"
