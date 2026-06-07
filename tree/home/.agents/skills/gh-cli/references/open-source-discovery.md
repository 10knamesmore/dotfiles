# 子工作流：寻找开源项目或解决方案

## 目标

当用户想找现成开源项目、类库、工具、脚手架、配置方案或实现思路时，使用这个工作流。

## 工作流

1. 先检查 `gh` 是否可用。
   优先运行 `gh auth status`。
   即使 public repo 搜索和查看在 token 失效时有时仍可用，也不要假设所有查询都稳定可用。

2. 先明确搜索目标。
   提炼语言、平台、使用场景、约束条件和排除项。
   例如：`neovim plugin`、`python cli framework`、`self-hosted kanban`。

3. 先广搜，再收敛。
   优先使用 `gh search repos` 或 `gh search code` 做第一轮搜索。
   不要一开始就只盯一个项目。

4. 快速筛选候选项目。
   优先看这些信号：
   活跃度、stars、最近提交时间、issue/PR 活跃度、README 完整度、发布记录、默认分支状态。

5. 对 3 到 5 个候选项目做横向比较。
   比较功能覆盖、维护状态、技术栈适配度、集成复杂度、许可证和社区活跃度。

6. 最后再深入阅读最可能符合需求的 1 到 2 个项目。
   这一步再进入源码级阅读，避免过早下钻。

## 常用命令

- 搜索仓库：`gh search repos "keyword"`
- 搜索时排除归档仓库：`gh search repos "keyword archived:false"`
- 按星数排序搜索：`gh search repos "keyword stars:>200" --sort stars --order desc`
- 限定语言：`gh search repos "keyword language:Rust"`
- 限定主题：`gh search repos "keyword topic:neovim"`
- 搜索代码实现：`gh search code "pattern repo:OWNER/REPO"`
- 查看仓库信息：`gh repo view OWNER/REPO`
- 结构化查看仓库信息：`gh repo view OWNER/REPO --json name,description,stargazerCount,forkCount,updatedAt,licenseInfo,repositoryTopics`

优先组合使用限定词，例如：

- `gh search repos "python cli framework stars:>5000 archived:false"`
- `gh search repos "self-hosted kanban archived:false"`

## 输出要求

- 给出推荐名单时，不只列名字，要说明为什么入选。
- 如果没有完美匹配项，要明确说明“最接近的替代方案”。
- 如果用户要“解决方案”而不是“项目名单”，把候选项目按方案类别归纳。

## 适合的结果格式

- 1 句总结结论
- 3 到 5 个候选项目
- 每个项目一句优点，一句风险或限制
- 最后给出推荐优先级
