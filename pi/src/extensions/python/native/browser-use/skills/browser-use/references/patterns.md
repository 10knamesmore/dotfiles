# 操作流程

## 从桌面窗口接入

Linux Hyprland 的窗口 PID 可以直接交给 browser-use：

```python
import computer_use as computer
import browser_use as browser

with computer.connect_host() as desktop:
    windows = desktop.hyprland.query("clients")
print([{key: window.get(key) for key in ("address", "pid", "class", "title")} for window in windows])
```

根据列表选定窗口，保存为 `window` 后连接：

```python
session = browser.connect(pid=window["pid"])
pages = session.tabs()
print([page.info() for page in pages])
```

这一步连接浏览器进程。多个 Chrome 窗口可能共享 PID，需要再根据标签页的标题、URL、target_id 和 Chrome window_id 选择 `page`。不要把 Hyprland address 传给 CDP window_id。

日常 Chrome 未启用调试时，让用户打开 `chrome://inspect/#remote-debugging`。Chrome 的连接授权由用户操作；如果另有活动的当前桌面对象，等待用户前先 `desktop.close()`；上面的 `with` 已在查询后交还桌面。Chrome 用户授权耗时应计入连接 timeout 和外层 Python timeout。

完成任务后：

```python
session.close()
```

这会保留连接前已有的 Chrome 窗口与标签页。

## 启动独立持久会话

```python
from pathlib import Path
import browser_use as browser

session = browser.launch(
    user_data_dir=Path.home() / ".local/share/browser-use/work",
    headless=False,
)
page = session.new_page("https://example.com")
print(page.info())
display_image(page.screenshot())
```

使用有界面模式完成登录，保留独立 profile，关闭后下次传同一个目录复用。默认省略 `user_data_dir` 时使用临时 profile，关闭会话后删除。SDK 使用本机安装的浏览器；需要指定版本时传 `executable_path`。

## 观察、操作、确认

先读取可访问性结构与必要的 DOM 信息，再选择角色、文本或 CSS：

```python
print(page.snapshot())
print(page.evaluate('''Array.from(document.querySelectorAll('input, textarea, button')).map(
    element => ({tag: element.tagName, id: element.id, name: element.getAttribute('name'), type: element.getAttribute('type'), text: element.textContent})
)'''))
```

确认实际页面提供 `input[name="q"]` 和提交按钮后：

```python
field = page.locator('input[name="q"]')
field.fill("检索内容")
page.locator('button[type="submit"]').click()
```

点击完成仅表示输入已发送。根据观察到的页面结构选择结果元素并 `wait_for()`，或再次检查 `page.info()`、DOM 和截图。保存图片时使用返回的 PNG 字节：

```python
from pathlib import Path
image = page.screenshot(full_page=True)
display_image(image)
Path("browser-result.png").write_bytes(image)
```

取消或超时后，先读取页面状态；不要假设导航、点击或输入没有发生。需要结束整个浏览器工作流时使用 `browser.close_all()`。

## 语义定位与组合键

在快照中确认文本框名称为「搜索词」、按钮名称为「保存更改」后：

```python
field = page.get_by_role("textbox", name="搜索词", exact=True)
field.fill("第一版")
field.press("ctrl", "a")  # macOS 使用 cmd
page.keyboard.insert_text("更新内容🙂")
page.get_by_role("button", name="保存更改", exact=True).click()
page.get_by_text("已保存", exact=True).wait_for()
```

role 的 name 是 Chrome 计算的可访问名称，不一定等于元素正文。文本查询会合并空白；多个匹配时细化名称或改用已经确认的 CSS。

## 文件上传与下载

确认页面上的文件输入框和下载按钮后：

```python
from pathlib import Path
page.locator('input[type="file"]').set_input_files(Path("report.csv"))
# 支持 multiple 的控件可以传路径列表，空列表清空。

session.set_download_directory(Path("artifacts/downloads"))
page.get_by_role("link", name="下载报告", exact=True).click()
download = session.wait_for_download(timeout=30)
print(download.state, download.suggested_filename)
print(download.save_as(Path("artifacts/report.csv")))
```

收集在下载触发之前启用；下载完成前也可用 `session.downloads()` 获取对象、查看进度并调用 `cancel()`。等待超时保留队列中的下载，后续可继续等待。Chrome 使用 GUID 文件名，最终路径由 `save_as()` 显式选择。需要更换收集目录时先 `session.reset_downloads()`。

## 请求规则

下面的规则为选定页面的 API 请求提供 JSON 响应，Python 正在等待 fetch 时仍由 Rust 后台处理：

```python
import json
rule_id = page.route(
    "**/api/profile",
    action="fulfill",
    response={
        "status": 200,
        "content_type": "application/json",
        "body": json.dumps({"name": "演示用户"}, ensure_ascii=False),
    },
)
print(page.evaluate("fetch('/api/profile').then(response => response.json())"))
page.unroute(rule_id)
```

阻断使用 `page.route("**/analytics/**", action="abort")`。改写请求可用 `action="continue", request={"method": "POST", "post_data": "..."}`；如果提供 headers，它替换原请求的 header 集合。最后注册的匹配规则优先，`page.unroute()` 移除本页全部规则。

## Cookie 保存与恢复

```python
session.add_cookies([{
    "name": "preferred_language",
    "value": "zh-CN",
    "url": "https://example.com",
    "same_site": "Lax",
}])
cookies = session.cookies()
# cookies 可序列化保存，稍后恢复到选定会话：
session.add_cookies(cookies)
```

字段采用 snake_case；会话 cookie 的 expires 为 `None`。`session.clear_cookies()` 清空默认 context 的全部 cookie，包括 HttpOnly；操作已有日常会话时会改变它的登录态。
