#!/usr/bin/env bash
# Release insert-series stress smoke (P0-1): repeated runs must not SIGABRT.
# Usage: CI_INSERT_STRESS_ITERS=5 bash scripts/ci_insert_stress.sh [action_binary]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    BIN="${CARGO_TARGET_DIR}/${TARGET}/release/action"
else
    BIN="./target/${TARGET}/release/action"
fi
if [[ $# -ge 1 ]]; then
    BIN="$1"
fi

ITERS="${CI_INSERT_STRESS_ITERS:-5}"
BENCHES=(
    bench_insert2
    bench_insert10
    bench_insert50
    bench_insert100
    bench_all
)

test -x "$BIN" || {
    echo "ci_insert_stress: missing release binary: $BIN" >&2
    exit 1
}

echo "=== insert stress (${ITERS}× release) ==="
for b in "${BENCHES[@]}"; do
    i=1
    while [[ "$i" -le "$ITERS" ]]; do
        "$BIN" run "examples/${b}.at" >/dev/null || {
            echo "insert stress failed: ${b} iteration ${i}/${ITERS}" >&2
            exit 1
        }
        i=$((i + 1))
    done
done
echo "insert stress ok (${ITERS}× × ${#BENCHES[@]} benches)"
