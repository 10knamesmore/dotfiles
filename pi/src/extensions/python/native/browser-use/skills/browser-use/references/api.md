# API

通过 `import browser_use as browser` 使用。所有方法级 `timeout` 均为秒，必须有限且大于 0。连接、启动、导航、下载等待和保存默认 30 秒；其他可等待操作默认 10 秒。

## 实例与连接

| 调用 | 行为 |
| --- | --- |
| `browser.discover() -> list[BrowserInstance]` | 列出当前用户的 Chrome、Chromium、Edge、Brave 主进程，不打开连接或授权对话框。 |
| `browser.connect(*, pid=None, endpoint=None, timeout=30) -> Session` | pid 与 endpoint 必须且只能提供一个。PID 可以是浏览器主进程、窗口或子进程，SDK 沿父进程查找浏览器。endpoint 为 CDP HTTP 或 WebSocket 地址。 |
| `browser.launch(*, executable_path=None, user_data_dir=None, headless=False, timeout=30) -> Session` | 启动本机已有 Chrome/Chromium；不下载浏览器。省略数据目录时创建临时 profile；显式目录保留登录态。 |
| `browser.close_all()` | 关闭本 worker 的全部会话。已连接的用户浏览器保留，SDK 启动的浏览器退出。 |

`BrowserInstance` 字段只读：`pid: int`、`name: str`、`executable: str`、`user_data_dir: str`。数据目录是 profile 根目录，不是其中的 `Default` 或 `Profile N`。发现结果是进程快照，不保证远程调试已启用或浏览器仍存活。

PID 连接读取对应目录的 `DevToolsActivePort`。使用固定 `--remote-debugging-port` 的实例也可以按进程参数连接。没有调试端点时直接报错，不启动替代实例。Chrome 144+ 的日常会话可通过 `chrome://inspect/#remote-debugging` 启用远程调试，并由用户接受 Chrome 的连接授权。

路径由操作系统解析，不自动展开字符串中的 `~`；需要时传 `Path.home() / ...`。不同独立实例使用不同的数据目录，Chrome 不允许同时占用同一个 profile。

## Session

| 成员 | 行为 |
| --- | --- |
| `session.pid` | PID 连接时为解析出的浏览器主进程；启动模式为创建的 Chrome PID；显式 endpoint 连接时为 `None`。 |
| `session.mode` | `"attached"` 或 `"launched"`。 |
| `session.closed` | 是否已请求关闭这个 SDK 会话。浏览器外部退出导致的连接错误由后续操作报告。 |
| `session.tabs(*, timeout=10) -> list[Page]` | 返回当前浏览器的 page targets，不按桌面窗口筛选。使用 `target_id` 识别页面，不保存列表序号作为身份。 |
| `session.new_page(url="about:blank", *, timeout=30) -> Page` | 在默认浏览器 context 新建标签页。 |
| `session.close()` | 幂等。停止本会话的网络规则与下载收集；连接模式断开 WebSocket，启动模式请求 Chrome 退出，无法正常退出时终止所属进程。 |

Session 支持 `with browser.launch(...) as session:`。跨 cell 工作保留变量并显式关闭，不要在每个 cell 都重连。`close()` 后原有 Page 和 Locator 失效。

## Page

| 调用 | 行为 |
| --- | --- |
| `page.target_id` | 标签页存活期间稳定的 CDP target ID。 |
| `page.info(*, timeout=10) -> dict` | 返回 `target_id`、`window_id`、`title`、`url`。 |
| `page.goto(url, *, timeout=30)` | 导航并等待文档 load 生命周期；页面后续异步数据需另行等待。 |
| `page.evaluate(expression, *, timeout=10)` | 在主 frame 执行 JavaScript 表达式，等待 Promise，返回 JSON 可序列化的值。函数需显式调用，例如 `"(() => document.title)()"`。 |
| `page.snapshot(*, timeout=10) -> list[dict]` | Chrome accessibility tree 中未被忽略的节点，保留 CDP 的 role、name、properties、关系及 backend DOM ID。 |
| `page.screenshot(*, full_page=False, timeout=10) -> bytes` | PNG 字节，可直接 `display_image(...)` 或写入文件。 |
| `page.locator(selector) -> Locator` | 创建主文档标准 CSS 定位器，不立即查询。 |
| `page.get_by_role(role, *, name=None, exact=False) -> Locator` | 根据 Chrome 计算的 accessibility role 和可访问名称定位；排除被 accessibility tree 忽略的节点。 |
| `page.get_by_text(text, *, exact=False) -> Locator` | 根据规范化文本定位，选择最深层的匹配元素。 |
| `page.keyboard -> Keyboard` | 向当前页面的焦点发送组合键或文字。 |
| `page.bring_to_front(*, timeout=10)` | 激活 Chrome 中的这个标签页。 |
| `page.close(*, timeout=10)` | 关闭具体标签页；也会关闭用户原有的标签页。 |

`window_id` 是 Chrome 自己的窗口 ID，不是 Hyprland address 或 macOS window ID。桌面窗口的 PID 能定位浏览器实例，无法单独确定多个窗口中的哪一个标签页；连接后查看 `info()` 再选择。需要操作浏览器工具栏或系统对话框时使用 computer-use。

`evaluate()` 不接受 Python 参数对象；嵌入外部值时用 `json.dumps()` 编码，避免拼接未转义的字符串。它执行的是表达式，`undefined`、DOM 对象和不可序列化值不属于返回契约。

## Locator

| 调用 | 行为 |
| --- | --- |
| `locator.count(*, timeout=10) -> int` | 读取当前匹配数量，不等待元素出现。 |
| `locator.inner_text(*, timeout=10) -> str` | 等待一个匹配元素出现后返回文本；不要求元素可见。 |
| `locator.click(*, timeout=10)` | 等待单一元素可见、启用，滚动入视口，确认中心点击点未遮挡且两次采样位置相同，然后发送鼠标事件。 |
| `locator.fill(text, *, timeout=10)` | 等待可见且可编辑的单一元素，选中原内容并通过 Chrome 输入事件替换。空字符串清空内容。 |
| `locator.press(*keys, timeout=10)` | 等待单一元素可见、启用并可聚焦，聚焦后发送完整组合键。 |
| `locator.set_input_files(files, *, timeout=10)` | 为单一 `input[type=file]` 设置本机文件；接受一个路径或路径列表，空列表清空，支持隐藏输入框。多文件要求控件具有 `multiple`。 |
| `locator.wait_for(*, state="visible", timeout=10)` | state 为 `attached`、`detached`、`visible` 或 `hidden`。 |

需要单个元素的方法遇到多个匹配立即报错。每次查询重新执行选择器，locator 可以跨重绘和导航保留。等待可见只检查非零布局尺寸及 visibility；点击还检查启用状态和命中位置。点击后不自动等待导航，请等待下一页目标元素或检查页面结果。

`fill()` 支持 text/search/email/url/tel/password input、textarea 和 contenteditable；只读元素等待到超时，不支持的控件类型报错。控件的 input/change 行为由 Chrome 输入和后续焦点变化决定。

`get_by_role()` 的 `role` 使用 Chrome 的角色名称，例如 `button`、`textbox`、`heading`；`name` 来自可访问名称，能够识别 label、aria-label 和 aria-labelledby。`get_by_role(..., name=...)` 与 `get_by_text(...)` 默认大小写不敏感的子串匹配；`exact=True` 使用大小写敏感的完整匹配。两者匹配前均去掉首尾空白、合并连续空白。text 查询使用文本内容及 button/submit/reset input 的 value，排除 script、style、noscript、head，不以可见性过滤；点击等操作仍执行各自的等待。

CSS 和文本查询仅在主文档执行；没有 iframe/shadow root 的定位接口，也不接受 Playwright 的 `text=`、`:has-text()` 专用选择器。role 查询使用当前文档的 Chrome accessibility 计算结果，不提供隐藏节点查询或正则名称匹配。

上传路径接受 `str` 或 `Path`，必须是本机已有普通文件；SDK 转为绝对路径交给 Chrome，不操作系统文件选择对话框。远程 endpoint 场景需要 Chrome 也能访问这些路径。

## Keyboard

| 调用 | 行为 |
| --- | --- |
| `page.keyboard.press(*keys, timeout=10)` | 按下并释放完整组合键，修饰键作为独立参数。 |
| `page.keyboard.insert_text(text, *, timeout=10)` | 在当前焦点插入 Unicode 文字，不解释为键名，不产生逐字符 keydown/keyup。 |

例如 `field.press("ctrl", "a")`、`page.keyboard.press("shift", "1")`、`page.keyboard.press("Enter")`。组合最多包含一个普通键，可组合 `ctrl/control`、`shift`、`alt`、`super/meta/cmd`；使用 US 键盘定义，字母大写通过 `shift` 表达。支持 Enter、Tab、Backspace、Escape、方向键、Home、End、PageUp、PageDown、功能键等。macOS 的 Command 使用 `meta` 或 `cmd`，不会把 `ctrl` 自动改为 Command。

这些是页面键盘事件；操作 Chrome 工具栏或桌面快捷键使用 computer-use。发生超时或取消时，后台仍会尝试释放已按下的键；同页下一次键盘操作等待释放完成。

## 下载

| 调用 | 行为 |
| --- | --- |
| `session.set_download_directory(path, *, timeout=10)` | 创建本机目录并启用默认浏览器 context 的下载收集，必须在触发下载之前调用。 |
| `session.downloads(*, timeout=10) -> list[Download]` | 返回自启用以来观察到的所有下载，包括进行中、完成和取消的项目；不消费等待队列。 |
| `session.wait_for_download(*, timeout=30) -> Download` | 按开始顺序等待并取出下一项，完成后返回；该项被取消时移出队列并报错。超时不消费下载。 |
| `session.reset_downloads(*, timeout=10)` | 停止收集并恢复 Chrome 的默认下载行为；更改目录前调用。不会删除文件或取消已经开始的下载。 |

Chrome 按 GUID 命名文件，避免用服务端提供的文件名覆盖同名产物。完成后用 `save_as()` 指定最终名称。收集范围是默认 context 的所有标签页，不只最后操作的页面；不管理隐身 context。下载目录必须位于 Chrome 和 Python 均能访问的文件系统。reset 或关闭会话后，未完成下载不再更新；已完成文件可继续读取和保存。

`Download` 只读字段：`guid`、`url`、`suggested_filename`、`state`、`received_bytes`、`total_bytes`、`path`。`state` 为 `in_progress`、`completed`、`canceled`；完成前 `path` 为 `None`，完成后为本机文件路径字符串。总大小在 Chrome 未知时可能为 0。

| 调用 | 行为 |
| --- | --- |
| `download.wait(*, timeout=30) -> str` | 等待完成并返回 GUID 文件路径；取消时报错。 |
| `download.save_as(path, *, timeout=30) -> str` | 等待完成，创建目标父目录并复制文件；目标已存在时覆盖，返回绝对路径。 |
| `download.cancel(*, timeout=10)` | 取消进行中的下载；已结束时无操作。 |

## 网络拦截

| 调用 | 行为 |
| --- | --- |
| `page.route(pattern, *, action, response=None, request=None, timeout=10) -> str` | 添加规则并返回规则 ID。Rust 在后台处理匹配的请求，Python 等待或两次 cell 之间也继续运行。 |
| `page.unroute(route_id=None, *, timeout=10)` | 删除指定规则；省略 ID 删除全部规则。 |

pattern 使用覆盖完整 URL 的 glob，`*` 可以跨 `/`，例如 `**/api/**`。最后添加的匹配规则优先；未匹配请求原样放行。规则作用于当前 page target 发出的请求，不自动继承到其他标签页或 popup；Service Worker 处理的请求不在保证范围内。启用规则时关闭该页 HTTP cache，移除最后一条规则或关闭页面/会话后恢复缓存并解除 Fetch 暂停。

支持以下三种动作：

- `action="abort"`：以 blocked-by-client 终止请求，不接受 request/response。
- `action="fulfill"`：提供 `response={"status": 200, "headers": {...}, "content_type": "application/json", "body": ...}`。只有 response 必需；status 默认 200，body 默认空。body 接受 UTF-8 字符串或 bytes，headers 的键和值都是字符串。
- `action="continue"`：原样放行，或通过 `request={"url": ..., "method": ..., "headers": {...}, "post_data": ...}` 改写。提供的 headers **替换**原请求 headers；post_data 接受字符串或 bytes。

规则采用声明式数据，不接受 Python 回调；处理请求不需要重新进入 Python 解释器。自定义响应或请求改写失败时会终止该请求并记日志，避免请求一直暂停。

## Cookie

| 调用 | 行为 |
| --- | --- |
| `session.cookies(*, timeout=10) -> list[dict]` | 读取默认浏览器 context 的全部 cookie，包括 HttpOnly。 |
| `session.add_cookies(cookies, *, timeout=10)` | 添加或替换 cookie，接受字典列表。 |
| `session.clear_cookies(*, timeout=10)` | 删除默认 context 的全部 cookie。 |

cookie 字典必需 `name`、`value`，并提供 `url` 或 `domain` 与 `path`。可选字段使用 Python snake_case：`expires`（Unix 秒；`None` 表示会话 cookie）、`http_only`、`secure`、`same_site`（`"Strict"`、`"Lax"`、`"None"`）和 `partition_key`。分区键为 `{"top_level_site": "https://example.com", "has_cross_site_ancestor": False}`。

读取结果包含 name、value、domain、path 及上述可选字段，不包含 url，可直接保存并交给 `add_cookies()` 恢复。读写不影响其他浏览器 context。连接日常浏览器时，这些接口操作的是它的真实 cookie。

## 错误、取消和日志

参数错误通常为 `ValueError`，方法超时为 `TimeoutError`，浏览器/协议/JavaScript 错误为 `RuntimeError`。等待期间释放 Python GIL，约每 25ms 检查 Python 中断；中断取消 Rust 中的当前等待，已发送的 CDP 命令仍可能执行，先观察状态再决定是否重试。

会话绑定当前 Python worker。正常退出执行 `close_all()`；Pi 重启、reload 或切换会话树后旧 Python 变量不可用。普通异常或单次取消不会主动关闭仍存活的会话。

`BROWSER_USE_LOG` 可指定日志路径；默认系统临时目录下 `browser-use-<worker-pid>.log`，到 1 MiB 后重写。记录操作起止、连接失败与清理，不记录 URL、页面正文、输入、选择器和 WebSocket token。
