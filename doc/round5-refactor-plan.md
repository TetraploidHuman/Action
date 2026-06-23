# 第五轮重构计划书（2026-06-23）

> **状态**：✅ 已完成（2026-06-23）  
> **目标**：在 R4 基础上继续拆分剩余 >1500 行 codegen 源文件，提升可导航性与模块内聚。  
> **约束**：语言语义不变；172 integration 全绿；禁止 SubAgent。

---

## 背景

R4 完成后 `iter`/`mono`/`expr`/`for_loop`/LSP helpers 已拆分，但仍有若干单文件 >1500 行：

| 文件 | 行数 | 问题 |
|------|------|------|
| `define_hash_table.rs` | 2260 | Map/Set 运行时 IR 全挤一处 |
| `hir_compile.rs` | 1611 | HIR 编译入口与 expr/stmt/value 混杂 |
| `builtins/lazy.rs` | 1508 | LazyList codegen 操作未分域 |

R5 聚焦上述三处，沿用 R4 的「多 `impl CodeGen` 子模块 + `pub(crate)` 跨文件可见」约定。

---

## 待办项

| 项 | 状态 | 说明 |
|----|------|------|
| **R5-1** `hash_table/` | ✅ | hash_rehash / from_list / helpers / insert / query / remove / rc_dec / accessors；最大片段 helpers.rs 643 行 |
| **R5-2** `hir_compile/` | ✅ | mod / control / expr / stmt / values；`compile_hir` 与 expr 分发留在 mod.rs |
| **R5-3** `builtins/lazy/` | ✅ | mod / take_drop / map_filter / take_while / head_zip |
| **R5-4** 稳定性测试 | ✅ | `runtime_defines_ht_insert`、`codegen_hir_lazy_map`、`codegen_hir_destructure` |
| **R5-5** 文档 | ✅ | 本文件 + `ARCHITECTURE.md` R5 行与 ownership 表 |
| **R5-6** 验证与 CI | ✅ | fmt / build / integration / 语义快检 / `ci-linux.sh core` / push / CI 绿 |

---

## 拆分约定（继承 R4）

- 子模块内多个 `impl<'ctx> CodeGen<'ctx>` 块；跨子模块方法使用 `pub(crate) fn`
- HT 常量与 `define_hash_table` 留在 `hash_table/mod.rs`
- `compile_hir` / `compile_hir_stmt` / `compile_hir_expr` 分发留在 `hir_compile/mod.rs`
- lazy call_args 薄包装留在 `builtins/lazy/mod.rs`

---

## 验证结果

```bash
nix-shell --run 'cargo fmt --all'
nix-shell --run 'cargo build --release'
nix-shell --run 'cargo test --test integration -- --test-threads=1'
nix-shell --run 'cargo test -p action-codegen --lib'
./target/release/action run examples/bench_cow.ac
./target/release/action run examples/bench_all.ac
nix-shell --run 'bash scripts/ci-linux.sh core'
```

---

## 完成后 Top 可编辑源文件（不含 body.inc.rs 生成物）

```
2260  runtime_decl/define_hash_table.rs  → 已拆为 hash_table/（最大 643 行）
1877  builtins/stdlib/datetime.rs
1700  builtins/stdlib/collection.rs
1676  runtime_decl/define_str_adv.rs
1668  action-frontend/src/lexer.rs
1611  hir_compile.rs                    → 已拆为 hir_compile/（最大 514 行）
1508  builtins/lazy.rs                   → 已拆为 lazy/（最大 515 行）
```

---

*文档版本：v1.0 · R5 完成态*
