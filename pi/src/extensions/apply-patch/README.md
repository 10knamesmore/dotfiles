# apply_patch

默认与内置 `edit`、`write` 同时提供，也可通过个人配置替换它们。

## 工具选择

在 `~/.pi/agent/config.json` 中设置：

```json
{
  "apply-patch": {
    "replaceEditAndWrite": true
  }
}
```

默认值为 `false`，不干预原有工具选择。设为 `true` 后，扩展在 session 启动时从活动工具中移除 `edit`、`write`，其他工具不变；若 `apply_patch` 本身未启用，则不移除它们。显式 CLI 工具筛选仍然有效。

开启可通过 `/reload` 应用。关闭后重启 Pi，按原有工具配置重新初始化；关闭配置不会在当前会话强制重新启用工具。

## 运行依赖

PATH 中必须有 `codex`。扩展通过 `codex --codex-run-as-apply-patch <patch>` 调用本机补丁引擎，不启动 Codex agent，不调用模型，也不需要登录。`dots` 不安装 Codex；没有该命令时工具调用失败，不切换实现。

支持 Add / Delete / Update / Move、多文件、多 hunk、上下文定位和 EOF 标记。文件操作语义由本机 Codex 版本决定。失败或取消不回滚已完成的写入。

调用按顺序执行，并为所有源路径和移动目标获取 Pi 的文件修改队列，与 `edit`、`write` 共享同文件串行化。锁覆盖执行前后的文件内容快照；只将生成的差异保存在工具结果中，不保留完整快照。

## 模型接口

- Pi 内部参数：`{ input: string }`。
- `constrainedSampling` 使用 Codex 的 Lark grammar。Pi 对声明支持 grammar tools 的模型发送 custom/freeform 工具；其余模型收到上述 JSON 参数。
- description 区分这两种输入形式；patch 语法通过 `promptGuidelines` 提供。
- 不提供 Codex 的多环境选择或审批沙箱。Pi 的 `tool_call` hook 照常触发，但按 `Edit` / `Write` 名称匹配的共享规则不会自动匹配 `apply_patch`。

## 调用显示

每个文件使用独立的 Pi 风格背景块，块间留空，不再包在同一个工具背景块中。生成中的块显示操作和路径；执行结束后的块显示实际变化的路径和增删行数。折叠隐藏各块的 diff，不隐藏文件块。

生成中展开显示补丁预览，不附加行号。执行结束后展开显示文件前后快照生成的实际差异，复用 Pi 的 diff 生成、着色和单行词级高亮：删除行和上下文使用修改前行号，新增行使用修改后行号。移动分别展示源路径和目标路径的实际变化；失败或中断也显示已经发生的变化，而非声称整份补丁已生效。

这些块仍属于一次工具调用，共用原生展开/折叠状态，不拆分模型调用或执行过程。界面文案统一使用英文，文件名与代码内容保持原样。生成、执行、成功、失败和中断状态分别展示，后端诊断只返回模型，不直接作为界面文案。

## 来源

`apply_patch.lark` 原样来自 [OpenAI Codex](https://github.com/openai/codex/blob/ce0323f2f2d3b25e82806cd555237da3db4e1897/codex-rs/core/assets/tools/apply_patch.lark)，Copyright 2025 OpenAI，Apache-2.0，许可证见 `LICENSE.codex`。
