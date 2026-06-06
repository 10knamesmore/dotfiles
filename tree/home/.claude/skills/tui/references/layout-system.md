# 布局系统

## 目录
- [Constraint 类型](#constraint-类型)
- [Layout 基础用法](#layout-基础用法)
- [嵌套布局](#嵌套布局)
- [Flex 空间分配](#flex-空间分配)
- [布局缓存](#布局缓存)
- [常用布局模式](#常用布局模式)

---

## Constraint 类型

| Constraint | 含义 | 优先级 |
|-----------|------|--------|
| `Length(n)` | 固定 n 个字符/行 | 最高 |
| `Max(n)` | 最多 n 个字符/行 | 高 |
| `Min(n)` | 至少 n 个字符/行 | 低 |
| `Percentage(p)` | 占父区域 p% | 中 |
| `Ratio(a, b)` | a/b 的比例 | 中 |
| `Fill(w)` | 按权重 w 填充剩余空间 | 最低 |

**优先级规则**：`Length > Max > Min > Percentage/Ratio > Fill`

高优先级约束先满足，剩余空间再分配给低优先级。

```rust
use ratatui::layout::{Constraint, Direction, Layout};

// 经典三段布局：固定头/尾，中间自适应
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3),   // 顶部导航栏：固定 3 行
        Constraint::Min(1),      // 主内容：最少 1 行，撑满剩余
        Constraint::Length(1),   // 状态栏：固定 1 行
    ])
    .split(frame.area());

// chunks[0] = 顶部, chunks[1] = 中间, chunks[2] = 底部
```

---

## Layout 基础用法

```rust
// 水平分割：左侧固定宽度 + 右侧自适应
let columns = Layout::horizontal([
    Constraint::Length(20),   // 侧边栏 20 列
    Constraint::Min(0),       // 主区域撑满
])
.split(frame.area());

// 等比分割：两列各 50%
let halves = Layout::horizontal([
    Constraint::Percentage(50),
    Percentage(50),
])
.split(frame.area());

// 三列等分（使用 Fill）
let thirds = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Fill(1),
    Constraint::Fill(1),
])
.split(area);

// 带边距
let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0)])
    .margin(1)  // 四边各留 1 个字符边距
    .split(frame.area());
```

### 简写方法

```rust
// v0.26+ 引入的快捷构造方法
Layout::vertical([Constraint::Length(3), Constraint::Min(0)])
Layout::horizontal([Constraint::Percentage(30), Constraint::Min(0)])
```

---

## 嵌套布局

布局可以任意嵌套，每次 `split()` 得到的是 `Vec<Rect>`，继续用其中的 Rect 做下一层 split：

```rust
// 外层：垂直三段
let outer = Layout::vertical([
    Constraint::Length(3),  // header
    Constraint::Min(0),     // body
    Constraint::Length(1),  // footer
])
.split(frame.area());

// 中间段：水平两列
let body = Layout::horizontal([
    Constraint::Percentage(30),  // 左侧边栏
    Constraint::Percentage(70),  // 右侧主内容
])
.split(outer[1]);

// 右侧主内容：再竖向分割
let content = Layout::vertical([
    Constraint::Length(5),   // 工具栏
    Constraint::Min(0),      // 内容区
])
.split(body[1]);

frame.render_widget(header, outer[0]);
frame.render_widget(sidebar, body[0]);
frame.render_widget(toolbar, content[0]);
frame.render_widget(main_content, content[1]);
frame.render_widget(footer, outer[2]);
```

---

## Flex 空间分配

`Flex` 控制当 Constraints 不能精确填满可用空间时如何分配多余空间。

```rust
use ratatui::layout::Flex;

Layout::horizontal([
    Constraint::Length(10),
    Constraint::Length(10),
    Constraint::Length(10),
])
.flex(Flex::SpaceAround)  // 元素间均匀分布空间
.split(area);
```

| Flex 值 | 行为 |
|---------|------|
| `Start`（默认） | 元素左/上对齐，多余空间在末尾 |
| `End` | 元素右/下对齐 |
| `Center` | 元素居中 |
| `SpaceBetween` | 元素间均匀分布，首尾无空间 |
| `SpaceAround` | 元素两侧均匀分布 |
| `Legacy` | tui-rs 兼容模式，多余空间给最后元素 |

**居中单个 widget**：

```rust
let popup_area = Layout::vertical([
    Constraint::Fill(1),    // 上方空间
    Constraint::Length(10), // 弹窗高度
    Constraint::Fill(1),    // 下方空间
])
.flex(Flex::Center)
.split(frame.area());

// 水平再居中
let popup = Layout::horizontal([
    Constraint::Fill(1),
    Constraint::Length(40),
    Constraint::Fill(1),
])
.flex(Flex::Center)
.split(popup_area[1])[1];
```

---

## 布局缓存

ratatui 内置 LRU 缓存，相同的 `(Rect, Layout)` 参数不会重复计算：

```toml
# 默认已启用（ratatui 默认 features 包含 layout-cache）
ratatui = { version = "0.29", features = ["layout-cache"] }
```

缓存大小默认 500 条目，可调整：

```rust
Layout::init_cache(1000);  // 在 main() 开头调用一次
```

**实践**：通常不需要手动管理缓存，框架会自动处理。只有极端性能要求时才需要调整缓存大小。

---

## 常用布局模式

### 弹窗/Modal

```rust
/// 在指定区域内创建居中弹窗
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

// 使用
let popup = centered_rect(50, 40, frame.area());
// 先渲染背景清空（可选）
frame.render_widget(Clear, popup);
frame.render_widget(my_popup_widget, popup);
```

### 带 Padding 的内容区

```rust
// 方法 1：Block 的 inner() 方法
let block = Block::bordered().padding(Padding::uniform(1));
let inner = block.inner(area);
block.render(area, frame.buffer_mut());
// 在 inner 内渲染内容

// 方法 2：Layout margin
let inner = Layout::default()
    .margin(2)
    .constraints([Constraint::Min(0)])
    .split(area)[0];
```

### 固定尺寸 Widget 居中

```rust
// ratatui 0.29+ Rect 自带居中方法
let widget_area = Rect::new(0, 0, 40, 10)
    .centered_horizontally(frame.area())
    .centered_vertically(frame.area());

// 或者
let centered = frame.area().centered(); // 居中到自身
```

### 响应终端尺寸

```rust
fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 根据终端尺寸调整布局策略
    if area.width < 80 {
        // 小屏：单列布局
        draw_compact(frame, app, area);
    } else {
        // 宽屏：双列布局
        draw_full(frame, app, area);
    }
}
```

## 参考链接

- https://ratatui.rs/concepts/layout/
- https://docs.rs/ratatui/latest/ratatui/layout/struct.Layout.html
- https://docs.rs/ratatui/latest/ratatui/layout/enum.Constraint.html
- https://docs.rs/ratatui/latest/ratatui/layout/enum.Flex.html
- https://docs.rs/ratatui/latest/ratatui/layout/struct.Rect.html
