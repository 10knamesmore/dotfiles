# TUI 架构模式

## 目录
- [Elm Architecture (TEA)](#elm-architecture-tea)
- [Component Architecture](#component-architecture)
- [Action Pattern（异步变体）](#action-pattern)
- [Flux Architecture](#flux-architecture)
- [选型决策](#选型决策)
- [生产项目参考](#生产项目参考)

---

## Elm Architecture (TEA)

最推荐的 TUI 架构，数据流单向、状态可追踪、可测试性强。

### 三要素

```
Model（状态）→ View（渲染）→ 用户操作 → Message → Update（纯函数）→ 新 Model
```

- **Model**：应用全局状态的完整表示，所有 UI 所需数据都在这里
- **Message**：枚举，描述"发生了什么"，不包含业务逻辑
- **Update**：纯函数 `(Model, Message) → Model`，不产生副作用
- **View**：纯函数，从 Model 映射到 Widget，相同 Model 必然产生相同 UI

### 完整实现模板

```rust
// message.rs
#[derive(Debug, Clone)]
pub enum Message {
    Quit,
    KeyPressed(KeyEvent),
    Tick,
    DataLoaded(Vec<Item>),
    SelectNext,
    SelectPrev,
}

// model.rs
pub struct Model {
    pub should_quit: bool,
    pub items: Vec<Item>,
    pub selected: usize,
    pub mode: Mode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
}

impl Model {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            items: vec![],
            selected: 0,
            mode: Mode::Normal,
        }
    }
}

// update.rs
pub fn update(model: Model, msg: Message) -> Model {
    match msg {
        Message::Quit => Model { should_quit: true, ..model },
        Message::SelectNext => Model {
            selected: (model.selected + 1).min(model.items.len().saturating_sub(1)),
            ..model
        },
        Message::SelectPrev => Model {
            selected: model.selected.saturating_sub(1),
            ..model
        },
        Message::DataLoaded(items) => Model { items, ..model },
        _ => model,
    }
}

// view.rs
pub fn view(model: &Model, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let items: Vec<ListItem> = model.items.iter()
        .map(|i| ListItem::new(i.name.as_str()))
        .collect();

    let mut list_state = ListState::default().with_selected(Some(model.selected));
    frame.render_stateful_widget(
        List::new(items).highlight_symbol("> "),
        chunks[0],
        &mut list_state,
    );

    frame.render_widget(
        Paragraph::new(format!("Mode: {:?}", model.mode)),
        chunks[1],
    );
}

// main.rs
fn run(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let mut model = Model::new();

    loop {
        terminal.draw(|f| view(&model, f))?;

        let msg = handle_events()?;
        if let Some(msg) = msg {
            model = update(model, msg);
        }

        if model.should_quit {
            break;
        }
    }
    Ok(())
}
```

### 返回副作用（命令模式扩展）

当 Update 需要触发异步操作时，返回可选命令：

```rust
pub enum Command {
    None,
    LoadData,
    SaveFile(PathBuf),
}

pub fn update(model: Model, msg: Message) -> (Model, Command) {
    match msg {
        Message::Refresh => (model, Command::LoadData),
        _ => (model, Command::None),
    }
}
```

---

## Component Architecture

面向对象风格，每个组件封装自身的状态、事件和渲染。适合多个独立 UI 区域。

### Component Trait

```rust
pub trait Component {
    /// 初始化，返回可选的首次 Action
    fn init(&mut self) -> Result<Option<Action>> {
        Ok(None)
    }

    /// 处理终端事件，返回可选 Action
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        match event {
            Some(Event::Key(key)) => self.handle_key_event(key),
            Some(Event::Mouse(mouse)) => self.handle_mouse_event(mouse),
            _ => Ok(None),
        }
    }

    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn handle_mouse_event(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    /// 更新组件状态，返回可选 Action（支持链式响应）
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }

    /// 渲染到指定区域
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>;
}
```

### 完整组件示例

```rust
pub struct FileList {
    items: Vec<PathBuf>,
    state: ListState,
    is_focused: bool,
}

impl Component for FileList {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if !self.is_focused { return Ok(None); }
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.state.selected()
                    .map(|i| (i + 1).min(self.items.len() - 1))
                    .unwrap_or(0);
                self.state.select(Some(next));
                Ok(None)
            }
            KeyCode::Enter => {
                let path = self.state.selected()
                    .map(|i| self.items[i].clone());
                Ok(path.map(Action::OpenFile))
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let items: Vec<ListItem> = self.items.iter()
            .map(|p| ListItem::new(p.to_string_lossy().to_string()))
            .collect();

        let border_style = if self.is_focused {
            Style::new().cyan()
        } else {
            Style::default()
        };

        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().border_style(border_style).title("Files"))
                .highlight_symbol("> "),
            area,
            &mut self.state,
        );
        Ok(())
    }
}
```

### App 组装多个组件

```rust
pub struct App {
    components: Vec<Box<dyn Component>>,
    focused: usize,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                    .split(f.area());

                for (i, (component, area)) in
                    self.components.iter_mut().zip(chunks.iter()).enumerate()
                {
                    component.draw(f, *area).unwrap();
                }
            })?;

            if let Some(event) = read_event()? {
                let mut actions = vec![];
                for component in &mut self.components {
                    if let Some(action) = component.handle_events(Some(event.clone()))? {
                        actions.push(action);
                    }
                }
                for action in actions {
                    self.handle_action(action)?;
                }
            }
        }
    }
}
```

---

## Action Pattern

TEA 的异步变体，使用 `Action` 枚举 + mpsc channel，适合有大量异步操作的应用（网络、数据库等）。

```rust
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Tick,
    Render,
    Resize(u16, u16),
    Key(KeyEvent),
    // 应用特定 Action
    FetchData(String),
    DataReceived(Vec<Item>),
    Error(String),
}

// 主循环
async fn run(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let mut app = App::new(action_tx.clone());

    let mut tick_interval = tokio::time::interval(Duration::from_millis(250));
    let mut render_interval = tokio::time::interval(Duration::from_millis(16)); // ~60fps
    let mut events = EventStream::new();

    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                action_tx.send(Action::Tick)?;
            }
            _ = render_interval.tick() => {
                action_tx.send(Action::Render)?;
            }
            Some(Ok(event)) = events.next() => {
                match event {
                    Event::Key(key) => action_tx.send(Action::Key(key))?,
                    Event::Resize(w, h) => action_tx.send(Action::Resize(w, h))?,
                    _ => {}
                }
            }
            Some(action) = action_rx.recv() => {
                match action {
                    Action::Render => {
                        terminal.draw(|f| app.draw(f))?;
                    }
                    Action::Quit => break,
                    action => {
                        if let Some(next_action) = app.update(action)? {
                            action_tx.send(next_action)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
```

---

## Flux Architecture

强调单向数据流，适合需要多个 Store 的复杂应用。

```
用户操作 → Action → Dispatcher → Store(s) → View 重绘
```

- **Action**：描述事件的数据对象
- **Dispatcher**：接收 Action，分发给所有注册的 Store
- **Store**：持有特定领域的状态，响应 Action 更新
- **View**：从 Store 读取状态渲染

适合场景：多个相互独立的状态领域（如菜单状态、编辑器状态、网络状态分别管理）。

---

## 选型决策

```
需要异步操作（网络/DB）？
├─ 是 → Action Pattern（tokio + mpsc channel）
└─ 否
    ├─ 有多个独立 UI 区域？
    │   └─ 是 → Component Architecture
    ├─ 强调可测试性/函数式？
    │   └─ 是 → Elm Architecture (TEA)
    └─ 简单工具
        └─ 扁平 App struct 即可
```

---

## 生产项目参考

| 项目 | 架构 | 特点 |
|------|------|------|
| [gitui](https://github.com/extrawurst/gitui) | 集中式 App State | 同步轮询，状态机清晰 |
| [yazi](https://github.com/sxyazi/yazi) | Action + tokio LocalSet | Micro/Macro 任务队列，极高性能 |
| [bottom](https://github.com/ClementTsang/bottom) | 扁平 + tick 驱动 | 实时监控，定时刷新 |
| [spotify-tui](https://github.com/Rigellute/spotify-tui) | 异步线程分离 | UI 线程和网络线程解耦 |

## 参考链接

- https://ratatui.rs/concepts/application-patterns/the-elm-architecture/
- https://ratatui.rs/concepts/application-patterns/component-architecture/
- https://ratatui.rs/concepts/application-patterns/flux-architecture/
- https://github.com/ratatui/templates/tree/main/component
- https://github.com/ratatui/awesome-ratatui
