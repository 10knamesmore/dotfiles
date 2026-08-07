---
name: handoff
description: 当需要把 Wayfinder Spec/Subspec 中的任务通过 handoff 文档分发给另一个 Agent session，并要求对方完成后回写 review transport 时使用。
---

开始前完整读取 [HANDOFF-FORMAT.md](references/HANDOFF-FORMAT.md)。

## 通信模型

`handoff` 与 `wayfinder` 配合，把一次跨 Agent 推进建模为两个方向相反的 once transport：

```text
Agent A：维护 wayfinder / Spec
    |
    | task transport
    v
Agent B：claim、实现、验证、写 Resolution/Evidence
    |
    | review transport
    v
Agent A：review 真实效果、更新 Spec/frontier
```

- Agent A 是本轮的 planner/reviewer，负责选择 frontier、维护 Spec，并在 B 返回后 review。
- Agent B 是本轮的 executor，负责推进目标 Subspec，并把结果和 evidence 返回给 A。
- `handoff` Skill 只由 A 使用。B 只接收生成的 task transport，不需要安装或读取本 Skill。
- task transport 必须包含 Parent Spec、目标 Subspec 与直接 dependencies 的精确指针；它传递导航、live state 与本轮动作，不复制 Spec contract。
- B 一般可以使用 `wayfinder` 与 `implement`。task transport 应要求 B 优先使用可用的对应 Skill，同时内联 claim、Resolution、Evidence、验证和回传步骤作为明确 fallback。
- transport 文件构成一个 single-slot half-duplex channel。同一 channel 同一时刻只允许一份未完成 transport；不得覆盖、追加或改写对方尚未完成的 transport。
- 接收方完成 transport 要求的动作后删除 inbound transport，再由自己写一份反向 transport。handoff 不保存长期会话历史。

## 不变量

- Spec、Subspec、domain docs、源码与项目指令是 canonical authority；transport 只传递本轮任务或 review 请求。
- A 必须亲自核对 live source、worktree、git state 与验证结果并生成 outbound transport，不把写 transport 再委托出去。
- receiver 不因 transport 的结论而跳过 source-grounded verification。A review 时必须重新检查真实 diff、源码和测试效果。
- 完成、剩余、失败和未验证必须分开；不得用进度百分比或模糊的完成了代替 evidence。
- 保护用户和其他 Agent 的 dirty changes。不得为整理 channel 而 reset、checkout、clean、reformat、stage、commit 或 push。
- 所有面向接收 Agent 的 instruction 使用中文；代码 identifier、命令、路径和无法自然翻译的技术名词保持原文。

## 流程

### 1. 识别当前方向

使用本 Skill 的当前 Agent 是 A。先确定 channel 所处阶段：

- A 准备把 wayfinder frontier 交给 B：写 `task transport`。
- channel 中存在 B 返回的 review transport：A 先完成 review，不得直接生成下一轮任务。

channel 中已有 task transport 时，说明 B 的工作仍在进行，A 不得覆盖或发送第二份消息。channel 中已有 review transport 时，A 在完成 review 前不得删除。

### 2. 重新读取 authority 与 live state

向上定位并完整读取适用的 `AGENTS.md`、`CLAUDE.md`、README 和维护说明。一般应存在对应的 Parent Spec 与目标 Subspec；读取完整 Spec、目标 Subspec、直接 dependencies、domain glossary 和相关源码，不依赖对话摘要或旧行号。

逐个相关 repo/worktree 检查 branch、`git status --short --branch`、scoped staged/unstaged diff、lockfile、生成物和验证记录。引用代码事实前重新读取对应文件，并使用 `path/to/file:line`。

区分 user-owned、other-agent-owned、本任务和 ownership 未知的 dirty state。无法安全重跑的验证明确标记来源、时间和限制。

### 3. A 写 task transport

A 使用 `wayfinder` 选择一个位于 frontier、尚未被 claim 的目标 Subspec。A 不代替 B claim，也不把 Spec contract 复制成另一套计划。

task transport 至少写清：

- Parent Spec、目标 Subspec、直接 dependencies 与 domain glossary 的 absolute path 或可解析链接，以及 canonical read order；
- outcome、scope、out-of-scope 和 settled contract；
- live code seam、dirty ownership 与保护边界；
- dependency-aware 执行顺序、验证命令、Definition of Done 和停止条件；
- B 完成后必须更新的 Resolution/Evidence/status；
- B 可用时应调用的 `wayfinder` / `implement`，以及不可用时仍可执行的显式 fallback；
- B 需要返回给 A review 的具体效果、evidence、review transport 格式与 exact output path。

A 验证 transport 后交给 B。A 不删除 outbound transport，也不在 B 完成前写第二份消息。

### 4. 把 B 的执行与回传闭环写进 task transport

A 生成的 task transport 必须直接要求 B：

1. 完整读取 handoff 指向的 Parent Spec、目标 Subspec、直接 dependencies、项目指令和源码；
2. 可用时使用 `wayfinder` claim 目标 Subspec，并在 `kind: implementation` 时使用 `implement`；如果 Skill 不可用，按 handoff 内联步骤完成相同的 frontmatter、Resolution、Evidence 和验证动作；
3. 只有实际满足 acceptance criteria、写入 Resolution/Evidence 并完成要求的验证后，才把 Subspec 标记为 `resolved`；
4. 基于 live state 准备 review transport，写明 changed artifacts、acceptance criteria evidence、验证结果、偏差、未验证项、dirty ownership 和希望 A 重点 review 的效果；
5. 删除已完成的 inbound task transport，再按 handoff 给出的 exact path 与格式写入 review transport。

task transport 还要说明真正阻塞时的 fallback：B 写 `status: blocked` 的 review transport，提供 blocker、已尝试方案、现有 evidence 和需要 A 或用户决定的事项。这个 status 只属于 transport；Subspec frontmatter 按 Wayfinder contract 保持 `in-progress` 或释放为 `ready`，绝不写 `blocked`。

### 5. A review 并推进 wayfinder

A 完整读取 review transport，然后独立检查真实 diff、源码、运行过的命令、测试覆盖和用户可见效果。transport 的完成声明只是 review index，不是验收证据本身。

- review 通过：按 `wayfinder` 更新适用的 Parent Spec canonical state、frontier、fog 和整体 status；只有 resolved decision 才进入 decision summary。
- review 发现精确问题：把问题写入新的 Subspec 或明确 follow-up，再生成下一轮 task transport；不要把评论追加进旧 transport。
- review 需要用户决定：写回 decision Subspec 或 fog，并停止，不替用户选择。

A 完成 review 和 canonical state 更新后删除 inbound review transport。需要继续下一轮时，再创建新的 task transport；删除前不得复用同一路径。

### 6. 验证 outbound transport

交付前完整回读并检查：

- direction、sender、receiver、transport type 和目标 Spec/Subspec 明确；
- Parent Spec、目标 Subspec 和直接 dependency 指针都能解析，且 B 不需要读取 `handoff` Skill；
- `wayfinder` / `implement` route 与无 Skill 时的 fallback 都写进 task transport；
- channel 中没有另一份尚未完成的 transport；
- branch、status、claim、dependency、命令、路径和 line reference 与 live state 一致；
- 完成、剩余、失败和未验证没有矛盾；
- dirty ownership 足以防止覆盖、误格式化或误 stage；
- 删除命令只指向当前 inbound transport，不含 glob、变量或递归删除；
- 不包含 secret，也没有授权未获批准的 commit、push、部署或 side effect；
- transport 删除后，Spec、Subspec、源码和 evidence 仍能还原长期状态。

向用户返回 transport 的 clickable absolute path、方向、接收者和验证边界。sender 不删除刚发布的 outbound transport。
