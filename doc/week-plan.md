# 一周改进计划（2026-06-20）

> 优先级：**正确性** > **CI 稳定** > **性能** > **文档**

## 已完成（本会话）

| ID | 项 | 状态 |
|----|-----|------|
| P0-1 | `insert_rec` CoW：`list_root_rc`、CoW-before-recurse、条件 xfer | ✅ |
| P0-2 | assign 延迟整树释放（有其他 List 绑定时仅 `rc_dec` 根） | ✅ |
| P0-3 | `store_child` 条件 `rc_inc`；scope cleanup 逆序 | ✅ |
| P0-4 | 集成测试：`test_insert_exit` / `test_cow_insert_isolation` / `test_list_cow_property` | ✅ |
| P1-1 | `ci-linux.sh` release 冒烟（insert 系列 + test_insert_exit） | ✅ |
| P1-2 | `benchmark_regression.py` FAIL 行检测 | ✅ |
| P3-1 | `ARCHITECTURE.md` CI/测试同步 | ✅ |

## 待办（后续迭代）

| ID | 项 | 说明 |
|----|-----|------|
| P1-3 | GitHub hosted Linux fallback job | `ubuntu-latest` + integration + bench_cow |
| P1-4 | Windows release 冒烟 | `bench_cow` + `test_insert_exit` |
| P1-5 | Proptest merge gate | 256 cases 稳定后 `continue-on-error: false` |
| P2-1 | AOT baseline 重刷 | insert 修复后 `./benchmark.sh --mode aot --opt 2` |
| P2-2 | `list_get_cached` 扩展 | iter 热点补 cached get |
| P3-2 | `doc/language-spec-outline.md` | 语义大纲 + integration 索引 |
| P3-3 | `doc/stdlib-layers.md` | lib / stdlib / builtins 分层 |
| P4 | VSCode Marketplace / atom.toml | 超出本周范围 |

## 验收命令

```bash
nix-shell --run 'cargo fmt && cargo build --release'
nix-shell --run 'cargo test --release --test integration -- --test-threads=1'
for i in $(seq 1 30); do ./target/release/action run examples/bench_insert10.at || exit 1; done
./benchmark.sh --mode aot --opt 2 --iterations 3
```

## 风险

- **自托管 runner 不可用**：需 P1-3 hosted fallback。
- **中间 insert 树延迟释放**：当前策略在 assign 时仅 dec 根，scope 退出时整树释放；长期可改为 per-node RC 精确释放以减少泄漏。
