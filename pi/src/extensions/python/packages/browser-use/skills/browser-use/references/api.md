# Pi 浏览器适配接口

`browser_use` 只提供连接发现、同步执行入口和资源生命周期。回调里的对象来自 `playwright.async_api`；不提供另一套 Page、Locator、Keyboard 或 Download 包装。

## 模块

| 调用 | 行为 |
| --- | --- |
| `browser.discover() -> list[BrowserInstance]` | 列出当前用户的 Chrome/Chromium/Edge/Brave 主进程，不打开连接或授权对话框。 |
| `browser.connect(*, pid=None, endpoint=None, configure_browser=False, timeout=30) -> Session` | pid 和 endpoint 必须且只能提供一个。PID 可以是窗口、renderer 或主进程，沿父进程解析浏览器。endpoint 接受 CDP HTTP/WebSocket 地址。 |
| `browser.launch(*, executable_path=None, user_data_dir=None, headless=False, timeout=30) -> Session` | 启动本机浏览器，创建独立进程组；默认有界面，省略 profile 时使用临时目录。不下载浏览器。 |
| `browser.close_all()` | 关闭当前 worker 的所有会话；附加的用户 Chrome 保留，自己启动的 Chrome 退出。 |

`BrowserInstance` 是冻结的进程快照，字段为 `pid: int`、`name: str`、`executable: str`、`user_data_dir: str`。发现结果不保证调试已启用或进程仍存活。profile 指数据根目录，不是 `Default`/`Profile N` 子目录。

PID 连接读取 profile 的 `DevToolsActivePort`，也支持进程参数中的固定 `--remote-debugging-port`。没有调试端点直接失败，不创建替代实例。连接 Chrome 日常会话需要用户启用远程调试并接受授权。

路径接受 `str`/`Path`，字符串 `~` 不自动展开。显式 profile 必须独立于日常 Chrome，同一目录不能被两个实例同时占用。文件上传与下载要求 Chrome 和 Python 使用可访问的文件系统；远程 CDP 端点不承诺本地文件传输。

## Session

| 成员 | 行为 |
| --- | --- |
| `session.pid` | PID 连接或 launch 的浏览器主 PID；显式 endpoint 连接为 `None`。 |
| `session.mode` | `"attached"` 或 `"launched"`。 |
| `session.closed` | 适配层是否已关闭，不是浏览器的实时健康探测。 |
| `session.run(action, *, timeout=30)` | 同步调用 async callback，传入原生默认 `BrowserContext`，返回 callback 的结果或抛出其异常。 |
| `session.close()` | 幂等。解除本连接的页面/context 请求路由，恢复已接管的下载设置，断开 CDP；启动模式还退出自有 Chrome。 |

`Session` 支持普通 `with browser.launch(...) as session:`。跨 cell 工作保留变量并显式关闭。关闭后原有 Playwright 对象失效。

### 执行与线程

每个 Session 有一个持续运行的 asyncio 后台线程，以及 Playwright 自带的 Node driver。Python cell 主线程在 `run()` 中阻塞等待；后台线程在两次 cell 之间也继续处理网络、下载与事件。没有额外的 browser-use 服务或 RPC 协议。

callback 必须返回 awaitable；通常用 `async def action(context)`，单个调用可用 `lambda context: page.title()`。传入的是默认 `BrowserContext`，不自动创建隔离 context。

Page/Locator/Download 等对象可以作为结果保存到 Python 工作区，但只能在同一 Session 的 callback 中操作。普通数据可在主线程使用；截图 bytes 返回主线程再 `display_image()`。不要跨 Session 传递 Playwright 对象，不要从 callback 调用同步的 browser-use 入口、computer-use 或 terminal-use。后台事件需要把数据交给主线程时，使用 `queue.Queue` 等线程安全容器。

async callback 不自动让阻塞代码变成异步。callback 中的 `time.sleep()`、CPU 循环、同步文件/网络调用会阻塞浏览器事件处理；请使用对应异步 API，或把其他工作放回 Python 主线程。长时间持有 GIL 的原生调用也可能延迟后台线程。

### 时间、错误与取消

- browser-use 的 `timeout` 是有限正数，单位秒；`run` 限制整个 callback。
- 原生 Playwright 方法使用毫秒，也可按其文档使用 `timedelta`。默认普通操作 10,000 ms，导航 30,000 ms。
- 外层 `python_repl` 仍有自己的 deadline。Chrome 授权等待、截图、结果处理和退出清理都需要留时间。
- 适配层 deadline 抛出 Python `TimeoutError`；Playwright 操作错误原样返回，例如 `playwright.async_api.TimeoutError`。进程发现还可能抛出 `psutil` 的进程/权限错误。
- Python 中断取消当前 callback，并等待它处理取消。已发送的导航/点击/输入不能撤回；会话保留时先观察状态再决定重试。
- callback 吞掉取消或阻塞后台线程时，Pi 的外层期限可能终止整个 worker。不要在 callback 中屏蔽 `asyncio.CancelledError`。

## 原生 Playwright 能力

| 任务 | callback 内使用 |
| --- | --- |
| 现有页面、新页面 | `context.pages`、`await context.new_page()` |
| 标题、URL、结构 | `await page.title()`、`page.url`、`await page.locator("body").aria_snapshot()` |
| 定位与等待 | `page.get_by_role(...)`、`page.get_by_text(...)`、`page.locator(...)`、`page.frame_locator(...)` |
| 输入 | `await locator.fill(text)`、`await locator.press("Control+A")`、`await page.keyboard.insert_text(text)` |
| 页面 JavaScript | `await page.evaluate(function, arg)`；参数由 Playwright 传递，不拼接未转义字符串。 |
| 截图 | `await page.screenshot(full_page=True)` 返回 bytes。 |
| 上传下载 | `await locator.set_input_files(...)`、`async with page.expect_download()`、`await download.save_as(path)` |
| 请求规则 | `await page.route(pattern, async_handler)`；handler 使用 `await route.abort()/fulfill()/continue_()`。 |
| Cookie | `await context.cookies()`、`await context.add_cookies(records)`、`await context.clear_cookies()`；字段沿用 Playwright，例如 `httpOnly`、`sameSite`。 |
| 原始 CDP | `await context.new_cdp_session(page)`，使用后 `await cdp.detach()`。 |

完整签名和语义以 [Playwright Python API](https://playwright.dev/python/docs/api/class-browsercontext) 为准。连接使用 CDP；[Playwright 明确说明](https://playwright.dev/python/docs/api/class-browsertype#browser-type-connect-over-cdp)它与 Playwright 自己的协议连接并非完全等价。iframe、shadow DOM、定位与自动等待交给 Playwright，不自行模拟。

附加模式默认传给 Playwright `no_defaults=True`，避免默认改动用户浏览器的焦点、媒体和下载设置。需要原生下载事件时，在连接时显式传 `configure_browser=True`，允许 Playwright 应用这些默认设置并把默认 context 的下载写入会话临时目录；关闭前需 `Download.save_as()`。启动模式始终使用 Playwright 默认设置。关闭时恢复的是 Chrome 默认下载行为，不是其他客户端先前设置的自定义目录；页面覆盖按 Playwright/CDP 断开语义处理，不提供任意设置的快照回滚。多个控制客户端同时修改同一浏览器设置时不提供隔离。

## 日志与退出

`BROWSER_USE_LOG` 指定日志路径，默认系统临时目录中的 `browser-use-<worker-pid>.log`。达到 1 MiB 后重写。记录操作起止、时长、异常类型、PID 与资源所有权；不记录页面正文、输入、选择器、URL 或 CDP token。

worker 正常退出执行 `close_all()`。启动 Chrome 后立即向 Pi 上报独立进程组；worker 被强杀时由父进程回收。附加的用户 Chrome 从不登记为自有进程。强杀不会运行 Python 清理，临时 profile/artifacts 可能留在临时目录。
