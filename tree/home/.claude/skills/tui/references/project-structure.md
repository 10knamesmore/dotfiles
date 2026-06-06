# 项目结构与模块组织

## 目录
- [标准目录结构](#标准目录结构)
- [各文件职责](#各文件职责)
- [Action enum 设计](#action-enum-设计)
- [组件拆分时机](#组件拆分时机)
- [Cargo.toml 配置](#cargotoml-配置)

---

## 标准目录结构

官方 component template 推荐结构，适合中大型 TUI 应用：

```
my-tui-app/
├── Cargo.toml
├── src/
│   ├── main.rs          # 入口：初始化 terminal、运行主循环、cleanup
│   ├── app.rs           # App struct：持有所有组件、分发事件
│   ├── tui.rs           # Terminal 封装：setup/teardown/draw
│   ├── action.rs        # Action 枚举定义
│   ├── config.rs        # 配置文件解析（可选）
│   ├── errors.rs        # 自定义错误类型
│   └── components/
│       ├── mod.rs        # 重新导出所有组件
│       ├── home.rs       # 主页面组件
│       ├── header.rs     # 顶部状态栏
│       ├── footer.rs     # 底部帮助栏
│       └── list.rs       # 通用列表组件
└── references/          # 配置文件、样例数据等（可选）
```

**核心原则**：基础设施（main/tui/errors）稳定后不再修改，业务扩展只在 `components/` 目录内进行。

---

## 各文件职责

### `main.rs` — 入口，不含业务逻辑

```rust
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;
    tui.enter()?;  // 进入原始模式 + 备用屏幕

    let mut app = App::new();
    app.run(&mut tui)?;

    tui.exit()?;  // 恢复终端
    Ok(())
}
```

### `tui.rs` — Terminal 生命周期封装

```rust
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut Frame)) -> Result<()> {
        self.terminal.draw(f)?;
        Ok(())
    }
}

// 确保 panic 时也能恢复终端
impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().ok();
    }
}
```

### `app.rs` — 组件容器，协调事件分发

```rust
pub struct App {
    pub components: Vec<Box<dyn Component>>,
    pub focused_idx: usize,
    pub should_quit: bool,
    pub action_tx: UnboundedSender<Action>,
    action_rx: UnboundedReceiver<Action>,
}

impl App {
    pub fn new() -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Self {
            components: vec![
                Box::new(Home::new()),
            ],
            focused_idx: 0,
            should_quit: false,
            action_tx,
            action_rx,
        }
    }

    pub fn run(&mut self, tui: &mut Tui) -> Result<()> {
        loop {
            // 处理终端事件
            if event::poll(Duration::from_millis(16))? {
                let event = event::read()?;
                for component in &mut self.components {
                    if let Some(action) = component.handle_events(Some(event.clone()))? {
                        self.action_tx.send(action)?;
                    }
                }
            }

            // 处理 Actions
            while let Ok(action) = self.action_rx.try_recv() {
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::Render => {
                        let components = &mut self.components;
                        tui.draw(|f| {
                            for c in components.iter_mut() {
                                c.draw(f, f.area()).ok();
                            }
                        })?;
                    }
                    action => {
                        for component in &mut self.components {
                            if let Some(next) = component.update(action.clone())? {
                                self.action_tx.send(next)?;
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }
}
```

### `action.rs` — 应用动作枚举

见下节。

### `errors.rs` — 统一错误类型

```rust
// 多数项目直接用 color_eyre::Result，无需自定义
// 如需细化：
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Config error: {0}")]
    Config(String),
}
```

---

## Action enum 设计

Action 是连接事件处理与业务逻辑的枢纽。

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // 系统级（基础设施处理）
    Quit,
    Render,
    Tick,
    Resize(u16, u16),
    Error(String),

    // 输入级（转换自终端事件）
    Key(KeyEvent),
    Mouse(MouseEvent),

    // 应用级（业务逻辑）
    NavigateUp,
    NavigateDown,
    Select,
    GoBack,
    OpenFile(PathBuf),

    // 异步结果（后台任务回传）
    DataLoaded(Vec<Item>),
    LoadingStarted,
    LoadingFinished,
}
```

**设计原则**：
- 系统级 Action 由基础设施处理，业务组件无需关心
- Key/Mouse 只作为中间层，组件优先响应语义 Action（NavigateUp）而非原始键值
- 异步任务通过 Action 回传结果，保持单向数据流

---

## 组件拆分时机

**单文件（< 200 行）**：扁平结构，不拆组件

**拆分为组件（> 200 行 / 有独立生命周期）**：
- 独立的键盘焦点区域（如左侧文件列表 + 右侧预览）
- 有自己的滚动/选择状态
- 可能在多处复用（如通用 SelectList 组件）
- 有独立的异步数据加载逻辑

**拆分为子模块（组件内过于复杂）**：
```
components/
├── explorer/
│   ├── mod.rs       # Explorer 组件主体
│   ├── tree.rs      # 目录树渲染
│   └── preview.rs   # 文件预览
└── statusbar.rs
```

---

## Cargo.toml 配置

```toml
[package]
name = "my-tui-app"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
color-eyre = "0.6"
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
futures = "0.3"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## 参考链接

- https://ratatui.rs/templates/component/project-structure/
- https://github.com/ratatui/templates/tree/main/component
- https://ratatui.github.io/async-template/02-structure.html
- https://github.com/ratatui/awesome-ratatui
