# Terminal Use

面向 Python worker 的 Unix PTY 与终端屏幕 SDK。本目录的 Rust crate 直接构建公开 Python 扩展 `terminal_use`。

## 结构

- `src/` — Rust 公开 Python API 与内部实现：参数校验、按键与 paste 编码、会话注册、PTY 读写、Ghostty 终端状态、图片快照、原始输出环形缓冲。
- `Cargo.toml` / `pyproject.toml` — Rust 与 maturin 构建配置（`module-name = "terminal_use"`）。
- `build_backend.py` — 将隔离构建环境中的 Zig 放入 PATH，其余打包工作交给 maturin。

## 构建与维护

- Rust 使用 stable 工具链，仓库不 pin 版本。`libghostty-vt` 通过 Rust bindings 静态链接，绑定的构建脚本下载其固定 revision 的 Ghostty 源码。
- Rust 或 Python 源码变更后，从仓库根执行 `uv sync --project pi/src/extensions/python --locked`；worker 的 uv 项目依赖本目录，由 maturin 按 lockfile 构建扩展。不要往其他解释器单独安装。已经加载扩展的 worker 需要 reload 才会使用新 binary。
- `ziglang==0.15.2` 是 Python 的隔离构建依赖，为绑定固定的 Ghostty revision 提供 Zig；无需系统安装 Zig，运行时也不依赖 Zig。首次构建需要访问 Ghostty 源码与 Zig 包依赖。直接运行 Cargo 时，需将该版本的 `zig` 放入 PATH。
- `TERMINAL_USE_LOG` 指定原生日志文件路径；未设置时写入系统临时目录。日志记录会话生命周期、输入写入结果、图片快照元数据与错误，不记录键名、PTY 输入输出或图片字节。

## 运行时模型

- 仅支持 macOS 与 Linux。
- 会话属于创建它的 worker 进程：worker 重启或 reload 后会话丢失，终端随 worker 进程结束；没有 daemon，也没有跨进程持久化。
- 每个会话只有一条 PTY 流，子进程的 stdout 与 stderr 合并。Ghostty 终端核心在所属线程中解析输出、维护文字与 Kitty 图片状态；不会启动 Ghostty GUI。`read_raw()` 读取 bounded raw ring，默认从保留窗口的最旧字节开始，不等于直接读取最新尾部。
- `start(cell_size=(8, 16))` 设置 headless 终端的虚拟单元格像素大小；PTY 尺寸、终端尺寸查询与图片布局使用同一套几何。它不代表实际字体或屏幕像素。
- `read(images=True)` 返回当前活动屏幕中 placement 引用的源图快照，图片对象可直接传给 `display_image()`。快照不随后续重绘、删除或会话关闭改变。图片不是图文合成截图，不根据文字 `rect` 裁剪；虚拟 placement 不提供已解析的占位字符位置，也不等于图片当前可见。
- 不提供整屏 PNG 渲染、Sixel 图片、hyperlink 或剪贴板对象。
- `terminal_use` 是 Rust 编译出的模块；导入时注册 `atexit` 清理，`close_all` 逐会话终止子进程并回收 PTY。screen emulator 不保留 scrollback，长输出由 bounded raw ring 提供最近原始字节。
- `send_key(session_id, "ctrl", "c")` 使用与 computer-use 相同的分参数组合键写法和 US 基础键位含义。它只编码本次输入，不保留按住状态；字面文字使用 `send_text()`。终端不支持的组合明确报错，不模拟桌面按键事件。
- 公开入口为 `import terminal_use as terminal`。方法签名、返回字段和交互约定见 [terminal-use skill](skills/terminal-use/references/api.md)，流程与故障处理见 [patterns](skills/terminal-use/references/patterns.md)。

## 与 worker 的集成契约

worker 启动时调用 Rust 扩展的 `terminal_use._set_lifecycle_hook(callback)` 接管会话所有权事件。回调：

- 每个会话生命周期变化调用一次，参数为 `{'type': 'terminal_ownership', 'action': 'opened' | 'closed', 'id': str, 'pid': 正整数}`。
- `start` 在原生创建成功后立即发出 `opened`；回调抛错时原生层会尝试关闭该会话，再把回调错误作为主错误抛出。调用方不要把这条失败路径当作一次成功的 ownership 生命周期通知。
- `close` 在原生清理成功后才发出 `closed`；`close_all` 对每个会话分别发出。
- 只能从注册 lifecycle hook 的线程调用 `start`、`close`、`close_all`；跨线程调用会在创建或销毁会话前失败，原生线程不触碰 Python 回调。
- 默认保护 worker control fd `[3, 4]`，防止 PTY 子进程继承并污染 worker 协议。Python embedding 可在创建会话前调用 `terminal_use._set_worker_control_fds(sequence)` 覆盖这份 fd 列表；传 `None` 恢复默认值，传空序列表示不额外保护 fd。该函数只设置 `FD_CLOEXEC`，不会创建、移动或替换通信 fd。
