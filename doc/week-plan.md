# 一周改进计划（2026-06-22 → 2026-06-28）

> **基线**：`main` @ `0f35d99` + 未提交三项（`map_filter_map` 融合、`insert_rec` 满叶 append、`apply` 去虚拟化 + `map` LICM）  
> **AOT -O2 基线**（2026-06-21）：`bench_all` 21ms · `bench_concat_depth` 14ms · `bench_funcall` 12ms · `bench_for_chain` 11ms  
> **优先级**：**List P0 热点** > **提交/CI** > **P1 迭代器融合** > **P2 Concat** > **P3 调用/Map**

---

## 上周已完成（2026-06-21）

| ID | 项 |
|----|-----|
| ✅ | `range_walk_rec`：take/drop/slice/chunks/windows |
| ✅ | `remove(0)` → `drop(1)` 入口快路径（含 ConcatNode） |
| ✅ | Concat remove `index>0` 恢复左右递归 + concat |
| ✅ | `map(filter(map(...)))` identity 融合；非 identity 双遍（待提交改为 `map_filter_map_walk`） |
| ✅ | `list_get_cached` 推广；reverse walk；chunks/windows 改 slice |
| ✅ | integration **163/163**；`bench_cow` → **11** |

---

## 本周目标概览

| 轨道 | 目标 | 里程碑 |
|------|------|--------|
| A 收尾 | 提交上轮三项优化 + CI 绿 | M-commit |
| B **P0 List** | `insert_rec` 满叶 split + internal overflow；`find` concat 早退 | M-list-P0 |
| C **P1 融合** | `filter+map+fold` 单遍；LICM 扩展 | M-iter |
| D **P2 Concat** | balance/flatten 调优；`bench_all` concat contains 回归 | M-concat |
| E 基线 | AOT 重刷 + 无回归 | M-perf |

---

## 任务清单

### Phase A — 提交上轮优化（Day 1）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| A-1 | **commit + push** | `map_filter_map_walk`、`insert_rec` append-on-full、`apply` 去虚拟化、`map` LICM | CI 绿 |
| A-2 | **基线快照** | `./benchmark.sh --mode aot --opt 2 -n 5` → `benchmark_results_aot_o2.txt` | 记录 avg ms |

**快检：**

```bash
nix-shell --run 'cargo test --release --test integration -- --test-threads=1'
./target/release/action run examples/bench_cow.ac          # 11
./target/release/action run examples/bench_all.ac
./target/release/action run examples/map_filter.ac         # 210215
```

---

### P0 — List 运行时（Day 1–3，**本周首要**）

| ID | 任务 | 目标基准 | 说明 | 验收 |
|----|------|----------|------|------|
| P0-1 | **`insert_rec` 满叶中间 split** | `bench_all`、`bench_insert100` | h=0 len=64 中间 insert：`insert_rec` → `split_child` 快路径 | ✅ `test_insert_h0_mid.ac` |
| P0-2 | **`insert_rec` internal overflow** | 同上 | 子 `insert_rec` null → `int_split_child`：CoW + split + 兄弟子树；internal 满 64 → 仿 `lp_split_intl` | ✅ `split_intl` |
| P0-3 | **`find_walk_rec` Concat 早退** | `bench_concat_depth`、语义 | 仿 `contains_walk`：concat 左命中跳过右；internal 子树命中跳过后续 sibling | ✅ |
| P0-4 | **回归夹具** | CI | `bench_all.ac` 增加 `c.contains(...)`；`opt_pass` 修正 `action_list_find` 命名 | ✅ |

**约束（必守）：**

- Concat **中间 insert 禁止** take+drop 替代递归（已证 `bench_all` 19→27ms 回归）
- 共享引用 `rc>1` 必须 CoW 后再 split
- 分裂几何与 push 一致：左 32、右最多 33、fanout 64

**参考 IR 模板（`define_list_core.rs`）：**

| 块 | 行号 | 用途 |
|----|------|------|
| `lp_split_leaf` | ~1000–1097 | 满叶 32/33 分裂 |
| `lp_h0_full` | ~259–437 | 根 h=0 晋升 |
| `lp_add_child` | ~1119–1250 | internal 追加兄弟 |
| `lp_split_intl` | ~1253–1632 | internal 满 64 溢出 |

---

### P1 — 迭代器融合（Day 3–4）

| ID | 任务 | 目标基准 | 说明 | 验收 |
|----|------|----------|------|------|
| P1-1 | **`filter+map+fold` 单遍 walk** | `bench_for_chain`（11ms） | 新 `action_list_filter_map_fold_walk` + codegen 融合 | ✅ |
| P1-2 | **`map+fold` / `filter+fold` 融合** | `bench_concat_depth` | UFCS / builtin 识别链式调用 | ✅ `filter_fold_walk` + UFCS |
| P1-3 | **LICM 扩展** | `bench_for_chain` | 循环外提 `filter(lst)`、`fold(0, lst)` 等不变式 | ✅ filter/fold/map_fold/filter_fold LICM |

---

### P2 — Concat 深树（Day 4–5）

| ID | 任务 | 目标基准 | 说明 | 验收 |
|----|------|----------|------|------|
| P2-1 | **Concat balance 阈值调优** | `bench_concat_depth`（14ms） | 深度 > 32 flatten；只调参/补路径 | ✅ 无 SIGSEGV |
| P2-2 | **`push_subtree` / walk nounwind** | AOT 全局 | 扩 `opt_pass.rs` | ✅ `*_walk_rec` + `push`/`create` |
| P2-3 | **`indexOf` / `findIndex` cached walk** | `bench_all` | 替代 `get` 循环（若 profiling 命中） | ✅ `index_of_walk` |

---

### P3 — 调用与 Map（Day 5–6，可选）

| ID | 任务 | 目标基准 | 说明 |
|----|------|----------|------|
| P3-1 | **`fib(30)` 字面量特化** | `bench_funcall` | 常量参数编译期求值；不改变一般 `fib(n)` |
| P3-2 | **HO 去虚拟化扩展** | `bench_lambda` | `FunctionRef`、局部 `val f = fn` |
| P3-3 | **Map 增量更新** | `bench_map_10k` | 单独阶段；`define_map.rs` O(n) rebuild | ✅ union `bulk_copy` |

---

## 建议日程

| 日 | 重点 | 产出 |
|----|------|------|
| 一 | Phase A 提交 + **P0-1** 叶 split | commit + insert_rec 中间索引 |
| 二 | **P0-2** internal overflow | `int_split_child` + split_intl |
| 三 | **P0-3/4** find 早退 + bench 夹具 | P0 commit + CI |
| 四 | **P1-1** filter+map+fold | `bench_for_chain` 下降 |
| 五 | P1-2/3 + P2-1 + AOT 基线重刷 | 第二 commit；`gh run watch` |

---

## 验收总表（周末 checklist）

- [x] Phase A + P0 各至少 1 commit；push 后 CI 绿
- [x] `cargo test --release --test integration -- --test-threads=1` → **172 passed**
- [x] `bench_cow.ac` → **11**；`map_filter.ac` → **210215**；`bench_all.ac` 无 SIGSEGV
- [x] insert 系列 **30×** release 无 crash
- [x] `./benchmark.sh --mode aot --opt 2 -n 3` 相对基线无 FAIL；`bench_all` / `bench_insert100` 有改善或持平
- [x] P1 完成时 `bench_for_chain` avg **< 11ms**（目标 ~8ms）

---

## 风险与不做项

| 风险 | 缓解 |
|------|------|
| insert split 破坏 CoW | 先 CoW 再 split；跑 `test_list_cow_property` / `bench_cow` |
| Concat insert 回归 | 禁止 take+drop 中间路径；必跑 `bench_all` + `bench_concat_depth` |
| find 早退改语义 | 保持中序首次匹配；补 integration |

**本周明确不做：**

- 重写 `define_list_core.rs` 整块
- 全 module runtime IR PassManager O2
- 为 benchmark 改语言语义（如默认 fib 改迭代）

---

## 参考

- 语义：`.cursor/rules/preserve-language-semantics.mdc`
- 工作流：`.cursor/rules/phased-optimization-workflow.mdc`
- 性能：`BENCHMARK.md`、`scripts/perf_map_note.md`
- 架构：`doc/ARCHITECTURE.md`
