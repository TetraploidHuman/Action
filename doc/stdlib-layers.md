# 标准库分层

Action 运行时与标准库分四层；自举前端只需理解 **builtin + lib/**，codegen/host 长期留 Rust。

## 职责表

| 层 | 路径 | 职责 | 加载方式 | 自举可用 |
|----|------|------|----------|----------|
| **builtin** | `crates/action-frontend/src/builtin/registry.rs` + `action-codegen` `builtins_*.rs` | 语言内置类型（`List`/`Map`/`String`/`Int`…）、UFCS 方法表、LLVM dispatch 元数据 | 编译器自动注入 | 类型表可移植；IR 生成留 Rust |
| **lib/** | `lib/math.ac`, `lib/json.ac` | 纯 Action 或薄 FFI 包装的用户库 | `loader` 默认注入（与 builtin 合并） | ✅ 纯 Action 部分可重写 |
| **stdlib/** | `stdlib/*.atom` | 包清单 + `external fun` 声明（I/O、HTTP、线程） | 用户 `import` / 路径依赖 | ❌ 首版 bootstrap 禁止 import |
| **host-rt** | `crates/host-rt/` | AOT/JIT 链接的 C ABI 运行时：`libaction_host_rt.a`（JSON、HTTP、线程、GC 辅助） | 链接期 | ❌ 长期 Rust |

## 数据流

```
源码 .ac
  → loader 注入 builtin 类型 + lib/math.ac + lib/json.ac
  → resolve imports（stdlib/*.atom 若用户 import）
  → typecheck（registry 统一查 builtin + 用户定义）
  → lower → HIR
  → compile_hir → 调用 builtins_* / runtime_decl
  → JIT 或 AOT + libaction_host_rt.a
```

## 边界规则

1. **语义单一来源**：builtin 签名在 `frontend/builtin/registry.rs`；codegen 只读 re-export，禁止 typecheck 依赖 codegen。
2. **lib/** 证明纯 Action stdlib 可行；新纯函数优先放 `lib/` 而非 Rust。
3. **stdlib/*.atom** 仅声明；实现必须在 host-rt 或 `external fun`。
4. **Bootstrap M4–M6** 仅允许 `doc/bootstrap-subset.md` 特性；禁止 `import` / `external fun` / `lazy val`。

## 相关文档

- `doc/bootstrap-subset.md` — 自举语言子集
- `doc/ARCHITECTURE.md` — crate 分层
- `doc/language-spec-outline.md` — 语义与集成测试索引
