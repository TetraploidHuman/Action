#!/usr/bin/env bash
# CI helpers executed inside nix-shell (LLVM + nix Rust on PATH).
# Usage: nix-shell --run "bash scripts/ci-linux.sh <command>"
#
# Optional: CARGO_TARGET_DIR (persistent self-hosted cache) overrides ./target.
set -euo pipefail

TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    BIN_ROOT="${CARGO_TARGET_DIR}/${TARGET}"
else
    BIN_ROOT="./target/${TARGET}"
fi
ACTION="${BIN_ROOT}/debug/action"
RELEASE_ACTION="${BIN_ROOT}/release/action"

verify_env() {
    test -f shell.nix
    echo "LLVM: $(llvm-config --version)"
    echo "Rust: $(rustc --version)"
    echo "CARGO_TARGET_DIR: ${CARGO_TARGET_DIR:-./target (default)}"
}

nix_clippy_path() {
    export PATH
    PATH="$(echo "$PATH" | tr ':' '\n' | grep -v '.cargo/bin' | tr '\n' ':' | sed 's/:$//')"
}

run_test() {
    verify_env
    cargo build --target "$TARGET"
    "$ACTION" check examples/hello.at
    "$ACTION" run examples/hello.at
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{}}}' \
        | timeout 5 "$ACTION" lsp 2>&1 || true
    PROPTEST_CASES="${PROPTEST_CASES:-50}" \
        cargo test --lib --target "$TARGET" -- --test-threads=1
    cargo test --test integration --target "$TARGET" -- --test-threads=1
}

run_clippy() {
    verify_env
    nix_clippy_path
    cargo clippy --target "$TARGET" -- -W clippy::all
}

run_frontend() {
    verify_env
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

# Push CI: build once, test, frontend (clippy runs in lint.yml).
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
    cargo build -p action-frontend --target "$TARGET"
    cargo test -p action-frontend --target "$TARGET"
}

usage() {
    echo "usage: $0 {test|clippy|frontend|benchmark|debug|core}" >&2
    exit 1
}

case "${1:-}" in
    test) run_test ;;
    clippy) run_clippy ;;
    frontend) run_frontend ;;
    benchmark) run_benchmark ;;
    debug) run_debug ;;
    core) run_core ;;
    *) usage ;;
esac
