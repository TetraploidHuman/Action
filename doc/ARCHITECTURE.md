# Action 编译器架构

> 本文描述重构后的编译器分层与模块边界。重构目标：清晰依赖、可测试、为前端自举与 HIR 引入做准备。

## 分层概览

```
┌──────────────────────────────────────────────────────────────────┐
│  action-cli（Binary Layer）                                       │
│  crates/action-cli/src/main.rs · repl.rs · test_runner.rs         │
│  编排：load → compile → run / emit / diagnose / fmt / lsp        │
└───────────────────────────┬──────────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────────┐
│  action-driver crate（编排层）                                    │
│  compile_checked · load_checked · emit_hir · emit_diagnostics_json │
│  依赖 frontend + codegen，无二进制耦合                             │
└──────────┬─────────────────────────────────────┬─────────────────┘
           │                                     │
           ▼                                     ▼
┌───────────────────┐                 ┌──────────────────────┐
│  action-frontend  │                 │  action-codegen      │
│  无 LLVM 依赖      │                 │  inkwell + runtime   │
│                   │    AST +        │  JIT / AOT           │
│  lexer parser     │  TypeRegistry   │                      │
│  ast types        │ ──lower──► HIR  │  compile_hir (HIR)   │
│  typecheck loader │                 │                      │
│  builtin fmt      │                 │                      │
└─────────┬─────────┘                 └──────────────────────┘
          │
          ▼
┌───────────────────┐     ┌───────────────────┐
│  action-span      │     │  action-lsp         │
│  源码位置（零依赖） │     │  仅依赖 frontend    │
└───────────────────┘     └───────────────────┘
```

## 目录结构

```
crates/
  action-span/              # Span 类型（零依赖）
  action-frontend/          # lex → parse → typecheck → lower → HIR
    src/
      ast.rs lexer.rs parser/ typecheck/ loader/ hir/
      builtin/registry.rs   # stdlib + UFCS 类型表
      type_registry.rs      # Struct/Enum TypeRegistry（`registry` 为兼容别名）
      session.rs error.rs fmt.rs checked.rs
  action-codegen/           # LLVM IR + runtime_decl + JIT/AOT
    src/
      scope.rs typed_value.rs   # CodeGen 核心类型
      compile.rs hir_compile.rs call_arg.rs call_hir.rs ufcs.rs
      builtins/               # call / iter / list / stdlib / …
      runtime_decl/           # define_* runtime IR
        list/                 # List B-tree 领域聚合
  action-driver/            # load_checked · compile_checked · emit_*
  action-lsp/               # LSP 语言服务
  host-rt/                  # AOT + JIT host runtime（libaction_host_rt.a）
    lib.rs runtime_json.rs http_runtime.rs runtime_threading.rs
  runtime-bc-emit/          # 运行时 bitcode 发射工具
  action-cli/               # CLI 二进制源码（根 [[bin]] 指向此处）
    src/
      main.rs repl.rs test_runner.rs

src/
  lib.rs                    # 向后兼容 facade（re-export 各 crate）

include/
  action_rt.h               # Runtime C ABI（scripts/generate_action_rt_header.py）

tests/
  integration.rs            # 语义 oracle（172 项）
  hir_golden.rs
  lexer_golden.rs
  bootstrap_subset.rs
  diagnostics_json.rs
  fixtures/                 # 多文件 compile-error 夹具
```

## 依赖规则

| 层 | 允许依赖 | 禁止依赖 |
|----|----------|----------|
| `action-span` | std | 其它 crate |
| `action-frontend` | `action-span`, 内部, `toml`, `ariadne`, `serde` | `action-codegen`, `inkwell` |
| `action-codegen` | `action-frontend`, `inkwell` | — |
| `action-driver` | `action-frontend`, `action-codegen` | — |
| `action-lsp` | `action-frontend`, `lsp-server`, `lsp-types` | `action-codegen`, `inkwell` |
| `action-cli`（bin） | 全部 workspace crate | — |
| `action`（lib facade） | 全部（re-export only） | — |

**关键约束：** `typecheck` 不得 `use action_codegen`。builtin 元数据在 `frontend/builtin/`，codegen 经 re-export 读取。

## 公共 API（crate 根 re-export）

`src/lib.rs` 继续导出历史路径：

```rust
pub use action_frontend::{ast, lexer, parser, typecheck, loader, error, ...};
pub use action_codegen as codegen;
pub use action_driver as driver;
pub use action_lsp as lsp;
```

新代码应优先使用：

- `action_frontend::session::FrontendSession`
- `action_frontend::loader::load_program`
- `action_codegen::CodeGen`
- `action_driver::{compile_checked, emit_hir, emit_diagnostics_json}`

## 编译流水线

1. **`loader::load_checked(path)`** / **`load_program`**  
   读文件 → lex → parse → 注入 stdlib/builtins → resolve imports → typecheck → **`lower_program` → HIR**

2. **`CodeGen::compile_checked(checked)`** → **`compile_hir(&checked.hir)`**  
   生产路径**仅 HIR**（AST codegen 已删除）；表达式携带 `ty`；链接 runtime → LLVM Module

3. **执行 / 发射**  
   - JIT：`run_jit()`  
   - AOT：`emit_object` / `--emit exe` + `libaction_host_rt.a`（`host-rt` crate）  
   - HIR：`action check --emit hir` → `<file>.hir.json`  
   - 诊断 JSON：`action check --format json` / `--emit diagnostics`

## 重构阶段

| 阶段 | 内容 | 状态 |
|------|------|------|
| R1–R7 | frontend/backend 拆分、HIR、driver、LSP 统一 | ✅ |
| P1 | `Expr { kind, span }` 结构迁移 | ✅ |
| **R8** | AST codegen 删除；registry 统一；HIR golden；token-aware fmt | ✅ |
| **R9a** | `host-rt` 迁入 `crates/host-rt/src/` | ✅ |
| **R9b** | CLI 源码迁入 `crates/action-cli/`；根 lib 纯 facade | ✅ |
| **R10** | host-rt 依赖统一；codegen scope/builtins/list 模块树；type_registry 重命名 | ✅ |
| **R3** | 健康度：list core/tree 拆分、CodeGen 状态分组、runtime extern 提取、LSP handlers 拆分、driver emit、测试补强 | ✅ |
| **R4** | 代码质量与稳定性：完成 R3-7 次级 codegen 拆分、LSP helpers 再拆、push.inc 拆分、build 集成 list concat、稳定性测试 | ✅ |
| **R5** | 运行时与 HIR 大文件拆分：`hash_table/`、`hir_compile/`、`builtins/lazy/`、稳定性测试 | ✅ |
| **R6** | stdlib 与字符串运行时拆分：`collection/`、`datetime/`、`str_adv/`、稳定性测试 | ✅ |
| **B0** | Bootstrap 语言子集（`doc/bootstrap-subset.md`） | ✅ |

## 模块 ownership（改 API 需动哪些文件）

| 能力 | 类型表 / 前端 | Codegen 调度 | Runtime IR | Host |
|------|---------------|--------------|------------|------|
| **List** | `frontend/builtin/registry.rs` | `codegen/builtins/list.rs` | `runtime_decl/list/core/*.inc.rs` + `tree/*.inc.rs`（`build.rs` 生成 `body.inc.rs`） | — |
| **Iter / mono** | 同上 | `builtins/iter/` · `mono/` | `action_list_*_walk` in runtime_decl | — |
| **Expr / for** | 同上 | `expr/` · `for_loop/` | — | — |
| **Map** | 同上 | `builtins/` + `map_set.rs` | `define_map.rs`, `hash_table/` | — |
| **HIR compile** | 同上 | `hir_compile/` | — | — |
| **LazyList** | 同上 | `builtins/lazy/` | `define_lazy_list.rs` | — |
| **String** | 同上 | `builtins/stdlib/` | `str_adv/`、`define_str_*.rs` | — |
| **Builtin UFCS** | `registry.rs` | `ufcs.rs`, `call_hir.rs` | `runtime_decl/` | `host-rt/`（JSON/HTTP） |

详见 `doc/stdlib-layers.md` 四层模型。

## 性能优化（P2）

| 项 | 说明 | 状态 |
|----|------|------|
| `insert_rec` 路径拷贝 | 中间索引 insert（`li_split_bb` 优先 `action_list_insert_rec`，null 回退 concat） | ✅ |
| `remove(0)` 快速路径 | h>0 树 `remove(0)` → `drop(list,1)` | ✅ |
| `list_get_cached` | for / reduce / iter 序贯 get 缓存 | ✅ |
| ConcatNode balance | depth > 32 flatten | ✅ |
| Map Robin-Hood | 40B entry + probe | ✅ |
| Lambda mono / fused iter | map+filter / flatMap+filter 单遍 | ✅ |
| AOT LTO | `atom.toml` `lto = true` → `-flto` | ✅ |

## 语义测试覆盖（P0/P3）

| 类别 | 说明 | 覆盖 |
|------|------|------|
| List/Map CoW | 写时复制、共享引用隔离、语句形式 mutating UFCS | ✅ + `test_map_cow_properties` / `test_collection_stmt_mut` / `test_list_cow_property` / `test_insert_exit` / `test_list_alias_*` |
| compile-error oracle | import 循环/非法名、泛型、重载 | ✅ |
| diagnostics JSON | `tests/diagnostics_json.rs` | ✅ |
| Lexer / bootstrap 子集 | golden token、允许/禁止夹具 | ✅ `lexer_golden.rs` / `bootstrap_subset.rs` |
| Nullable / UFCS / TCO / 泛型 | 见 integration.rs | ✅ |

## 测试纪律

```bash
nix-shell --run 'cargo test --release --test integration -- --test-threads=1'
nix-shell --run 'cargo test --release --test diagnostics_json -- --test-threads=1'
./target/release/action run examples/bench_cow.ac   # 预期 11
```

**CI（`.github/workflows/ci.yml`）**

Linux 侧**全部**在 **自托管 NixOS runner** 上执行，开发/CI 环境由仓库根目录 `shell.nix` 提供（LLVM 21 + Rust），**不**使用 GitHub `ubuntu-latest` 或系统 apt 装 LLVM。

| Job | Runner 标签 | 入口 |
|-----|-------------|------|
| Linux CI | `[self-hosted, linux, ci]` | `nix-shell --run "bash scripts/ci-linux.sh core"` |
| Linux Proptest | `[self-hosted, linux, ci]` | `nix-shell --run "bash scripts/ci-linux.sh proptest"` |
| Linux Benchmark | `[self-hosted, linux, benchmark]` | `nix-shell --run "bash scripts/ci-linux.sh benchmark"` |
| Windows CI | `windows-2025`（GitHub hosted） | 下载 LLVM 21 预编译包 + `cargo test` |

`scripts/ci-linux.sh` 在 **nix-shell 内**运行：`fmt`/clippy、194 项 integration、debug/release 冒烟（`bench_cow` / insert 系列 / `test_insert_exit` / `test_cow_insert_isolation` 等）。Benchmark job 另跑全量 JIT/AOT + `benchmark_regression.py`。

持久化编译缓存：`CARGO_TARGET_DIR` 指向 runner 本地目录（如 `~/桌面/Runner/ci-target`），与开发者本机 `nix-shell` 行为一致。

集成测试 **194 项**为语义权威；重构不得降低通过数。类型标注使用 **colon 语法**（`val x: Int = 1`），与 bootstrap 子集及 `doc/language-spec-outline.md` 一致。

## 与自举的关系

- **可移植到 Action 的**：`action-frontend/`（~10K LOC 等价）
- **长期留 Rust 的**：`action-codegen/` + runtime IR + `host-rt`
- **自举对接点**：HIR JSON（`--emit hir`）→ Rust `compile_hir`

详见 `doc/roadmap-and-bootstrap-analysis.md`、`doc/bootstrap-subset.md`（M4–M6）、`doc/language-spec-outline.md`、`doc/stdlib-layers.md`。
