# Bootstrap 语言子集（v0）

> Action-in-Action 自举编译器首版**仅允许**使用下列语言特性。随 M4–M6 里程碑逐步扩大。

## 允许

| 类别 | 特性 |
|------|------|
| 绑定 | `val` / `var` / `fun` |
| 类型 | 基本类型、`struct` / `enum`、显式类型标注 |
| 控制流 | `when`（…）、`for`（含 `for v in map` 值、`for k in map` 键（`len(k)` 启发式）、`for x in set`、`for k, v in map`、**break** / **continue**）、`return` |
| 表达式 | 二元 `+ - * / %`、逻辑 **`and` / `or`**、一元 `+ - not`、下标 `m["k"]`、字段 `.x`（可链式）、**nullary UFCS**（`x.len()` / `x.isEmpty()`，M117）、fallible **`or { … }`**（M118）、**lambda / `it` / 多参 / trailing / 无参块**（`{ it * 2 }(21)` / `{ x, y -> … }` / `map(xs) { it }` / `{ 21 * 2 }()`，M119–M123） |
| 集合 | `List` / `Map` / `Set` / `String` |
| 模块 | 单文件；flat 自举模块（`prelude`…`pscan`）裸名；**M120** 起亦可 `import` 其它 path-safe 的 `bootstrap/{name}.ac`（非 flat → `mod.fn()` / `mod_` 前缀）；环与缺文件拒绝 |
| 输出 | `print` / `println`（bootstrap 编译器均已实现） |

## 禁止（首版）

| 类别 | 特性 |
|------|------|
| 并发 | `Task` / `Stream` / 协程 |
| 高级 | `lazy val`、函数重载、扩展方法 |
| 链式 | 复杂 UFCS 方法链（带参 / 链式；nullary 单调用见 M117） |
| 类型 | 隐式 `Int` 默认（须显式标注参数/返回值） |
| 互操作 | `external fun` 仅作 host I/O / 会话 hook（prescan 进 env，**不 emit HIR**）；禁止一般 FFI |

## 对接点

- 前端 emit **HIR JSON**（`action check --emit hir`）
- Rust codegen 消费 HIR（`compile_hir`）
- 语义 oracle：`tests/integration.rs` + `tests/hir_golden.rs`

## 里程碑

| 里程碑 | 验收 |
|--------|------|
| M4 Action lexer | golden token 与 Rust lexer 一致（含 **bootstrap_keywords** 子集关键字表） |
| M5 Action parser 子集 | 解析本子集源码 → HIR |
| M6 自举 Alpha | Action 前端编译自身 lexer + **compiler** 源码（经 Rust codegen） |
| M8 Rust oracle | bootstrap HIR 与 Rust 前端结构一致 |
| M9 MCJIT | bootstrap HIR 可 JIT 执行（含 tokenize_keywords alpha） |
| M10 用户项 oracle | bootstrap 与 Rust 前端 HIR 形状一致（**33** main 夹具） |
| M11 compiler JIT | `compiler.ac` HIR → MCJIT 解析真实输入（`enum_simple.ac`） |
| M12 compiler JIT lexer | `compiler.ac` HIR → MCJIT 解析 `lexer.ac`（≥40 fun） |
| M13 compiler JIT self | `compiler.ac` HIR → MCJIT 解析 `compiler.ac`（≥196 fun，运行时自举） |
| M14 lexer JIT tokenize | `lexer.ac` HIR → MCJIT → `lexer/*.ac` token kinds 与 Rust golden 一致（多 Context 链接 runtime bitcode 时 codegen 同步 `__action_str`/`list` 类型） |
| M15 verify + stdout 闭环 | `lexer.ac`/`compiler.ac` bootstrap HIR 通过 LLVM verify；`tokenize_keywords` JIT stdout 与 M4 keywords golden 一致 |
| M16 AOT 执行 | bootstrap HIR → `emit_object` + `cc` link → 原生 exe；`jit_smoke`=42，`compiler.ac` 解析 `enum_simple.ac`=0 |
| M17 AOT 自解析 | `compiler.ac` bootstrap HIR → AOT 解析 `compiler.ac`（≥196 fun，对齐 M13） |
| M18 AOT 解析 lexer | `compiler.ac` bootstrap HIR → AOT 解析 `lexer.ac`（≥40 fun，对齐 M12） |
| M19 lexer AOT tokenize | `lexer.ac` bootstrap HIR → AOT → `lexer/*.ac` token kinds 与 M4 golden 一致（对齐 M14） |
| TC2 return 检查 | bootstrap 编译器检测 `return` Int↔Bool 不匹配（exit 1） |
| TC3 env 作用域 | global/local 分层；每函数 `envLocalClear`；未定义标识符 `return` → exit 1；`env_scope_good` JIT=154；`env_scope_leak` / `bad_undef_var` 禁止夹具 |
| TC4 return Int↔String | bootstrap 编译器检测 `return` Int↔String 不匹配；`Add` 任一 String 操作数 → tag 5；`TypeAlias` 字段表驱动 `FieldAccess` 标签；嵌套 `when` 消费后 `accReset(_arms)`；`return_string_concat.ac` 正向回归 |
| TC5 return Bool↔String | bootstrap 编译器检测 `return` Bool↔String 不匹配；`return_bool_cmp.ac` 比较表达式正向回归 |
| TC6 return Named↔primitive | bootstrap 编译器检测 Named（Token/Point/…）与 primitive 互返不匹配；struct literal 按字段名轻量推断（`{x,y}`→Point、`{kind,start,end,line,col}`→Token）；`bad_return_int_point_lit.ac` 禁止夹具；`return_token_make.ac` struct 正向回归 |
| TC7 return Named↔Named | bootstrap 编译器检测不同 Named 互返（Point vs Token）；`return_point_make.ac` 正向回归 |
| TC8 parse / when 分支 | bootstrap 编译器对 `expect` 失败、非法顶层、`when` 分支 tag 不一致 exit 1 |
| M20 Path B 文档 + TC verify | `bootstrap/README.md` Path B 流水线图；TC4–TC6 正向夹具 LLVM verify 子进程；**32** fixture AOT 返回值 oracle（对齐 M9 JIT；跳过 `infinite_for`） |

## 下一阶段（M21–M71 ✅；M72+ 见执行计划）

> **M72 及以后**：权威执行计划与状态见 [`doc/bootstrap-m72-plan.md`](bootstrap-m72-plan.md)。

| 里程碑 | 目标 | 验收（草案） |
|--------|------|--------------|
| M21 | 动态 struct 类型表 | ✅ `_struct_types.txt` + env 驱动 `tyAnnTag`；自定义 `type` 分配 tag≥8 |
| M22 | 迭代 env 扫描 | ✅ `envScan`/`envScanFound`/`structFieldScan` 改 `for` 循环 + hit 文件，降低自解析栈深 |
| M23 | `bootstrap/prelude.ac` | ✅ 共享 `keywordKind*` / `skipWsWhitespace` / `skipLineComment`；`scripts/check_bootstrap_prelude.py` 校验 `import prelude` |
| M24 | 子集 `import` 试点 | ✅ 单模块 loader（`import prelude`）+ `import_prelude.ac` 夹具 oracle |
| M25 | 前端拆分试点 | ✅ `bootstrap/parser.ac` scannerless lexer（`import parser`） |
| M26 | HIR emit 拆分 | ✅ `bootstrap/emit.ac`（`import emit`）；JSON 输出从 `compiler.ac` 切出 |
| M27 | 类型环境 + when unify 拆分 | ✅ `bootstrap/typeenv.ac` + `bootstrap/whenty.ac`；`emit.jEscape` JSON 转义 |
| M28 | 真实 span + 结构卫生 | ✅ `lineColEnsure` 前向增量 + span oracle；HIR sep / `external fun`；`sessionReset` 含 `_ty`/`_recv`/`_pat` |
| M29 | import 注册表拆分 | ✅ `bootstrap/modload.ac`（26 helpers）；`parseImport`/`preScanImport` 编排仍在 `compiler.ac`（避免对 `parseProgram` 静态环） |
| M30 | 表达式解析拆分 | ✅ `bootstrap/pexpr.ac`（79 funs）；stmt/`parseProgram` 仍在 `compiler.ac` |
| M31 | 语句/块拆分 | ✅ `bootstrap/pstmt.ac`（20 funs）；decl/import/`parseProgram` 仍在 `compiler.ac` |
| M32 | 顶层声明拆分 | ✅ `bootstrap/pdecl.ac`（13 funs）；import/`preScan*`/`parseProgram` 仍在 `compiler.ac` |
| M33 | 预扫描拆分 | ✅ `bootstrap/pscan.ac`（28 funs）；import load/`parseProgram` 仍在 `compiler.ac`（避免对 `parseProgram` 静态环） |
| M34 | 去会话文件循环驱动 | ✅ `preScanProgram`/`parseProgram` 本地 `var`；嵌套 import 只保存 span；`skipBraceDepth` 用 Int pack（无 `_sbd_*`） |
| M35 | lineCol 本地前进 | ✅ `lineColEnsure` 一次读 cursor、本地扫描、一次 flush；删除 `_lc_target`/`_lc_done` 与 per-char bump 驱动 |
| M36 | typeenv 扫描本地化 | ✅ `envScan`/`structFieldScan` 迭代返回 tag；删除 `_env_scan_hit`/`_env_scan_result` |
| M37 | per-token span 去 IO | ✅ `cur`/`advance` 不再写 `_span_start`/`_span_end`；仅 `spanSeal`/`spanMark*` 写入 |
| M38 | lex 单次写 substring | ✅ `writeFile(s.substring(..))`；宿主 `action_string_data` + `action_host_file_write` |
| M39 | 去 `_lex` 邮箱 | ✅ `lexKindAt` 直接返回 String；HIR lower 注入 fun 参数 locals（修 `when { wrap(s) }` 的 `ty=Unit`）；嵌套 substring 用相对 offset；`charAt`/string index 经 `action_string_data` |
| M40 | 路径 IO 切片完备 | ✅ `readFile`/`exists`/`deleteFile` → `action_host_file_*`(ptr,len)；`parseInt` 经 `action_string_data`；与 M38 write/append 对称 |
| M41 | 算术左结合 | ✅ 经典 Pratt：同优先级链折入 `left`（`10-3-2`→5）；`**` 仍左结合；修原先同优先级折入 `right` 导致的伪右结合 |
| M42 | env 表进内存 | ✅ `bsBufClear/Append/Get` + `action_host_bs_buf_*`；`_env_global`/`_env_local`/`_struct_fields` 不再读写文件；扫描布局仍为 `name:tag\|` |
| M43 | expr 邮箱进内存 | ✅ `bsBufSet` + slots 3/4；`setExpr`/`getExpr`/`lastExprTy*`；`tyJson` 改为纯函数去掉 `_ty.json` 往返 |
| M44 | 累加器进内存 | ✅ slots 5–18：`_arms`/`_args`/`_list`/`_map`/`_set`/`_fields` + `_args_0..7`；`emit.acc*` 改 slot API |
| M45 | span 标量进内存 | ✅ `bsIntSet/Get`；`_span_*`/`_lc_pos`/mark/nest 不再读写文件；`spanJson` 形状不变；去掉 `_span_src` nest 拷贝 |
| M46 | HIR 输出缓冲 | ✅ `bsBuf` slot 19 承载 `jOut`；`jOutBegin/Flush`；测试仍读 `bootstrap/_hir_out.json` |
| M47 | 会话 Int 标量 | ✅ `bsInt` 13–20：`type_error`/`call_depth`/`add_left_ty`/`pp_sep`/`pp_hir_emitted`/when tags/unify_off |
| M48 | 短字符串邮箱 | ✅ `bsBuf` 20–30：`_recv`/`_pat`/`_guard`/`_last_ident`/`_lit_ty`/`_undef`/`_fn_*`/`_bind_*`/`_struct_name` |
| M49 | modload/struct 表 | ✅ `bsBuf` 31–36 + `bsInt` 21–23：imports/prefix/src、struct_types/lit_names/next；`_import_scan_hit` 改 Int |
| M50 | 死 scratch 清理 | ✅ 去掉 `_jesc.txt`；`compiler`/`modload`/`typeenv` 删除未用 `write/appendFile`（`compiler` 仅保留 `readFile`） |
| M51 | 模块 dead extern | ✅ `parser`/`pexpr`/`pstmt`/`pdecl`/`whenty`/`emit`/`pscan` 去掉未用文件 IO extern；仅 `compiler`/`pscan` 保留 `readFile`，`emit` 保留 `writeFile`（flush） |
| M52 | lexer tok 标量 | ✅ `bsInt` 24–26：`tok_done`/`tok_pos`/`tok_steps`；去掉 `_tok_*.txt` 与 `atoi`；保留 `_run_source.txt` 作 harness 邮箱 |
| M53 | builtin registry 补全 | ✅ 宿主 `registry.rs`：math/assert/string/list/map·set/bsBuf·bsInt/rand*；类型检查可见；未知调用硬错误仍待 lazy/datetime 等下批 |
| M54 | lazy/HOF + 空安全一致 | ✅ 登记 lazy_*/toList/flatMap/partition/count/IO extras；`List[String]` 下标按元素类型；Call/嵌套 Index 的 R6/R7 同树判定；flatMap 调度先于 registry |
| M55 | 未知调用硬错误（E004） | ✅ `check_call`：非 type_env / 非 overload / 非 registry / 非 codegen 特例 → E004；允许 `delay`/`mapFilter`/协程 helper |
| M56 | 收紧 E004 白名单 | ✅ 登记 `delay`/`mapFilter`/`mapMapValues`/`mapFold` + Map UFCS `filter`/`mapValues`/`fold`；白名单仅留 `keywordKindOpsTail`；`withTimeout`/`coroutineScope` 走 registry |
| M57 | datetime 调用 API 入册 | ✅ 登记 `today`/`now`/`date`/`datetime`/`format`/`parseDate`/`addDays`/…；`date`/`datetime`/`parseDate` fallible；`examples/datetime_api.ac` oracle |
| M58 | datetime fallible → FallibleStruct | ✅ `date`/`datetime`/`parseDate` 改走 `FallibleStruct`（对齐 registry/`or {}`）；去掉伪 Option Enum；字段访问与 fallback 可跑通 |
| M59 | 日历正确 addDays/addHours | ✅ JDN 正反变换；月/年进位；`addHours` 隔日进位；与 `diffDays` 同源；`datetime_api.ac` 含闰年/跨月/跨日 oracle |
| M60 | parseDate 尊重 format | ✅ `sscanf` 使用调用方 format（三 `%d`）；错配走 `or {}`；去掉 unused `mangle_name` import |
| M61 | partition 返回类型诚实 | ✅ registry → `{List, List}` Struct；Index 字面量选字段；`parts[i]` 无 E006，`parts[i][j]` 仍要 `or {}` |
| M62 | E004 覆盖未知 UFCS | ✅ `x.noSuchMethod()` → E004；登记 Map UFCS `keys`/`values`/`entries`/`union` 与 codegen 重映射对齐 |
| M63 | nextInt 诚实入册 | ✅ registry → `{Random, Int}` Struct；pair Index 字段类型；codegen 正确存 Random 聚合；`examples/random_next.ac` → `52` |
| M64 | Tuple Index 硬边界 E005 | ✅ OOB / 负索引 / 非常量 → **E005**（非 E006）；合法 `parts[0]`/`p[1]` 仍无需 `or {}` |
| M65 | 未知 Named 字段 E013 | ✅ 已知 struct 上 `p.z` → **E013**；`p.x` 合法；`p.method()` UFCS 不误报；调用位点跳过字段检查 |
| M66 | 未知 when 构造器 E014 | ✅ `when c { Fake -> … }` → **E014**；未知名不计入穷尽覆盖；合法 `Red`/`Blue` 与 call 位 **E004** 不变 |
| M67 | Struct 初始化/赋值卫生 | ✅ 注解下字面量缺字段 **E015**、多余字段 **E013**（顺序无关）；`p.z = …` 赋值路径 **E013** |
| M68 | StructLiteral 期望类型闭环 | ✅ `-> Point` / `return` / 调用参数 / `p = {…}` 走同一套 E013/E015；block peel；合法乱序 OK |
| M69 | Struct 字面量顺序无关 | ✅ `find_struct_by_fields` 按字段名集合匹配；codegen 按声明下标落位；`examples/struct_reorder.ac` → `10203` |
| M70 | StructLiteral 字段值类型 | ✅ 期望 Named 下校验字段值（**E016**）；走 M68 全通道；InferVar 跳过；修正旧 Person 测试 |
| M71 | 无注解唯一形状也 E016 | ✅ `find_struct_by_fields` 命中时在 `StructLiteral` 步行上复用 M70 helper；堵住 codegen 撞 IR 的洞 |
| M72 | 多位数 tag + Named 反查 | ✅ sentinel 1000/1001；`readTagAt` 读至 `\|`；`tyTagName`/`isNamedTag` 覆盖自定义 tag；`many_structs.ac`（tag≥10 + String 字段）golden+JIT=9；`custom_struct` 绑定 ty 由 Unit→Rect |
| M73 | 调用 arity + 实参 tag（TC9） | ✅ fun sig 表；单文件 call 检查（当时 import 程序跳过；M115 起有 sig 则检查）；`bad_call_arity`/`bad_call_arg_ty`；host-rt bsBuf 37–39 / bsInt 27–30 |
| M74 | Struct 卫生 E013/E015/E016（TC10） | ✅ `structFieldRequire` / lit 缺余字段 / 字段值 tag；Point/Token 字段 seed；import 时跳过；`bad_struct_*` 四夹具 |
| M75 | 集合元素类型 + for-in 绑定 tag | ✅ List/Map/Set 字面量推断元素 tag；`collEnv` 恢复；下标/for-in 用真实 tag；`list_string`/`for_string`；codegen List[String] `len(var)`→Str |
| M76 | Driver Path-B 子集门控 | ✅ `action check --frontend bootstrap`；allowlist=`BOOTSTRAP_FIXTURE_STEMS`；`compile_hir` verify；双 oracle 抽样 |
| M77 | enum 注册表 + when 穷尽（E014） | ✅ `preScanEnum` 变体表；未知构造器 / 非穷尽 → exit 1；`when_exhaustive.ac` 正向 |
| M78 | 自定义 enum 变体→父 tag（TC11） | ✅ `enumRegisterVariants`；`custom_enum.ac`；`bad_custom_enum_*`；去掉 Color `litTag` 硬编码 |
| M79 | when guard 须 Bool | ✅ `Pat and …` value-match 探测；`tyCheckGuard`；`when_guard_bool` / `bootstrap_bootstrap_only/bad_when_guard_not_bool` |
| M80 | and/or 操作数须 Bool | ✅ `tyCheckLogical`；`bad_logical_and_int` / `bad_logical_or_int`；`logical_ops` 仍绿 |
| M81 | not 操作数须 Bool | ✅ `parseUnaryNot`+`tyCheckGuard`；`logical_not`；`bad_logical_not_int` |
| M82 | allowlist 扩容 + Rust `not` 对齐 | ✅ 四夹具入 stems/allowlist/golden；Rust `UnaryOp::Not` 须 Bool；forbidden 同步 |
| M83 | Rust when guard 须 Bool | ✅ `check_stmt` arm.guard；`bad_when_guard_not_bool` → forbidden；对齐 M79 |
| M84 | call Named 实参检查 | ✅ `callCheckArgTy`←`structFieldTyOk`；`bad_call_arg_{int_point,token_point,point_int}`；`call_point_ok` |
| M85 | let/assign Named 检查 | ✅ `tyCheckBind`；`bad_let_*`/`bad_assign_*`；`let_point_ok`/`assign_point_ok`；Map lit `lastExprTySet(7)`；Rust 对齐 |
| M86 | 字段赋值检查 | ✅ `parseAssignExprRhs`+`tyCheckBind`；`bad_field_assign_{ty,unknown}`；`field_assign_ok` |
| M87 | 集合字面量同质 tag | ✅ `noteCollElemTag`/`noteMapValueTag`+`tyCheckBind`；`bad_{list,set,map}_mixed`；`coll_homo_ok`；Rust 对齐 |
| M88 | Path B allowlist 扩容 | ✅ 五夹具入 stems/allowlist/golden；JIT/AOT oracles；allowlist 48 |
| M89 | 下标赋值类型检查 | ✅ Bootstrap M86 已覆盖；Rust Index bare List/Map→Int；`bad_*index_assign*`；`index_assign_ok` |
| M90 | index_assign Path B 入册 | ✅ `index_assign_ok` stems/allowlist/golden；JIT/AOT oracle；allowlist 48 |
| M91 | Index 键类型检查 | ✅ `tyCheckIndexKey`；`bad_list_index_key`/`bad_map_index_key`；`index_key_ok`；Rust 对齐 |
| M92 | index_key Path B 入册 | ✅ `index_key_ok` stems/allowlist/golden；JIT/AOT oracle；allowlist 49 |
| M93 | 算术操作数类型检查 | ✅ `tyCheckArith`；`bad_arith_{sub_bool,mul_string,div_bool}`；`arith_ok`；对齐 Rust |
| M94 | arith_ok Path B 入册 | ✅ `arith_ok` stems/allowlist/golden；JIT/AOT oracle（→4）；allowlist 50 |
| M95 | 比较混型检查 | ✅ `tyCheckCmp`；`bad_cmp_{lt_int_bool,gt_bool_int}`；`cmp_ok`；对齐 Rust |
| M96 | cmp_ok Path B 入册 | ✅ `cmp_ok` stems/allowlist/golden；JIT/AOT oracle（→0）；allowlist 51 |
| M97 | 一元 Neg 操作数检查 | ✅ `tyCheckNeg`；`bad_unary_neg_{bool,string}`；`unary_neg_ok`；Rust 对齐 |
| M98 | unary_neg_ok Path B 入册 | ✅ `unary_neg_ok` stems/allowlist/golden；JIT/AOT oracle（→2）；allowlist 52 |
| M99 | range 两端须 Int | ✅ `tyCheckRange`；`bad_range_{bool_end,string_start}`；`range_ok`；Rust 对齐 |
| M100 | range_ok Path B 入册 | ✅ `range_ok` stems/allowlist/golden；JIT/AOT oracle（→3）；allowlist 53 |
| M101 | if/when 条件须 Bool | ✅ `if` OneLine / ConditionChain + `tyCheckGuard`；`bad_when_{cond,chain}_int`；`when_cond_ok`；Rust 对齐 |
| M102 | when_cond_ok Path B 入册 | ✅ `when_cond_ok` stems/allowlist/golden；JIT/AOT oracle（→0）；allowlist 54 |
| M103 | for 条件须 Bool | ✅ `parseForCond`+`tyCheckGuard`；`bad_for_cond_int`；`for_cond_ok`；Rust 对齐 |
| M104 | for_cond_ok Path B 入册 | ✅ `for_cond_ok` stems/allowlist/golden；JIT/AOT oracle（→3）；allowlist 55 |
| M105 | tyCheckReturn→tyCheckBind | ✅ return 检查复用 `structFieldTyOk`；删 `tyCheckReturnNamedDistinct`；`bad_return*` 回归 |
| M106 | Add 拒 Bool | ✅ `tyCheckArith`/`BinaryOp::Add`；`bad_arith_add_bool{,_left}`；`arith_add_string_ok` |
| M107 | arith_add_string Path B 入册 | ✅ `arith_add_string_ok` stems/allowlist/golden；JIT/AOT oracle（→2）；allowlist 56 |
| M108 | 一元 Pos 拒 Bool/String | ✅ `tyCheckPos`；Rust `UnaryOp::Pos`；`bad_unary_pos_{bool,string}`；`unary_plus` 仍绿 |
| M109 | val 不可重新赋值 | ✅ `tyCheckAssignIdent`；env `name:tag:mut`；`bad_val_assign{,_expr}`；`assign_expr` 仍绿 |
| M110 | val 根 Field/Index 赋值拒 | ✅ `lvalue_root_ident` / `jsonFirstIdentName`；`bad_{field,index}_assign_val`；ok 夹具仍绿 |
| M111 | 拒未定义 Call | ✅ `callCheckUndefCallee`；`bad_undef_call`；内置/import/已声明仍绿 |
| M112 | 拒非 return undef Ident | ✅ `markUndefIdentUse` 硬拒；`bad_undef_{bind,arg,arith,var}`；import 仍软 |
| M113 | String 下标键须 Int | ✅ `tyCheckIndexKey` recvTag==5；Rust String→Int；`bad_string_index_{bool,string}` |
| M114 | string_index_ok Path B 入册 | ✅ stems/allowlist/golden；JIT/AOT oracle（→0）；allowlist 57 |
| M115 | import funSig + call 检查 | ✅ 白名单 import 的 fun 写入 funSig；有 sig 时检查 arity/实参；`import_call_ok`/`bad_import_call_*`；allowlist 58 |
| M116 | external funSig | ✅ `preScanExternal` 写入 arity/param tags；`external_call_ok`/`bad_external_call_*`；不入 Path B allowlist |
| M117 | 基本 nullary UFCS | ✅ `.method(` → `Call(FieldAccess)`；`ufcs_len_ok`→3；`bad_ufcs_unknown`；allowlist 59 |
| M118 | fallible `or {}` | ✅ `or {` → HIR `OrBlock`；`or_block_ok`→0；`bad_or_block_ty`；allowlist 60 |
| M119 | lambda / `it` | ✅ `{ it * 2 }(21)` → HIR `Lambda`+Call；`lambda_it_ok`→42；`bad_lambda_it_ty`；allowlist 61 |
| M120 | 开放 import 图（窄） | ✅ path-safe + cycle + 缺文件拒；`import_graph_ok`→42；`bad_import_{cycle,unknown}`；allowlist 62 |
| M121 | 多参 lambda | ✅ `{ x, y -> x + y }(20, 22)` →42；`bad_lambda_multi_ty`；allowlist 63 |
| M122 | trailing lambda | ✅ `map(List[21]) { it * 2 }[0] or { 0 }` →42；`bad_trailing_lambda_ty`；allowlist 64 |
| M123 | 无参 `{ expr }` lambda | ✅ `{ 21 * 2 }()` →42；`bad_lambda_block_ty`；allowlist 65 |
| M124 | 更广搜索根 | ✅ `bootstrap/`→`tests/fixtures/bootstrap/`；`import_fixtures_ok`→42；allowlist 66 |
| M125 | 多语句 `{ expr; … }` lambda | ✅ `{ 21; 21 * 2 }()`→42；`bad_lambda_stmts_ty`；allowlist 67 |
| M126 | `if`/`or {}` 多语句 PlainBlock | ✅ `if true { 21; 21 * 2 } else { 0 }`→42；`bad_if_stmts_ty`；allowlist 68 |
| M127 | `{ val …; }` PlainBlock | ✅ `{ val a: Int = 21; a * 2 }`→42；`bad_plain_block_val_ty`；allowlist 69 |
| M128 | lambda 体内 `val` | ✅ `{ 21; val a: Int = 21; a * 2 }()`→42；`bad_lambda_val_ty`；allowlist 70 |
| M129 | PlainBlock `return` | ✅ `if true { return 42; 0 }`→42；`bad_plain_block_return_ty`；allowlist 71 |
| M130 | PlainBlock 窄 `for-in` | ✅ `for i in 0..42 { s = s + 1 }`→42；`bad_plain_block_for_ty`；allowlist 72 |
| M131 | PlainBlock `for` Condition | ✅ `for s < 42 { s = s + 1 }`→42；`bad_plain_block_for_cond_ty`；allowlist 73 |
| M132 | PlainBlock `for` WithIndex | ✅ `for idx, n in List[1,2,3]`→6；`bad_plain_block_for_with_index_ty`；allowlist 74 |
| M133 | PlainBlock `for` Infinite | ✅ `for { return 42 }`→42；`bad_plain_block_for_infinite_ty`；allowlist 75 |
| M134 | PlainBlock Map `for-in` 值绑定 | ✅ `for v in Map`→15；`bad_plain_block_map_values_ty`；allowlist 76 |
| M135 | PlainBlock `break`/`continue` | ✅ break→15；continue→12；`bad_plain_block_break_ty`；allowlist 78 |
| M136 | PlainBlock Map `for-in` 键绑定 | ✅ `collEnvBind` + `len(k)`→3；`bad_plain_block_map_keys_ty`；allowlist 79 |

**M72+ 执行计划**：[bootstrap-m72-plan.md](bootstrap-m72-plan.md)（M72–M136 ✅）。

CI 维护：`scripts/check_bootstrap_goldens.sh`（`ci-linux.sh core` 内执行，防 golden drift）；`check_bootstrap_{prelude,parser,emit}.py` 校验模块 import 与 fixture 同步。

详见 `doc/roadmap-and-bootstrap-analysis.md`。
