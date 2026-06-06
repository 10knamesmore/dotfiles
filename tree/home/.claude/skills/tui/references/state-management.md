# 状态管理

## 目录
- [状态分层原则](#状态分层原则)
- [状态所有权：UI-local vs 后端同步态](#状态所有权ui-local-vs-后端同步态)
- [StatefulWidget 的 State 设计](#statefulwidget-的-state-设计)
- [脏检测优化渲染](#脏检测优化渲染)
- [内部可变性（Cell/RefCell）](#内部可变性)
- [多线程共享状态](#多线程共享状态)
- [状态持久化](#状态持久化)

---

## 状态分层原则

将状态分为三层，每层有明确职责：

```
┌─────────────────────────────────────┐
│  App State（全局应用状态）           │
│  - 当前模式（Normal/Insert/...)     │
│  - 全局错误信息                      │
│  - 路由/焦点信息                     │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│  Component State（组件业务状态）    │
│  - 当前加载的数据列表               │
│  - 输入缓冲区                        │
│  - 筛选条件                          │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│  View State（视图展示状态）         │
│  - 选中的列表索引（ListState）       │
│  - 滚动偏移                          │
│  - 展开/折叠状态                     │
└─────────────────────────────────────┘
```

**原则**：View State 不要混入 Component State；业务数据不要放进 `ListState` / `TableState` 等 ratatui 内置 State。

---

## 状态所有权：UI-local vs 后端同步态

**当你的 TUI 从一个外部来源周期性同步状态时**（client/server、轮询服务端、镜像守护进程、监控数据源——任何「每 tick `state = backend.snapshot()`」的架构），有一类极隐蔽、单元测试和组件快照都抓不到的 bug:

> **用户用按键改了某个 UI 状态，下一帧 tick 又把整份 snapshot 灌进来，把它覆盖回去。**

表现是「这个键好像没反应 / 光标一闪就弹回」。根因是**两类状态被混为一谈**：

| | 谁拥有 | 例子 | 同步规则 |
|---|---|---|---|
| **UI-local 状态** | 用户（本地） | 浮层里的选择光标、滚动位置、筛选词、折叠状态 | **永不被后端 snapshot 覆盖** |
| **后端权威状态** | 后端 | 播放位置、下载进度、连接状态、服务端列表 | 每 tick 从 snapshot 刷新 |

最容易踩的是**两者语义相近、却被塞进同一个字段**：比如「队列里的当前播放项下标」(后端的播放锚点) 和「队列浮层里用户选中的行」(UI 光标)——名字都叫 `selected`，于是同步时一并覆盖，用户根本移不动光标。

**正解**：

```rust
// ❌ 同步时无脑覆盖整个镜像，连 UI-local 字段一起冲掉
fn apply_snapshot(&mut self, snap: Snapshot) {
    self.state = snap.into();   // sel / scroll / filter 全没了
}

// ✅ 只刷新后端拥有的字段；UI-local 字段保留，至多做合法性 clamp
fn apply_snapshot(&mut self, snap: Snapshot) {
    self.items = snap.items;              // 后端态：刷新
    self.playback = snap.playback;        // 后端态：刷新
    // self.sel 是 UI 光标：不动它，只在列表变短时夹住防越界
    self.clamp_selection();
}
```

设计准则：

- **字段层面就分清谁拥有**。UI-local 字段像 `sel_*` / `scroll_*` 一样,是纯客户端的,同步逻辑碰都不该碰。
- **别让 UI 状态和后端状态共用一个字段**——哪怕语义「看起来一样」。命名上就区分（`backend_cursor` vs `ui_sel`）。
- 「当前后端项」要在 UI 里高亮,**用独立标记**（比对 id 画个 `▶`），而不是让 UI 光标去兼任。
- 这类 bug 的回归只有「喂键 → 跨一次 tick → 断言 UI 状态没被覆盖」的集成测试抓得住（见 testing.md 模式 C）。

---

## StatefulWidget 的 State 设计

### 使用 ratatui 内置 State

内置 State 只管"看哪里"，不管"有什么"：

```rust
pub struct ItemList {
    // 业务数据
    items: Vec<Item>,
    filtered: Vec<usize>,  // filtered[i] = items 中的原始索引
    filter: String,

    // 视图状态
    list_state: ListState,
    scroll_state: ScrollbarState,
}

impl ItemList {
    pub fn select_next(&mut self) {
        let len = self.filtered.len();
        self.list_state.select_next();
        // 同步 scrollbar
        if let Some(i) = self.list_state.selected() {
            self.scroll_state = self.scroll_state.position(i);
        }
    }

    pub fn selected_item(&self) -> Option<&Item> {
        self.list_state.selected()
            .and_then(|i| self.filtered.get(i))
            .and_then(|&idx| self.items.get(idx))
    }
}
```

### 自定义复合 State

对于复杂组件，自定义 State struct 更清晰：

```rust
#[derive(Default)]
pub struct EditorState {
    pub content: String,
    pub cursor_pos: usize,
    pub scroll_offset: usize,
    pub selection: Option<(usize, usize)>,
    pub is_dirty: bool,  // 内容是否已修改（用于保存提示）
}

impl EditorState {
    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        self.is_dirty = true;
    }

    pub fn visible_lines(&self, height: usize) -> &str {
        // 根据 scroll_offset 返回可见内容
        let lines: Vec<&str> = self.content.lines().collect();
        // ...
        todo!()
    }
}
```

---

## 脏检测优化渲染

**默认场景**：每帧都调用 `terminal.draw()`，ratatui 的双缓冲 diff 保证只写入变化的单元格，通常性能足够。

**需要脏检测的场景**：应用大部分时间处于空闲，不需要每帧都跑渲染逻辑（如编辑器、监控工具低频更新时）。

```rust
pub struct App {
    data: Vec<Item>,
    dirty: bool,       // 全局脏标记
    last_render: Instant,
}

impl App {
    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            // 只有 dirty 或超过最大等待时间才渲染
            if self.dirty || self.last_render.elapsed() > Duration::from_millis(500) {
                terminal.draw(|f| self.draw(f))?;
                self.dirty = false;
                self.last_render = Instant::now();
            }

            if event::poll(Duration::from_millis(16))? {
                if let Some(action) = self.handle_event(event::read()?)? {
                    self.apply_action(action);
                    self.dirty = true;  // 状态变化，标记需要重绘
                }
            }

            if self.should_quit { break; }
        }
        Ok(())
    }
}
```

**细粒度脏检测**（组件级）：

```rust
bitflags::bitflags! {
    pub struct DirtyFlags: u8 {
        const HEADER   = 0b001;
        const LIST     = 0b010;
        const FOOTER   = 0b100;
        const ALL      = 0b111;
    }
}

pub struct App {
    dirty: DirtyFlags,
}

impl App {
    fn on_data_changed(&mut self) {
        self.dirty |= DirtyFlags::LIST;
    }

    fn on_mode_changed(&mut self) {
        self.dirty |= DirtyFlags::HEADER | DirtyFlags::FOOTER;
    }

    fn draw(&mut self, frame: &mut Frame) {
        // 始终重绘（ratatui diff 很高效），或者检查 dirty
        if self.dirty.contains(DirtyFlags::HEADER) {
            frame.render_widget(self.header(), header_area);
        }
        // ...
        self.dirty = DirtyFlags::empty();
    }
}
```

---

## 内部可变性

当 trait 边界要求 `&self` 但需要内部修改状态时使用：

```rust
use std::cell::{Cell, RefCell};

// Cell：适合 Copy 类型（数字、bool）
pub struct Counter {
    count: Cell<u32>,
}

impl Counter {
    pub fn increment(&self) {
        self.count.set(self.count.get() + 1);
    }
}

// RefCell：适合非 Copy 类型，运行时借用检查
pub struct CachedData {
    source: Vec<u8>,
    cache: RefCell<Option<ParsedData>>,
}

impl CachedData {
    pub fn get_parsed(&self) -> std::cell::Ref<ParsedData> {
        {
            let mut cache = self.cache.borrow_mut();
            if cache.is_none() {
                *cache = Some(parse(&self.source));
            }
        }
        std::cell::Ref::map(self.cache.borrow(), |opt| opt.as_ref().unwrap())
    }
}
```

**注意**：`RefCell` 的 borrow_mut() 如果在 borrow() 存活时调用会 panic。避免跨 await 点持有 RefCell 引用。

---

## 多线程共享状态

### Arc<Mutex<T>> 基础模式

```rust
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;  // 异步 Mutex

#[derive(Default)]
pub struct SharedState {
    pub items: Vec<Item>,
    pub is_loading: bool,
    pub error: Option<String>,
}

// 在 App 中持有
pub struct App {
    state: Arc<Mutex<SharedState>>,
}

// 后台任务中更新
let state = Arc::clone(&app.state);
tokio::spawn(async move {
    let data = fetch_data().await;

    // 关键：只在 await 完成后才获取锁
    // 不要在 async 函数的 await 点持有 Mutex guard
    let mut s = state.lock().unwrap();
    match data {
        Ok(items) => {
            s.items = items;
            s.is_loading = false;
        }
        Err(e) => {
            s.error = Some(e.to_string());
            s.is_loading = false;
        }
    }
});
```

### Mutex 最小化原则（Spotify-tui 教训）

```rust
// ❌ 错误：在 await 点持有锁
async fn bad_update(state: Arc<Mutex<State>>) {
    let mut guard = state.lock().unwrap();  // 持有锁
    let data = fetch_data().await;           // 在锁内 await！阻塞 UI
    guard.data = data;
}

// ✅ 正确：await 后再获取锁
async fn good_update(state: Arc<Mutex<State>>, tx: mpsc::Sender<Action>) {
    let data = fetch_data().await;  // 无锁情况下 await
    let mut guard = state.lock().unwrap();  // await 完成后再加锁
    guard.data = data;
    // 或者通过 Action channel 传递结果，完全避免跨线程锁
    drop(guard);
    tx.send(Action::DataLoaded(data)).await.ok();
}
```

### 优先用 Channel 代替共享内存

```rust
// 最干净的方案：后台任务通过 Action channel 回传，主循环统一更新状态
// 无需跨线程共享 Mutex

// 后台任务
tokio::spawn(async move {
    let result = fetch_data().await;
    tx.send(Action::DataLoaded(result.unwrap_or_default())).await.ok();
});

// 主循环
Action::DataLoaded(items) => {
    app.items = items;  // 只在主线程修改状态，无锁
}
```

---

## 状态持久化

### 会话状态（程序退出保存，下次启动恢复）

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub last_selected: usize,
    pub bookmarks: Vec<PathBuf>,
    pub column_widths: Vec<u16>,
}

impl PersistedState {
    pub fn load() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_default()
            .join("my-app/state.json");

        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = dirs::data_dir()
            .unwrap_or_default()
            .join("my-app/state.json");
        std::fs::create_dir_all(path.parent().unwrap())?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
```

## 参考链接

- https://ratatui.rs/concepts/application-patterns/
- https://deepwiki.com/ratatui/ratatui/4.3-state-management-patterns
- https://docs.rs/ratatui/latest/ratatui/widgets/struct.ListState.html
- https://docs.rs/ratatui/latest/ratatui/widgets/struct.TableState.html
- https://keliris.dev/articles/improving-spotify-tui （Mutex 最小化案例）
