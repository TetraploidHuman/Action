# 第四轮重构计划书（2026-06-23）

> **状态**：✅ 已完成（2026-06-23）  
> **目标**：在 R3 基础上提升**代码质量与稳定性**——完成遗留大文件拆分、构建链自动化、稳定性回归测试。  
> **约束**：语言语义不变；172 integration 全绿；禁止 SubAgent（主 Agent 串行执行）。

---

## 背景

R3 完成后仍有多处单文件 >1500 行，且 `body.inc.rs` 依赖手工运行 `concat_list_body.py`。R4 聚焦：

1. 完成 R3-7 遗留的 codegen 大文件拆分（`iter` / `mono` / `expr` / `for_loop`）
2. LSP `helpers.rs` 按功能再拆
3. `push.inc.rs` 拆分 + `build.rs` 自动生成 list body
4. 稳定性测试与架构文档同步

---

## 待办项

| 项 | 状态 | 说明 |
|----|------|------|
| **R4-1** `builtins/iter/` | ✅ | 10 个子模块（map / fuse / filter / fold_core / any_all / find / reduce / advanced / extract / callback）；最大文件 ≤720 行 |
| **R4-2** `mono/` | ✅ | 自 `lambda_mono.rs` 迁入；cache + 4× walk 子模块；`DirectLambdaTarget` 保留于 `mod.rs` |
| **R4-3** `expr/` | ✅ | lambda / literal / fat_return / binop / coerce + `mod.rs` 自由函数 |
| **R4-4** `for_loop/` | ✅ | store / iterate / cache / hir；`ForExprSrc` 保留于 `mod.rs` |
| **R4-5** LSP `helpers/` | ✅ | completion / scope / signature / tests；`pub(crate) use *` 保持 handler 兼容 |
| **R4-6** `push.inc` 拆分 | ✅ | `push_head.inc.rs` + `push_tail.inc.rs`（各 ~780 行） |
| **R4-7** `build.rs` 集成 | ✅ | `rerun-if-changed` + 构建前调用 `concat_list_body.py` |
| **R4-8** 稳定性测试 | ✅ | `runtime_defines_list_push`、`codegen_map_emits_walk_or_mono`、`codegen_for_loop_emits_body` |
| **R4-9** 文档 | ✅ | 本文件 + `ARCHITECTURE.md` R4 行与 ownership 表 |
| **R4-10** 验证与 CI | ✅ | fmt / build / integration / 语义快检 / `ci-linux.sh core` / push / CI 绿 |

---

## 拆分约定（R4 新增）

### 多文件 `impl CodeGen`

与 list `*.inc.rs` 不同，Rust 子模块内**多个** `impl<'ctx> CodeGen<'ctx> { … }` 块会自动合并。注意：

- 片段边界必须在**完整函数**或**完整 doc+函数**处切断，避免 orphan `///`
- 跨子模块调用的方法使用 `pub(crate) fn`（原单文件 `pub(super)` 仅对父模块可见）
- 类型/枚举（如 `DirectLambdaTarget`、`ForExprSrc`）留在 `mod.rs` preamble

### List body 生成

```
list/core/*.inc.rs  ──► scripts/concat_list_body.py ──► body.inc.rs
                              ▲
                         build.rs (CARGO build 时)
```

---

## 验证结果

```bash
nix-shell --run 'cargo fmt --all'
nix-shell --run 'cargo build --release'
nix-shell --run 'cargo test --test integration -- --test-threads=1'   # 172 passed
nix-shell --run 'cargo test -p action-codegen --lib'                  # 含 R4 稳定性测试
nix-shell --run 'cargo test -p action-lsp --lib'                    # 74 passed
./target/release/action run examples/bench_cow.ac                       # 11
./target/release/action run examples/bench_all.ac                       # 无 SIGSEGV
nix-shell --run 'bash scripts/ci-linux.sh core'
```

---

## 完成后 Top 可编辑源文件（不含 body.inc.rs 生成物）

```
1877  builtins/stdlib/datetime.rs
2260  runtime_decl/define_hash_table.rs
1137  for_loop/hir.rs
1028  list/core/walk_map.inc.rs
 987  list/tree/remove.inc.rs
 849  list/core/cow.inc.rs
 780  list/core/push_head.inc.rs
 780  list/core/push_tail.inc.rs
 777  list/tree/insert.inc.rs
 720  builtins/iter/extract.rs
 708  mono/any_all_walk.rs
```

**R3-7 遗留项已全部完成**；无 >2000 行手写 codegen 源文件（生成物 `body.inc.rs` 除外）。

---

*文档版本：v1.0 · R4 完成态*
