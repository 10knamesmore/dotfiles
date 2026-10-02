# Browser Use

面向 Pi Python worker 的原生 Chromium SDK。Rust crate 通过 PyO3 导出 `browser_use`；CDP 通信复用 `chromiumoxide`，进程发现使用 `sysinfo`。运行时不依赖 Playwright、Node driver 或独立 daemon。

## 结构

- `src/discovery.rs`：浏览器进程与 profile 发现，桌面窗口/子进程 PID 解析及调试端点读取。
- `src/session.rs`、`src/session/`：连接、独立浏览器启动、进程组所有权、默认 context 的 cookie 与下载收集。
- `src/page.rs`、`src/page/`：页面观察、导航、截图，CSS/text/role 定位器、键盘、文件上传与请求规则。Rust 管理轮询、超时和 CDP 输入；嵌入的 JavaScript 查询及操作 DOM，role/name 由 Chrome accessibility API 计算。
- `src/runtime.rs`：一个后台工作线程驱动 Tokio/CDP，Python 侧释放 GIL 等待结果并响应取消。
- `src/diagnostics.rs`：操作与连接诊断日志。
- `skills/browser-use/`：随 Python extension 注册的操作说明。

## 构建

从仓库根运行：

```sh
cargo check --manifest-path pi/src/extensions/python/native/browser-use/Cargo.toml
uv sync --project pi/src/extensions/python --locked
pnpm --dir pi typecheck
```

`dots sync` 的 Python hook 同样执行 locked uv sync。修改原生源码后需要 reload Pi 或使用新 worker；已经导入的动态库不会在原进程中更新。SDK 使用本机安装的 Chrome/Chromium，不安装浏览器。

## 生命周期

连接与页面保存在当前 worker 中，正常退出时通过 atexit 关闭会话。附加到已有浏览器时只断开 CDP，启动的独立实例由 SDK 负责退出；临时 profile 随所属连接清理。显式数据目录在退出后保留。

独立 Chrome 使用自己的进程组，避免单次 Python 调用取消时收到 worker 的 SIGINT。worker 启动时注册 `browser_use._set_lifecycle_hook(callback)`；SDK 在启动进程后、等待 CDP 就绪前报告 `opened`，清理进程后报告 `closed`，事件为 `{"action": "opened" | "closed", "id": str, "pid": int}`。生命周期操作在注册 hook 的 Python 线程执行。

worker 将浏览器和 terminal-use 的所有权统一转发为 `process_ownership`，附带 `kind: "browser" | "terminal"`。父进程记录每个进程组，在 worker 被强制结束时回收它们。正常关闭优先使用 Chrome 的关闭命令，无法正常退出时终止所属进程组；附加的用户浏览器不纳入进程所有权，也不接收关闭命令。强制结束 worker 不执行 Rust 析构，临时 profile 目录可能留在系统临时目录中。

方法级 timeout 和 Python 中断取消当前 Rust future，无法撤回已提交的 CDP 动作。组合键取消后安排后台释放，下一次键盘操作等待释放完成。请求规则和下载事件由 Rust 后台任务处理；关闭页面或会话会解除本 SDK 的 Fetch 拦截、恢复 HTTP cache，关闭会话还会恢复默认下载行为。普通 cell 失败保留连接，调用方观察页面后继续或主动关闭。所有等待时间以秒表示。

## 能力与来源

API 提供 CSS、文本及可访问角色定位，元素输入与组合键、上传下载、声明式网络规则和 cookie 管理。定位器保留 `click()`、`fill()`、`wait_for()` 等表达；查询范围、等待与清理语义见 [API](skills/browser-use/references/api.md)。桌面接入、独立 profile 与新增能力示例见 [patterns](skills/browser-use/references/patterns.md)。

- [Chrome 现有会话远程调试](https://developer.chrome.com/blog/chrome-devtools-mcp-debug-your-browser-session)
- [Chrome 用户数据目录](https://chromium.googlesource.com/chromium/src/+/main/docs/user_data_dir.md)
- [chromiumoxide](https://github.com/mattsse/chromiumoxide)
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo)

日志路径、内容和大小限制见 [API 的日志约定](skills/browser-use/references/api.md#错误取消和日志)。
