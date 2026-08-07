# Handoff Format

本文件只供生成 task transport 的 Agent A 使用，Agent B 不需要读取本 Skill 或本 reference。A 必须把 Spec 指针、执行要求和 review 回传协议写入生成的 handoff。`task transport` 与 `review transport` 共享一个 single-slot half-duplex channel：接收方完成当前 transport 要求的动作后删除它，sender 才能在反方向发布下一份 transport。

## 目录

- [Channel contract](#channel-contract)
- [公共文件头](#公共文件头)
- [Task transport](#task-transport)
- [完成后的回传协议](#完成后的回传协议)
- [Review transport](#review-transport)
- [删除命令](#删除命令)
- [最终自检](#最终自检)

## Channel contract

- 一份 transport 只服务一个方向、一个 receiver 和一个目的。
- channel 中已有未完成 transport 时，不得覆盖、append、rename 成回复或创建并行的第二份消息。
- receiver 完成 transport 请求的动作后删除 inbound，再发布自己的 outbound transport。
- transport 不记录累计历史。长期 contract、status、Resolution 和 Evidence 必须写入 Spec/Subspec 或源码。
- task transport 必须指向 Parent Spec、目标 Subspec 与直接 dependencies；不复制 Spec，也不把查找 canonical contract 留给 B 猜。
- B 不依赖 `handoff` Skill。可用时由 B 使用 `wayfinder` / `implement`，不可用时执行 task transport 内联的等价步骤。
- 默认把 transport 放在活动 Spec 的 artifact 目录，文件名遵循 repo 约定；同一路径只能在上一份 transport 删除后复用。
- 不把 transport 加入 staging、commit、issue tracker 或长期文档索引。

## 公共文件头

每份文件先声明方向和生命周期：

```markdown
# <工作名称>：<任务交接或结果回传>

> 方向：Agent <sender> -> Agent <receiver>
> 类型：<task transport 或 review transport>
> 这是一次性 transport。receiver 完成本文要求的任务或 review 后删除本文件；不要提交、移动、归档、复制或把回复追加到本文件。长期 authority 是下文列出的 Spec、Subspec、源码与 Evidence。
>
> 删除命令：`<只删除本 transport 的 exact command>`
```

sender/receiver 使用本轮可辨认的 Agent/session label，不写无法定位的我、你或下一个 Agent。

## Task transport

task transport 由维护 wayfinder 的 A 写给执行目标 Subspec 的 B。

### 任务

定义唯一 outcome，并列出：

- Parent Spec、目标 Subspec 与直接 dependencies 的精确指针；
- B 必须完成的 acceptance criteria；
- scope 与 out-of-scope；
- 用户已经授予和没有授予的操作权限；
- B 完成后要返回 A review 的具体效果。

不要写继续处理、看情况收尾或完成剩余工作。目标 Subspec 必须位于 frontier 且尚未被其他 session claim。

### 必读顺序

按依赖顺序列出 absolute path：

1. repo/global instructions；
2. Parent Spec 的 absolute path 或可解析链接；
3. 目标 Subspec 的 absolute path 或可解析链接；
4. 直接 dependency Resolution 与 domain glossary 的精确指针；
5. 关键源码、配置、lockfile 和测试入口。

要求 B 在修改前重新运行 live-state 检查。行号只是定位提示，不代替重新读取。

### 当前状态

分成互斥集合：

| 集合 | 内容要求 |
| --- | --- |
| 已完成 | 只有已有 artifact 或当前 evidence 的结果 |
| B 需要完成 | 达到目标 Subspec acceptance criteria 仍缺少的事项 |
| 未验证或失败 | 没有运行、结果过时、运行失败或被 unrelated baseline 阻断的检查 |

不要把设计完成写成实现完成，也不要把编译通过写成行为验证通过。

### 已定 contract

只写已经由用户、Spec、resolved dependency 或代码确定且 B 不得重新选择的约束。每项指向 authority；不要把 A 的建议包装成决议。

### 当前代码与数据入口

只列能开始工作的最小 live map：

- exact repo/worktree root 与 branch；
- 实现入口、关键 module、schema、配置、测试和生成路径；
- 多 repo 版本、lockfile、发布或本地 link 关系；
- 必须先重新读取的文件和原因。

### Dirty worktree 与 ownership

每个相关 repo 单独写：

- `git status --short --branch` 摘要；
- 本任务修改；
- user-owned、other-agent-owned 或 ownership 未知的修改；
- staged、unstaged、untracked 和 generated state；
- 禁止修改、revert、stage、格式化或清理的路径。

禁止声称所有 dirty changes 都属于本任务。

### 执行顺序

给出 dependency-aware numbered steps。每一步写明目标、seam、需要先读取或验证的内容、产生的 canonical artifact 和进入下一步的可观察条件。

要求 B 开始时自行 claim Subspec。B 可用时使用 `wayfinder`，implementation 使用 `implement`；同时写明无 Skill 时等价的 frontmatter transition、owner、Resolution、Evidence 和验证步骤。A 不代替 B 写 owner。

### 验证与完成定义

分别列出：

- 已运行的 exact command、cwd、结果和限制；
- B 仍需运行的 exact command、cwd 和覆盖面；
- 禁止运行的真实客户端、部署、发消息、下单、删除数据或其他 side-effect command；
- 可逐条核验的 Definition of Done；
- B 必须写回 Subspec 的 Resolution、Evidence 和最终 status。

### 停止条件

具体列出需要 B 停止并返回 blocked review transport 的情形，例如改变 settled contract、public API、persisted schema、安全边界，或需要新权限、secret、发布、部署、真实业务操作和 dirty ownership 冲突。

### 完成后的回传协议

task transport 必须把以下协议直接写给 B，不能只让 B 读取本 reference：

1. 完成或确认阻塞后，先把长期状态写入 handoff 指向的 Subspec、源码与 Evidence；
2. 准备 review transport 所需的实际结果、acceptance criteria evidence、changed artifacts、验证、偏差、未验证项和 dirty ownership；
3. 删除 inbound task transport；
4. 在 task transport 指定的 exact output path 创建 review transport；
5. review transport 使用下节的字段，并要求 A review 真实 diff、源码、测试和用户可见效果。

把 review transport 的 required section、transport status 语义和删除命令完整内联。不得写成使用 `$handoff` 生成回复，也不得假设 B 能访问本 reference。

## Review transport

review transport 由完成或阻塞任务的 B 按 task transport 内联的协议写回 A。B 不需要 `handoff` Skill；本节用于 A 生成那段回传协议。review transport 请求 A review 真实效果，不只是阅读完成摘要。

### Review 请求

声明：

- Parent Spec 与目标 Subspec；
- `status: completed` 或 `status: blocked`；
- 希望 A 验证的用户可见效果、contract 和高风险 seam；
- A 完成 review 后应更新的 canonical state。

这里的 `status` 是 review transport 的状态，不是 Subspec frontmatter。Subspec 继续遵守 `wayfinder`：不得持久化 `blocked`，阻塞时保持 `in-progress` 或释放为 `ready` 并记录原因。

### 实际结果

按 acceptance criteria 逐条映射：

| Acceptance criterion | Result | Evidence |
| --- | --- | --- |
| `<criterion>` | `satisfied`、`not satisfied` 或 `not verified` | `path/to/file:line`、test 或 artifact |

不要只写全部完成。`status: completed` 仍可包含明确标注、不会阻止 acceptance 的 known limitation；阻止 acceptance 的问题必须使用 `status: blocked`。

### Changed artifacts

列出：

- 修改、新增和删除的 source/artifact；
- 关键 diff seam 和对应 `path/to/file:line`；
- 未修改但 A 容易误判为本任务的 dirty files；
- Spec/Subspec、domain docs、generated state 或 lockfile 的实际变化。

不要粘贴整段 diff；A 必须自己读取 live diff。

### 验证结果

分开记录 formatter、type check、lint、unit test、integration test、manual/visual check 与禁止运行的真实 side effect。每项给出 exact command、cwd、退出结果、覆盖面和 unrelated baseline。

没有运行的验证写 `not run`，不能从其他 gate 推断通过。

### 偏差、未验证项与 blocker

- 记录与 task transport 或 Spec 的偏差及原因；
- 记录未验证行为、平台、feature 或 distribution boundary；
- `status: blocked` 时写明重复 blocker、已尝试方案、现有 evidence 和需要 A 或用户决定的事项；
- 不在 review transport 中擅自扩大 scope 或提出临时双写、字符串 fallback、跳过验证等方案。

### Dirty worktree 与 ownership

返回最新的 scoped git state，明确 B 的修改、保留的 user-owned/other-agent-owned 修改、staging 状态和禁止 A 在 review 时清理的路径。

### A 的 review checklist

列出 A 必须亲自检查的最小集合：

- 真实 diff 与关键 source path；
- acceptance criteria 与 external behavior；
- validation command 和失败边界；
- security、persistence、compatibility 或 side-effect seam；
- Subspec Resolution/Evidence/status 与实际结果是否一致；
- review 通过后应推进的 Spec/frontier，或 review 失败时应创建的精确 follow-up。

review transport 不替 A 宣布验收通过。

## 删除命令

- 使用当前 transport 的 absolute path。
- POSIX 使用 `rm -- '<absolute-path>'`，PowerShell 使用 `Remove-Item -LiteralPath '<absolute-path>'`。
- 不使用 `rm -r`、`rm -rf`、glob、环境变量、`~` 或未解析的相对路径。
- command 只删除当前 inbound transport，不删除父目录、Spec、Subspec 或 artifact。
- receiver 只有在完成本文请求的任务或 review 后才执行；sender 不删除刚发布的 outbound transport。

## 最终自检

1. direction、sender、receiver 和 transport type 是否明确？
2. channel 是否只有一份未完成 transport，且没有覆盖对方消息？
3. 一个看不到原对话的 receiver 能否找到 Spec/Subspec 并直接开始？
4. 每个完成声明是否有 current evidence？
5. dirty ownership 是否足以防止误覆盖、误格式化和误 stage？
6. review transport 是否要求 A 检查真实效果，而不是直接信任 B？
7. 删除命令是否只命中当前 transport？
8. transport 删除后，canonical artifacts 是否仍保留全部长期状态？
