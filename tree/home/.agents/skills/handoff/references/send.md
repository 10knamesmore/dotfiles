# 发出任务

A 交接的是已明确、可验收的完整 coding outcome。仍需用户决定的问题先在原 Spec 中澄清，不把未定设计交给 B 猜。

1. 读取适用项目指令、完整 Parent Spec、所有 in-scope Subspec 及其外部依赖结论。按本次目标查阅相关领域文档和源码，不要求无关的全仓库阅读。
2. 检查每个相关 repo/worktree 的 branch、scoped staged/unstaged diff、未跟踪文件和验证记录。区分本任务、用户、其他 agent 和归属未知的改动；只有涉及依赖或生成产物时才扩展到 lockfile 与生成路径。
3. 用 Wayfinder 核对首个目标位于 frontier，后续目标只依赖已完成或同次交接中较早的工作。A 不代替 B claim；通道已有未处理消息时先完成对应处理，不覆盖文件。
4. 按 [HANDOFF-FORMAT.md](HANDOFF-FORMAT.md) 写任务：完整 outcome、scope、已定要求、源码入口、依赖顺序、验收标准、权限、dirty ownership 与实际停止条件。
5. 把 B 的执行和回传要求内联：按依赖 claim、实现、验证、写 Resolution/Evidence、更新状态、删除已处理的 inbound，再按指定目录和命名规则写 review transport。B 可用时使用 Wayfinder/implement；不可用时按内联的等价步骤操作。

回读文件，核对路径、依赖、现场状态、权限和验收条件，确认 B 不读取本 skill 也能完成执行与回传。时间戳由写文件的执行者在写入时求值，不沿用上一份消息的时间；删除命令只指向当前 inbound 文件。完成、剩余、失败和未验证分别说明，不用进度百分比替代证据。
