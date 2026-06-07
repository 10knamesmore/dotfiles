# 性能优化

## 目录
- [ratatui 内置优化（双缓冲）](#ratatui-内置优化)
- [帧率控制](#帧率控制)
- [避免渲染闭包内的重计算](#避免渲染闭包内的重计算)
- [异步 I/O 分离](#异步-io-分离)
- [高负载场景：任务队列模式](#高负载场景任务队列模式)
- [tokio LocalSet 减少同步开销](#tokio-localset)
- [条件渲染](#条件渲染)
- [性能分析](#性能分析)

---

## ratatui 内置优化

**双缓冲 diff 机制**：ratatui 维护当前帧和上一帧两个 `Buffer`，每次 draw 后自动 diff，只向终端写入变化的单元格。终端最大约 65535 个单元格（256×256），diff 本身极快。

**结论**：不需要自己实现脏单元格追踪，ratatui 已经做好了。优化应集中在**应用层逻辑**，而非渲染层。

布局缓存也是内置的，`Layout::split()` 对相同参数有 LRU 缓存，重复的布局计算几乎零开销。

---

## 帧率控制

最重要的性能调节手段：分离 **逻辑 tick 频率** 和 **渲染频率**。

```rust
// 逻辑更新：4 Hz（250ms）
// - 数据轮询、动画状态步进、定时任务
let mut tick = interval(Duration::from_millis(250));

// 渲染更新：30 Hz（33ms）
// - 对绝大多数 TUI 已足够流畅
let mut render = interval(Duration::from_millis(33));

// 或 60 Hz（16ms），适合有动画的应用
let mut render = interval(Duration::from_millis(16));

tokio::select! {
    _ = tick.tick() => {
        app.tick();  // 更新数据/动画状态
    }
    _ = render.tick() => {
        terminal.draw(|f| app.draw(f))?;
    }
    // ... 事件处理
}
```

**实践经验**：
- 大多数 TUI 应用 30fps 已完全够用
- 有 spinner/进度动画时用 60fps，其他时候 30fps 节省 CPU
- 数据刷新（网络/文件）频率可以远低于渲染频率（如 1Hz 更新数据，30fps 渲染）

---

## 避免渲染闭包内的重计算

`terminal.draw()` 的闭包应该是**纯粹的渲染逻辑**，不做数据处理：

```rust
// ❌ 错误：在渲染时计算（每帧都算）
terminal.draw(|f| {
    let sorted = app.items.iter()
        .filter(|i| i.matches(&app.filter))  // 每帧重新过滤！
        .sorted_by_key(|i| i.name.clone())   // 每帧重新排序！
        .collect::<Vec<_>>();

    render_list(f, &sorted);
})?;

// ✅ 正确：数据变化时预先计算，渲染时直接使用
// 在 update/on_tick 中：
fn update_filter(&mut self) {
    self.filtered_items = self.items.iter()
        .filter(|i| i.matches(&self.filter))
        .sorted_by_key(|i| &i.name)
        .collect();
}

// 在渲染时：
terminal.draw(|f| {
    render_list(f, &app.filtered_items);  // 直接用预计算结果
})?;
```

**原则**：任何不依赖 Rect 大小的计算都应该移到渲染闭包外。

---

## 异步 I/O 分离

TUI 卡顿的最常见原因：在主线程/渲染循环里做了阻塞 I/O。

### 错误模式

```rust
// ❌ 阻塞主线程
fn handle_key(&mut self, key: KeyEvent) {
    if key.code == KeyCode::Char('r') {
        // 这会阻塞 UI 几百毫秒！
        self.items = blocking_network_call();
    }
}
```

### 正确模式：tokio::spawn + Action 回传

```rust
// ✅ 后台任务异步执行，结果通过 channel 回传
fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
    if key.code == KeyCode::Char('r') {
        let tx = self.action_tx.clone();
        self.is_loading = true;

        tokio::spawn(async move {
            let result = fetch_data().await;
            match result {
                Ok(items) => tx.send(Action::DataLoaded(items)).ok(),
                Err(e) => tx.send(Action::Error(e.to_string())).ok(),
            };
        });
    }
    Ok(())
}

// 主循环处理回传
Action::DataLoaded(items) => {
    app.items = items;
    app.is_loading = false;
}
```

### Mutex 最小化（避免 UI 等待锁）

```rust
// ❌ 在 await 点持有 Mutex
async fn bad(state: Arc<Mutex<State>>) {
    let mut guard = state.lock().unwrap();
    guard.loading = true;
    let data = network_call().await;  // 持锁 await，UI 线程无法获取锁！
    guard.data = data;
    guard.loading = false;
}

// ✅ await 结束后才加锁
async fn good(state: Arc<Mutex<State>>) {
    {
        state.lock().unwrap().loading = true;  // 快速加锁/释放
    }
    let data = network_call().await;  // 无锁情况下 await
    {
        let mut s = state.lock().unwrap();
        s.data = data;
        s.loading = false;
    }
}
```

---

## 高负载场景：任务队列模式

参考 [yazi](https://github.com/sxyazi/yazi) 的双优先级任务队列：

```rust
// 区分轻量操作和重操作
pub enum TaskPriority {
    /// Micro task：轻量、快速（元数据读取、图标加载）
    Micro,
    /// Macro task：重操作（文件拷贝、图片预览生成）
    Macro,
}

pub struct TaskScheduler {
    micro_queue: VecDeque<Box<dyn Future>>,
    macro_queue: VecDeque<Box<dyn Future>>,
    running: usize,
    max_concurrent: usize,
}

impl TaskScheduler {
    pub fn schedule(&mut self, task: impl Future, priority: TaskPriority) {
        match priority {
            TaskPriority::Micro => self.micro_queue.push_front(task),  // 优先执行
            TaskPriority::Macro => self.macro_queue.push_back(task),
        }
    }

    pub fn tick(&mut self) {
        // 优先处理 micro 任务
        while self.running < self.max_concurrent {
            if let Some(task) = self.micro_queue.pop_front()
                .or_else(|| self.macro_queue.pop_front())
            {
                tokio::spawn(task);
                self.running += 1;
            } else {
                break;
            }
        }
    }
}
```

**实际简化版**：

```rust
// 对于大多数应用，用 semaphore 限制并发就够了
let semaphore = Arc::new(tokio::sync::Semaphore::new(4));  // 最多 4 个并发任务

let sem = semaphore.clone();
tokio::spawn(async move {
    let _permit = sem.acquire().await.unwrap();
    do_heavy_work().await;
    // _permit 离开作用域自动释放
});
```

---

## tokio LocalSet

当所有任务可以在同一线程上运行时，用 `LocalSet` 避免跨线程同步开销（参考 yazi）：

```rust
// 所有 spawn 的任务都在同一线程运行，不需要 Send
tokio::task::LocalSet::new().run_until(async {
    tokio::task::spawn_local(async {
        // 可以使用 !Send 类型
        let rc = std::rc::Rc::new(42);
        do_work(rc).await;
    });
}).await;
```

**适用场景**：大量小任务，共享不可 Send 的数据结构，减少 Arc/Mutex 开销。

---

## 条件渲染

当应用大部分时间空闲时，避免每帧都调用 draw：

```rust
pub struct App {
    needs_render: bool,
}

impl App {
    fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        let mut last_render = Instant::now();
        const MAX_IDLE: Duration = Duration::from_millis(500);

        loop {
            let timeout = if self.needs_render {
                Duration::ZERO
            } else {
                MAX_IDLE.saturating_sub(last_render.elapsed())
            };

            if event::poll(timeout)? {
                self.handle_event(event::read()?)?;
                self.needs_render = true;
            }

            if self.needs_render || last_render.elapsed() > MAX_IDLE {
                terminal.draw(|f| self.draw(f))?;
                self.needs_render = false;
                last_render = Instant::now();
            }
        }
    }
}
```

---

## 性能分析

当真正遇到性能问题时（先确认是渲染还是业务逻辑的问题）：

```bash
# 火焰图分析
cargo install flamegraph
cargo flamegraph --bin my-app

# 或使用 samply（更现代）
cargo install samply
samply record cargo run --release
```

**常见性能问题排查**：

1. **高 CPU 使用率（空闲时）** → 检查事件循环是否有紧密的忙轮询（`poll(Duration::ZERO)`）
2. **卡顿/帧率不稳** → 检查主线程是否有阻塞操作（文件 I/O、网络调用）
3. **内存持续增长** → 检查是否向 Vec 无限追加（历史记录、日志）而没有截断

```rust
// 日志/历史记录的环形缓冲
const MAX_LOG_LINES: usize = 1000;
if self.log.len() >= MAX_LOG_LINES {
    self.log.drain(0..100);  // 批量删除比逐个删除快
}
```

## 参考链接

- https://ratatui.rs/concepts/rendering/under-the-hood/
- https://github.com/ratatui/ratatui/discussions/579 （性能讨论）
- https://github.com/sxyazi/yazi （高性能异步架构参考）
- https://keliris.dev/articles/improving-spotify-tui （异步 I/O 分离案例）
- https://github.com/ClementTsang/bottom （实时监控 TUI 参考）
