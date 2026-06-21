# 性能测试

## 快速开始

在 `nix-shell` 内、项目根目录执行：

```bash
# 全量基准（JIT 全路径，默认预热 1 次 + 计时 3 次）
nix-shell --run "./benchmark.sh"

# 纯运行时（AOT 编译一次后只测 exe 执行）
nix-shell --run "./benchmark.sh --mode aot -O2"

# 指定迭代次数、优化级别
nix-shell --run "./benchmark.sh -n 5 -O0"
```

## benchmark.sh 选项

| 选项 | 说明 |
|------|------|
| `-n, --iterations N` | 每项基准计时次数（默认 3） |
| `--mode run\|aot` | `run`：每次 `action run`（含 JIT 编译+执行）；`aot`：先编译 exe 一次，**仅对 exe 执行计时**（不含 emit/link） |
| `-O0`…`-O3` / `--opt N` | LLVM 优化级别 |
| `--warmup` / `--no-warmup` | 是否先跑 1 次不计时（默认预热） |
| `-b, --build` | 运行前先 `cargo build --release` |
| `-l, --list` | 列出所有 `examples/bench_*.ac` |
| `--results FILE` | 结果输出文件（默认 `benchmark_results.txt`） |
| `-p, --profile` | 向 `action run` 传递 `--profile`（仅 run 模式） |

环境变量：`ACTION_BENCH_ITER`、`ACTION_BENCH_MODE`、`ACTION_BENCH_OPT`、`ACTION_BENCH_WARMUP`

## 深度分析脚本（`scripts/`）

在 **release 二进制已构建** 的前提下：

```bash
cd ~/Action && nix-shell --run "python3 scripts/perf_phase_split.py"
cd ~/Action && nix-shell --run "python3 scripts/perf_report.py"
```

| 脚本 | 作用 |
|------|------|
| `scripts/perf_phase_split.py` | 拆分 check / build / run / JIT+执行 耗时（毫秒） |
| `scripts/perf_report.py` | 完整报告：跑 `benchmark.sh`、阶段拆分、AOT 纯执行、step 增量、快慢 Top5 |
| `scripts/git-push.sh` | 推送 GitHub（优先直连，失败再走 FlClash 7890） |

## 基准用例

程序位于 `examples/bench_*.ac`：

| 类别 | 文件 |
|------|------|
| 全量 | bench_all |
| 渐进定位 | bench_step1 ~ bench_step6 |
| 循环 / 高阶 | bench_for_chain, bench_for_collect, bench_for_list, bench_for_method, bench_for_nested, bench_for_range |
| 集合 | bench_map, bench_map_10k, bench_set, bench_set_10k |
| 枚举 / 模式 | bench_enum, bench_when |
| 函数 | bench_funcall, bench_lambda |
| 字符串 / 数学 | bench_string, bench_math |
| CoW | bench_cow |
| 深度 concat / fused map+filter | bench_concat_depth（1000 元素 append 树 + `filter(map(lst){f}){g}` 融合链；集成测试 `test_bench_concat_depth`） |
| insert 梯度 | bench_insert2/10/50/100；bisect 调试见 `examples/_dev/bench_insert_bisect*` |

## 测量说明

1. **`run` 模式（默认）**：墙钟时间 = 编译 IR + JIT 冷启动 + 程序执行；小用例中 JIT 固定开销可达 ~50–60 ms。**`-O2` 会显著增加 JIT 编译耗时**，轻量用例总时间可能反而变长。
2. **`aot` 模式**：每个用例先 `action run --emit exe` 编译一次（不计时），预热与计时段**只运行 exe**，反映纯运行时；适合对比 `-O0` vs `-O2` 的代码生成质量。
3. **阶段拆分**：`perf_phase_split.py` 用 `build` 与 `run` 差值估算 JIT+执行。
4. 对比优化前后应在同一机器、同一 `nix-shell` 环境下，使用 **release** 二进制，且 **aot 模式用同一 `--mode`**。

## 优化后验证清单

```bash
nix-shell --run "cargo test -- --test-threads=1"
nix-shell --run "./target/release/action run examples/bench_cow.ac"   # 预期输出 11
nix-shell --run "./benchmark.sh -n 3"
nix-shell --run "python3 scripts/perf_report.py"
```


## Post-optimization（2026-06）

在 Remote-SSH 远端 NixOS、`target/release/action`、**AOT 纯执行**（`./benchmark.sh --mode aot --opt 2 -n 3`，预热 1 次 + 计时 3 次，emit/link 不计入）下的快照。完整 32 项见 `benchmark_results_aot_o2.txt`。

| 用例 | Min (ms) | Avg (ms) | Max (ms) |
|------|----------|----------|----------|
| bench_map_10k | 6 | 6 | 7 |
| bench_set_10k | 8 | 9 | 10 |
| bench_all | 19 | 19 | 20 |
| bench_cow | 5 | 5 | 6 |
| bench_insert100 | 17 | 18 | 19 |
| bench_concat_depth | 15 | 16 | 16 |

**说明**：`bench_map_10k` / `bench_set_10k` 用于 1 万元素级 Map/Set 压力；与 `bench_map` / `bench_set`（小规模 smoke）互补。`bench_concat_depth` 覆盖深度 ConcatNode 与 **fused map+filter**（`filter(map(lst){f}){g}` → 单遍 `action_list_map_filter_walk`）；语义用例见 `examples/map_filter.ac`（输出 `210215`）。对比优化前后请固定 `--mode` 与 `--opt`。

性能改动须保持语言语义（见 `.cursor/rules/preserve-language-semantics.mdc`）。
