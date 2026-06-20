# Bootstrap 语言子集（v0）

> Action-in-Action 自举编译器首版**仅允许**使用下列语言特性。随 M4–M6 里程碑逐步扩大。

## 允许

| 类别 | 特性 |
|------|------|
| 绑定 | `val` / `var` / `fun` |
| 类型 | 基本类型、`struct` / `enum`、显式类型标注 |
| 控制流 | `when`（简单模式）、`for` / `while`、`return` |
| 集合 | `List` / `Map` / `String` |
| 模块 | 单文件，**无** `import` |
| 输出 | `print` / `println` |

## 禁止（首版）

| 类别 | 特性 |
|------|------|
| 并发 | `Task` / `Stream` / 协程 |
| 高级 | `lazy val`、函数重载、扩展方法 |
| 链式 | 复杂 UFCS 方法链 |
| 类型 | 隐式 `Int` 默认（须显式标注参数/返回值） |
| 互操作 | `external fun`（除 host I/O hook） |

## 对接点

- 前端 emit **HIR JSON**（`action check --emit hir`）
- Rust codegen 消费 HIR（`compile_hir`）
- 语义 oracle：`tests/integration.rs` + `tests/hir_golden.rs`

## 里程碑

| 里程碑 | 验收 |
|--------|------|
| M4 Action lexer | golden token 与 Rust lexer 一致 |
| M5 Action parser 子集 | 解析本子集源码 → HIR |
| M6 自举 Alpha | Action 前端编译自身 lexer 源码 |

详见 `doc/roadmap-and-bootstrap-analysis.md`。
