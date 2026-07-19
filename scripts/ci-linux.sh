#!/usr/bin/env bash
# CI helpers executed inside nix-shell (LLVM + nix Rust on PATH).
# Usage: nix-shell --run "bash scripts/ci-linux.sh <command>"
#
# Semantic authority: 212 integration tests (tests/integration.rs) — must stay green.
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
    unset RUSTUP_TOOLCHAIN RUSTUP_OVERRIDE_TOML RUSTUP_HOME
}

run_clippy() {
    verify_env
    nix_clippy_path
    cargo-clippy
}

ALL_INTEGRATION_TESTS="integration hir_golden lexer_golden bootstrap_subset diagnostics_json"

run_all_integration_tests() {
    cargo test --test integration --target "$TARGET" -- --test-threads=1
    cargo test --test hir_golden --target "$TARGET" -- --test-threads=1
    cargo test --test lexer_golden --target "$TARGET" -- --test-threads=1
    cargo test --test bootstrap_subset --target "$TARGET" -- --test-threads=1
    cargo test --test diagnostics_json --target "$TARGET" -- --test-threads=1
}

run_lsp_smoke() {
    local msg='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"capabilities":{}}}'
    local len=${#msg}
    local out
    # Capture output: grep -q closes the pipe early and LSP exits non-zero under pipefail.
    out=$(printf 'Content-Length: %d\r\n\r\n%s' "$len" "$msg" | timeout 5 "$ACTION" lsp 2>&1 || true)
    echo "$out" | grep -q '"result"' \
        || { echo "LSP initialize failed" >&2; echo "$out" >&2; return 1; }
}

run_crate_unit_tests() {
    cargo test -p action-frontend --lib --target "$TARGET" -- --test-threads=1 --skip proptest
    cargo test -p action-driver --lib --target "$TARGET" -- --test-threads=1
    cargo test -p action-codegen --lib --target "$TARGET" -- --test-threads=1
    cargo test -p action-lsp --lib --target "$TARGET" -- --test-threads=1
}

run_test() {
    verify_env
    cargo build --target "$TARGET"
    "$ACTION" check examples/hello.ac
    "$ACTION" run examples/hello.ac
    run_lsp_smoke
    run_crate_unit_tests
    run_all_integration_tests
    bash scripts/check_bootstrap_goldens.sh
    python3 scripts/check_bootstrap_prelude.py
    python3 scripts/check_bootstrap_parser.py
    python3 scripts/check_bootstrap_emit.py
    python3 scripts/check_bootstrap_typeenv.py
    python3 scripts/check_bootstrap_whenty.py
    python3 scripts/check_bootstrap_modload.py
    python3 scripts/check_bootstrap_pexpr.py
    python3 scripts/check_bootstrap_pstmt.py
    python3 scripts/check_bootstrap_pdecl.py
    python3 scripts/check_bootstrap_pscan.py
}

run_frontend() {
    verify_env
    cargo build -p action-frontend --target "$TARGET"
    cargo test -p action-frontend --target "$TARGET" -- --skip proptest
}

run_benchmark() {
    verify_env
    cargo build --release --target "$TARGET"
    test -x "$RELEASE_ACTION"
    ./benchmark.sh --iterations 3
    python3 scripts/benchmark_regression.py \
        benchmark_results_jit_ci_baseline.txt benchmark_results.txt \
        --threshold 0.35 --min-delta-ms 20
    ./benchmark.sh --mode aot --opt 2 --iterations 3 --results benchmark_results_aot_o2_ci.txt
}

CI_PERF_BENCHES="${CI_PERF_BENCHES:-bench_cow,bench_all,bench_concat_depth,bench_insert100}"

run_perf_smoke_debug() {
    echo "=== perf smoke (debug JIT) ==="
    local out
    out=$("$ACTION" run examples/bench_cow.ac 2>&1) || return 1
    echo "$out" | tail -1 | grep -qx '11' || {
        echo "bench_cow: expected 11, got: $out" >&2
        return 1
    }
    local b
    for b in bench_all bench_concat_depth bench_insert100; do
        "$ACTION" run "examples/${b}.ac" >/dev/null || {
            echo "perf smoke failed: ${b}" >&2
            return 1
        }
    done
}

run_perf_smoke_release() {
    echo "=== perf smoke (release JIT) ==="
    cargo build --release --target "$TARGET"
    test -x "$RELEASE_ACTION"
    local out b
    out=$("$RELEASE_ACTION" run examples/bench_cow.ac 2>&1) || return 1
    echo "$out" | tail -1 | grep -qx '11' || {
        echo "bench_cow (release): expected 11, got: $out" >&2
        return 1
    }
    for b in bench_insert2 bench_insert10 bench_insert100 bench_for_nested test_insert_exit test_list_alias_insert test_cow_insert_isolation; do
        "$RELEASE_ACTION" run "examples/${b}.ac" >/dev/null || {
            echo "release smoke failed: ${b}" >&2
            return 1
        }
    done
    local mf
    mf=$("$RELEASE_ACTION" run examples/map_filter.ac 2>&1) || return 1
    echo "$mf" | tail -1 | grep -qx '210215' || {
        echo "map_filter: expected 210215, got: $mf" >&2
        return 1
    }
    bash scripts/ci_insert_stress.sh "$RELEASE_ACTION"
}

run_aot_smoke_release() {
    echo "=== AOT smoke (release -O2) ==="
    test -x "$RELEASE_ACTION"
    local bench="examples/bench_cow.ac"
    local exe="examples/bench_cow"
    rm -f "$exe"
    "$RELEASE_ACTION" run -O2 --emit exe "$bench" >/dev/null || {
        echo "AOT compile bench_cow failed" >&2
        return 1
    }
    test -x "$exe" || {
        echo "AOT exe missing: $exe" >&2
        return 1
    }
    local out
    out=$("$exe" 2>&1) || return 1
    echo "$out" | tail -1 | grep -qx '11' || {
        echo "AOT bench_cow: expected 11, got: $out" >&2
        return 1
    }
    rm -f "$exe" "${bench%.ac}.o"
}

run_perf_quick() {
    echo "=== perf quick (release JIT, subset) ==="
    cargo build --release --target "$TARGET"
    export TARGET
    ./benchmark.sh --no-warmup --iterations 1 \
        --only "$CI_PERF_BENCHES" \
        --results benchmark_results_ci_smoke.txt
    python3 scripts/benchmark_regression.py \
        benchmark_results_ci_baseline.txt benchmark_results_ci_smoke.txt \
        --only "$CI_PERF_BENCHES" \
        --threshold 0.30 \
        --min-delta-ms 15
}

run_proptest() {
    verify_env
    PROPTEST_CASES="${PROPTEST_CASES:-256}" \
        cargo test -p action-frontend --target "$TARGET" proptest -- --test-threads=1
}

run_debug() {
    verify_env
    "$ACTION" run examples/hello.ac
}

# Push CI: build once, test, frontend, debug perf smoke (full suite in parallel benchmark job).
run_core() {
    verify_env
    RUSTFLAGS="-D warnings" cargo build --target "$TARGET" -p action-frontend -p action-codegen -p action
    run_clippy
    cargo build --target "$TARGET"
    "$ACTION" check examples/hello.ac
    "$ACTION" run examples/hello.ac
    run_lsp_smoke
    run_crate_unit_tests
    run_all_integration_tests
    run_perf_smoke_debug
    run_perf_smoke_release
    run_aot_smoke_release
    run_perf_quick
    cargo build -p action-frontend --target "$TARGET"
    cargo test -p action-frontend --target "$TARGET" -- --skip proptest
}

usage() {
    echo "usage: $0 {test|clippy|frontend|benchmark|proptest|debug|core}" >&2
    exit 1
}

case "${1:-}" in
    test) run_test ;;
    clippy) run_clippy ;;
    frontend) run_frontend ;;
    benchmark) run_benchmark ;;
    proptest) run_proptest ;;
    debug) run_debug ;;
    core) run_core ;;
    *) usage ;;
esac
