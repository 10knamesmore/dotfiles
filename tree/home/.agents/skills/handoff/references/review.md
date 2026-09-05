# 审查回传

A 先读取 B 的 review transport，再核对相关 repo 的真实 diff、关键源码、验收证据和用户可见效果。已有检查足以证明结果时不重复跑全套测试；有新改动、失败或证据缺口时执行相关验证。

- 验收通过：按 [Wayfinder](../../wayfinder/SKILL.md#完成与修正) 更新 Subspec、Parent Spec 和后续 frontier；只有整体验收满足时才将 Spec 设为 complete。
- 局部 bug 或与已定要求不符：在原 Subspec 中记录差异并直接修复、验证，不另开多轮交接。检查受影响的依赖结论，不重做无关工作。
- 必须由用户决定的问题：写入对应 decision Subspec 或 fog，仅暂停依赖该决定的工作。
- 新的独立大范围工作：记录为新的 Subspec，授权与约定明确后再发新的任务交接。

长期结果与状态更新完成后，删除当前 inbound review transport。删除规则见 [HANDOFF-FORMAT.md](HANDOFF-FORMAT.md#删除命令)。通道空闲后才可发下一份任务；不能把 review 评论追加到旧消息。
