# Bootstrap（M4 自举试点）

Action-in-Action 编译器前端的首个试点目录。首版仅使用 `doc/bootstrap-subset.md` 允许的语言特性。

## 目标

| 里程碑 | 内容 |
|--------|------|
| M4 | `lexer.ac` token 输出与 Rust lexer golden 一致 |
| M5 | 解析 bootstrap 子集 → HIR JSON |
| M6 | Action 前端编译自身 lexer 源码 |

## 当前文件

- `lexer.ac` — recursive scanner; emits `keywords.ac` golden token kinds via `tokenize`/`containsAt`/…

## 验证

```bash
nix-shell --run 'cargo test --test lexer_golden -- --test-threads=1'
nix-shell --run 'cargo test --test bootstrap_subset -- --test-threads=1'
nix-shell --run 'cargo test --test hir_golden -- --test-threads=1'
```

## 对接

Rust codegen 消费 HIR JSON（`action check --emit hir`）。详见 `doc/ARCHITECTURE.md`。
