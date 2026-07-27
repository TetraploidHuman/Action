# 语法改革契约（Syntax Reform）

> 状态：Phase 0–7 已完成（Rust 前端 + Path B 对齐 + 文档清理）。  
> 制定：2026-07-26

## 终态摘要

| 领域 | 终态写法 | 废除 |
|------|----------|------|
| 记录类型 | `type Point { x: Int; y: Int; fun … }` | `type Point = { … }`（记录）；纯别名 `type A = Int` 保留 |
| 构造 | `Point { x = 1, y = 2 }` | 匿名 `{ x = 1, y = 2 }` / `{ x, y }` |
| 方法 | type 体内 `fun`；`self` 只读；可无 `self` 关联函数 | — |
| `extension` | 保留 | — |
| 闭包 | `lambda a, b { }` / `lambda { }` / `lambda { it }` | `{ a, b -> }` 匿名闭包 |
| Trailing | 末位形参为函数类型；`fold(xs, 0) { acc, x; body }` | 依赖 `->` 的 trailing |
| 裸块 | `{}` → 立刻执行，`Unit`；非空 `{ stmts }` → 立刻执行块（末值） | `{}` 作为空元组 / 匿名空闭包 |

## 分期

| Phase | 内容 | 状态 |
|-------|------|------|
| 0 | 本契约 + 触达清单 + 基线 | ✅ |
| 1 | `TypeName { fields }` 构造；匿名 `{ x = }` 仍可解析（deprecated）；fixtures 已迁命名形式 | ✅ |
| 2 | `type Name { 字段 }` 声明体（尚无方法）；`type A = Int` 保留；旧 `type A = {…}` 仍可解析 | ✅ |
| 3 | type 体内方法 + 关联函数 | ✅ |
| 4 | `lambda` + 裸块语义 | ✅ |
| 5 | Trailing 形参行 + 类型驱动 | ✅ |
| 6 | Bootstrap Path B 对齐 | ✅ |
| 7 | 文档清理 | ✅ |

## Phase 1 触达清单（构造）

- AST / HIR / parser（`lambda.rs`、`pratt.rs`）
- typecheck（`expr_infer`、`check_stmt`）、codegen（`struct_ops` / `compile_hir_struct_lit`）
- `tests/fixtures/**/*.ac`、`examples/**/*.ac` 中匿名 `{field =` 构造
- tutorial / README 结构体章节（Phase 7 可补全；Phase 1 至少改示例可编译路径）

## Phase 1 细节（已落地）

- 语法：`Point { x = 1, y = 2 }`（类型名须 **PascalCase**，避免 `for x { s = … }` 被误解析）
- AST：`StructLiteral { type_name: Option<String>, fields }`；HIR 同步携带 `type_name`
- 匿名 `{ x = … }`：仍可解析（deprecated），fixtures/examples 已迁命名形式
- Codegen：按声明字段顺序插入（支持乱序字面量）
- 单字段简写 `Point { x }`：**不支持**（须 `Point { x = x }`），以免与块混淆

## 验证（Phase 1）

- `cargo test -p action-frontend --lib`：187 passed
- `cargo test --test integration`：206 passed
- `bench_cow.ac` → 11

## Phase 2 细节（已落地）

- 语法：`type Point { x: Int, y: Int }`（字段可用 `,` 或 `;`）
- 纯别名：`type UserId = Int` 不变
- 旧式：`type Point = { … }` 仍解析为同一 `Stmt::TypeAlias`（deprecated）
- Registry / HIR / codegen 无变：仍注册为 Named + StructInfo
- fixtures / examples 已迁新声明体；`bootstrap/*.ac` 自举源码暂留旧式至 Phase 6

## 验证（Phase 2）

- `cargo test -p action-frontend --lib`：189 passed
- `cargo test --test integration`：206 passed
- `struct.ac` / `test_struct_nested.ac` / `bench_cow.ac` 绿

## Phase 3 细节（已落地）

- `type Point { x: Int; y: Int; fun sum(self) -> Int { … } }`
- `self` 未标注时自动注入 `self: Point`；禁止 `self.x = …`
- 实例调用：`p.sum()`；关联函数：`Point::origin()`（既有 `Type::method`）
- 与 `extension` 共用方法表 / `Type_method` mangling / UFCS
- 示例：`examples/type_methods.ac`、`examples/test_error_self_field_assign.ac`

## 验证（Phase 3）

- `cargo test -p action-frontend --lib`：190 passed
- `cargo test --test integration`：208 passed
- `type_methods.ac` → `30112200`；`extension.ac` / `bench_cow.ac` 绿

## Phase 4 细节（已落地）

- 语法：`lambda a, b { … }` / `lambda { }` / `lambda { it … }`
- 裸 `{ }`：表达式位立即执行块；`{}` → `Block([])` / `Unit`（不再是空元组或空闭包）
- 表达式位 `{ x -> … }` / `{ it … }` 作为闭包：**废除**（报错或解析为块）；trailing `f(…) { … }` 仍保留至 Phase 5
- Path B：`pexpr` 软关键字双轨（`lambda … { }` + 旧 brace 闭包并存）；完整块语义对齐留给 Phase 6
- 迁移：examples + `tests/fixtures/bootstrap*lambda*`；trailing 用例未改

## 验证（Phase 4）

- `cargo test -p action-frontend --lib`：195 passed
- `cargo test --test integration`：208 passed
- `cargo test -p action-lsp --lib`：绿
- Path B：`lambda_it_ok` / `lambda_multi_ok` / `lambda_block_ok` 等 allowlist 烟测绿（软关键字双轨）
- `examples/lambda.ac` → `42423042`；`map_filter.ac` → `210215`；`bench_cow.ac` → `11`；`tutorial.ac` 绿

## Phase 5 细节（已落地）

- Trailing 形参行：`fold(xs, 0) { acc, x` + 换行 + `body }` 或 `{ acc, x; body }`（**无** `->`）
- `{ it … }` / 无参 `{ body }` trailing 保留
- `{ a, b -> body }` trailing：**废除**（迁移提示）
- 类型驱动：trailing 仅当 callee 末位形参为函数类型，或 builtin `supports_trailing_lambda` / `launch`/`coroutineScope`
- Lexer：token `span` 指向 lexeme（跳过前导空白/注释），以支持换行形参行检测
- 顺带修复：`mapFold` 循环 `mfld_nxt` 缺 terminator
- Path B trailing 仍为 `{ it }` / `{ x -> }` 双轨至 Phase 6；fixtures 未改

## 验证（Phase 5）

- `cargo test -p action-frontend --lib`：199 passed
- `cargo test --test integration`：208 passed
- `cargo test --test lexer_golden`：绿（span 指向 lexeme 后已刷新 golden）
- Path B trailing allowlist 烟测绿；`map_filter.ac` → `210215`；`map_hof.ac` → `2100`；`test_coroutine.ac` → `322`；`bench_cow.ac` → `11`

## Phase 6 细节（已落地）

- **双轨 type 声明**：`type Name { … }` 与旧式 `type Name = { … }` 并存（自举源码可逐步迁移）
- **命名构造**：`Point { x = … }` 经 PascalCase Ident 解析；HIR `StructLiteral` 携带 `type_name`
- **`lambda` 软关键字**：`lambda a, b { … }`；trailing 形参行 `{ a, b; body }`（**无** `->`）；`{ a, b -> … }` trailing **拒绝**
- **块语义（Path B）**：裸 `{}` → Unit Block；语句头 `{ stmts }` → PlainBlock；其余表达式位 `{ … }` 仍保留旧闭包路径供自举
- **自举迁移**：`bootstrap/token.ac`、`bootstrap/lexer.ac` 已迁 `type Token { … }`
- **`no_trailing_lambda`**：`if` / `when` / `for` 禁止 trailing 闭包（host slot 35）
- **踩坑**：Path B 在自举源码中对 `if a || b {` 与 `if a && b && c {` 会挂起——改用嵌套 `if` 或 `val` 绑定拆分条件
- **延后**：type 体内方法、纯别名 `type A = Int` 在 Path B 的完整对齐、Rust 侧 brace-else PlainBlock 完全 parity

## 验证（Phase 6）

- Path B smoke：`assign_point_ok` / `lambda_*_ok` / `struct_when` / `lambda_stmts_ok` 绿
- `cargo test --test bootstrap_subset`：315 passed（17 ignored subprocess twins）
- `cargo test --test integration`：208 passed
- `bench_cow.ac` → `11`；bootstrap HIR goldens 已 `--write` 刷新（`StructLiteral` 对象形）

## Phase 7 细节（已落地）

- **`README.md` / `doc/tutorial.md`**：结构体、`lambda`、多参 trailing、HTTP/Date 构造示例迁到终态写法
- **`doc/bootstrap-subset.md`**：允许表表达式行对齐 `lambda` / trailing `{ acc, x; … }`（里程碑验收行保留历史写法）
- 历史计划稿（`bootstrap-m72-plan.md` 等）不改写

## 验证（Phase 7）

- 用户可见示例与 `doc/syntax-reform.md` 终态摘要一致
- 无代码变更；语义以 Phase 0–6 测试为准

## 刻意不做（本轮改革）

- class / 继承 / `var self`
- Action 侧 codegen（除上述 mapFold terminator 修复）
- 匿名与 `TypeName { }` 长期双轨（Phase 1 结束删除匿名构造）
