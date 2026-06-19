#!/usr/bin/env bash
# CI helpers executed inside nix-shell (LLVM + nix Rust on PATH).
# Usage: nix-shell --run "bash scripts/ci-linux.sh <command>"
set -euo pipefail

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
ACTION="./target/${TARGET}/debug/action"
RELEASE_ACTION="./target/${TARGET}/release/action"

verify_env() {
    test -f shell.nix
    echo "LLVM: $(llvm-config --version)"
    echo "Rust: $(rustc --version)"
}

run_core() {
    verify_env

    cargo build --target "$TARGET"

    "$ACTION" check examples/hello.at
    "$ACTION" run examples/hello.at

    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{}}}' \
        | timeout 5 "$ACTION" lsp 2>&1 || true

    PROPTEST_CASES="${PROPTEST_CASES:-50}" \
        cargo test --lib --target "$TARGET" -- --test-threads=1

    cargo test --test integration --target "$TARGET" -- --test-threads=1

    # nix rustc/clippy; drop rustup shims to avoid mismatch.
    export PATH
    PATH="$(echo "$PATH" | tr ':' '\n' | grep -v '.cargo/bin' | tr '\n' ':' | sed 's/:$//')"
    cargo clippy -- -W clippy::all

    cargo build -p action-frontend --target "$TARGET"
    cargo test -p action-frontend --target "$TARGET"
}

run_benchmark() {
    verify_env

    cargo build --release --target "$TARGET"
    test -x "$RELEASE_ACTION"

    ./benchmark.sh --iterations 3
    ./benchmark.sh --mode aot --opt 2 --iterations 3 --results benchmark_results_aot_o2_ci.txt
}

run_debug() {
    verify_env
    "$ACTION" run examples/hello.at
}

usage() {
    echo "usage: $0 {core|benchmark|debug}" >&2
    exit 1
}

case "${1:-}" in
    core) run_core ;;
    benchmark) run_benchmark ;;
    debug) run_debug ;;
    *) usage ;;
esac
