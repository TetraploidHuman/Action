#!/usr/bin/env bash
# Regenerate bootstrap HIR goldens and verify they match committed files.
# Usage:
#   bash scripts/check_bootstrap_goldens.sh          # check only (CI-friendly)
#   bash scripts/check_bootstrap_goldens.sh --write # regenerate + report diff
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

WRITE=0
if [[ "${1:-}" == "--write" ]]; then
    WRITE=1
fi

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    ACTION="${CARGO_TARGET_DIR}/${TARGET}/release/action"
else
    ACTION="./target/${TARGET}/release/action"
fi

if [[ ! -x "$ACTION" ]]; then
    echo "check_bootstrap_goldens: building release action..." >&2
    cargo build --release
fi

GOLDEN_DIR="tests/fixtures/bootstrap"
mapfile -t GOLDENS < <(find "$GOLDEN_DIR" -name '*.bootstrap_hir.json' | sort)

if [[ ${#GOLDENS[@]} -eq 0 ]]; then
    echo "check_bootstrap_goldens: no *.bootstrap_hir.json under $GOLDEN_DIR" >&2
    exit 1
fi

echo "=== regenerate bootstrap HIR goldens (${#GOLDENS[@]} files) ==="
python3 scripts/gen_bootstrap_hir_golden.py --all

CHANGED=0
for f in "${GOLDENS[@]}"; do
    if ! git diff --quiet -- "$f" 2>/dev/null; then
        CHANGED=1
        echo "DRIFT: $f"
        git diff --stat -- "$f" || true
    fi
done

if [[ "$CHANGED" -eq 1 ]]; then
    if [[ "$WRITE" -eq 1 ]]; then
        echo "check_bootstrap_goldens: goldens updated; review diff and commit if intentional." >&2
        exit 0
    fi
    echo "check_bootstrap_goldens: golden drift detected; run with --write to refresh." >&2
    exit 1
fi

echo "=== bootstrap golden check OK ==="
