---
name: gh-cli
description: 在需要 GitHub 相关操作、寻找合适的开源项目或解决方案、以及阅读某个开源项目源码时使用。通过 GitHub CLI `gh` 完成仓库查看、搜索、PR 和 Issue 查询、Actions 检查，以及按需调用 GitHub API。
---

# GitHub CLI

用这个 skill 把 GitHub 任务转换成明确的 `gh` 命令序列。

## 工作方式

1. 先确认上下文。
   如果用户说“这个仓库”，先确认当前目录、远程仓库和默认目标仓库。
   任何依赖 GitHub 访问的操作前，优先检查 `gh auth status`。

2. 先读后写。
   优先用只读命令确认目标对象，例如 `gh repo view`、`gh pr view`、`gh issue list`、`gh run list`。
   在执行创建、编辑、评论、合并、关闭、删除等变更前，明确对象和参数。

3. 优先结构化输出。
   如果结果要给后续命令使用，优先选择 `--json`、`--jq` 或 `gh api`。
   只有在用户明确要可读摘要时，再偏向纯文本输出。

4. 尽量避免依赖交互式提示。
   有非交互参数时，优先使用显式 flag。
   让每条命令都能清楚表达它是“查看”“创建”“编辑”“合并”还是“重跑”。

## 命令选择

- 用 `gh auth ...` 处理登录状态、token、host 与权限刷新。
- 用 `gh repo ...` 处理仓库信息、克隆、创建、fork、同步和默认仓库。
- 用 `gh pr ...` 处理 PR 列表、查看、checkout、创建、review、checks 和 merge。
- 用 `gh issue ...` 处理 issue 列表、查看、创建、评论、编辑和状态流转。
- 用 `gh run ...` 与 `gh workflow ...` 处理 GitHub Actions 的运行记录、日志、重跑和手动触发。
- 当高级子命令拿不到需要的字段，或需要分页、GraphQL、自定义请求时，用 `gh api ...`。

具体命令模式见 [gh-workflows.md](/home/wanger/dotfiles/general/skills/gh-cli/references/gh-workflows.md)。
需要完整命令说明、全部参数、示例和子命令列表时，直接查看 GitHub CLI 官方 manual 原文：<https://cli.github.com/manual/gh>。

如果任务偏向开源调研或源码阅读，优先进入对应子工作流：

- 寻找开源项目或解决方案：见 [open-source-discovery.md](/home/wanger/dotfiles/general/skills/gh-cli/references/open-source-discovery.md)
- 阅读某个开源项目源码：见 [open-source-source-reading.md](/home/wanger/dotfiles/general/skills/gh-cli/references/open-source-source-reading.md)

## 执行规则

- 当前目录未必就是目标仓库时，优先加 `--repo OWNER/REPO`。
- 优先使用 `--json` 加 `--jq`，避免脆弱的文本解析。
- 判断 CI 状态时，优先用 `gh pr checks`、`gh run view`、`gh run watch`，不要只凭列表页下结论。
- 多页 API 结果优先考虑 `gh api --paginate`。
- 对 `gh pr merge`、`gh issue edit`、`gh variable set`、`gh secret set` 这类写操作，先明确目标对象再执行。

## 典型触发

- “列出我负责的 open PR”
- “基于当前分支创建一个 PR”
- “看一下最近失败的 GitHub Action”
- “给 issue 123 留个评论”
- “查看这个仓库的默认分支和可见性”
- “用 GitHub API 查一个 `gh` 子命令拿不到的字段”
