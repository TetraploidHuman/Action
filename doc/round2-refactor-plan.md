# 第二轮重构计划书（2026-06-23）

> **目标**：解决 R1–R9 拆分后仍存在的功能分散、命名混淆与模块边界不清问题，提升高内聚低耦合，**不启动自举**。  
> **验收**：172 项 integration 全绿；语义快检不变；GitHub Actions Linux + Windows 绿。

---

## 背景与问题诊断

| 区域 | 现状问题 | 影响 |
|------|----------|------|
| **根 `src/lib.rs`** | 通过 `#[path]` 二次编译 `host-rt` 三文件 | 与 `crates/host-rt` 重复、JIT/AOT 双轨维护 |
| **`action-codegen/lib.rs`** | ~915 行，Scope/TypedValue/CodeGen 混在同一文件 | 难以导航，子模块边界模糊 |
| **`runtime_decl/`** | 15 个 `define_list_*.rs` 与 map/str 平铺 | List 运行时逻辑碎片化，无领域聚合 |
| **`builtins_*.rs`** | 18 个平铺文件，`builtins_stdlib_*` 与 `builtins_*` 命名不一致 | codegen 入口 `lib.rs` 模块列表过长 |
| **`frontend/registry.rs`** | 与 `builtin/registry.rs` 同名概念不同（TypeRegistry vs BuiltinDef） | 新人易混淆「registry」含义 |
| **`typecheck/infer.rs`** | 与 `inference.rs`（HM 引擎）职责边界不直观 | 类型推断代码难定位 |

**不在本轮范围**：自举（B0+）、重写 `define_list_core.rs` 算法、benchmark 语义变更、LLVM PassManager 独立管线。

---

## 重构项（R2-1 … R2-6）

### R2-1 host-rt 依赖统一 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| 根 `Cargo.toml` 增加 `action-host-rt` path 依赖 | 替代 `#[path]` 二次编译 | ✅ |
| `host-rt` 增加 `rlib` crate-type | JIT 链入符号 | ✅ |
| `src/lib.rs` 删除三处 `#[path]` | `use action_host_rt as _` | ✅ |

### R2-2 CodeGen 核心类型提取 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| `scope.rs` | ValKind / ScopeVar / Scope | ✅ |
| `typed_value.rs` | InnerType / TypedValue | ✅ |

### R2-3 runtime_decl/list 子模块 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| `runtime_decl/list/` 聚合 15 个 list 定义文件 | 领域聚合 | ✅ |

### R2-4 frontend type_registry 重命名 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| `registry.rs` → `type_registry.rs` | `pub use type_registry as registry` | ✅ |

### R2-5 builtins 模块树 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| `builtins/mod.rs` + `builtins/stdlib/` | call/iter/list/stdlib 子树 | ✅ |

### R2-6 typecheck 模块澄清 ✅

| 任务 | 说明 | 状态 |
|------|------|------|
| `infer.rs` → `expr_infer.rs` | 模块文档注释 | ✅ |

---

## 执行顺序与依赖

```
R2-1 host-rt ──┐
R2-2 scope ────┼──► R2-3 list ──► R2-5 builtins ──► 全量测试
R2-4 type_reg ─┘         ▲
R2-6 typecheck ──────────┘
```

---

## 验证清单

```bash
nix-shell --run 'RUSTFLAGS="-D warnings" cargo build --release'
nix-shell --run 'cargo test --test integration -- --test-threads=1'   # 172 passed
./target/release/action run examples/bench_cow.ac      # 11
./target/release/action run examples/map_filter.ac     # 210215
./target/release/action run examples/bench_all.ac      # 无 SIGSEGV
```

---

## 风险与回滚

| 风险 | 缓解 |
|------|------|
| staticlib 链接遗漏 JIT 符号 | 保留 integration 中 HTTP/JSON/thread 用例 |
| 文件移动破坏 `build.rs` rerun-if-changed | 路径仍指向 `crates/host-rt/` |
| List 运行时 impl 块路径错误 | 仅改 `use` 路径，不改 LLVM IR 逻辑 |

每完成 R2-x 可独立 commit；全部完成后单次 push 触发 CI。

---

## 第三轮展望（超出本轮，不实施）

- `define_list_core.rs` 按 walk/mutate/split 进一步拆分
- `CodeGen::define_runtime` 声明与 `runtime_decl` 生成解耦
- `action-driver` 拆 emit 子模块
- 自举 M4–M6（见 `doc/bootstrap-subset.md`）
