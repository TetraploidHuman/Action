# Action 仓库 — Agent 指令

## 最高优先级：墙钟效率，不计 token

1. **强烈建议同轮多开 SubAgent**：无依赖小类 ≥2 时，**默认**一条消息内并发多个 `Task`（如 explore + shell）。非强制，但单开 / 串行须有理由并在汇报中说明。
2. **主 Agent 角色**：规划、写码、协调、提交。探索 / 跑测 **首选** SubAgent；SubAgent 跑着时主 Agent **应并行写码**，勿空等。
3. **首选起手**：`TodoWrite` 拆小类 → **同消息** `Task[explore]` + `Task[shell]`（+ 更多）→ 据结果改码。仅极小任务可退化为单路径。

完整流程见 `.cursor/rules/phased-optimization-workflow.mdc`。语义约束见 `preserve-language-semantics.mdc`。
