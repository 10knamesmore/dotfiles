---
name: grill-with-docs
description: 通过持续追问打磨 Spec 或 design，并同步维护当前 Subspec、domain glossary 与必要的 ADR。
---

# Grill with Docs

使用 grilling 与 domain-modeling，一次只问一个能够改变当前 decision 的问题。

如果存在活动 Spec：

1. 将本轮唯一问题放在一个 `kind: decision` 的 Subspec 中；格式见 `../wayfinder/references/SUBSPEC-FORMAT.md`。
2. 每当用户确认一项事实，就立即更新该 Subspec 的 context 或 resolution，不要等到 session 末尾批量回忆。
3. 只把跨多个 Subspec 生效的 contract 摘要写入 Spec；详细 reasoning 保留在当前 Subspec。
4. 新问题已经清晰时创建 Subspec；仍说不清时写回 Spec 的 `Not yet specified`。
5. 同步维护 domain glossary。只有满足难以逆转、缺少上下文会令人意外、存在真实 trade-off 三项条件时才创建 ADR。
6. decision 与 evidence 完整后把 Subspec 标记为 `resolved`，再更新 Spec status。

如果没有活动 Spec，仍执行 grilling 与 domain-modeling；只有工作规模确实需要跨 session 推进时才创建 Spec。
