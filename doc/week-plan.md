# 一周改进计划（2026-06-21 → 2026-06-27）

> **基线**：`main` @ `7aaa858`（P0-3 assign per-node RC、155 integration 全绿、CI 绿）  
> **优先级**：**正确性 & RC** > **CI 门禁** > **性能基线** > **自举 M4 起步** > **文档**

---

## 上周已完成（2026-06-20）

| ID | 项 |
|----|-----|
| ✅ | `insert_rec`：`list_root_rc`、CoW-before-recurse、条件 xfer / `store_child` rc_inc |
| ✅ | assign 延迟整树释放（同作用域另有 List 绑定时仅 `rc_dec` 根） |
| ✅ | scope cleanup 逆序；集成 `test_insert_exit` / `test_cow_insert_isolation` / `test_list_cow_property` |
| ✅ | `ci-linux.sh` release 冒烟；`benchmark_regression.py` FAIL 行检测 |
| ✅ | Windows release 冒烟（`bench_cow` + `test_insert_exit`） |
| ✅ | `ARCHITECTURE.md` CI/测试同步 |

---

## 本周目标概览

| 轨道 | 目标 | 里程碑 |
|------|------|--------|
| A 语义/RC | List 别名路径无 flaky；延迟释放策略可观测、可测 | M1.1 |
| B CI | proptest blocking；release 冒烟扩展 | M-CI |
| C 性能 | AOT `-O2` 基线重刷；insert 系列无回归 | M-perf |
| D 自举 | M4 lexer golden 框架 + 首版 `bootstrap/lexer.at` 试点 | M4α |
| E 文档 | 语义大纲 + stdlib 分层（支撑子集冻结） | M-doc |

---

## 任务清单

### P0 — List / RC 正确性（Mon–Tue）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| P0-1 | **别名压力集成测试** ✅ | `append`/`remove`/`insert` 与 `lst`+`ins` 同作用域；内层 block 提前释放一条绑定 | 新增 2–3 个 `.at` + integration；30× release 无 SIGABRT |
| P0-2 | **延迟释放可观测性** ✅ | assign 大循环 insert 内存峰值可记录（`scripts/measure_mem_peak.sh` + `mem_peak_note.md`） | 已记录 pre/post P0-3 MaxRSS（−2.6%～−3.2%） |
| P0-3 | **per-node RC 释放** ✅ | assign 时对旧根子树 walk + 条件 dec，跳过 live 绑定与 RHS 新值 | `test_rc_pressure` / insert 30× 稳定；`define_list_rc_assign.rs` |
| P0-4 | **修正测试计数** ✅ | `ARCHITECTURE.md` 与 `integration.rs` 均为 **155** | 已一致 |

**快检命令：**

```bash
nix-shell --run 'cargo test --release --test integration -- --test-threads=1'
for b in bench_insert2 bench_insert10 bench_insert50 bench_insert100 bench_all; do
  for i in $(seq 1 30); do ./target/release/action run examples/${b}.at >/dev/null || exit 1; done
done
./target/release/action run examples/bench_cow.at | tail -1   # 11
```

---

### P1 — CI 稳定（Tue–Wed）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| P1-1 | **Proptest 升格** ✅ | 256 cases；`ci.yml` 无 `continue-on-error` | CI blocking 且绿 |
| P1-2 | **Scheduled Benchmark** ✅ | cron Benchmark job 全绿 | schedule run success |
| P1-3 | **CI 冒烟扩展** ✅ | release 冒烟含 `test_cow_insert_isolation.at` | `ci-linux.sh` 第 90 行已含 |

---

### P2 — 性能基线（Wed–Thu）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| P2-1 | **AOT 基线重刷** ✅ | `./benchmark.sh --mode aot --opt 2 -n 5` → `benchmark_results_aot_o2.txt` | post P0-3 + list_get_cached |
| P2-2 | **JIT 对照** ✅ | `./benchmark.sh -n 3` → `benchmark_results.txt` | 已重刷 |
| P2-3 | **`list_get_cached` 缺口** ✅ | `for_loop`/`hir_compile`/`misc` index + UFCS `get` 在顺序 for 内用 loop cache | 见 `perf_map_note.md` |
| P2-4 | **Map rebuild 采样** ✅ | `bench_map_10k` JIT ~133–137 ms；AOT exe ~7–8 ms；rebuild 非 step smoke 主导 | `scripts/perf_map_note.md` |
| P2-5 | **`action_list_reverse` 叶扫描** ✅ | B-tree reverse walk 替代 get+push | `define_list_xform.rs` |
| P2-6 | **`ht_from_list` get_cached** ✅ | 顺序 list→set 用 `list_get_cached` | `define_hash_table.rs` |
| P2-7 | **opt_pass nounwind 扩展** ✅ | reverse/walk/ht 热函数 | `opt_pass.rs` |

### P3 — 自举 M4 起步（Thu–Fri）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| P3-1 | **Lexer golden 框架** | `tests/lexer_golden.rs` 或扩展现有 lexer 测试：fixtures `tests/fixtures/lexer/*.at` → token JSON | `cargo test lexer_golden` 绿 |
| P3-2 | **`bootstrap/lexer.at` 试点** | 仅 bootstrap 子集：读 `String`、输出 token 描述（可先 `println` 固定样例） | 目录 `bootstrap/` + README 一页 |
| P3-3 | **HIR round-trip 夹具** | 为 3–5 个 bootstrap 子集 `.at` 加 `hir_golden` 条目 | `cargo test hir_golden` 绿 |
| P3-4 | **子集边界测试** | 禁止特性（`import` / `Task` / `lazy val`）在 bootstrap 样例中 compile-error oracle | integration 或 fixtures |

详见 `doc/bootstrap-subset.md` 里程碑 M4–M6。

---

### P4 — 文档（Fri，可与实现并行）

| ID | 任务 | 说明 | 验收 |
|----|------|------|------|
| P4-1 | **`doc/language-spec-outline.md`** | 章节：类型、可空、CoW 集合、UFCS、模式、TCO；每项链到 integration 测试名 | 新 doc + ARCHITECTURE 链接 |
| P4-2 | **`doc/stdlib-layers.md`** | `builtin` / `lib/*.at` / `stdlib/*.atom` / `host-rt` 职责表 | 新 doc |
| P4-3 | **更新 roadmap 数字** | `roadmap-and-bootstrap-analysis.md` §2.3：140 → 152 integration、HIR 状态 | 只改度量表，不大改结论 |

---

## 建议日程

| 日 | 重点 | 产出 |
|----|------|------|
| 一 | P0-1 压力测试 + 30× bench | 新 integration + 稳定跑通 |
| 二 | P0-3（若需要）+ P1-1 hosted | RC 或 CI 一项闭环 |
| 三 | P1-2 proptest + P2-1 AOT 基线 | blocking proptest；新 baseline commit |
| 四 | P3-1–P3-3 bootstrap 框架 | lexer golden + bootstrap/ 目录 |
| 五 | P4 文档 + P2-3 热点 + 全量 CI | 文档 PR；`gh run watch` 全绿 |

---

## 验收总表（周末 checklist）

- [x] `cargo test --release --test integration -- --test-threads=1` → **155 passed**
- [x] insert 系列 bench **30×** release 无 crash
- [x] `bench_cow.at` → `11`；`map_filter.at` → `210215`
- [x] `./benchmark.sh --mode aot --opt 2` regression 相对新基线无 CRASH/FAIL
- [x] Linux（自托管 NixOS）/ Windows / Benchmark CI 绿
- [x] （若 P1-2 完成）proptest job blocking 且绿
- [x] （若 P3 完成）`lexer_golden` 或 `bootstrap/README.md` 存在
- [x] （若 P4 完成）`language-spec-outline.md` + `stdlib-layers.md`

---

## 风险与不做项

| 风险 | 缓解 |
|------|------|
| 延迟释放导致内存峰值 | P0-2 记录；必要时 P0-3 per-node dec |
| 自托管 runner 离线 | 监控 runner 健康；本地 `nix-shell` + `ci-linux.sh` 复现 |
| 自举范围膨胀 | 严格 `bootstrap-subset.md`；本周只做 lexer 试点 |
| 性能与语义冲突 | 遵守 `preserve-language-semantics.mdc`；先快检再 benchmark |

**本周明确不做：**

- 重写 `define_list_core.rs` 大块算法
- 全量 port `parser.rs` / `typecheck.rs`
- VSCode Marketplace / `atom.toml` 生态
- LLVM PassManager 默认 `O2` 浇在整 module runtime IR 上

---

## 参考

- 语义约束：`.cursor/rules/preserve-language-semantics.mdc`
- 架构：`doc/ARCHITECTURE.md`
- 路线图：`doc/roadmap-and-bootstrap-analysis.md`
- 子集：`doc/bootstrap-subset.md`
- 性能：`BENCHMARK.md`
