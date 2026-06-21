#!/usr/bin/env bash
# Measure peak RSS (KB) for Action programs via GNU time.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TIME="${GNU_TIME:-/run/current-system/sw/bin/time}"
ACTION="${ACTION_BIN:-$ROOT/target/release/action}"

if [[ ! -x "$ACTION" ]]; then
  echo "missing $ACTION — run: cargo build --release" >&2
  exit 1
fi

for prog in "$@"; do
  echo "=== $prog ==="
  "$TIME" -f "MaxRSS_KB=%M wall_s=%e" "$ACTION" run "$prog" 2>&1 | tail -3
done
