# Context Inspector

`/context` 只读查看最近一次主会话上下文，以及独立采集的 provider payload。命令直接打开导航列表，首项为总览；有 provider payload 时，第二项为最近一次观察的 **Full request JSON**。用 `j/k` 选择，`Enter/l` 打开，`h/Esc` 返回；列表筛选、选中项和滚动位置在返回后保留，重新打开详情时恢复其滚动位置。

分区标题使用整行背景条，字段标签、数值和说明分色；增删改分别使用成功、错误和警告色。JSON 与 prompt 标签使用语法高亮并显示行号。列表按内容类别标注，选中项使用整行背景。颜色跟随当前 Pi 主题。

可查看：

- 会话配置生成的 runtime prompt，包含 forced-prompt 覆盖；另列 transcript 中的 prompt section 和工具声明。
- 转换后的上下文消息，按体积排序；显示转换后序号与原消息类型，包括压缩摘要。
- 相比上次主会话请求变化的 runtime prompt、transcript section、工具声明和消息。
- 配置中的 context 文件正文是否出现在 runtime prompt 中；此项是文本查找，不判断正文来自哪个 section。
- 配置中的 skill 目录，以及匹配的成功 `read(path)` 结果是否仍在上下文中。读取结果可能只是片段；通过其他工具读取、调用记录被移除或历史已压缩时，不能据此判断 skill 正文加载情况。
- `before_provider_request` 观察到的完整原生 payload JSON，以及拆分后的各顶层字段。

## 快捷键

普通模式沿用 `/agents`、`/workflows` 的移动与滚动约定：

| 按键 | 列表 | 详情 |
| --- | --- | --- |
| `j/k`、`↓/↑` | 移动选中项 | 滚动一行 |
| `J/K` | 移动七项 | 滚动七行 |
| `Enter/l/→` | 打开选中项 | — |
| `h/Esc/←` | 关闭检查器 | 返回上一层 |
| `g/G`、`Home/End` | 首项／末项 | 顶部／底部 |
| `Ctrl+U/D` | 滚动一行，不改变选中项 | 滚动一行 |
| `Ctrl+B/F` | 滚动半页，不改变选中项 | 滚动半页 |
| `PgUp/PgDn` | 按一页移动选中项 | 滚动一页 |
| `/`、`f` | 进入筛选模式 | — |
| `?` | 打开按键说明 | 打开说明；已在说明页时返回 |
| `q`、`Ctrl+C` | 关闭检查器 | 关闭检查器 |

筛选模式独占文字输入，`j/k/h/l/q` 都作为筛选词输入。`Enter` 保留筛选并回到普通模式，`Esc` 或 `Ctrl+C` 取消本次编辑，恢复之前的筛选、选中项和视口。

## 数据边界

上下文在 `context_with_system` 事件采集，早于 Pi 的 forced-prompt 替换、hidden-tool 过滤、provider 转换和图片过滤。transcript prompt 和工具由 Pi 的 replay API 计算；它们是投影前的状态。另用 `ctx.getSystemPrompt()` 读取当时会话配置生成的 runtime prompt，方便查看 forced-prompt 覆盖。context 文件与 skill 来源目录取自 `before_agent_start` 的 prompt 配置。

payload 在 `before_provider_request` 采集。主请求、重试和缓存预热都会触发此 hook，且事件不提供请求身份，因此 payload 使用独立编号与时间戳，单独比较最近两次观察，不归属到某个上下文快照。后续 extension handler 与 provider transport 仍可能修改数据。

体积使用 UTF-8 字节：prompt 是原始文本，其他内容按紧凑 JSON 计量，详情展示格式化 JSON。section、工具与消息显示各条目体积之和，不含条目间分隔符；payload 显示完整 JSON 的体积。消息体积排除响应 usage、时间戳与工具显示详情，但包含图片数据、opaque signature 和序列化结构。prompt、工具也出现在 system 消息中，几项体积有重叠，不能相加当作请求总量。字节数不换算成 token；token 使用当前请求对应 assistant 响应中由 Pi 归一化的 usage，全零时注明未观察到非零统计，不做 section 级归因。请求模型与 provider 返回的 `responseModel` 分开显示，后者缺失时明确标注。

只在交互式 TUI 中采集，在内存分别保留最近两次主会话上下文与最近两次 provider payload，不写日志、不追加会话消息、不调用模型。`/reload`、切换会话或树导航后清空。打开的详情是当时的只读副本，运行中的新请求不会改变当前页面。
