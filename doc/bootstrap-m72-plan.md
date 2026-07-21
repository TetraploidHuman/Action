# Bootstrap 下一阶段计划（M72+）

> 制定日期：2026-07-16  
> 前置：M4–M71 / TC1–TC8 全部 ✅；Path B（Action 前端 → HIR JSON → Rust `compile_hir`）闭环已跑通。  
> 基线（本计划启动时）：`cargo test --test bootstrap_subset -- --test-threads=1` → **77 passed / 0 failed / 17 ignored**。  
> 当前（M145 后）：**215 passed / 0 failed / 17 ignored**（232 `#[test]`）；allowlist **87** stems。

## 1. 战略定位

| 原则 | 说明 |
|------|------|
| **继续 Path B** | Bootstrap 只做前端（lex/parse/typecheck/HIR）；codegen + RC 运行时 **永久** 留 Rust |
| **主题切换** | M53–M71 多在加固 **Rust 宿主**；M72+ 重心回到 **Action 侧 typecheck 深度**，向 Phase 7（逐步退役 Rust 前端模块）推进 |
| **子集纪律** | 仍遵守 `doc/bootstrap-subset.md`；扩特性须先改子集文档再实现 |
| **双前端契约** | 并存期间以 HIR golden（剥 ty/span）+ `bootstrap_subset` 为同步边界 |

## 2. 阶段总览

```
Phase A  文档与契约对齐          ← ✅ M141
Phase B  M72–M74 类型系统深度    ← 核心推进
Phase C  M75 集合/ for-in 类型
Phase D  M76 Driver 子集门控
Phase E  M77 enum 穷尽 + E014     ← ✅
Phase F  M78 自定义 enum 变体 tag  ← ✅
Phase G  M79 when guard 须 Bool     ← ✅
Phase H  M80 and/or 操作数须 Bool   ← ✅
Phase I  M81 not 操作数须 Bool      ← ✅
Phase J  M82 allowlist + not 双端对齐 ← ✅
Phase K  M83 when guard Rust 对齐    ← ✅
Phase L  M84 call Named 实参检查     ← ✅
Phase M  M85 let/assign Named 检查   ← ✅
Phase N  M86 字段赋值 E013/E016      ← ✅
Phase O  M87 集合字面量同质 tag      ← ✅
Phase P  M88 Path B allowlist 扩容    ← ✅
Phase Q  M89 下标赋值类型检查         ← ✅
Phase R  M90 index_assign allowlist   ← ✅
Phase S  M91 Index 键类型检查         ← ✅
Phase T  M92 index_key Path B 入册    ← ✅
Phase U  M93 算术操作数类型检查       ← ✅
Phase V  M94 arith_ok Path B 入册     ← ✅
Phase W  M95 比较混型检查             ← ✅
Phase X  M96 cmp_ok Path B 入册       ← ✅
Phase Y  M97 一元 Neg 操作数检查      ← ✅
Phase Z  M98 unary_neg_ok Path B 入册 ← ✅
Phase AA M99 range 两端须 Int         ← ✅
Phase AB M100 range_ok Path B 入册    ← ✅
Phase AC M101 when 条件须 Bool        ← ✅
Phase AD M102 when_cond_ok Path B 入册 ← ✅
Phase AE M103 for 条件须 Bool         ← ✅
Phase AF M104 for_cond_ok Path B 入册 ← ✅
Phase AG M105 tyCheckReturn→tyCheckBind ← ✅
Phase AH M106 Add 拒 Bool               ← ✅
Phase AI M107 arith_add_string Path B   ← ✅
Phase AJ M108 一元 Pos 拒 Bool/String   ← ✅
Phase AK M109 val 不可重新赋值          ← ✅
Phase AL M110 val 根 Field/Index 赋值拒 ← ✅
Phase AM M111 拒未定义 Call             ← ✅
Phase AN M112 拒非 return undef Ident   ← ✅
Phase AO M113 String 下标键须 Int       ← ✅
Phase AP M114 string_index_ok Path B    ← ✅
Phase AQ M115 import funSig + call 检查 ← ✅
Phase AR M116 external funSig            ← ✅
Phase AS M117–M140 PlainBlock / lambda / Map·Set / when Path B  ← ✅（见 §3 表）
```

每完成一个编号里程碑：本机验证 → 更新本文件状态表 →（用户要求时）commit。

## 3. 里程碑（M72–M145）

| ID | 名称 | 目标 | 验收 | 难度 | 依赖 | 状态 |
|----|------|------|------|------|------|------|
| **M72** | 多位数 tag + Named 反查 | 修复 `envScan`/`structFieldScan` 单位数读取；`tyTagName`/`isNamedTag` 从 `struct_types` 解析自定义 tag（≥8，含 ≥10） | 夹具 `many_structs.ac`（≥3 自定义类型）；`String` 字段访问 JIT；tag `10` round-trip；既有 golden 全绿 | S | — | ✅ |
| **M73** | 调用 arity + 实参 tag（TC9） | prescan 存参数 tag；单文件程序 `parseCall` 拒错误 arity / 实参类型（含 import 的程序跳过，保自举） | `bad_call_arity.ac` / `bad_call_arg_ty.ac` → exit 1；`BOOTSTRAP_FIXTURE_STEMS` + self-host 全绿 | M | M72 | ✅ |
| **M74** | Bootstrap 侧 E013/E015/E016（TC10） | struct 字段表 miss→undef；Named 字面量缺/余字段与字段值类型；import 嵌套时跳过 | `bad_struct_unknown_field` / `lit_missing` / `field_ty` / `lit_extra` → exit 1；positives + self-host 全绿 | M | M72 | ✅ |
| **M75** | 集合元素类型 + for-in 绑定 tag | 去掉硬编码 `List[Int]`/`Map[String,Int]`；下标与 for-in 绑定正确 tag | `list_string.ac` / `for_string.ac` golden+JIT；`map_keys` 仍返回 3 | M | M72 | ✅ |
| **M76** | Driver Path-B 子集门控 | `action check --frontend bootstrap`（或等价）；CI 双 oracle | N 个 allowlisted 夹具不经 Rust parse/typecheck 也能 `compile_hir`；不一致则 CI 红 | M | M72–M75 | ✅ |
| **M77** | enum 注册表 + when 穷尽（E014） | `preScanEnum` 登记变体；value-match `when` 拒未知构造器与非穷尽；`else` 短路 | `bad_when_non_exhaustive` / `bad_when_unknown_variant` → exit 1；`when_exhaustive.ac` / `enum_simple` 绿 | M | M72 | ✅ |
| **M78** | 自定义 enum 变体→父 tag（TC11） | `enumRegister` 将变体写入全局 env；去掉 `litTag` 对 Color 硬编码 | `custom_enum.ac` 接受 + `compile_hir`；`bad_custom_enum_*` → exit 1；`enum_simple` 仍绿 | M | M77 | ✅ |
| **M79** | when guard 须 Bool | `Pat and <guard>` 探测修复；`tyCheckGuard` 要求 tag 2 | `when_guard_bool.ac` 接受；`bad_when_guard_not_bool` → exit 1；既有 value-match 仍绿 | S | M77 | ✅ |
| **M80** | and/or 操作数须 Bool | `parseAnd`/`parseOr` 两侧 tag==2；对齐 Rust logical-op 检查 | `logical_ops` 绿；`bad_logical_and_int` / `bad_logical_or_int` → exit 1 | S | M79 | ✅ |
| **M81** | not 操作数须 Bool | `parseUnaryNot` 调用 `tyCheckGuard` | `logical_not.ac` 接受；`bad_logical_not_int` → exit 1 | S | M80 | ✅ |
| **M82** | allowlist 扩容 + Rust `not` 对齐 | 四夹具入 stems/allowlist/golden；Rust `UnaryOp::Not` 须 Bool | `custom_enum`/`when_exhaustive`/`when_guard_bool`/`logical_not` golden+allowlist；`bad_logical_not_int` 入 forbidden；m76 allowlist 同步 | M | M76–M81 | ✅ |
| **M83** | Rust when guard 须 Bool | `check_stmt` 对 arm.guard 要求 Bool；对齐 bootstrap M79 | `bad_when_guard_not_bool` 入 forbidden；`when_guard_bool` 仍绿；frontend 单测 | S | M79/M82 | ✅ |
| **M84** | call Named 实参检查 | `callCheckArgTy` 复用 `structFieldTyOk`（Named↔Named / Named↔prim） | `bad_call_arg_{int_point,token_point,point_int}` → exit 1；`call_point_ok` 绿；`bad_call_arg_ty` 仍绿 | M | M73/M74 | ✅ |
| **M85** | let/assign Named 检查 | `tyCheckBind`←`structFieldTyOk`；Rust `collect_stmt` Let/Assign 对齐 | `bad_let_*` / `bad_assign_*` → exit 1；`let_point_ok` / `assign_point_ok` 绿 | M | M74/M84 | ✅ |
| **M86** | 字段赋值检查 | `parseAssignExprRhs` 对 lhs tag↔rhs `tyCheckBind`；未知字段靠 lhs `structFieldRequire` | `bad_field_assign_{ty,unknown}` → exit 1；`field_assign_ok` 绿 | S | M74/M85 | ✅ |
| **M87** | 集合字面量同质 tag | List/Set 元素与 Map 值后续项须匹配首项；Rust `check_call` / MapLiteral 对齐 | `bad_{list,set,map}_mixed` → exit 1；`coll_homo_ok` / 既有 list_string 绿 | S | M75/M85 | ✅ |
| **M88** | Path B allowlist 扩容 | M84–M87 正向夹具入 stems/allowlist/golden + JIT/AOT return oracles | 五夹具 golden；allowlist=48；m76 stems 对齐；oracles 绿 | S | M76/M84–M87 | ✅ |
| **M89** | 下标赋值类型检查 | Bootstrap 已由 M86 覆盖；Rust Index 对 bare `List`/`Map` 按元素/值 Int 推断以对齐 Assign | `bad_index_assign_ty` / `bad_map_index_assign_ty` → exit 1；`index_assign_ok` 绿；Rust 单测 | S | M75/M86 | ✅ |
| **M90** | index_assign Path B 入册 | `index_assign_ok` 入 stems/allowlist/golden + return oracle | allowlist 48；golden+JIT/AOT；m76 对齐 | S | M76/M89 | ✅ |
| **M91** | Index 键类型检查 | List 下标须 Int；Map 键须 String；不改 Named Map 的 E008 门控 | `bad_list_index_key` / `bad_map_index_key` → exit 1；`index_key_ok` / `map_index` 绿 | S | M75/M89 | ✅ |
| **M92** | index_key Path B 入册 | `index_key_ok` 入 stems/allowlist/golden + return oracle | allowlist 49；golden+JIT/AOT；m76 对齐 | S | M76/M91 | ✅ |
| **M93** | 算术操作数类型检查 | Sub/Mul/Div/Mod 拒 Bool/String；对齐 Rust `check_binary_op`；Add 仍允许字符串拼接 | `bad_arith_{sub_bool,mul_string,div_bool}` → exit 1；`arith_ok` 绿 | S | M80 | ✅ |
| **M94** | arith_ok Path B 入册 | `arith_ok` 入 stems/allowlist/golden + return oracle（`10-3*2`→4） | allowlist 50；golden+JIT/AOT；m76 对齐 | S | M76/M93 | ✅ |
| **M95** | 比较混型检查 | 有序比较拒 Bool/非 Bool 混型；Eq/Neq 软通过；对齐 Rust `check_binary_op` | `bad_cmp_{lt_int_bool,gt_bool_int}` → exit 1；`cmp_ok` 绿 | S | M93 | ✅ |
| **M96** | cmp_ok Path B 入册 | `cmp_ok` 入 stems/allowlist/golden + return oracle（`1<2`→0） | allowlist 51；golden+JIT/AOT；m76 对齐 | S | M76/M95 | ✅ |
| **M97** | 一元 Neg 操作数检查 | `-` 拒 Bool/String（修 bootstrap 误标 Int）；Rust `UnaryOp::Neg` 对齐 | `bad_unary_neg_{bool,string}` → exit 1；`unary_neg_ok` 绿 | S | M93 | ✅ |
| **M98** | unary_neg_ok Path B 入册 | `unary_neg_ok` 入 stems/allowlist/golden + return oracle（`-3+5`→2） | allowlist 52；golden+JIT/AOT；m76 对齐 | S | M76/M97 | ✅ |
| **M99** | range 两端须 Int | `..`/`..<` 两端须 Int；双端同步收紧 | `bad_range_{bool_end,string_start}` → exit 1；`range_ok`/`for_range` 绿 | S | M93 | ✅ |
| **M100** | range_ok Path B 入册 | `range_ok` 入 stems/allowlist/golden + return oracle（`1..<3` sum→3） | allowlist 53；golden+JIT/AOT；m76 对齐 | S | M76/M99 | ✅ |
| **M101** | when 条件须 Bool | OneLine / ConditionChain 条件须 Bool；ValueMatch 不受影响；Rust 对齐 | `bad_when_{cond,chain}_int` → exit 1；`when_cond_ok` / `when_condition_chain` 绿 | S | M79/M83 | ✅ |
| **M102** | when_cond_ok Path B 入册 | `when_cond_ok` 入 stems/allowlist/golden + return oracle（`when true`→0） | allowlist 54；golden+JIT/AOT；m76 对齐 | S | M76/M101 | ✅ |
| **M103** | for 条件须 Bool | `for <cond> { … }` 条件须 Bool；Rust `ForKind::Condition` 对齐 | `bad_for_cond_int` → exit 1；`for_cond_ok` 绿 | S | M101 | ✅ |
| **M104** | for_cond_ok Path B 入册 | `for_cond_ok` 入 stems/allowlist/golden + return oracle（`for x < 3`→3） | allowlist 55；golden+JIT/AOT；m76 对齐 | S | M76/M103 | ✅ |
| **M105** | tyCheckReturn→tyCheckBind | return 检查复用 `structFieldTyOk`；删除 `tyCheckReturnNamedDistinct` | 全部 `bad_return*` 仍 exit 1；`return_*` 正向仍绿；自举 JSON OK | S | M85 | ✅ |
| **M106** | Add 拒 Bool | `+` 拒 Bool；仍允许字符串拼接；Rust `BinaryOp::Add` 对齐 | `bad_arith_add_bool{,_left}` → exit 1；`arith_add_string_ok` / `arith_ok` 绿 | S | M93 | ✅ |
| **M107** | arith_add_string Path B 入册 | `arith_add_string_ok` 入 stems/allowlist/golden + return oracle（`len("a"+"b")`→2） | allowlist 56；golden+JIT/AOT；m76 对齐 | S | M76/M106 | ✅ |
| **M108** | 一元 Pos 拒 Bool/String | `+` 与 Neg 对称拒 Bool/String；Rust 新增 `UnaryOp::Pos`（此前消掉 `+`） | `bad_unary_pos_{bool,string}` → exit 1；`unary_plus` 仍绿（→15） | S | M97 | ✅ |
| **M109** | val 不可重新赋值 | env 记 mut 标志；Ident 赋值拒 `val`（对齐 Rust `mutable_vars`） | `bad_val_assign{,_expr}` → exit 1；`assign_expr` 仍绿 | S | M85 | ✅ |
| **M110** | val 根 Field/Index 赋值拒 | FieldAccess/Index 沿链查根 Ident；双端拒 `val` 根写入 | `bad_{field,index}_assign_val` → exit 1；`field_assign_ok`/`index_assign_ok` 仍绿 | S | M109 | ✅ |
| **M111** | 拒未定义 Call | 单文件 `Call`：无 funSig 且 env miss → typeError；import/内置仍跳过 | `bad_undef_call` → exit 1；`call_point_ok`/`len` 仍绿；self-host 仍绿 | S | M73 | ✅ |
| **M112** | 拒非 return undef Ident | `markUndefIdentUse` 单文件硬拒；import 仍软记名 | `bad_undef_{bind,arg,arith,var}` → exit 1；正向夹具仍绿 | S | M111 | ✅ |
| **M113** | String 下标键须 Int | `tyCheckIndexKey` recvTag==5；Rust `check_index_key_type` String→Int | `bad_string_index_{bool,string}` → exit 1；`string_index_ok` 绿 | S | M91 | ✅ |
| **M114** | string_index_ok Path B 入册 | stems/allowlist/golden + return oracle | allowlist 57；oracle →0；m76 对齐 | S | M76/M113 | ✅ |
| **M115** | import funSig + call 检查 | `funSigCommit` 不再因 `importDepth` 跳过；`callCheckBegin` 在有 sig 时检查 arity/实参（import 程序亦然）；undef 仍 soft | `import_call_ok` →7；`bad_import_call_{arity,arg_ty}` → exit 1；allowlist 58；self-host 绿 | M | M73/M76 | ✅ |
| **M116** | external funSig | `preScanExternal` 复用 `preScanFunParams`+`funSigCommit`；仍不发射 HIR | `external_call_ok` 接受；`bad_external_call_{arity,arg_ty}` → exit 1；self-host 绿；不入 Path B allowlist | S | M115 | ✅ |
| **M117** | 基本 UFCS（nullary 试点） | `.method(` 跳过 `structFieldRequire`；HIR `Call(FieldAccess)`；`len`/`isEmpty` 种子 ret tag；List/Map/Set lit 接 `parsePostfix`（不禁带参 UFCS，以免破坏 `s.substring`） | `ufcs_len_ok` →3；`bad_ufcs_unknown` → exit 1；allowlist 59；self-host 绿 | M | M76 | ✅ |
| **M118** | fallible `or {}` 试点 | `parseOrAfterKw`：`or {` → HIR `OrBlock`；否则逻辑 Or；`tyCheckOrBlock`；种子 `parseInt` | `or_block_ok` →0；`bad_or_block_ty` → exit 1；allowlist 60；`logical_ops` 仍绿 | M | M80/M117 | ✅ |
| **M119** | lambda / `it` 试点 | `{` 消歧：`x=`→struct，`x->`/`it …`→`Lambda`；立即调用跳过 funSig；locals snapshot | `lambda_it_ok` →42；`bad_lambda_it_ty` → exit 1；allowlist 61；struct lit 仍绿 | M | M76 | ✅ |
| **M120** | 开放 import 图（窄） | path-safe 名；visiting DFS 环拒；缺文件拒；非白名单可加载（`mod_` 前缀）；flat allowlist 仍控制 bare 名 | `import_graph_ok` →42；`bad_import_{cycle,unknown}` → exit 1；allowlist 62；self-host 绿 | M | M115 | ✅ |
| **M121** | 多参 lambda | `{ x, y -> … }`（`,` 消歧）；params 全绑 Int；立即调用 | `lambda_multi_ok` →42；`bad_lambda_multi_ty` → exit 1；allowlist 63；`lambda_it_ok` 仍绿 | S | M119 | ✅ |
| **M122** | trailing lambda | `f(args) { it…}`；种子 `map`；HIR `trailing_lambda`；`[0] or { 0 }` | `trailing_lambda_ok` →42；`bad_trailing_lambda_ty` → exit 1；allowlist 64 | M | M119/M121 | ✅ |
| **M123** | 无参 `{ expr }` lambda | `{ 21 * 2 }()` → `Lambda` params=[] + Block body；非 `x=`/`it`/`->` 消歧 | `lambda_block_ok` →42；`bad_lambda_block_ty`（`1 and true`）→ exit 1；allowlist 65；struct/`or {}`/`if f(){}` 仍绿 | S | M119 | ✅ |
| **M124** | 更广搜索根 | `bootstrap/` → `tests/fixtures/bootstrap/`（`exists`）；无 canonicalize 逃逸 | `import_fixtures_ok` →42（`m124_lib` 仅 fixtures）；`bad_import_unknown` 仍拒；allowlist 66 | S | M120 | ✅ |
| **M125** | 多语句 `{ expr; … }` lambda | `parseLambdaBlock` 循环 stmt；Block 类型=末 expr；slot 44 | `lambda_stmts_ok` →42；`bad_lambda_stmts_ty` → exit 1；allowlist 67；`lambda_block_ok` 仍绿 | S | M123 | ✅ |
| **M126** | `if`/`or {}` 多语句 PlainBlock | `parsePlainBlockBody`（复用 stmt 循环，HIR Block 非 Lambda） | `if_stmts_ok` →42；`bad_if_stmts_ty` → exit 1；`or_block_ok` 仍绿；allowlist 68 | S | M125 | ✅ |
| **M127** | `{ val …; }` PlainBlock | `parsePlainBlockLet` + `letAsStmt`；`parseBraceExpr` 认 `val`/`var` | `plain_block_val_ok` →42；`bad_plain_block_val_ty` → exit 1；if/or 臂可用 `val`；allowlist 69 | S | M126 | ✅ |
| **M128** | lambda 体内 `val` | `parseLambdaBlock` 复用 `parsePlainBlockStmt`；前导 `val` 仍 PlainBlock | `lambda_val_ok` →42；`bad_lambda_val_ty` → exit 1；allowlist 70 | S | M127 | ✅ |
| **M129** | PlainBlock `return` | `returnAsStmt` + `parsePlainBlockReturn`；`parseBraceExpr` 认 `return` | `plain_block_return_ok` →42；`bad_plain_block_return_ty` → exit 1；allowlist 71 | S | M128 | ✅ |
| **M130** | PlainBlock 窄 `for-in` | `forAsStmt` Iterate + nested slot44 body；仅 `for v in expr` | `plain_block_for_ok` →42；`bad_plain_block_for_ty` → exit 1；allowlist 72 | M | M129 | ✅ |
| **M131** | PlainBlock `for` Condition | `forCondAsStmt` + `tyCheckGuard`；`in` vs cond 分派 | `plain_block_for_cond_ok` →42；`bad_plain_block_for_cond_ty` → exit 1；allowlist 73 | S | M130 | ✅ |
| **M132** | PlainBlock `for` WithIndex | `forWithIndexAsStmt` + List/Map bind；`,` 分派 | `plain_block_for_with_index_ok` →6；`bad_plain_block_for_with_index_ty` → exit 1；allowlist 74 | S | M131 | ✅ |
| **M133** | PlainBlock `for` Infinite | `forInfiniteAsStmt`；`for {` 分派；ok 须 early `return` | `plain_block_for_infinite_ok` →42；`bad_plain_block_for_infinite_ty` → exit 1；allowlist 75 | S | M132 | ✅ |
| **M134** | PlainBlock Map `for-in` 值绑定 | `pexpr.plainForInBindTag`（`len(v)` 启发式）；对齐 pstmt/Rust | `plain_block_map_values_ok` →15；`bad_plain_block_map_values_ty` → exit 1；allowlist 76 | S | M133 | ✅ |
| **M135** | PlainBlock `break`/`continue` | 复用 `exprAsStmt` 升格；夹具入 Path B | `plain_block_break_ok` →15；`plain_block_continue_ok` →12；`bad_plain_block_break_ty` → exit 1；allowlist 78 | S | M134 | ✅ |
| **M136** | PlainBlock Map `for-in` 键绑定 | `parsePlainBlockLet`→`collEnvBind`（Ident 恢复键/值 tag）+ `len(k)`；夹具入 Path B | `plain_block_map_keys_ok` →3；`bad_plain_block_map_keys_ty` → exit 1；allowlist 79 | S | M135 | ✅ |
| **M137** | PlainBlock 嵌套 for | Path B smoke（slot44 已 save/restore）；对齐 `nested_for` | `plain_block_nested_for_ok` →3；`bad_plain_block_nested_for_ty` → exit 1；allowlist 80 | S | M136 | ✅ |
| **M138** | PlainBlock Map `for k, v` | Path B 入册 IterateWithIndex（M132 已实现绑定）；对齐 `map_iter` | `plain_block_map_iter_ok` →15；`bad_plain_block_map_iter_ty` → exit 1；allowlist 81 | S | M137 | ✅ |
| **M139** | PlainBlock Set for-in | Path B 对齐 `set_iter` | `plain_block_set_iter_ok` →6；`bad_plain_block_set_iter_ty` → exit 1；allowlist 82 | S | M138 | ✅ |
| **M140** | PlainBlock `when` | Path B 对齐 `when_for`；guard 否定夹具 | `plain_block_when_ok` →37；`bad_plain_block_when_ty` → exit 1；allowlist 83 | S | M139 | ✅ |
| **M141** | Phase A 文档核对 | 修正过时数字 / 模块表 / `external fun` 契约 | README + plan + subset + `stdlib-layers` 与 allowlist **83** / **207** tests 一致 | S | M140 | ✅ |
| **M142** | PlainBlock 字段赋值 | Path B 对齐 `field_assign_ok` | `plain_block_field_assign_ok` →7；`bad_plain_block_field_assign_ty` → exit 1；allowlist 84 | S | M141 | ✅ |
| **M143** | PlainBlock 下标赋值 | Path B 对齐 `index_assign_ok` | `plain_block_index_assign_ok` →0；`bad_plain_block_index_assign_ty` → exit 1；allowlist 85 | S | M142 | ✅ |
| **M144** | PlainBlock `when` guard | Path B 对齐 `when_guard_bool` | `plain_block_when_guard_ok` →1；`bad_plain_block_when_guard_ty` → exit 1；allowlist 86 | S | M143 | ✅ |
| **M145** | PlainBlock `print` | Path B 对齐 `print_stmt` | `plain_block_print_ok` →0；`bad_plain_block_print_ty` → exit 1；allowlist 87 | S | M144 | ✅ |

### 后续批次（规划）

| ID | 目标 | 依赖 |
|----|------|------|
| **M146+** | PlainBlock when-exhaustive / ufcs / or | M145 |

### 刻意延后（非本批次）

- 任意目录 import / canonicalize 逃逸层（仍固定双根：`bootstrap/` + `tests/fixtures/bootstrap/`，见 M124）
- 空体 Infinite 入 return oracle（会挂；仅 compile-smoke，见既有 `infinite_for`）
- lazy / **复杂** UFCS 方法链（子集仍禁止；M117 仅 nullary）
- Action 实现 codegen（L3，不做）

## 4. Phase A — 文档与契约（✅ M141）

| 项 | 动作 | 状态 |
|----|------|------|
| A1 | 本文件作为 M72+ 权威入口；`bootstrap/README.md` / `bootstrap-subset.md` 链到此处 | ✅ |
| A2 | 修正过时数字：`bootstrap_subset` 测试数、fixture stem 数、「规划中」标题 | ✅ |
| A3 | 子集「禁止 `external fun`」与 README（prescan 已支持）对齐 | ✅ |
| A4 | 确认模块树 13 个 `.ac` + 10 个 `check_bootstrap_*.py` 纳入日常验证 | ✅ |

## 5. 验证清单（每个 Mx）

在 `nix-shell` 内至少：

```bash
cargo test --test bootstrap_subset -- --test-threads=1
# 涉及脚本契约时：
bash scripts/check_bootstrap_goldens.sh
# 语义快检（若动到宿主/codegen）：
./target/release/action run examples/bench_cow.ac   # 预期 11
```

涉及 List/Map/RC 宿主改动时，另跑 `cargo test --test integration -- --test-threads=1`。

## 6. 模块职责（实现时改哪里）

| 里程碑 | 主要改动文件 |
|--------|----------------|
| M72 | `bootstrap/typeenv.ac`（tag 编解码 / `tyTagName`）；必要时 `emit.ac` |
| M73 | `bootstrap/pscan.ac` + `pexpr.ac`（call）+ `typeenv.ac` |
| M74 | `bootstrap/typeenv.ac` + `pexpr.ac`（struct literal） |
| M75 | `bootstrap/pexpr.ac` + `pstmt.ac` |
| M76 | `crates/action-cli` / `action-driver` + `tests/` |
| M77 | `bootstrap/typeenv.ac` + `pscan.ac` + `pexpr.ac`；host-rt `bsBuf` 41–42 / `bsInt` 33–34 |
| M78 | `bootstrap/typeenv.ac`（`enumRegisterVariants`）；夹具 `custom_enum*` |
| M79 | `bootstrap/pexpr.ac`（`isValueMatchArm` + `parseWhenGuardAfter`）；`typeenv.tyCheckGuard` |
| M80 | `bootstrap/pexpr.ac`（`parseAnd`/`parseOr`）；`typeenv.tyCheckLogical` |
| M81 | `bootstrap/pexpr.ac`（`parseUnaryNot`） |
| M82 | `BOOTSTRAP_FIXTURE_STEMS` / `BOOTSTRAP_FRONTEND_ALLOWLIST`；`check_stmt.rs` Unary Not；四 golden |
| M83 | `check_stmt.rs` when arm.guard Bool；`bad_when_guard_not_bool` → forbidden |
| M84 | `bootstrap/typeenv.ac`（`callCheckArgTy` ← `structFieldTyOk`） |
| M85 | `typeenv.tyCheckBind`；`pstmt` let/assign；`pexpr` Map lit tag；`check_stmt.rs` Let/Assign |
| M86 | `pexpr.parseAssignExprRhs`（lhs tag↔rhs `tyCheckBind`）；未知字段靠 `structFieldRequire` |
| M87 | `pexpr.noteCollElemTag` / `noteMapValueTag` + `tyCheckBind`；`check_stmt` List/Set/MapLiteral |
| M88 | `BOOTSTRAP_FIXTURE_STEMS` / `BOOTSTRAP_FRONTEND_ALLOWLIST`；五 golden；JIT/AOT oracles |
| M89 | Index Named List/Map → Int；夹具 `bad_*index_assign*`；`index_assign_ok` |
| M90 | `index_assign_ok` → stems/allowlist/golden + return oracle |
| M91 | `typeenv.tyCheckIndexKey`；`pexpr.parseIndex`；`check_index_key_type` |
| M92 | `index_key_ok` → stems/allowlist/golden + return oracle |
| M93 | `typeenv.tyCheckArith`；`pexpr.parseMulStep`/`parseAddStep` |
| M94 | `arith_ok` → stems/allowlist/golden + return oracle |
| M95 | `typeenv.tyCheckCmp`；`pexpr.parseCmpStep` |
| M96 | `cmp_ok` → stems/allowlist/golden + return oracle |
| M97 | `typeenv.tyCheckNeg`；`pexpr.parseUnaryNeg`；`check_stmt` Unary Neg |
| M98 | `unary_neg_ok` → stems/allowlist/golden + return oracle |
| M99 | `typeenv.tyCheckRange`；`pexpr.parseRangeStep`；`check_binary_op`/`ExprKind::Range` |
| M100 | `range_ok` → stems/allowlist/golden + return oracle |
| M101 | `pexpr` OneLine/ConditionChain + `tyCheckGuard`；`check_stmt` when cond Bool |
| M102 | `when_cond_ok` → stems/allowlist/golden + return oracle |
| M103 | `pstmt.parseForCond` + `tyCheckGuard`；`check_stmt` ForKind::Condition |
| M104 | `for_cond_ok` → stems/allowlist/golden + return oracle |
| M105 | `typeenv.tyCheckReturn` ← `tyCheckBind`；删 `tyCheckReturnNamedDistinct` |
| M106 | `tyCheckArith` Add 拒 Bool；`check_binary_op` Add |
| M107 | `arith_add_string_ok` → stems/allowlist/golden + return oracle |
| M108 | `tyCheckPos`/`tyCheckNumericUnary`；Rust `UnaryOp::Pos` + typecheck/codegen |
| M109 | `envAppendMut`/`tyCheckAssignIdent`；`parseAssignStmt`/`parseAssignExprChecked` |
| M110 | `lvalue_root_ident`；`jsonFirstIdentName` + Field/Index 根 mut 检查 |
| M111 | `callCheckUndefCallee`；单文件未知 callee；import/内置软跳过 |
| M112 | `markUndefIdentUse` 单文件硬拒；import 仍 `markUndefIdentSet` |
| M113 | `tyCheckIndexKey` String→Int；`parseIndex` outTag；Rust `check_index_key_type` |
| M114 | `string_index_ok` → stems/allowlist/golden + return oracle |
| M115 | `funSigCommit` 始终提交；`callCheckBegin` 有 sig 即检查；`import_call_ok` / `bad_import_call_*` |
| M116 | `preScanExternal` → funSig；`external_call_ok` / `bad_external_call_*` |

## 7. 成功标准（本批次结束时）

- [x] M72–M142 状态表全部 ✅，或明确 `cancelled` 并写原因
- [x] Phase A 文档核对（M141）
- [x] PlainBlock 字段赋值（M142）
- [x] PlainBlock 下标赋值（M143）
- [x] PlainBlock `when` guard（M144）
- [x] PlainBlock `print`（M145）
- [x] 子集程序的 **拒绝/接受** 路径可在不依赖 Rust `typecheck` 的情况下对 allowlisted 夹具成立（M76）
- [x] Bootstrap 侧 enum `when` 穷尽 / 未知构造器（M77；Rust `exhaustive.rs` 仍为双前端权威之一）
- [x] 自定义 enum 变体 Ident 解析为父 enum tag（M78）
- [x] value-match `and <guard>` 须 Bool（M79；M83 起 Rust 对齐）
- [x] `and`/`or` 操作数须 Bool（M80）
- [x] `not` 操作数须 Bool（M81；M82 起 Rust 对齐）
- [x] M77–M81 正向夹具入 Path B allowlist + golden（M82）
- [x] Rust when guard 须 Bool（M83）
- [x] call Named 实参检查（M84；复用 M74 规则）
- [x] let/assign Named 检查（M85；复用 M74 规则；Rust 对齐）
- [x] 字段赋值检查（M86；expr-assign + E013 via FieldAccess lhs）
- [x] 集合字面量同质 tag（M87；List/Set/Map；Rust 对齐）
- [x] M84–M87 正向夹具入 Path B allowlist + golden（M88）
- [x] 下标赋值类型检查（M89；Rust Index bare List/Map→Int）
- [x] index_assign_ok 入 Path B allowlist（M90）
- [x] Index 键类型检查（M91；List→Int / Map→String）
- [x] index_key_ok 入 Path B allowlist（M92）
- [x] 算术操作数类型检查（M93；Sub/Mul/Div/Mod 拒 Bool/String）
- [x] arith_ok 入 Path B allowlist（M94）
- [x] 比较混型检查（M95；有序比较拒 Bool/非 Bool）
- [x] cmp_ok 入 Path B allowlist（M96）
- [x] 一元 Neg 操作数检查（M97；拒 Bool/String）
- [x] unary_neg_ok 入 Path B allowlist（M98）
- [x] range 两端须 Int（M99；双端同步）
- [x] range_ok 入 Path B allowlist（M100）
- [x] when 条件须 Bool（M101；OneLine/ConditionChain）
- [x] when_cond_ok 入 Path B allowlist（M102）
- [x] for 条件须 Bool（M103）
- [x] for_cond_ok 入 Path B allowlist（M104）
- [x] tyCheckReturn 复用 tyCheckBind（M105）
- [x] Add 拒 Bool（M106；字符串拼接仍可）
- [x] arith_add_string_ok 入 Path B allowlist（M107）
- [x] 一元 Pos 拒 Bool/String（M108；与 Neg 对称）
- [x] val 不可重新赋值（M109；对齐 Rust mutable_vars）
- [x] val 根 Field/Index 赋值拒绝（M110；双端对齐）
- [x] 拒未定义 Call（M111；单文件；import/内置跳过）
- [x] 拒非 return undef Ident（M112；单文件硬拒）
- [x] String 下标键须 Int（M113；双端对齐）
- [x] string_index_ok 入 Path B allowlist（M114）
- [x] import funSig 提交 + 有 sig 时 call 检查（M115）
- [x] external fun 写入 funSig（M116）
- [x] nullary UFCS / `or {}` / lambda `it`（M117–M119）
- [x] 开放 import 图 path-safe + cycle（M120）
- [x] 多参 `{ x, y -> }` lambda（M121）
- [x] trailing `f(){…}` + 种子 `map`（M122）
- [x] 无参 `{ expr }()` lambda（M123）
- [ ] Path B `compile_hir` 仍仅 Rust；无语义回归
- [ ] 文档数字与标题与仓库一致
- [x] 更广搜索根 `bootstrap/` + `tests/fixtures/bootstrap/`（M124）
- [x] 多语句 `{ expr; … }` 无参 lambda（M125）
- [x] `if`/`or {}` 多语句 PlainBlock（M126）
- [x] `{ val …; }` 表达式 PlainBlock / if·or 臂内 `val`（M127）
- [x] lambda 体内 `val`（M128）
- [x] PlainBlock `return`（M129）
- [x] PlainBlock 窄 `for-in` Iterate（M130）
- [x] PlainBlock `for` Condition（M131）
- [x] PlainBlock `for` WithIndex（M132）
- [x] PlainBlock `for` Infinite（M133）
- [x] PlainBlock Map `for-in` 值绑定（M134）
- [x] PlainBlock `break`/`continue`（M135）
- [x] PlainBlock Map `for-in` 键绑定（M136）
- [x] PlainBlock 嵌套 for（M137）
- [x] PlainBlock Map `for k, v`（M138）
- [x] PlainBlock Set for-in（M139）
- [x] PlainBlock `when`（M140）
- [x] Phase A 文档核对（M141）
- [x] PlainBlock 字段赋值（M142）
- [x] PlainBlock 下标赋值（M143）
- [x] PlainBlock `when` guard（M144）
- [x] PlainBlock `print`（M145）

## 8. 与既有文档关系

| 文档 | 角色 |
|------|------|
| `doc/roadmap-and-bootstrap-analysis.md` | 战略（Path B、不做 L3/L4）；时间线部分已部分过时 |
| `doc/bootstrap-subset.md` | 子集允许/禁止 + M4–M140 状态表 |
| `bootstrap/README.md` | 操作说明 + M4–M20/TC + M72+ 指针与当前 harness 数字 |
| **本文件** | **M72+ 执行计划与状态（权威）** |
