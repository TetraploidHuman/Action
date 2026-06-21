# Map / runtime hotspot notes (2026-06-21)

From `./benchmark.sh` (post P0-3 assign RC + list loop cache):

| Benchmark | JIT (ms) | AOT -O2 exe (ms) | Notes |
|-----------|----------|------------------|--------|
| `bench_all.ac` | 215–220 | 25–29 | List CoW + concat + method chain |
| `bench_insert100.ac` | 153–249 | 8–11 | insert alias + assign release |
| `bench_cow.ac` | 132–108 | — | CoW micro |
| `bench_for_method.ac` | 137–113 | — | for + UFCS; iter cached |
| `bench_map_10k.ac` | 133–137 | 7–8 | Map O(n) ops; rebuild not step-smoke dominant |
| `bench_map.ac` | 130–135 | 4–5 | small map |

**Codegen — list_get_cached (P2-3):**

| Path | Status |
|------|--------|
| `builtins_iter.rs` | all loop walks use `list_get_cached_fat` |
| `for_loop.rs` | `for (i,x) in list`, sequential peephole, **`list_loop_get_cache` on `for i < n` body** |
| `hir_compile.rs` | list rest-destructure tail uses cached get |
| `misc.rs` | `lst[i]` uses loop cache when inside sequential condition-for |
| `builtins_call.rs` | UFCS `.get(i)` uses loop cache in same context |
| `builtins_call.rs` head/last | single-shot; leave uncached |

**Map rebuild:** profile before changing `define_map.rs`; `bench_map_10k` total JIT ~135 ms — dominated by compile+run overhead on medium workload, not List insert path.
