---
name: handoff
description: 将已明确的实现任务通过一次性交接文档交给另一 Agent 会话，或审查其回传结果。用于一个完整交付目标，可跨 Subspec 或仓库。
---

# Handoff

当前使用者 A 负责确定交付目标、写任务交接并审查结果；接收者 B 实现、验证并回传证据。B 不需要安装本 skill，执行和回传要求由 A 写进交接文件。

## 路由

- 准备发出任务：读取 [send.md](references/send.md) 和 [HANDOFF-FORMAT.md](references/HANDOFF-FORMAT.md) 的任务与回传格式。
- 收到结果：读取 [review.md](references/review.md)，基于真实源码与结果完成 review。

## 约定

- 每份交接服务一个可独立验收的完整目标，可覆盖多个依赖有序的 Subspec、crate、repo 或 worktree。不要因首个 frontier 较小而截断用户要求的交付，也不拼接互不相关的目标。
- Spec、Subspec、领域文档、源码与项目指令保存长期约定；交接文件只传递本次任务、现场状态和回传要求，不复制完整 Spec。
- 同一交接通道一次只保留一份未处理消息。发送者不覆盖或删除已发出的文件；接收者完成任务或记录实际阻塞后删除收到的消息，再写反向回传。长期结果先记入 Spec/Subspec。
- A 亲自核对相关源码、git 状态和验证证据并写交接，不再委托别人生成交接文件。回传的完成声明不能代替 A 的 review。
- 状态按 [Wayfinder 的完成与修正规则](../wayfinder/SKILL.md#完成与修正) 更新。review 发现的局部问题由 A 在原 Subspec 就地修复；需要用户决定时仅暂停依赖该决定的工作。
- 保护用户和其他 agent 的未提交改动；交接不授予 commit、push、部署或其他外部操作权限。面向接收 agent 的指令使用中文，标识符、命令与路径保留原文。

交付时给出文件的可点击绝对路径、方向、接收者和实际验证范围。发送者保留刚发布的文件供接收者处理。
