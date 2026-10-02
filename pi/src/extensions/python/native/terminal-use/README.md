# Terminal Use

面向 Python worker 的 Unix PTY 与终端屏幕 SDK。本目录的 Rust crate 直接构建公开 Python 扩展 `terminal_use`。

## 结构

- `src/` — Rust 公开 Python API 与内部实现：参数校验、按键与 paste 编码、会话注册、PTY 读写线程、Alacritty terminal core 屏幕解析、原始输出环形缓冲。
- `Cargo.toml` / `pyproject.toml` — maturin 构建配置（`module-name = "terminal_use"`）。

## 构建与维护

- Rust 使用 stable 工具链，仓库不 pin 版本。
- Rust 或 Python 源码变更后，从仓库根执行 `uv sync --project pi/src/extensions/python --locked`；worker 的 uv 项目依赖本目录，由 maturin 按 lockfile 构建扩展。不要往其他解释器单独安装。
- `TERMINAL_USE_LOG` 指定原生日志文件路径；未设置时写入系统临时目录。日志只记录会话生命周期与错误，不记录 PTY 输入或输出内容。

## 运行时模型

- 仅支持 macOS 与 Linux。
- 会话属于创建它的 worker 进程：worker 重启或 reload 后会话丢失，终端随 worker 进程结束；没有 daemon，也没有跨进程持久化。
- 每个会话只有一条 PTY 流，子进程的 stdout 与 stderr 合并。屏幕由 Alacritty terminal core 解析；当前 SDK 不返回 graphics、hyperlink 或剪贴板对象，也不承诺完整真实终端渲染能力。`read_raw()` 读取 bounded raw ring，默认从保留窗口的最旧字节开始，不等于直接读取最新尾部。
- `terminal_use` 是 Rust 编译出的模块；导入时注册 `atexit` 清理，`close_all` 逐会话终止子进程并回收 PTY。screen emulator 不保留 scrollback，长输出由 bounded raw ring 提供最近原始字节。
- 公开入口为 `import terminal_use as terminal`。方法签名、返回字段和交互约定见 [terminal-use skill](skills/terminal-use/references/api.md)，流程与故障处理见 [patterns](skills/terminal-use/references/patterns.md)。

## 与 worker 的集成契约

worker 启动时调用 Rust 扩展的 `terminal_use._set_lifecycle_hook(callback)` 接管会话所有权事件。回调：

- 每个会话生命周期变化调用一次，参数为 `{'type': 'terminal_ownership', 'action': 'opened' | 'closed', 'id': str, 'pid': 正整数}`。
- `start` 在原生创建成功后立即发出 `opened`；回调抛错时原生层会尝试关闭该会话，再把回调错误作为主错误抛出。调用方不要把这条失败路径当作一次成功的 ownership 生命周期通知。
- `close` 在原生清理成功后才发出 `closed`；`close_all` 对每个会话分别发出。
- 只能从注册 lifecycle hook 的线程调用 `start`、`close`、`close_all`；跨线程调用会在创建或销毁会话前失败，原生线程不触碰 Python 回调。
- 默认保护 worker control fd `[3, 4]`，防止 PTY 子进程继承并污染 worker 协议。Python embedding 可在创建会话前调用 `terminal_use._set_worker_control_fds(sequence)` 覆盖这份 fd 列表；传 `None` 恢复默认值，传空序列表示不额外保护 fd。该函数只设置 `FD_CLOEXEC`，不会创建、移动或替换通信 fd。
