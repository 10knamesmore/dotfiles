# Browser Use

独立的 Python 包 `browser-use-sdk`，导入名 `browser_use`。内部依赖 Playwright，负责把浏览器进程发现、所有权和生命周期接入 Pi 的同步 Python worker。包安装在 worker 的同一个 uv 环境，不运行额外的 browser-use 服务。

## 执行方式

```python
import browser_use as browser
session = browser.launch(headless=True)

async def inspect(context):
    page = await context.new_page()
    await page.goto("https://example.com")
    return await page.title(), await page.screenshot()

title, image = session.run(inspect)
print(title)
display_image(image)
session.close()
```

`session.run()` 是同步入口，阻塞到 async callback 返回。callback 在会话自己的后台 asyncio 线程运行，收到原生 Playwright 默认 `BrowserContext`。Page、Locator、Download 等对象不经过自研代理；其所有操作必须位于该 Session 的 callback 中。后台循环在两次 Python cell 之间持续处理事件；回调内阻塞代码和持有 GIL 的原生调用仍会延迟它。

browser-use 的 timeout 使用秒；原生 Playwright 方法使用毫秒。Python 取消会取消对应异步任务并等待其清理，不能撤回已发送的浏览器动作。`display_image` 留在 cell 主线程，用 callback 返回的 bytes 展示图片。

## 资源与依赖

- `discover()` / `connect(pid=...)` 用 psutil 从桌面/renderer PID 找到浏览器主进程、profile 与 CDP 地址。
- `launch()` 使用本机 Chrome/Chromium，不下载浏览器。独立进程组与 worker 的 SIGINT 隔离；显式 profile 保留，临时 profile 随会话清理。
- Playwright 自带 Node driver 属于 worker 的进程组；它忽略 SIGINT、在控制管道关闭后退出。Pi 强制退出 worker 时回收这个组。浏览器组另外登记，避免只杀 driver 后遗留 Chrome。
- `_set_lifecycle_hook(callback)` 在 worker 主线程注册。启动 Chrome 后、等待 CDP 就绪前发送 `{"action":"opened","id":str,"pid":int}`；清理后发送 `closed`。仅自己的 Chrome 登记所有权。
- 附加会话默认采用 `no_defaults=True`，不接管用户下载；需要原生下载事件时，连接时显式 `configure_browser=True`，允许 Playwright 应用默认页面设置并接管下载。关闭时恢复 Chrome 默认下载行为，保留用户浏览器与标签页。
- 正常退出通过 atexit 关闭全部会话；worker 被强杀时由 Pi 父进程回收自有浏览器。强杀无法清理临时 profile/artifacts 目录。

模型通过 browser-use skill 发现推荐入口。全量安装清单仍可包含 Playwright；安装可见性不等于推荐直接创建浏览器，也不是 import 隔离。

## 结构

- `src/browser_use/_discovery.py`：本机浏览器与调试端点发现。
- `src/browser_use/_process.py`：独立 Chrome 进程组和 Pi ownership hook。
- `src/browser_use/_runtime.py`：同步调用与后台事件循环之间的等待、超时和取消。
- `src/browser_use/_session.py`：连接、Playwright 对象作用域、下载接管与清理。
- `src/browser_use/_diagnostics.py`：不含页面数据的操作日志。
- `skills/browser-use/`：模型入口与使用说明，由 Python extension 注册。

## 安装与验证

worker 的 `pyproject.toml` 使用 editable 本地路径依赖。本包用 `uv_build` 构建，不需要 Rust；其他 native SDK 仍由各自构建后端处理。`dots sync` 的 Python hook 执行 locked uv sync。

从仓库根执行：

```sh
uv sync --project pi/src/extensions/python --locked
uvx ty check --python pi/src/extensions/python/.venv/bin/python pi/src/extensions/python/packages/browser-use/src/browser_use
pnpm --dir pi run typecheck
```

修改包源码后使用新 worker；Pi 中 `/reload` 会清空工作区并加载更新后的包与 skill。修改依赖需先 `uv lock --project pi/src/extensions/python`。

适配接口见 [API](skills/browser-use/references/api.md)，完整同步/异步示例见 [skill](skills/browser-use/SKILL.md) 和 [操作流程](skills/browser-use/references/patterns.md)。原生操作以 [Playwright Python 文档](https://playwright.dev/python/docs/api/class-browsercontext) 为准；CDP 连接不是 Playwright 协议连接的完整等价物。
