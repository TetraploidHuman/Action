# Action examples

~234 `.ac` files in this directory. **204** integration tests in `tests/integration.rs` reference ~200 unique example files.

## Tutorial

| File | Purpose |
|------|---------|
| `tutorial.ac` | Runnable language tour (full golden integration test) |
| `hello.ac` | Minimal hello world |

## Integration-tested

Programs referenced from `tests/integration.rs`. These are the **semantic regression suite** — changing compiler behavior usually requires updating expected output here.

Notable smoke examples:

| File | Expected |
|------|----------|
| `bench_cow.ac` | `11` |
| `map_filter.ac` | `210215` |
| `bench_all.ac` | len lines + `true`/`false` (contains HT fusion path) |
| `bench_set.ac` / `bench_map.ac` / `bench_math.ac` | stdout golden |
| `test_ffi.ac` | CString round-trip |
| `test_lazyhead_empty.ac` | `true` / `false` on empty lazy list |

## Benchmark-only

`bench_*.ac` — timed by `benchmark.sh`. Integration asserts stdout for key benches; others are exit-code smoke via `benchmark.sh` / CI perf smoke.

Use `./benchmark.sh --mode aot --opt 2 -n 3` for runtime comparisons. See [BENCHMARK.md](../BENCHMARK.md).

## Manual / dev-only

| File | Notes |
|------|-------|
| `http_test.ac`, `deepseek_chat.ac` | Require network — not in CI |
| `_dev/` (gitignored) | Local scratch and bisect files |

## Orphan examples (not in integration)

Prefer wiring new tests or moving to `_dev/`:

`test_ffi2.ac`, `ffi_test2.ac`, `streq_test.ac`, `map_set_test1.ac`, `when_test.ac`, `when_condition_chain.ac`, `test_bind.ac`, `test_cb.ac`, `test_when_cb.ac`, `stream_send_only.ac`, `stream_minimal.ac`, `stream_test2.ac`, `external_type.ac`, `stdlib_test.ac`, `test_and_simple.ac`, `test_and_simple2.ac`, `string_fn.ac`, `bench_step1.ac`–`bench_step6.ac`, `bench_cmp.ac`, and other `bench_*` without integration golden.

Removed scratch: `manual_subset*.ac`, `test_ident_fix.ac`, duplicate `test_datetime.ac`.
