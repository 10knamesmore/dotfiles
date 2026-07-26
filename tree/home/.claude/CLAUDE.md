@~/.agents/AGENTS.md

## Claude Code 侧

- 上面「工具偏好」里标 hook 的条目由 `cc-hook`（本仓库 `cli/crates/cc-hooks/`）在 PreToolUse 阶段真正拦截，用错会被打回让你换；不是建议而是硬失败。
