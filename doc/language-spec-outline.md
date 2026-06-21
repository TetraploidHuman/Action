# Action 语言语义大纲（v0）

> 章节与 `tests/integration.rs` 集成测试对应；详细行为以测试输出为 oracle。

## 1. 类型系统

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| 基本类型 | `Int` / `Float` / `Bool` / `String` / `Char` | `test_number_literals`, `test_char_literal` |
| 显式标注 | `val x: Int = 42`（colon 语法） | `test_type_ann` |
| 结构体 / 枚举 | `struct` / `enum` / `when` 穷尽 | `test_struct`, `test_enum`, `test_non_exhaustive` |
| 泛型 | 类型参数、推断失败 | `test_error_generic_mismatch` |
| 函数类型 | 高阶、`::` 函数引用 | `test_fn_type`, `test_fn_ref`, `test_lambda` |

## 2. 可空与智能转换

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| `T?` 传播 | 算术、比较、链式 `?.` | `test_nullable_propagation`, `test_nullable_nested` |
| `or {}` / Elvis | 默认值、嵌套 | `test_nullable_elvis_chain` |
| 智能转换 | `when` / `if` 分支内收窄 | `test_smart_cast`, `test_smart_cast_if` |
| 模式 | nullable 模式边 | `test_nullable_pattern_edges` |

## 3. 持久化集合与 CoW

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| List 基本 | 字面量、`get`/`len`/`append` | `test_list` |
| 写时复制 | 共享绑定互不可见 | `test_list_cow_property`, `test_cow_properties` |
| Map CoW | 共享键空间隔离 | `test_map_cow_properties` |
| 语句形式变异 | `lst.append(x)` 无赋值 | `test_collection_stmt_mut` |
| 别名 + insert | 大列表 + 别名压力 | `test_insert_exit`, `test_cow_insert_isolation` |
| 别名 append/remove | 同作用域双绑定 | `test_list_alias_append`, `test_list_alias_remove` |
| 内层作用域释放 | for 体结束释放 `ins` | `test_list_alias_block` |

## 4. UFCS 与方法链

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| 方法链 | `lst.remove(0).len()` 无 double-eval | `test_ufcs_chain` |
| 扩展方法 | `extension` 块 | `test_extension` |

## 5. 控制流与模式

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| `when` / 模式 | 元组、结构体、字符串 | `test_when_match`, `test_str_match`, `test_is_match` |
| `for` / `while` | 迭代、索引 | `test_for_loop`, `test_for_with_index`, `test_nested_for` |
| 解构 | 列表 / 结构体 | `test_destructure` |
| TCO | 尾递归 | `test_tco`（若存在）或 `examples/tco.ac` 覆盖 |

## 6. 模块与错误

| 主题 | 说明 | 集成测试 |
|------|------|----------|
| import | 路径依赖、循环检测 | `test_import_cycle`, `test_import_invalid_name` |
| compile-error oracle | 重载、参数、泛型 | `test_error_*` 系列 |

## 7. HIR 与自举子集

| 主题 | 说明 | 测试 |
|------|------|------|
| HIR JSON | `--emit hir`、round-trip | `tests/hir_golden.rs` |
| Bootstrap 允许子集 | 单文件、无 import | `tests/bootstrap_subset.rs` + `tests/fixtures/bootstrap/` |
| Lexer golden | token JSON 稳定 | `tests/lexer_golden.rs` |

---

参见 `doc/ARCHITECTURE.md`、`doc/bootstrap-subset.md`、`preserve-language-semantics.mdc`。
