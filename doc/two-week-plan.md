# 两周改进计划（2026-06-21 → 2026-07-04）

> **目标**：源文件后缀统一为 `.ac`、补齐复杂回归、文档与工具链一致、CI 全绿。  
> **验收**：`cargo test --test integration -- --test-threads=1` 全部通过；GitHub Actions Linux + Windows 绿。

---

## 第一周（2026-06-21 → 2026-06-27）

### W1-1 源文件后缀 `.at` → `.ac`

| 状态 | 任务 | 验收 |
|------|------|------|
| [x] | 重命名 `examples/`、`tests/fixtures/`、`lib/`、`bootstrap/` 下全部 `*.at` | `find . -name '*.at'` 为 0（除 `.atom`） |
| [x] | 更新 `tests/integration.rs`、CI、`benchmark.sh`、文档中的路径 | 无残留 `.at` 引用（`.atom` 除外） |
| [x] | `loader/resolve.rs`：模块查找 `ac` 优先，保留 `at` 回退 | `import` 用例通过 |
| [x] | `loader/stdlib.rs`：`math.ac` / `json.ac`；VSCode `extensions: [".ac"]` | LSP / 扩展识别 `.ac` |

### W1-2 复杂集成测试

| 状态 | 任务 | 验收 |
|------|------|------|
| [x] | `complex_cow_persist.ac` — 持久化 + CoW | integration |
| [x] | `complex_map_cascade.ac` — Map insert/union/keys | integration |
| [x] | `complex_list_ufcs_chain.ac` — UFCS 方法链 | integration |
| [x] | `complex_filter_map_fold.ac` — filter+map+fold | integration |
| [x] | `complex_nullable_when.ac` — nullable / when | integration |
| [x] | `complex_concat_mutate.ac` — Concat 树 insert/remove | integration |

### W1-3 工具链与脚本

| 状态 | 任务 | 验收 |
|------|------|------|
| [x] | `scripts/perf_report.py`、`perf_phase_split.py` 等改为 `.ac` | benchmark 脚本可运行 |
| [x] | `vscode/extension.js` 文件监视 `**/*.ac` | 扩展配置一致 |
| [x] | `.cursor/rules` 语义快检路径更新 | 文档与仓库一致 |

---

## 第二周（2026-06-28 → 2026-07-04）

### W2-1 文档

| 状态 | 任务 | 验收 |
|------|------|------|
| [x] | `README.md`、`BENCHMARK.md`、`doc/tutorial.md` 示例后缀 `.ac` | 无错误 `.at` 示例 |
| [x] | `bootstrap/README.md`、`doc/stdlib-layers.md` 路径更新 | 与 `lib/*.ac` 一致 |
| [x] | 本计划书 `doc/two-week-plan.md` 待办全部勾选 | 本文档 |

### W2-2 质量门禁

| 状态 | 任务 | 验收 |
|------|------|------|
| [x] | `cargo build --release` | 无编译错误 |
| [x] | `cargo test --test integration -- --test-threads=1` | **169** 项全绿 |
| [x] | 语义快检：`bench_cow.ac`→11、`map_filter.ac`→210215、`bench_all.ac` 无 SIGSEGV | 手动 / CI |
| [x] | commit + push；`gh run watch` 直至 CI 绿 | Actions 全通过 |

---

## 快检命令

```bash
nix-shell --run 'cargo build --release && cargo test --test integration -- --test-threads=1'
./target/release/action run examples/bench_cow.ac      # 11
./target/release/action run examples/map_filter.ac   # 210215
./target/release/action run examples/bench_all.ac
```

---

## 后续（超出本计划，未纳入待办）

- `filter+map+fold` 单遍 walk（当前两趟 + 中间 List）
- `action-codegen` 编译 warning 清零
- List P0：`insert_rec` internal overflow 等（见 `doc/week-plan.md`）
