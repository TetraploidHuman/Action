# Contributing to Action

## Development environment

Recommended (matches CI):

```bash
cd Action
nix-shell
cargo build --release
```

Without Nix: install **LLVM 21** and set `LLVM_SYS_211_PREFIX` to your LLVM prefix, then `cargo build --release`.

## Before submitting changes

```bash
# Format
cargo fmt --all

# Lint (Linux CI runs this)
cargo clippy -- -W clippy::all

# Integration tests — semantic authority (204 tests in integration.rs; 229 total harness)
cargo test --test integration -- --test-threads=1
cargo test --test hir_golden -- --test-threads=1
cargo test --test lexer_golden -- --test-threads=1
cargo test --test bootstrap_subset -- --test-threads=1
cargo test --test diagnostics_json -- --test-threads=1

# Crate unit tests (also run in CI)
cargo test -p action-codegen --lib -- --test-threads=1
cargo test -p action-lsp --lib -- --test-threads=1
cargo test -p action-frontend -- --skip proptest

# Semantic smoke (List / CoW / method chains)
./target/release/action run examples/bench_cow.ac    # expect: 11
./target/release/action run examples/map_filter.ac  # expect: 210215
./target/release/action run examples/bench_all.ac   # must not SIGSEGV
```

Lib unit tests may include proptest; CI skips proptest in the core gate. Use integration tests as the source of truth for language semantics.

## List runtime source layout

List LLVM IR lives under `crates/action-codegen/src/runtime_decl/list/{core,tree}/`. Edit the fragment `*.inc.rs` files (not `body.inc.rs` directly); `build.rs` runs `scripts/concat_list_body.py` to regenerate `body.inc.rs` before compile.


Any change to the compiler or runtime must preserve documented language behavior: persistent List/Map/Set, copy-on-write when `rc > 1`, reference counting, fallible/`or {}`, and UFCS method chains. See `.cursor/rules/preserve-language-semantics.mdc`.

## Performance changes

Compare benchmarks with the **same mode**:

- JIT full path: `./benchmark.sh -n 3`
- AOT runtime only: `./benchmark.sh --mode aot --opt 2 -n 3`

See [BENCHMARK.md](BENCHMARK.md).

## Documentation

- Architecture: [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md)
- Examples index: [examples/README.md](examples/README.md)
- Agent workflow: [AGENTS.md](AGENTS.md)

## CI

Linux CI runs inside `nix-shell` via `scripts/ci-linux.sh core` (integration harness + codegen/lsp unit tests + perf smoke + quick JIT regression). Windows CI runs the same integration harness plus release smoke on GitHub-hosted runners. Full benchmarks: `scripts/ci-linux.sh benchmark` (JIT + AOT regression).
