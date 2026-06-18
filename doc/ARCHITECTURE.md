# Action 编译器架构

> 本文描述重构后的编译器分层与模块边界。重构目标：清晰依赖、可测试、为前端自举与 HIR 引入做准备。

## 分层概览

```
┌──────────────────────────────────────────────────────────────────┐
│  Binary Layer（main binary）                                     │
│  main.rs · repl.rs · test_runner.rs · lsp/                      │
│  编排：load → compile → run / emit / diagnose                    │
└───────────────────────────┬──────────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────────┐
│  action-driver crate（编排层）                                    │
│  compile_checked · load_checked · emit_hir · effective_opt_level │
│  依赖 frontend + codegen，无二进制耦合                             │
└──────────┬─────────────────────────────────────┬─────────────────┘
           │                                     │
           ▼                                     ▼
┌───────────────────┐                 ┌──────────────────────┐
│  action-frontend  │                 │  action-codegen      │
│  无 LLVM 依赖      │                 │  inkwell + runtime   │
│                   │    AST +        │  JIT / AOT           │
│  lexer parser     │  TypeRegistry   │                      │
│  ast types        │ ──────────────► │                      │
│  typecheck loader │                 │                      │
│  builtin fmt      │                 │                      │
└─────────┬─────────┘                 └──────────────────────┘
          │
          ▼
┌───────────────────┐
│  action-span      │  源码位置（零依赖，lexer/ast/error 共用）
└───────────────────┘
```

## 目录结构

```
src/
  span.rs                 # Span：字节偏移 + 行列
  lib.rs                  # 层间 re-export（向后兼容 crate::lexer 等路径）

  frontend/               # 编译器前端
    mod.rs
    ast.rs
    lexer.rs
    types.rs              # unify / mangle / types_compatible
    registry.rs             # TypeRegistry · StructInfo · EnumInfo
    exhaustive.rs           # when 穷尽性检查
    session.rs              # FrontendSession（loader / LSP 共用）
    typecheck/              # TypeChecker + infer / check_stmt
      mod.rs
      infer.rs
      check_stmt.rs
    loader/
      mod.rs
      resolve.rs
      stdlib.rs
    parser/
      mod.rs
      expr.rs
      stmt.rs
      type_parse.rs
      pattern.rs
    hir/                    # typed IR (HIR) — bootstrap boundary
      mod.rs
      nodes.rs
      lower.rs
      to_ast.rs
    checked.rs              # CheckedProgram { ast, registry, hir }
    error.rs              # CompilerError + ariadne 报告
    fmt.rs
    config.rs             # atom.toml
    builtin/              # builtin 类型表（与 codegen 解耦循环依赖）
      mod.rs
      registry.rs         # BuiltinDef · lookup · UFCS

  backend/
    mod.rs
    codegen/              # 原 src/codegen/（LLVM IR + runtime_decl）

  driver/                  # Bridge: re-exports from action-driver crate
    mod.rs                  #   → action_driver::{compile_checked, load_checked, emit_hir, ...}

  lsp/                    # 语言服务（依赖 frontend，暂不迁入 frontend/）
  repl.rs
  test_runner.rs
  main.rs
  http_runtime.rs         # AOT/JIT host 符号
  runtime_json.rs
  runtime_threading.rs

crates/
  action-span/            # Span 类型（零依赖）
  action-frontend/        # 前端：lex → parse → typecheck → HIR
  action-codegen/         # 后端：LLVM IR + runtime_decl + JIT/AOT
  action-driver/          # 编排：load_checked · compile_checked · emit_hir（依赖 frontend + codegen）
  host-rt/                # AOT host runtime（libaction_host_rt.a）
  runtime-bc-emit/        # 运行时 bitcode 发射工具
```

## 依赖规则

| 层 | 允许依赖 | 禁止依赖 |
|----|----------|----------|
| `span` | std | 其它 crate 模块 |
| `frontend/*` | `span`, `frontend` 内部, `toml`, `ariadne` | `backend`, `inkwell` |
| `backend/codegen` | `frontend`（经 `crate::ast` 等 re-export）, `inkwell` | — |
| `driver` (bridge) | `action-driver` crate | `frontend` / `backend` 直接依赖 |
| `lsp` / `repl` | `frontend`, `backend` | — |
| `main` | 全部 | — |

**关键约束：** `typecheck` 不得 `use backend::codegen`。builtin 元数据放在 `frontend/builtin/`，codegen 通过 `crate::builtin_registry`（alias）读取。

## 公共 API（crate 根 re-export）

为保持现有代码与测试不变，`lib.rs` 继续导出：

```rust
pub use frontend::{ast, lexer, parser, typecheck, loader, error, ...};
pub use backend::codegen;
pub use frontend::builtin as builtin_registry;  // 历史别名
pub use span::Span;
```

新代码应优先使用：

- `action::frontend::session::FrontendSession`
- `action::frontend::loader::load_program`
- `action::backend::CodeGen`
- `action::span::Span`

## 编译流水线

1. **`loader::load_checked(path)`** / **`load_program`**  
   读文件 → lex → parse → 注入 stdlib/builtins → resolve imports → typecheck → **`lower_program` → HIR**

2. **`CodeGen::compile_checked(checked)`** → **`compile_hir(&checked.hir)`**  
   Release codegen reads HIR directly (typed expressions carry `ty`); `compile(&Program)` is test-only. Link runtime → LLVM Module

3. **执行 / 发射**  
   - JIT：`run_jit()`  
   - AOT：`emit_object` / `--emit exe` + `libaction_host_rt.a`  
   - 自举调试：`action check --emit hir` / `action run --emit hir` → `<file>.hir.json`

## 重构阶段（进行中）

| 阶段 | 内容 | 状态 |
|------|------|------|
| R1 | `frontend/` + `backend/` 目录 + `span` 提取 | ✅ |
| R2 | `builtin` 迁入 frontend；`BuiltinDispatch` 拆至 `backend/codegen/builtin_dispatch.rs` | ✅ |
| R3 | `ParseError` 携带 `Span`；`load_program` 用 `to_compiler_error()` | ✅ |
| R4 | `TypeRegistry` 从 `typecheck.rs` 拆至 `frontend/registry.rs` | ✅ |
| R4b | `typecheck` 拆为 `infer` / `check_stmt`；`exhaustive.rs` | ✅ |
| R4c | `parser.rs` 拆为 `expr` / `stmt` / `type_parse` / `pattern` | ✅ |
| R4d | `loader` 拆为 `resolve` / `stdlib` | ✅ |
| R5 | 引入 HIR（`frontend/hir/`）作为 AST→codegen 边界 | ✅ |
| R5b | release 跳过 HIR round-trip；`driver/` 统一编排 | ✅ |
| R5c | `--emit hir` CLI；examples HIR golden 测试 | ✅ |
| R5d | Codegen reads HIR directly (`compile_hir`); REPL via `compile_checked` | ✅ |
| R6 | Cargo workspace：`action-frontend` / `action-codegen` 独立 crate | ✅ |
| R6b | driver 独立 crate (`action-driver`)：`compile_checked` / `load_checked` / `emit_hir` | ✅ |
| R5e | 去掉 HIR bridge，`compile_hir_expr/stmt` 原生 dispatch；`infer_return_type` 替换为 `body.ty` | ✅ |
| R7 | REPL 使用 `error::report_compiler_errors` 诊断路径，消除 regex re-parse | ✅（LSP 已接入；REPL 用 `compile_checked`） |

## 性能优化（P2）

| 项 | 说明 | 状态 |
|----|------|------|
| ConcatNode balance | `depth > 32` 时 flatten；修复 `cc_small_merge` 双 leaf 判定 | ✅ |
| Map Robin-Hood | 40B entry + probe distance；insert/get/rehash | ✅ |
| Lambda mono | map/filter btree walk；fold/any/all 单态化 | ✅ |
| Fused map+filter | 单遍组合 `map(f) |> filter(g)` 避免中间 ListNode 分配 | 🔄 |
| `list_get_cached` helper | 对 `map(lst) { it.f }` 等场景缓存 `get` 遍历位置 | 🔄 |

## 语义测试覆盖（P0）

| 类别 | 说明 | 覆盖 |
|------|------|------|
| List/Map CoW | 写时复制语义、共享引用隔离、`rc == 1` 原地更新 | ✅ |
| Nullable 类型 | `T?` 传播、智能转换、Elvis、`or {}`、方法链自动短路 | ✅ |
| 方法链 / UFCS | `lst.remove(0).len()` 无 SIGSEGV，receiver 不被 double-eval | ✅ |
| Concat 树操作 | append/insert/remove/take/flatten 在深度 concat 树上正确 | ✅ |
| TCO（尾调用） | 递归深度 > 5000 不栈溢出 | ✅ |
| 泛型函数 | 类型参数推断、多态函数调用 | ✅ |

## 测试纪律

每次结构性改动后必须：

```bash
nix-shell --run 'cargo test --release --test integration -- --test-threads=1'
./target/release/action run examples/bench_cow.at   # 预期 11
```

集成测试 **140+ 项**（含 `bench_concat_depth`、`test_cow_properties`、`test_ufcs_chain` 等）为语义权威；重构不得降低通过数（除非修复既有 bug 并更新测试）。

## 与自举的关系

- **可移植到 Action 的**：`frontend/` 全部（~10K LOC 等价）
- **长期留 Rust 的**：`backend/codegen/` + runtime IR + host runtime
- **自举对接点**：未来 `frontend/hir/` 序列化 → Rust codegen 消费

详见 `doc/roadmap-and-bootstrap-analysis.md`。
