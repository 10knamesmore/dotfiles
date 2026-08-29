# TUI 多页面 / 焦点 / 分层路由

本文约束多页面 TUI 的四个边界：**怎么建模当前全屏页、怎么建模同屏面板焦点、按键往哪一层路由、文本框与列表怎么复用。**

核心心法一句话：**「全屏页」和「面板焦点」是两个正交概念，必须拆开建模；事件路由的优先级只能有一处真相源。**

## 目录
- [全屏页 / 路由建模](#全屏页--路由建模)
- [焦点分两层正交建模](#焦点分两层正交建模)
- [优先级单一真相源 + 组件返回意图](#优先级单一真相源--组件返回意图)
- [Input / List 抽共用原语](#input--list-抽共用原语)

---

## 全屏页 / 路由建模

两种范式，按「页之间是否叠加 / 是否要深下钻回退」二选一：

### ① 页多 / 有深下钻回退 → view-stack

页能层层下钻、按 `back` 一层层弹回（专辑 → 歌手 → 相关专辑 …），用一个**栈**：栈顶就是当前页、就是焦点，`back` = `pop`。

```rust
/// 一个全屏页。具体页是闭集 enum 的一个变体（见下「Rust 提醒」）。
pub struct App {
    /// 页面栈：底是入口页，栈顶是当前页。下钻 = push，返回 = pop。
    pages: Vec<PageState>,
}

impl App {
    /// 当前页 = 栈顶。用类型保证非空，而不是 expect("non-empty")。
    fn current(&self) -> &PageState {
        // 入口页在构造时压入且永不弹空 → last() 用 unwrap_or 兜一个静止页，
        // 既不撞 unwrap 禁用 lint，又不会 panic。
        self.pages.last().unwrap_or(&PageState::Idle)
    }

    fn push(&mut self, page: PageState) {
        self.pages.push(page);
    }

    fn back(&mut self) {
        // 留住底层入口页：栈深 > 1 才弹，避免弹空。
        if self.pages.len() > 1 {
            self.pages.pop();
        }
    }
}
```

### ② 少量互斥页（不叠加）→ 派生 `active_layer()`

页就那么几个、彼此互斥、不存在「叠在一起」（如「主列表 / 帮助 / 输入框」三选一），不必上栈——写**一个派生函数**，按各组件的 visible / 优先级当场算出当前活跃层。渲染和路由共用这一个函数，天然不会漂移：

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveLayer {
    Input,
    Help,
    Main,
}

impl App {
    /// 单一函数：按优先级从高到低判定当前活跃层。
    /// 渲染、按键路由、上下文提示全读它 → 不可能各写一份而漂移。
    fn active_layer(&self) -> ActiveLayer {
        if self.input.is_open() {
            ActiveLayer::Input
        } else if self.help.is_open() {
            ActiveLayer::Help
        } else {
            ActiveLayer::Main
        }
    }
}
```

### 反面教材：单 enum 兼任「页面 + 焦点 + 输入路由」三职

用一个 `Mode` enum 同时表达页面、焦点和输入路由，会让 `match self.mode` 平行散落到多个文件。每加一个状态都必须同步补齐全部 arm；漏补的分支可能落入 `_ =>` 并静默吞掉输入。

> 必须把「全屏页是哪个」与「面板焦点在哪」拆成两个正交概念（见下一节）。一个 enum 别既当导航又当焦点又当输入模式。

### Rust 提醒

- **页面身份优先闭集 enum + 手动转发**，而不是 `Vec<Box<dyn Page>>` + downcast。依赖 `type_name` 字符串反查页面身份会在重命名时失效；闭集 enum 的穷尽 `match` 会要求新增页面补全所有分支。
- **取栈顶别 `expect("non-empty")`**（撞 `unwrap_used` / `expect_used` 禁用 lint）。要么入口页恒驻、`last().unwrap_or(&Idle)` 兜底，要么用「非空栈」类型（`(T, Vec<T>)` 头 + 尾）从类型上保证非空。

---

## 焦点分两层正交建模

「焦点」有两层，分开放、各管各：

| 层 | 是什么 | 存哪 |
|---|---|---|
| **全屏级焦点** | 当前在哪个全屏页 | **不存**——由 view-stack 栈顶 / `active_layer()` 派生函数隐式得出 |
| **面板级焦点** | 同屏多面板里哪个面板亮 | 该页**私有的一个 focus 枚举字段**，作用域收窄到需要它的那个页 |

关键：**面板焦点是页的私有字段，不是全局字段。** 只有「同屏真有多个可切换面板」的页才需要它；单视图页（一个列表 + 一个搜索框那种）把焦点折叠进 mode 即可，别强加一套焦点机制。

### 多面板焦点环 = focus 枚举 + 相邻对成环，不是 focus 栈

面板间循环切焦（Tab 转一圈）用「focus 枚举 + 相邻对定义 next/previous」：

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Sidebar,
    List,
    Detail,
}

impl Focus {
    /// 相邻对成环：next 一圈回到起点。
    fn next(self) -> Self {
        match self {
            Self::Sidebar => Self::List,
            Self::List => Self::Detail,
            Self::Detail => Self::Sidebar,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Sidebar => Self::Detail,
            Self::List => Self::Sidebar,
            Self::Detail => Self::List,
        }
    }
}
```

**focus 栈 ≠ 焦点环。** focus 栈只用于「打开浮层时记住来时焦点、关闭后撤销返回上一焦点」这种**撤销语义**；日常 Tab 循环切面板用上面的枚举环。两者别混。

### 反对「一个全局 focus 字段管所有面板」

全局焦点指针要求每次切换时向多个组件同步 focus/blur 状态，容易产生双写漂移。焦点字段属于**它所服务的那个页**，跟着页的生命周期走；页关闭后，其焦点状态一起消失。

---

## 优先级单一真相源 + 组件返回意图

### 铁律：overlay > page > global 的优先级只能定义在一处

输入优先级（浮层吃键 > 当前页吃键 > 全局快捷键兜底）**只能写一遍**——要么是「层栈逆序冒泡」（见 architecture-patterns.md），要么是「派生 `active_layer()`」。**严禁在「按键 handler」和「上下文采集」两处各写一遍。**

> 若 `handle_key` 与 `collect_key_context` 各自维护一份视图优先级，漏掉任一浮层分支就会出现按键有效但提示缺失。两者必须共用 `active_layer()` 或同一层栈。

### 组件返回意图，不就地做副作用

组件的 `on_key` / `on_action` **返回** `Consumed / Pass / Do(intent)`，**真副作用（改后端、发网络、写文件）一律在顶层执行**：

```rust
pub enum Response {
    Consumed,
    Pass,
    Do(Intent), // 顶层据此执行，组件自己不碰副作用
}

// 组件层：只表达「我想干什么」
impl Layer for SearchPage {
    fn on_action(&mut self, action: Action) -> Response {
        match action {
            Action::Confirm => Response::Do(Intent::Search(self.query.clone())),
            Action::MoveDown => { self.list.select_next(); Response::Consumed }
            _ => Response::Pass,
        }
    }
}

// 顶层：唯一执行副作用的地方
fn run_intent(&mut self, intent: Intent) -> color_eyre::Result<()> {
    match intent {
        Intent::Search(q) => self.client.search(&q)?, // 网络副作用只在这
        Intent::OpenFile(p) => self.open(p)?,
    }
    Ok(())
}
```

两个收益：①「组件 = 纯输入→Response 映射」**可单测**，喂键断言返回的 `Response` 即可，不用起后端；② **server-client 架构下 UI 进程天然不产生副作用**，所有改动都收敛到顶层一条意图通道。

### 配套：`on_action`（结构化）优先于 `on_key`（裸键）

组件接口优先收**结构化 Action**（`on_action(Action)`）而非裸 `KeyEvent`（`on_key`）。裸键到 Action 的映射在顶层按 config keymap 做一次，组件只认语义动作。组件内硬编码物理键会绕过用户 keymap，使重映射失效。

```rust
// ❌ 组件里硬编码物理键：用户改不了，config keymap 形同虚设
fn on_key(&mut self, key: KeyEvent) -> Response {
    match key.code {
        KeyCode::Tab => { /* ... */ }   // 焊死，重映射失效
        KeyCode::Enter => { /* ... */ }
        _ => Response::Pass,
    }
}

// ✅ 组件认语义 Action；KeyEvent→Action 在顶层按 keymap 映射
fn on_action(&mut self, action: Action) -> Response {
    match action {
        Action::NextField => { self.focus = self.focus.next(); Response::Consumed }
        Action::Confirm => Response::Do(Intent::Submit(self.draft())),
        _ => Response::Pass,
    }
}
```

> 文本输入框是例外的细节多发区：它确实要消费一批裸键（字符、退格、光标移动），但仍**不该把整套键映射焊进库**——见下一节 tui-textarea 反面。

---

## Input / List 抽共用原语

两类逻辑在多处重复写就是坏味道，收口成共用原语：

### 文本输入：事件解码与态更新两段分离（tui-input 范式）

照 tui-input 的分层：**「把事件解码成一个 `InputRequest`」与「纯态变更产出 `StateChanged`」两段分开**，核心可以**零 ratatui 依赖**、纯字符串 + 光标的状态机，好测：

```rust
/// 一次编辑请求（已从 KeyEvent / Action 解码出来，与终端库无关）。
pub enum InputRequest {
    InsertChar(char),
    DeletePrevChar,
    GoToPrevChar,
    GoToNextChar,
    GoToStart,
    GoToEnd,
}

/// 纯态：一个 String + 光标（按 grapheme 计）。无任何 ratatui / crossterm 依赖。
#[derive(Default)]
pub struct InputState {
    value: String,
    /// 光标位置，单位是 grapheme（见下「三种长度」）。
    cursor: usize,
}

impl InputState {
    /// 纯函数式状态机：吃一个 request，返回是否真的变了（便于脏检测 / 测试）。
    pub fn handle(&mut self, req: InputRequest) -> bool {
        match req {
            InputRequest::InsertChar(c) => { self.insert(c); true }
            InputRequest::DeletePrevChar => self.delete_prev(),
            InputRequest::GoToPrevChar => self.move_left(),
            // ... 其余 request
            _ => false,
        }
    }
}
```

**unicode 光标必须区分三种长度**，混用就会在中文 / emoji 上光标错位甚至 panic：

| 长度 | 是什么 | 用途 |
|---|---|---|
| **grapheme cluster** | 用户感知的「一个字」（含组合 emoji） | 光标按它移动、删除（`unicode-segmentation`） |
| **codepoint**（char） | Rust 的 `char` | 内部存储索引换算 |
| **display column** | 终端占几列（CJK 宽字符占 2，`unicode-width`） | 渲染光标 x 坐标、截断对齐 |

别用 byte 偏移或 char 数裸算光标位置——CJK / emoji 必错。

**反面教材**：tui-textarea 把约 300 行 Emacs 键映射**焊进库的 `input()` 方法**。一旦你的应用是 config-driven keymap，这套硬编码映射就和你的 keymap 直接冲突（同一个键库内库外抢着解释）。**keymap 必须数据驱动、留在应用层**，输入原语只暴露「吃一个已解码的 `InputRequest`」，不自带键绑定。

### 列表：泛型 `ListSelect<T>` / `Picker<T>`，选择/滚动只写一份

「选中索引、上下移动、自动滚动保持选中项可见、空列表夹紧」这套逻辑**到处各写一遍**是公认要消灭的坏味道。收口成一个**泛型**组件：

```rust
/// 泛型选择列表：选择 / 滚动逻辑只此一份，任何 T 复用。
pub struct ListSelect<T> {
    items: Vec<T>,
    selected: usize,
    offset: usize,
}

impl<T> ListSelect<T> {
    pub fn select_next(&mut self) {
        let max = self.items.len().saturating_sub(1);
        self.selected = (self.selected + 1).min(max);
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// 取当前选中项——用 .get() 不裸索引：列表异步变短时 selected 可能过期。
    pub fn selected(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    /// 替换数据后把 selected 夹回合法范围（后端刷新 / 筛选后必调）。
    pub fn set_items(&mut self, items: Vec<T>) {
        self.selected = self.selected.min(items.len().saturating_sub(1));
        self.items = items;
    }
}
```

Rust 列表原语优先用泛型复用选择与滚动逻辑，而不是为固定元素类型引入 `Box<dyn>`；泛型保留具体类型方法并避免虚调用。

---

## 参考链接

- unicode 分段 / 宽度：https://docs.rs/unicode-segmentation/ ・ https://docs.rs/unicode-width/
