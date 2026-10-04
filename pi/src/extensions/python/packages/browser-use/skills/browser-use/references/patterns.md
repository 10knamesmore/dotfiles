# 操作流程

## 从桌面窗口连接

先用 computer-use 查询窗口，再交还桌面控制：

```python
import computer_use as computer
import browser_use as browser

with computer.connect_host() as desktop:
    windows = desktop.hyprland.query("clients")
print([{key: window.get(key) for key in ("address", "pid", "class", "title")} for window in windows])
```

根据实际输出把目标保存为 `window`，然后连接并观察页面：

```python
session = browser.connect(pid=window["pid"])

async def inspect_tabs(context):
    return [(page, await page.title(), page.url) for page in context.pages]

tabs = session.run(inspect_tabs)
print([(index, title, url) for index, (_, title, url) in enumerate(tabs)])
```

选定 `tabs` 中的 page 后保留对象，不把列表索引当成跨调用的页面身份。多个 Chrome 窗口可能共享 PID；Hyprland address 与 Chrome window ID 不是同一个标识。需要 Chrome window ID 或原始 accessibility tree 时，可在 callback 内调用 CDP：

```python
async def inspect_chrome(context):
    cdp = await context.new_cdp_session(page)
    try:
        return {
            "window": await cdp.send("Browser.getWindowForTarget"),
            "accessibility": await cdp.send("Accessibility.getFullAXTree"),
        }
    finally:
        await cdp.detach()

print(session.run(inspect_chrome))
```

Chrome 未启用调试时请用户打开 `chrome://inspect/#remote-debugging`，连接授权由用户亲自处理。等待前关闭活动的当前桌面控制对象。

## 独立持久 profile 与截图

```python
from pathlib import Path
import browser_use as browser

session = browser.launch(
    user_data_dir=Path.home() / ".local/share/browser-use/work",
    headless=False,
)

async def open_page(context):
    page = await context.new_page()
    await page.goto("https://example.com")
    return page, await page.screenshot()

page, image = session.run(open_page)
display_image(image)
Path("browser-result.png").write_bytes(image)
```

有界面模式可用来登录。显式 profile 关闭后保留，下次传相同目录复用；省略 profile 则关闭时删除。使用完毕后 `session.close()`，不要关闭原生默认 context 来替代适配层收尾。

## 表单与页面结构

先观察结构，不猜测 label 或选择器：

```python
print(session.run(lambda context: page.locator("body").aria_snapshot()))
```

实际页面存在名称为“搜索词”的文本框与“搜索”按钮时：

```python
async def search(context):
    field = page.get_by_role("textbox", name="搜索词", exact=True)
    await field.fill("关键词")
    await page.get_by_role("button", name="搜索", exact=True).click()
    return await page.locator("body").aria_snapshot()

print(session.run(search, timeout=40))
```

Playwright 按键使用单字符串，例如 `await field.press("Control+A")`；macOS 用 `"Meta+A"`。iframe 用 `page.frame_locator(...)`；普通 locator 的 shadow DOM 行为遵循 Playwright，不注入自研选择器。

## 上传下载

使用 `browser.launch()`，或连接时显式 `browser.connect(pid=..., configure_browser=True)`，让 Playwright 接管下载。后者也允许 Playwright 对原有页面应用默认焦点/媒体设置，需要符合任务意图。已确认目标页面具有文件输入框和“下载报告”链接时：

```python
from pathlib import Path
upload = Path("report.csv").absolute()
destination = Path("artifacts/report.csv").absolute()
destination.parent.mkdir(parents=True, exist_ok=True)

async def transfer(context):
    await page.locator('input[type="file"]').set_input_files(upload)
    async with page.expect_download(timeout=20_000) as pending:
        await page.get_by_role("link", name="下载报告", exact=True).click()
    download = await pending.value
    await download.save_as(destination)
    return download.suggested_filename

print(session.run(transfer, timeout=30))
```

`expect_download` 的 20,000 是毫秒，`session.run` 的 30 是秒。监听必须先于触发。空文件列表清空上传选择；隐藏的文件输入框也可用。下载在保存前属于会话的临时目录，关闭会话会删除。

## 网络拦截与跨 cell 事件

请求处理使用原生 async handler，不是静态规则字典：

```python
async def install_route(context):
    async def profile(route):
        await route.fulfill(json={"name": "演示用户"})
    await page.route("**/api/profile", profile)

session.run(install_route)
```

后续 cell 中可以继续使用这个路由；Python 等待下一次工具调用时，事件循环仍运行：

```python
print(session.run(lambda context: page.evaluate("fetch('/api/profile').then(r => r.json())")))
session.run(lambda context: page.unroute("**/api/profile"))
```

后台事件数据交给主线程时，用线程安全队列，不在回调里调用 `display_image` 或其他 use：

```python
from queue import SimpleQueue
browser_events = SimpleQueue()

async def watch(context):
    page.on("download", lambda download: browser_events.put(download.suggested_filename))

session.run(watch)
# 后续在主线程检查 browser_events.empty() 并 get() 读取已发生的事件。
```

保存的原生 Playwright 对象仍只能在其 Session 的 callback 中操作；队列适合传普通数据。事件回调不要进行阻塞 I/O，不要打印敏感页面信息到后台输出。

## Cookie

```python
async def set_preference(context):
    await context.add_cookies([{
        "name": "preferred_language", "value": "zh-CN",
        "url": "https://example.com", "sameSite": "Lax",
    }])
    return await context.cookies("https://example.com")

cookies = session.run(set_preference)
```

字段使用 Playwright 的命名，而非自定义转换。读取结果可能包含敏感登录凭据，只在任务需要时读取和保存。连接日常 Chrome 时会改变它的真实 cookie。

## 取消后继续

```python
try:
    session.run(lambda context: page.get_by_role("button", name="等待出现").click(), timeout=2)
except TimeoutError:
    print("callback deadline reached")

image = session.run(lambda context: page.screenshot())
display_image(image)
```

这里捕获的是适配层 deadline；原生 Playwright 方法可能先抛出自己的 `playwright.async_api.TimeoutError`。取消不回滚已经发送的操作，先观察再决定是否继续。最终 `session.close()`，或者在主线程用 `browser.close_all()` 集中收尾。
