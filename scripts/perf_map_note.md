# Map / runtime hotspot notes (2026-06-21)

From `python3 scripts/perf_phase_split.py` (JIT full path, `-O0`):

| Benchmark | jit+rt (ms) | Notes |
|-----------|-------------|--------|
| `bench_all.at` | ~287 | List CoW + concat + method chain |
| `bench_insert100` (via bisect) | ~266 | insert alias assign path |
| `bench_cow.at` | ~113 | CoW property micro |
| `bench_for_method.at` | ~110 | for + UFCS; `list_get_cached` used in iter |

**Codegen scan**: `builtins_iter.rs` already uses `list_get_cached_fat`. Remaining `action_list_get` in UFCS one-shots (`builtins_call.rs`) and `misc.rs` index — not sequential loops.

**Map rebuild**: not dominant in step1–6 smoke; profile `bench_map_10k.at` before changing `define_map.rs`.

**Perf changes landed**: `for_loop.rs` always cached for `for (i,x) in list`; `hir_compile.rs` rest-destructure tail uses cached get.
