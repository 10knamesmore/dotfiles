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

面向对象风格，每个组件封装自身的状态、事件和渲染。适合有多页面 / 分层浮层 / 焦点路由的应用。

### 默认范式：有序栈/层 + 逆序冒泡 + 第一个 Consumed 短路

**把事件广播给所有组件、再由各组件自行检查焦点是反模式。** 广播会让每次按键触发全部组件更新，并把焦点判断分散到多个 owner。默认应只路由给当前聚焦者或栈顶层，未消费时再向外冒泡；只有同步输入到多个 pane 这类显式功能才使用广播。

正解是组件 / 页返回一个**响应枚举**，顶层按层级逆序派发、第一个 `Consumed` 即停：

```rust
/// 一次输入在某一层的处理结果。
pub enum Response {
    /// 已吃掉，停止向外冒泡。
    Consumed,
    /// 本层不关心，继续冒泡给下一层 / 全局。
    Pass,
    /// 已吃掉，并请求顶层执行一个意图（真副作用在顶层做，见下文）。
    Do(Intent),
}
```

```rust
/// 一层 UI（页 / 浮层 / 编辑器）。注意没有 `is_focused` 字段——
/// 谁在栈顶谁就是焦点，由栈位置隐式决定。
pub trait Layer {
    fn on_key(&mut self, key: KeyEvent) -> Response;

    /// `focused` 在渲染时传入，组件不自存一份（见「焦点即时派生」一节）。
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool);
}

pub struct App {
    /// 层栈：layers[0] 是基底页，越靠后越上层（浮层 / 子页）。栈顶即焦点。
    layers: Vec<Box<dyn Layer>>,
}

impl App {
    fn dispatch_key(&mut self, key: KeyEvent) -> color_eyre::Result<()> {
        // 逆序：从最上层往下冒泡，第一个 Consumed/Do 即短路。
        for layer in self.layers.iter_mut().rev() {
            match layer.on_key(key) {
                Response::Consumed => return Ok(()),
                Response::Do(intent) => return self.run_intent(intent),
                Response::Pass => continue, // 本层放行，交给下一层
            }
        }
        self.global_key(key) // 所有层都放行 → 全局快捷键兜底
    }
}
```

### 焦点即时派生：别把 `is_focused` 存成组件字段

即时模式下「谁聚焦」每帧都能算出来，**不该每个组件存一份 `is_focused: bool`**——那是双写漂移源（焦点切换时漏更新某个组件就花屏）。正解：焦点是渲染时从一个集中的 focus 枚举**瞬时派生**的 bool，作为参数喂给组件：

```rust
// ❌ 反模式：组件自存焦点，焦点切换要逐个组件同步，漏一个就错
pub struct FileList {
    items: Vec<PathBuf>,
    state: ListState,
    is_focused: bool, // 双写源：和 App 的 focus 枚举两头记，迟早不一致
}

// ✅ 焦点不进组件状态；渲染时由集中 focus 派生后传入
pub struct FileList {
    items: Vec<PathBuf>,
    state: ListState,
}

impl FileList {
    pub fn render(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused { Style::new().cyan() } else { Style::default() };
        let items = self.items.iter()
            .map(|p| ListItem::new(p.to_string_lossy().into_owned()));
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().border_style(border_style).title("Files"))
                .highlight_symbol("> "),
            area,
            &mut self.state,
        );
    }
}

// 顶层渲染：focused 由「页是否激活 && 集中 focus 指向本面板」当场算出
fn draw(&mut self, frame: &mut Frame, area: Rect) {
    let focused = self.is_active && self.focus == Focus::Files;
    self.file_list.render(frame, area, focused);
}
```

焦点应由页面激活状态与集中 focus 枚举即时计算，再作为参数传给组件。组件各自持有 `is_focused` 会要求切换时手动双写，并可能遗漏同步。

> 路由组件返回意图、顶层执行副作用、`Consumed/Pass/Do` 的完整设计与「优先级单一真相源」铁律，见 `references/tui/page-focus-routing.md`。

### 组件内列表导航的两个防回归点

无论用哪种架构，列表组件常踩这两个坑（异步刷新 / 筛选会让旧选择越界）：

```rust
fn on_key(&mut self, key: KeyEvent) -> Response {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            // saturating_sub 防空列表下溢：len()==0 时 len()-1 会 usize 回绕成
            // 巨大值，(i+1).min(那个) 不再夹住 → 选择越界。
            let max = self.items.len().saturating_sub(1);
            let next = self.state.selected().map_or(0, |i| (i + 1).min(max));
            self.state.select(Some(next));
            Response::Consumed
        }
        KeyCode::Enter => {
            // 用 .get() 不用 self.items[i]：选择索引相对 items 会**过期**——
            // 列表异步刷新 / 筛选后变短，旧 selected 就指向越界，裸索引直接 panic。
            match self.state.selected().and_then(|i| self.items.get(i)) {
                Some(path) => Response::Do(Intent::OpenFile(path.clone())),
                None => Response::Consumed,
            }
        }
        _ => Response::Pass, // 不关心的键放行，交给下一层 / 全局
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
    ├─ 强调可测试性/函数式？
    │   └─ 是 → Elm Architecture (TEA)
    └─ 简单工具
        └─ 扁平 App struct 即可

正交维度——只要有「多页面 / 深下钻回退 / 分层浮层 / 同屏多面板焦点」：
├─ 页多 / 有回退栈        → view-stack（Vec<Box<dyn Layer>>，栈顶即页即焦点）
├─ 少量互斥页（不叠加）    → 派生 active_layer()（单函数按优先级算当前层）
├─ 同屏多面板互切焦点      → 该页私有一个 focus 枚举 + 相邻对成环 next/previous
└─ 事件路由               → 有序层逆序冒泡，第一个 Consumed 短路（切忌广播）
                           详见 references/tui/page-focus-routing.md
```

> 「多页面 + 焦点」不是上面四种顶层架构的第五种，而是叠加在任意一种之上的**正交维度**。
> 别用一个 enum 同时兼任「顶层架构 + 页面身份 + 面板焦点 + 输入路由」——拆开。

---

## 参考链接

- https://ratatui.rs/concepts/application-patterns/the-elm-architecture/
- https://ratatui.rs/concepts/application-patterns/component-architecture/
- https://ratatui.rs/concepts/application-patterns/flux-architecture/
- https://github.com/ratatui/templates/tree/main/component
- https://github.com/ratatui/awesome-ratatui
