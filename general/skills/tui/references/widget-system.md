# Widget 系统

## 目录
- [Widget vs StatefulWidget](#widget-vs-statefulwidget)
- [WidgetRef（跨帧存储）](#widgetref)
- [自定义 Widget 实现](#自定义-widget-实现)
- [自定义 StatefulWidget](#自定义-statefulwidget)
- [Frame 渲染 API](#frame-渲染-api)
- [内置 Widget 速览](#内置-widget-速览)
- [文本与样式系统](#文本与样式系统)

---

## Widget vs StatefulWidget

| | Widget | StatefulWidget |
|---|---|---|
| **签名** | `fn render(self, area: Rect, buf: &mut Buffer)` | `fn render(self, area: Rect, buf: &mut Buffer, state: &mut S)` |
| **所有权** | 消费 self | 消费 self，借用 State |
| **状态** | 无状态，每帧重新创建 | 跨帧保存状态（选择位置、滚动偏移等） |
| **典型场景** | Paragraph、Block、Gauge | List、Table、Scrollbar |

**核心规则**：只要需要在帧间记住"选了哪个/滚到哪里"，就用 StatefulWidget + 独立的 State struct。

---

## WidgetRef

v0.26+ 新增，允许通过引用渲染，支持 `Box<dyn WidgetRef>` 动态 widget 集合：

```rust
// 对任何实现了 Widget for &T 的类型，自动提供 WidgetRef
impl Widget for &MyWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // self 是 &MyWidget
    }
}

// 使用 WidgetRef 存储异构 widget 集合
let widgets: Vec<Box<dyn WidgetRef>> = vec![
    Box::new(HeaderWidget::new()),
    Box::new(FooterWidget::new()),
];

terminal.draw(|f| {
    for (widget, area) in widgets.iter().zip(areas.iter()) {
        f.render_widget_ref(widget.as_ref(), *area);
    }
})?;
```

---

## 自定义 Widget 实现

### 最简实现

```rust
pub struct StatusBar {
    text: String,
    style: Style,
}

impl StatusBar {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 填充背景色
        buf.set_style(area, self.style);
        // 写入文本（自动截断到区域宽度）
        buf.set_string(area.x, area.y, &self.text, self.style);
    }
}

// 使用
frame.render_widget(
    StatusBar::new("Press q to quit").style(Style::new().on_dark_gray()),
    footer_area,
);
```

### 使用 Builder Pattern（推荐）

```rust
pub struct MyWidget<'a> {
    title: &'a str,
    content: Vec<&'a str>,
    block: Option<Block<'a>>,
    highlight_style: Style,
}

impl<'a> MyWidget<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            content: vec![],
            block: None,
            highlight_style: Style::new().reversed(),
        }
    }

    pub fn content(mut self, content: Vec<&'a str>) -> Self {
        self.content = content;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl Widget for MyWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 先渲染 block（边框），获取内部区域
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // 在 inner 区域渲染内容
        for (i, line) in self.content.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            buf.set_string(inner.x, inner.y + i as u16, line, Style::default());
        }
    }
}
```

---

## 自定义 StatefulWidget

### State 设计原则

- State 应只包含"视图状态"（选中索引、滚动偏移），不含业务数据
- 业务数据留在 App/Component 中，渲染时传入 widget

```rust
#[derive(Default)]
pub struct SelectableListState {
    pub selected: Option<usize>,
    pub scroll_offset: usize,
}

impl SelectableListState {
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    pub fn select_next(&mut self, len: usize) {
        self.selected = Some(match self.selected {
            None => 0,
            Some(i) => (i + 1).min(len.saturating_sub(1)),
        });
    }

    pub fn select_prev(&mut self) {
        self.selected = Some(match self.selected {
            None => 0,
            Some(0) => 0,
            Some(i) => i - 1,
        });
    }
}

pub struct SelectableList<'a> {
    items: Vec<ListItem<'a>>,
    highlight_style: Style,
}

impl<'a> SelectableList<'a> {
    pub fn new(items: Vec<ListItem<'a>>) -> Self {
        Self {
            items,
            highlight_style: Style::new().bold().on_blue(),
        }
    }
}

impl StatefulWidget for SelectableList<'_> {
    type State = SelectableListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // 自动滚动：确保选中项可见
        let visible_height = area.height as usize;
        if let Some(selected) = state.selected {
            if selected < state.scroll_offset {
                state.scroll_offset = selected;
            } else if selected >= state.scroll_offset + visible_height {
                state.scroll_offset = selected - visible_height + 1;
            }
        }

        // 渲染可见项
        for (i, item) in self.items.iter()
            .skip(state.scroll_offset)
            .take(visible_height)
            .enumerate()
        {
            let row = area.y + i as u16;
            let is_selected = state.selected == Some(i + state.scroll_offset);

            let style = if is_selected {
                self.highlight_style
            } else {
                Style::default()
            };

            buf.set_style(Rect::new(area.x, row, area.width, 1), style);
            // 渲染 ListItem 内容（简化）
        }
    }
}
```

---

## Frame 渲染 API

```rust
terminal.draw(|frame| {
    // 获取整个终端区域
    let area = frame.area();

    // 渲染无状态 widget（消费所有权）
    frame.render_widget(Paragraph::new("Hello"), area);

    // 渲染有状态 widget（需要 &mut State）
    frame.render_stateful_widget(
        List::new(items),
        list_area,
        &mut app.list_state,
    );

    // 渲染引用型 widget（v0.26+）
    frame.render_widget_ref(&my_widget, area);

    // 设置光标位置（输入框等）
    frame.set_cursor_position((cursor_x, cursor_y));

    // 获取帧序号（用于动画）
    let _frame_count = frame.count();
})?;
```

**注意**：`frame.area()` 是固定值，draw 闭包执行期间终端大小不会变化，可安全多次调用。

---

## 内置 Widget 速览

### Block — 边框容器

```rust
Block::new()
    .title("Title")
    .title_bottom(Line::from("footer").right_aligned())
    .borders(Borders::ALL)
    .border_style(Style::new().blue())
    .border_type(BorderType::Rounded)
    .padding(Padding::horizontal(1))
```

### Paragraph — 文本显示

```rust
Paragraph::new(Text::from(vec![
    Line::from(vec![
        Span::raw("Normal "),
        Span::styled("Bold", Style::new().bold()),
        Span::styled(" Red", Style::new().red()),
    ]),
]))
.block(Block::bordered().title("Output"))
.alignment(Alignment::Left)
.wrap(Wrap { trim: false })
.scroll((scroll_offset, 0))  // 垂直滚动
```

### List — 可选择列表

```rust
// 渲染
frame.render_stateful_widget(
    List::new(items)
        .block(Block::bordered().title("Items"))
        .highlight_style(Style::new().bold().on_blue())
        .highlight_symbol("> ")
        .repeat_highlight_symbol(true),
    area,
    &mut list_state,  // ListState
);

// 状态操作
let mut state = ListState::default().with_selected(Some(0));
state.select_next();
state.select_previous();
state.select_first();
state.select_last();
```

### Table — 表格

```rust
let rows = data.iter().map(|d| {
    Row::new(vec![
        Cell::from(d.name.as_str()),
        Cell::from(d.value.to_string()).style(Style::new().cyan()),
    ])
});

frame.render_stateful_widget(
    Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
        .header(Row::new(["Name", "Value"]).bold().underlined())
        .block(Block::bordered())
        .highlight_style(Style::new().reversed()),
    area,
    &mut table_state,  // TableState
);
```

### Gauge / LineGauge — 进度条

```rust
// 方块进度条
Gauge::default()
    .block(Block::bordered().title("Progress"))
    .gauge_style(Style::new().green())
    .ratio(0.65)  // 0.0 - 1.0
    .label("65%")

// 线条进度条
LineGauge::default()
    .gauge_style(Style::new().blue())
    .ratio(progress)
```

### Sparkline — 迷你图表

```rust
Sparkline::default()
    .block(Block::bordered().title("CPU"))
    .data(&cpu_history)  // &[u64]
    .max(100)
    .style(Style::new().green())
```

### Chart — 折线/散点图

```rust
let datasets = vec![
    Dataset::default()
        .name("CPU")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::new().cyan())
        .data(&cpu_data),  // &[(f64, f64)]
];

Chart::new(datasets)
    .block(Block::bordered().title("Metrics"))
    .x_axis(Axis::default().bounds([0.0, 100.0]).labels(["0", "50", "100"]))
    .y_axis(Axis::default().bounds([0.0, 100.0]))
```

---

## 文本与样式系统

### 文本层级

```
Text → Line(s) → Span(s)
```

```rust
use ratatui::text::{Text, Line, Span};
use ratatui::style::{Style, Color, Modifier};

// 从字符串直接构造
let text = Text::from("simple text");
let line = Line::from("a line");
let span = Span::styled("colored", Style::new().red().bold());

// 富文本
let text = Text::from(vec![
    Line::from(vec![
        Span::raw("Status: "),
        Span::styled("OK", Style::new().green().bold()),
    ]),
    Line::from(Span::styled("Error message", Style::new().red())),
]);
```

### Style 构造

```rust
// Builder 风格（推荐）
Style::new()
    .fg(Color::Red)
    .bg(Color::Black)
    .bold()
    .italic()
    .underlined()

// 颜色支持
Color::Red           // 基本 16 色
Color::Indexed(196)  // 256 色
Color::Rgb(255, 0, 0) // 真彩色（需终端支持）
```

## 参考链接

- https://ratatui.rs/concepts/widgets/
- https://docs.rs/ratatui/latest/ratatui/widgets/trait.Widget.html
- https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html
- https://ratatui.rs/recipes/widgets/custom/
- https://docs.rs/ratatui/latest/ratatui/text/
- https://docs.rs/ratatui/latest/ratatui/style/
