# 事件处理机制

## 目录
- [同步 Poll 模式](#同步-poll-模式)
- [异步 EventStream 模式](#异步-eventstream-模式)
- [Tick / Render 定时器分离](#tick--render-定时器分离)
- [键盘事件处理](#键盘事件处理)
- [鼠标事件处理](#鼠标事件处理)
- [Resize 事件](#resize-事件)
- [多源事件 Channel 架构](#多源事件-channel-架构)

---

## 同步 Poll 模式

适合简单应用，无异步依赖。

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

fn run_event_loop(terminal: &mut Terminal<impl Backend>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        // 只处理按下，忽略 Release / Repeat
                        app.handle_key(key)?;
                    }
                }
                Event::Mouse(mouse) => app.handle_mouse(mouse)?,
                Event::Resize(w, h) => app.handle_resize(w, h),
                Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
```

**注意**：必须过滤 `KeyEventKind::Press`，crossterm 默认会同时发送 Press/Release/Repeat 三种事件，不过滤会导致事件触发三次。

---

## 异步 EventStream 模式

crossterm 0.29+ 提供 `EventStream`，配合 tokio 实现完全异步。

```toml
crossterm = { version = "0.28", features = ["event-stream"] }
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

```rust
use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.enter()?;

    let mut app = App::new();
    let mut events = EventStream::new();
    let mut tick = interval(Duration::from_millis(250));
    let mut render = interval(Duration::from_millis(16)); // 60fps

    loop {
        tokio::select! {
            _ = tick.tick() => {
                app.on_tick();
            }
            _ = render.tick() => {
                tui.draw(|f| app.draw(f))?;
            }
            Some(Ok(event)) = events.next() => {
                app.handle_event(event).await?;
            }
        }

        if app.should_quit {
            break;
        }
    }

    tui.exit()?;
    Ok(())
}
```

**`tokio::select!` 语义**：并发等待多个 Future，哪个先就绪就处理哪个，其他继续等待（不取消）。

---

## Tick / Render 定时器分离

将 tick（逻辑更新）和 render（渲染）解耦，可以独立控制频率：

```rust
// 逻辑频率：4 Hz（250ms），用于动画、定时数据拉取
let mut tick = interval(Duration::from_millis(250));

// 渲染频率：60 Hz（16ms），用于流畅 UI
let mut render = interval(Duration::from_millis(16));

// 或者更保守：30 Hz（33ms）节省 CPU
let mut render = interval(Duration::from_millis(33));
```

**实践**：大多数 TUI 应用 30fps 就已足够流畅，不必追求 60fps。

---

## 键盘事件处理

### 按键匹配模式

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn handle_key(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    // 确保只处理 Press 事件
    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }

    match (key.modifiers, key.code) {
        // Ctrl 组合键
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => Ok(Some(Action::Quit)),
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => Ok(Some(Action::Quit)),

        // 功能键
        (_, KeyCode::Char('q')) => Ok(Some(Action::Quit)),
        (_, KeyCode::Esc) => Ok(Some(Action::GoBack)),

        // 导航
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => Ok(Some(Action::NavigateUp)),
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => Ok(Some(Action::NavigateDown)),
        (_, KeyCode::Left) | (_, KeyCode::Char('h')) => Ok(Some(Action::NavigateLeft)),
        (_, KeyCode::Right) | (_, KeyCode::Char('l')) => Ok(Some(Action::NavigateRight)),

        // 确认
        (_, KeyCode::Enter) => Ok(Some(Action::Select)),
        (_, KeyCode::Char(' ')) => Ok(Some(Action::Toggle)),

        // 功能键
        (_, KeyCode::F(1)) => Ok(Some(Action::Help)),
        (_, KeyCode::Tab) => Ok(Some(Action::NextFocus)),
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Ok(Some(Action::PrevFocus)),

        _ => Ok(None),
    }
}
```

### 模式感知键位

不同模式下同一键有不同含义（类 vim）：

```rust
fn handle_key(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    match self.mode {
        Mode::Normal => self.handle_normal_key(key),
        Mode::Insert => self.handle_insert_key(key),
        Mode::Command => self.handle_command_key(key),
    }
}

fn handle_insert_key(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    match key.code {
        KeyCode::Esc => {
            self.mode = Mode::Normal;
            Ok(None)
        }
        KeyCode::Char(c) => {
            self.input_buffer.push(c);
            Ok(None)
        }
        KeyCode::Backspace => {
            self.input_buffer.pop();
            Ok(None)
        }
        KeyCode::Enter => {
            let input = std::mem::take(&mut self.input_buffer);
            self.mode = Mode::Normal;
            Ok(Some(Action::Submit(input)))
        }
        _ => Ok(None),
    }
}
```

---

## 鼠标事件处理

```rust
// Cargo.toml: crossterm = { version = "0.28", features = ["event-stream"] }
// 启用鼠标捕获
crossterm::execute!(stdout, EnableMouseCapture)?;

use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};

fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // 检查点击位置是否落在某个区域内
            let pos = (mouse.column, mouse.row);
            if self.list_area.contains(pos.into()) {
                let row = mouse.row - self.list_area.y;
                return Ok(Some(Action::SelectIndex(row as usize)));
            }
        }
        MouseEventKind::ScrollDown => return Ok(Some(Action::ScrollDown)),
        MouseEventKind::ScrollUp => return Ok(Some(Action::ScrollUp)),
        _ => {}
    }
    Ok(None)
}
```

**`Rect::contains`**：使用 ratatui 的 `Rect` 判断坐标是否在区域内：

```rust
// area 是渲染时记录的 Rect
if area.contains(Position { x: mouse.column, y: mouse.row }) {
    // 在区域内
}
```

---

## Resize 事件

ratatui 的 `terminal.draw()` 每次自动查询当前终端尺寸，双缓冲也会自动调整大小，**通常不需要手动处理 Resize**。

但如果应用需要根据尺寸做特殊布局决策：

```rust
Event::Resize(width, height) => {
    // terminal.draw() 会自动适应新尺寸
    // 只在需要重新计算固定位置/预算时手动处理
    app.on_resize(width, height);
}

// App 中
fn on_resize(&mut self, width: u16, height: u16) {
    self.terminal_size = (width, height);
    // 可能需要重新分页/截断数据等
}
```

---

## 多源事件 Channel 架构

适合有后台任务的复杂应用：

```rust
#[derive(Debug)]
pub enum AppEvent {
    Terminal(crossterm::event::Event),
    Tick,
    BackgroundResult(BackgroundData),
    NetworkResponse(Response),
}

// 事件聚合器
async fn event_loop(tx: mpsc::Sender<AppEvent>) {
    let mut terminal_events = EventStream::new();
    let mut tick = interval(Duration::from_millis(250));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let _ = tx.send(AppEvent::Tick).await;
            }
            Some(Ok(event)) = terminal_events.next() => {
                let _ = tx.send(AppEvent::Terminal(event)).await;
            }
        }
    }
}

// 后台任务也通过同一 channel 回传
tokio::spawn(async move {
    let data = fetch_data().await?;
    tx.send(AppEvent::NetworkResponse(data)).await?;
    Ok::<_, Error>(())
});

// 主循环统一消费
while let Some(event) = rx.recv().await {
    match event {
        AppEvent::Terminal(e) => app.handle_terminal_event(e)?,
        AppEvent::Tick => app.on_tick(),
        AppEvent::NetworkResponse(r) => app.update_data(r),
        AppEvent::BackgroundResult(d) => app.process_background(d),
    }
    terminal.draw(|f| app.draw(f))?;
}
```

## 参考链接

- https://ratatui.rs/concepts/event-handling/
- https://ratatui.rs/tutorials/counter-async-app/async-event-stream/
- https://ratatui.rs/tutorials/counter-async-app/full-async-events/
- https://ratatui.rs/recipes/apps/terminal-and-event-handler/
- https://docs.rs/crossterm/latest/crossterm/event/struct.EventStream.html
