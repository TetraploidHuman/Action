# Deferred RC release — insert-loop memory peak

When `assign` to a `List` binding only `rc_dec`s the old root (because another binding in the same scope still holds a reference), the full subtree is **not** freed until scope exit. During tight insert/append loops that reassign the same `var`, each iteration may leave a large unreachable tree alive until cleanup runs.

## Observable effect

- **Correctness**: unchanged — CoW and alias isolation still hold (`test_insert_exit`, alias stress tests).
- **Memory**: peak RSS can grow roughly with loop count × tree size until the binding goes out of scope or is reassigned without aliasing peers.

## Mitigation

- **P0-3 (enabled)**: `define_list_rc_assign.rs` — post-order release skipping nodes reachable from live scope lists **and the incoming assign value**. Codegen: `emit_rc_release_list_on_assign` in `rc_ops.rs` (passes `new_data_ptr`/`new_height` so RHS-shared nodes are not freed early).
- Per-node conditional `rc_dec` on assign when safe (see week-plan P0-3).
- Prefer inner blocks to shorten alias lifetime (`test_list_alias_block.ac`).

## Measured peak RSS (2026-06-21, GNU time MaxRSS)

Environment: self-hosted NixOS, `./target/release/action run`, programs build 2000-element `lst` then 100× or 10× insert alias loop unless noted.

| Program | Pre P0-3 (`4edaec2`, root-only defer) | Post P0-3 (`main`) | Δ |
|---------|--------------------------------------:|-------------------:|--:|
| `bench_insert100.ac` | 45084 KB | 43648 KB | −3.2% |
| `test_insert_exit.ac` | 44472 KB | 43312 KB | −2.6% |
| `test_list_alias_block.ac` | — | 43812 KB | inner block baseline |

Reproduce:

```bash
cargo build --release
bash scripts/measure_mem_peak.sh examples/bench_insert100.ac examples/test_insert_exit.ac
```

Compare pre-P0-3: `git worktree add /tmp/pre 4edaec2 && (cd /tmp/pre && cargo build --release && …)`.

## Baseline

Record peak with `scripts/measure_mem_peak.sh` or `GNU_TIME=/run/current-system/sw/bin/time` while running `bench_insert100.ac` before/after assign-policy changes.
