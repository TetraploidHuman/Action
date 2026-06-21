# Deferred RC release — insert-loop memory peak

When `assign` to a `List` binding only `rc_dec`s the old root (because another binding in the same scope still holds a reference), the full subtree is **not** freed until scope exit. During tight insert/append loops that reassign the same `var`, each iteration may leave a large unreachable tree alive until cleanup runs.

## Observable effect

- **Correctness**: unchanged — CoW and alias isolation still hold (`test_insert_exit`, alias stress tests).
- **Memory**: peak RSS can grow roughly with loop count × tree size until the binding goes out of scope or is reassigned without aliasing peers.

## Mitigation (future)

- Per-node conditional `rc_dec` on assign when safe (see week-plan P0-3).
- Prefer inner blocks to shorten alias lifetime (`test_list_alias_block.at`).

## Baseline

Record peak with `/usr/bin/time -v` or `ps` while running `bench_insert100.at` × N before/after assign-policy changes.
