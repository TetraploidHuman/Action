# 第三轮重构计划书（2026-06-23）

> **状态**：✅ 已完成（2026-06-23）  
> **验收**：172 integration 全绿；语义快检不变；`scripts/ci-linux.sh core` 绿。

---

## 完成项摘要

| 项 | 状态 | 说明 |
|----|------|------|
| **R3-1** 仓库卫生与文档 | ✅ | 迁移脚本归档；`examples/_dev/` 删除并 gitignore；ARCHITECTURE/CI 更新 |
| **R3-2** CodeGen 状态分组 | ✅ | `LoopControl` / `NullableState` / `MonoCache` / `TypeLayoutCache` |
| **R3-3** Runtime extern 提取 | ✅ | `extern_decls.rs` + `declare_groups.rs`；`define_runtime_generate` ≤10 行 |
| **R3-4** list core 拆分 | ✅ | `list/core/*.inc.rs`（15 片段）+ `body.inc.rs`；最大片段 `push.inc.rs` 1560 行 |
| **R3-5** list tree 拆分 | ✅ | `list/tree/*.inc.rs`（8 片段）+ `body.inc.rs` |
| **R3-6** LSP handlers 拆分 | ✅ | `handlers/{document,navigation,editing,symbols,rename,helpers}.rs`；handler 文件 ≤317 行 |
| **R3-7** 次级 codegen 拆分 | ⚠️ 部分 | list 优先完成；`lambda_mono`/`iter`/`expr`/`for_loop` 留第四轮（仍 >1.5k 行） |
| **R3-8** driver emit | ✅ | `action-driver/src/emit/{hir,diagnostics}.rs` |
| **R3-9** 测试与文档 | ✅ | 4 项 runtime 符号 lib 测试；ARCHITECTURE ownership 表；ci-linux 注释 |
| **R3-10** 收尾 | ✅ | 本文件；CI core 绿 |

---

## 验证结果

```bash
nix-shell --run 'bash scripts/ci-linux.sh core'          # 绿
cargo test --test integration -- --test-threads=1       # 172 passed
./target/release/action run examples/bench_cow.ac       # 11
./target/release/action run examples/bench_all.ac       # 无 SIGSEGV
cargo test -p action-lsp --lib                          # 74 passed
```

---

## 附录：完成后 Top-12 可编辑源文件（不含 body.inc.rs 生成物）

```
1560  list/core/push.inc.rs
1028  list/core/walk_map.inc.rs
2437  builtins/iter.rs
2418  lambda_mono.rs
2260  runtime_decl/define_hash_table.rs
2193  expr.rs
1993  for_loop.rs
1258  action-lsp/handlers/helpers.rs
987   list/tree/remove.inc.rs
849   list/core/cow.inc.rs
777   list/tree/insert.inc.rs
1869  builtins/stdlib/datetime.rs
```

---

*文档版本：v1.1 · R3 完成态*
