# 错误处理与终端恢复

## 目录
- [核心问题：终端脏退出](#核心问题终端脏退出)
- [color-eyre 集成](#color-eyre-集成)
- [Terminal RAII Guard 模式](#terminal-raii-guard-模式)
- [原始模式与备用屏幕](#原始模式与备用屏幕)
- [Ctrl-C 与信号处理](#ctrl-c-与信号处理)
- [错误在 UI 内展示](#错误在-ui-内展示)
- [可选子系统的优雅降级（null object）](#可选子系统的优雅降级null-object)
- [完整 main.rs 模板](#完整-mainrs-模板)

---

## 核心问题：终端脏退出

TUI 应用在启动时会进入"原始模式"（Raw Mode）和"备用屏幕"（Alternate Screen）。如果程序崩溃或异常退出而**没有恢复终端**，用户的终端会陷入：

- 输入不显示（raw mode 未退出）
- 命令历史消失（仍在备用屏幕）
- 终端颜色混乱

因此，**任何退出路径都必须执行 cleanup**，包括：
- 正常退出（用户按 q）
- panic
- Ctrl-C 信号
- `?` 传播的错误

---

## color-eyre 集成

`color-eyre` 提供美观的错误报告和 **panic hook**，panic 时自动执行 cleanup：

```toml
color-eyre = "0.6"
```

```rust
fn main() -> color_eyre::Result<()> {
    // 必须在任何 TUI 初始化前调用
    // 注册 panic hook，panic 时会打印美观的错误和 backtrace
    color_eyre::install()?;

    let mut tui = Tui::new()?;
    tui.enter()?;

    // 如果这里 panic，color-eyre 的 hook 会：
    // 1. 调用我们注册的 cleanup（通过 Tui Drop 实现）
    // 2. 打印带颜色的 panic 信息和 backtrace
    let result = run_app(&mut tui);

    tui.exit()?;
    result
}
```

### 自定义 panic hook：必须**链式包裹**，不能替换

直觉上你会想「装个 panic hook 恢复终端」。陷阱在于：`color_eyre::install()` **已经装了它自己的 hook**（负责打印彩色报告 + backtrace）。如果你再 `set_hook` 一个**裸 hook**，就把 color-eyre 的 hook 顶掉了——结果二选一翻车：

- 裸 hook 只恢复终端、不打印 → **彩色 panic 报告丢失**，你看不到崩在哪。
- 裸 hook 在 alternate screen 里 / 未退 raw mode 时打印 → 报告被备用屏幕**吞掉**或终端**乱码**。

正解：**取走当前 hook（即 color-eyre 的），链式包一层**——先恢复终端，**再**调原 hook 打印。顺序很关键：

```rust
pub fn init_panic_hook() {
    let original_hook = std::panic::take_hook();   // ← 取走 color-eyre 的 hook，不是丢弃
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal().ok();   // 1. 先退 raw mode + alternate screen（脏也要尽力退）
        original_hook(panic_info);  // 2. 再交给 color-eyre 在正常屏幕上打印彩色报告
    }));
}

fn restore_terminal() -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    Ok(())
}
```

**调用顺序**：`color_eyre::install()` → `init_panic_hook()` → `tui.enter()`。先 install 才有 color-eyre 的 hook 可包；后于 enter 则 panic 时已能正确退屏。

> **不要**在 `main` 或 TUI 内部任何地方再装第二个绕过这条链的「裸」hook——会重新触发上面的二选一翻车。`Result` 冒泡走 `Tui::Drop` 的 `restore_terminal()`，顺序自然正确；panic 走这条链。两条退出路径就够，别再加。

---

## Terminal RAII Guard 模式

通过 `Drop` trait 保证 cleanup 必然执行：

```rust
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn enter(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
        )?;
        // 隐藏光标（可选）
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn exit(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut ratatui::Frame)) -> io::Result<()> {
        self.terminal.draw(f)?;
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        // 即使 exit() 之前已经调用过，多次调用也是安全的
        // （disable_raw_mode 幂等，crossterm 内部有状态检查）
        if let Err(e) = self.exit() {
            eprintln!("Failed to restore terminal: {e}");
        }
    }
}
```

---

## 原始模式与备用屏幕

### 原始模式（Raw Mode）

```rust
// 进入：禁用终端处理，所有输入直接发给程序
crossterm::terminal::enable_raw_mode()?;

// 退出：恢复终端行处理
crossterm::terminal::disable_raw_mode()?;
```

原始模式下：
- 不自动回显输入字符
- Enter 键不生成换行，而是 `\r`
- Ctrl-C 不发送 SIGINT，而是作为普通键事件发送

### 备用屏幕（Alternate Screen）

```rust
use crossterm::{execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen}};

// 进入：切换到新的空白屏幕缓冲区
execute!(io::stdout(), EnterAlternateScreen)?;

// 退出：回到原始终端内容（命令历史、之前的输出都还在）
execute!(io::stdout(), LeaveAlternateScreen)?;
```

**注意**：进入备用屏幕后，原来终端的内容被保留在"主缓冲区"，TUI 退出后自动恢复，用户看不到 TUI 的任何残留输出。

---

## Ctrl-C 与信号处理

**两条独立的终止路径都要兜住，缺一不可**：用户**主动退**走键事件，进程**被外部终止**走信号。只做前者，是新手 TUI 最常见的「终端被搞坏」根源。

### 路径 1：用户主动退 → 键事件

原始模式下 Ctrl-C **不会**发 SIGINT，而是作为 `KeyCode::Char('c') + Modifiers::CONTROL` 事件：

```rust
(KeyModifiers::CONTROL, KeyCode::Char('c')) => {
    self.should_quit = true;   // 走正常退出 → Tui::drop() 恢复终端
}
```

### 路径 2：进程被外部终止 → 信号 watcher（**别省**）

`kill <pid>`（SIGTERM）、关掉终端窗口（SIGHUP）、系统关机、`systemctl stop`……这些**没有任何键事件**，raw-mode 那套完全救不了——主循环若只等键，进程会在 raw mode + alternate screen 里被杀，终端留成脏状态。

robust 做法：起一个信号 watcher，收到 SIGTERM/SIGINT/SIGHUP 就**置一个原子标志**，主循环每轮检测、走和用户退出**同一条**正常路径（让 `Tui::exit` / `Drop` 恢复终端）：

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 起后台 watcher，收到终止信号置标志。返回给主循环轮询的 flag。
fn spawn_signal_watcher() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let f = Arc::clone(&flag);
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("SIGTERM");
        let mut int  = signal(SignalKind::interrupt()).expect("SIGINT");
        let mut hup  = signal(SignalKind::hangup()).expect("SIGHUP");
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv()  => {}
            _ = hup.recv()  => {}
        }
        f.store(true, Ordering::Release);   // 不在信号处理里碰终端，只置标志
    });
    flag
}

// 主循环每轮开头：
if shutdown.load(Ordering::Acquire) {
    self.should_quit = true;   // ← 与用户退出汇合到同一条干净退出路径
}
```

**关键**：信号处理器里**只置标志，不直接操作终端**（异步信号上下文里碰 I/O 不安全）；真正的恢复交给主循环退出后的 `Tui::exit` / `Drop`，顺序天然正确。同步（非 tokio）应用用 `signal-hook` crate 同理。

> 别在 `tokio::select!` 里直接 `_ = ctrl_c => break`：raw mode 下 SIGINT 本就不来，而真正该处理的是 SIGTERM/SIGHUP——置标志 + 主循环检测比把退出逻辑塞进 select 分支更清晰、也不会漏掉非 select 架构。

---

## 错误在 UI 内展示

在 TUI 内显示错误而不退出应用：

```rust
pub struct App {
    pub error: Option<String>,
    pub error_timeout: Option<Instant>,
}

impl App {
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
        self.error_timeout = Some(Instant::now() + Duration::from_secs(5));
    }

    pub fn tick(&mut self) {
        // 自动清除超时错误
        if let Some(timeout) = self.error_timeout {
            if Instant::now() > timeout {
                self.error = None;
                self.error_timeout = None;
            }
        }
    }
}

// 渲染错误浮层
fn draw(frame: &mut Frame, app: &App) {
    // ... 正常渲染 ...

    if let Some(ref error) = app.error {
        let error_area = Rect::new(
            frame.area().width / 4,
            frame.area().height - 5,
            frame.area().width / 2,
            3,
        );
        frame.render_widget(Clear, error_area);
        frame.render_widget(
            Paragraph::new(error.as_str())
                .block(Block::bordered().title("Error").red())
                .style(Style::new().red()),
            error_area,
        );
    }
}

// 使用
fn handle_action(&mut self, action: Action) {
    match action {
        Action::Error(msg) => self.set_error(msg),
        Action::DataLoaded(items) => self.items = items,
        _ => {}
    }
}
```

---

## 可选子系统的优雅降级（null object）

TUI 常包着一堆**可选能力**：图片协议（sixel/kitty）、音频、剪贴板、系统通知、媒体键（MPRIS/D-Bus）、网络抓取。这些子系统的初始化**经常会失败且不可控**——无显卡协议、无声卡、headless、无网、TLS/证书坏、无 D-Bus session。

**反模式**：启动时 `let x = Subsystem::init()?;` ——任何一个可选能力建不起来，整个 TUI 直接起不来。用户只是想看个界面，却因为「没声卡」打不开应用。

**正解**：失败时降级成 **null object**（一个语义完整、什么都不做的实现），`warn!` 一条后继续：

```rust
impl ImageRenderer {
    /// 真构造：探测终端图片协议、建解码器。
    pub fn detect() -> Result<Self> { /* … */ }

    /// 禁用态：null object。render() 静默 no-op、支持查询恒返 false。
    /// 不持有任何资源、不依赖 runtime。
    pub fn disabled() -> Self { Self { backend: Backend::None } }
}

// 启动时降级，而非 ? 冒泡：
let images = ImageRenderer::detect().unwrap_or_else(|e| {
    tracing::warn!(error = %e, "图片协议不可用，禁用图片渲染");
    ImageRenderer::disabled()
});
```

要点：

- **null object 必须是「正常对象」**，不是 `Option<Subsystem>` 到处 `if let Some`。让 `disabled()` 实现同一套接口（`render()` 丢弃、`poll()` 恒空、`is_available() == false`），调用点**零分支**照常调，复杂度不外泄。
- **降级 ≠ 假装成功**。要让用户**可感知**：状态栏标个 `⚠ no audio` / `图片已禁用`，别静默骗人。
- **这同时解锁了可测性**：`disabled()` 通常**不依赖 runtime / 设备 / 网络**，于是测试里能零依赖构造整个 App（见 testing.md 模式 B）。降级基建和测试基建是同一个东西。
- 典型实例：音频引擎无输出设备 → null sink 空跑（接受命令不发声）；封面/缩略图抓取器 isahc 建不起来 → 禁用态（请求丢弃、就绪队列恒空）；MPRIS 无 D-Bus → 跳过媒体键集成。三者都**不该**让 daemon / TUI 起不来。

---

## 完整 main.rs 模板

```rust
use color_eyre::Result;

fn main() -> Result<()> {
    // 1. 安装 color-eyre（装它自己的 panic hook + 美化错误输出）
    color_eyre::install()?;

    // 2. 链式包裹 color-eyre 的 hook：panic 时先退终端、再让它打印彩色报告。
    //    必须在 install 之后、enter 之前（见「自定义 panic hook」节）。
    init_panic_hook();

    // 3. 初始化日志（可选）。注意：**绝不**写 stdout/stderr，会污染 alternate screen；
    //    写文件，且错误链用单行无 ANSI 格式（带色码/backtrace 会污染日志文件）。
    // let log_file = std::fs::File::create("/tmp/my-tui.log")?;
    // tracing_subscriber::fmt().with_ansi(false).with_writer(log_file).init();

    // 4. 进入 TUI 模式
    let mut tui = Tui::new()?;
    tui.enter()?;

    // 5. 运行主循环
    let result = App::new().run(&mut tui);

    // 6. 退出 TUI 模式（Tui::drop 也会做，这里显式更清晰）
    tui.exit()?;

    // 7. 恢复终端后才 ? 传播应用层错误
    result
}
```

**关键顺序**：`color_eyre::install()` → `init_panic_hook()` → `tui.enter()`。三条退出路径各有归属——用户退/Result 冒泡走 `Tui::Drop`、panic 走链式 hook、外部信号走 watcher 置标志 + 主循环（见「Ctrl-C 与信号处理」）——**都不要再加第四条裸 hook 绕过它们**。

## 参考链接

- https://ratatui.rs/recipes/apps/panic-hooks/
- https://docs.rs/color-eyre/latest/color_eyre/
- https://docs.rs/crossterm/latest/crossterm/terminal/fn.enable_raw_mode.html
- https://github.com/ratatui/templates/blob/main/component/src/tui.rs （官方模板的 Tui struct）
- https://ratatui.rs/tutorials/hello-ratatui/ （基础 setup/teardown）
