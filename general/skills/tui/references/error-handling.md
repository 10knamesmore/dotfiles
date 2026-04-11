# 错误处理与终端恢复

## 目录
- [核心问题：终端脏退出](#核心问题终端脏退出)
- [color-eyre 集成](#color-eyre-集成)
- [Terminal RAII Guard 模式](#terminal-raii-guard-模式)
- [原始模式与备用屏幕](#原始模式与备用屏幕)
- [Ctrl-C 与信号处理](#ctrl-c-与信号处理)
- [错误在 UI 内展示](#错误在-ui-内展示)
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

### 自定义 panic hook 集成 terminal cleanup

```rust
pub fn init_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // 先恢复终端（不管是否成功）
        restore_terminal().ok();
        // 再调用原始 hook（打印 panic 信息）
        original_hook(panic_info);
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

### 方案 1：crossterm 事件处理（推荐）

原始模式下 Ctrl-C **不会**发送 SIGINT，而是作为 `KeyCode::Char('c') + Modifiers::CONTROL` 事件。直接在键盘处理中捕获：

```rust
(KeyModifiers::CONTROL, KeyCode::Char('c')) => {
    return Ok(());  // 正常退出，触发 Tui::drop() 恢复终端
}
```

### 方案 2：tokio signal（适合需要优雅关闭的应用）

```toml
tokio = { version = "1", features = ["signal"] }
```

```rust
use tokio::signal;

async fn run_with_signal_handling(
    mut terminal: Terminal<impl Backend>,
    mut app: App,
) -> Result<()> {
    let mut ctrl_c = signal::ctrl_c();

    loop {
        terminal.draw(|f| app.draw(f))?;

        tokio::select! {
            _ = ctrl_c => {
                break;  // 优雅退出
            }
            Some(Ok(event)) = events.next() => {
                app.handle_event(event)?;
            }
        }

        if app.should_quit { break; }
    }
    Ok(())
}
```

**注意**：如果启用了 raw mode，`signal::ctrl_c()` 在某些平台可能不触发（因为 raw mode 拦截了 Ctrl-C 信号）。推荐使用方案 1。

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

## 完整 main.rs 模板

```rust
use color_eyre::Result;

fn main() -> Result<()> {
    // 1. 安装 color-eyre（panic hook + 美化错误输出）
    color_eyre::install()?;

    // 2. 初始化日志（可选，调试时有用）
    // 注意：不要写到 stdout，会破坏 TUI 渲染
    // 写到文件：
    // let log_file = std::fs::File::create("/tmp/my-tui.log")?;
    // tracing_subscriber::fmt().with_writer(log_file).init();

    // 3. 初始化并进入 TUI 模式
    let mut tui = Tui::new()?;
    tui.enter()?;

    // 4. 运行应用主循环
    let result = App::new().run(&mut tui);

    // 5. 退出 TUI 模式（Tui::drop 也会做，这里显式做更清晰）
    tui.exit()?;

    // 6. 处理应用层错误（恢复终端后才 ? 传播）
    result
}
```

**关键顺序**：`color_eyre::install()` 必须在 `tui.enter()` **之前**调用，这样 panic hook 才能在 panic 时正确恢复终端。

## 参考链接

- https://ratatui.rs/recipes/apps/panic-hooks/
- https://docs.rs/color-eyre/latest/color_eyre/
- https://docs.rs/crossterm/latest/crossterm/terminal/fn.enable_raw_mode.html
- https://github.com/ratatui/templates/blob/main/component/src/tui.rs （官方模板的 Tui struct）
- https://ratatui.rs/tutorials/hello-ratatui/ （基础 setup/teardown）
