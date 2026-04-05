# GH Workflows

## 适用范围

当任务需要具体 `gh` 命令、常用 flag、结构化输出方式或典型工作流时，读取这个文件。
如果需要完整内容，不要只依赖这个摘要，直接查看 GitHub CLI 官方 manual 原文：<https://cli.github.com/manual/gh>。

## 认证

- 查看登录状态：`gh auth status`
- 交互式登录：`gh auth login`
- 输出当前 host 的 token：`gh auth token`
- 权限不足时刷新授权：`gh auth refresh`

## 仓库上下文

- 查看当前仓库：`gh repo view`
- 查看指定仓库：`gh repo view OWNER/REPO`
- 克隆仓库：`gh repo clone OWNER/REPO`
- 创建仓库：`gh repo create`
- 设置默认仓库：`gh repo set-default OWNER/REPO`
- 同步 fork：`gh repo sync`

如果当前目录不可靠，优先带上 `--repo OWNER/REPO`。

## Pull Request

- 列出 PR：`gh pr list`
- 以 JSON 列出 PR：`gh pr list --json number,title,headRefName,author,state`
- 查看单个 PR：`gh pr view 123`
- 检出 PR：`gh pr checkout 123`
- 从当前分支创建 PR：`gh pr create --fill`
- 创建 draft PR：`gh pr create --draft --fill`
- 查看 PR 检查状态：`gh pr checks 123`
- Approve PR：`gh pr review 123 --approve`
- Request changes：`gh pr review 123 --request-changes --body "原因"`
- 显式合并 PR：`gh pr merge 123 --merge`
- Squash 合并并删除分支：`gh pr merge 123 --squash --delete-branch`

用户问“我现在有什么 PR 需要处理”时，优先考虑 `gh pr status`。

## Issue

- 列出 issue：`gh issue list`
- 以 JSON 列出 issue：`gh issue list --json number,title,labels,assignees,state`
- 查看 issue：`gh issue view 123`
- 创建 issue：`gh issue create --title "..." --body "..."`
- 给 issue 评论：`gh issue comment 123 --body "..."`
- 编辑标签等元数据：`gh issue edit 123 --add-label bug --remove-label triage`
- 关闭或重新打开：`gh issue close 123`、`gh issue reopen 123`

用户要看自己相关 issue 总览时，优先考虑 `gh issue status`。

## GitHub Actions

- 列出最近运行：`gh run list`
- 查看单次运行摘要：`gh run view RUN_ID`
- 只看失败日志：`gh run view RUN_ID --log-failed`
- 持续观察运行：`gh run watch RUN_ID`
- 重跑运行：`gh run rerun RUN_ID`
- 列出 workflows：`gh workflow list`
- 手动触发 workflow：`gh workflow run WORKFLOW`

用户说“最新失败的 workflow”，先用 `gh run list` 锁定目标，再用 `gh run view` 深入。

## 直接调用 API

- 列出 releases：`gh api repos/{owner}/{repo}/releases`
- 给 issue 发评论：`gh api repos/{owner}/{repo}/issues/123/comments -f body='Hi from CLI'`
- 搜索 issue：`gh api -X GET search/issues -f q='repo:OWNER/REPO is:open label:bug'`
- 只取字段：`gh api repos/{owner}/{repo}/issues --jq '.[].title'`
- 调 GraphQL：`gh api graphql -f query='query { viewer { login } }'`
- 分页抓取：`gh api --paginate ...`
- 把分页结果包成一个数组：`gh api --paginate --slurp ...`

适合改用 `gh api` 的情况：

- 高级子命令拿不到目标字段
- 任务需要 GraphQL
- 任务需要自定义 header、preview 或特殊请求体
- 任务需要分页控制

## 输出模式

- 子命令支持 `--json` 时，优先用它。
- 只关心少量字段时，优先配合 `--jq` 提取。
- 只有最终输出要直接给人看时，再考虑 `--template`。

示例：

- 只输出 PR 编号：`gh pr list --json number --jq '.[].number'`
- 只输出 open issue 标题：`gh issue list --json title --jq '.[].title'`
- 只输出失败 run 的 ID：`gh run list --json databaseId,conclusion --jq '.[] | select(.conclusion=="failure") | .databaseId'`

## 安全提示

- `create`、`edit`、`merge`、`close`、`delete`、`rerun`、`secret set`、`variable set` 都视为写操作。
- 命令可能跨仓库或跨 host 时，把目标写全。
- workflow 触发、PR 合并这类动作依赖上下文时，先确认当前分支、目标 PR 和仓库。
