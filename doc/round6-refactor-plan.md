# 第六轮重构计划书（2026-06-23）

> **状态**：✅ 已完成（2026-06-23）  
> **目标**：在 R5 基础上继续拆分剩余 >1500 行 codegen 源文件（stdlib builtins + string runtime）。  
> **约束**：语言语义不变；172 integration 全绿。

---

## 背景

R5 完成后仍有多处单文件 >1500 行：

| 文件 | 行数 | 问题 |
|------|------|------|
| `builtins/stdlib/datetime.rs` | 1877 | 日期/时间/随机数 codegen 全挤一处 |
| `builtins/stdlib/collection.rs` | 1700 | List/Map/Set collection builtins |
| `runtime_decl/define_str_adv.rs` | 1676 | split/join/replace 等字符串运行时 IR |

R6 聚焦上述三处，沿用 R4/R5 的「多 `impl CodeGen` 子模块 + `pub(crate)` 跨文件可见」约定。

---

## 待办项

| 项 | 状态 | 说明 |
|----|------|------|
| **R6-1** `stdlib/collection/` | ✅ | list_basic / list_gen / list_misc / list_transform / map_set / aggregate；最大 list_basic.rs 609 行 |
| **R6-2** `stdlib/datetime/` | ✅ | format_parse / construct / random / accessors / weekday_utc / today_now；最大 construct.rs 508 行 |
| **R6-3** `runtime_decl/str_adv/` | ✅ | split / join / replace / contains / repeat / trim_start / trim_end；最大 replace.rs 489 行 |
| **R6-4** 稳定性测试 | ✅ | `runtime_defines_str_split`、`codegen_stdlib_collection_sum`、`codegen_stdlib_datetime_rand` |
| **R6-5** 文档 | ✅ | 本文件 + `ARCHITECTURE.md` R6 行与 ownership 表 |
| **R6-6** 验证与 CI | ✅ | fmt / build / integration / 语义快检 / `ci-linux.sh core` / push / CI 绿 |

---

## 拆分约定（继承 R5）

- stdlib collection/datetime：`mod.rs` 链式 `dispatch_*` 返回 `Option<TypedValue>`，子模块各持一段 `match name`
- str_adv：`mod.rs` 调用 `define_str_*`；各子模块自包含 preamble（`i64`/`str_ty`/C 函数句柄）
- 拆分脚本：`scripts/r6_split_stdlib.py`、`scripts/r6_split_str_adv.py`

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
1653  runtime_decl/define_misc.rs
1668  action-frontend/src/lexer.rs
1263  runtime_decl/list/define_list_xform.rs
1226  codegen/pattern.rs
1192  action-frontend/src/ast.rs
 609  builtins/stdlib/collection/list_basic.rs   → 已拆
 508  builtins/stdlib/datetime/construct.rs      → 已拆
 489  runtime_decl/str_adv/replace.rs            → 已拆
```

---

*文档版本：v1.0 · R6 完成态*
