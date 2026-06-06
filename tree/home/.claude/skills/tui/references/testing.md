# TUI 测试体系（Rust + ratatui）

TUI 常被认为「难测」，其实是误解。ratatui 提供 `TestBackend`，把整帧渲染进**内存缓冲**而非真终端，于是渲染、事件、状态机都能在普通 `cargo test` 里跑——**不需要 pty、不需要真终端**。

本文分两层：

- **[通用核心](#一通用核心任何-ratatui-项目零前提)**——任何 ratatui 项目都成立，零前提，可直接照搬。
- **[进阶模式](#二进阶模式按架构条件触发)**——只在你的 App 满足特定架构条件时才适用，每个模式自带「**什么时候用**」的触发判断。不要无脑套用：套错了是给简单项目强加不存在的复杂度。

> 例子里的 `Result<()>` 用你项目自己的错误类型别名（`color_eyre::Result` / `anyhow::Result` / 自定义皆可）。快照用 [`insta`](https://insta.rs)、跑测试用 [`nextest`](https://nexte.st) 是**推荐**而非必需，但下文的快照约定基于 insta。

---

## 目录
- [一、通用核心（任何 ratatui 项目，零前提）](#一通用核心任何-ratatui-项目零前提)
  - [TestBackend：渲染进内存，无需 pty](#testbackend渲染进内存无需-pty)
  - [用什么测什么](#用什么测什么)
  - [组件渲染 → 快照](#组件渲染--快照)
  - [纯逻辑 → assert_eq](#纯逻辑--assert_eq)
  - [不变量 → proptest](#不变量--proptest)
  - [驱动事件循环：喂真实 KeyEvent](#驱动事件循环喂真实-keyevent)
  - [测试放 `#[cfg(test)] mod` 内调私有](#测试放-cfgtest-mod-内调私有)
  - [快照的确定性陷阱](#快照的确定性陷阱)
- [二、进阶模式（按架构条件触发）](#二进阶模式按架构条件触发)
  - [模式 A：App 跨边界依赖外部后端/IO → 抽 trait 注 fake](#模式-aapp-跨边界依赖外部后端io--抽-trait-注-fake)
  - [模式 B：App 持有需 runtime 的副作用依赖 → runtime-free null object](#模式-bapp-持有需-runtime-的副作用依赖--runtime-free-null-object)
  - [模式 C：App 每帧从后端同步 state → 跨 tick 集成测试](#模式-capp-每帧从后端同步-state--跨-tick-集成测试)
- [CI 与 headless](#ci-与-headless)

---

# 一、通用核心（任何 ratatui 项目，零前提）

## TestBackend：渲染进内存，无需 pty

```rust
use ratatui::Terminal;
use ratatui::backend::TestBackend;

let mut terminal = Terminal::new(TestBackend::new(80, 24))?;  // 内存缓冲，不碰真终端
terminal.draw(|f| ui(f, &app))?;
let backend = terminal.backend();   // 渲染后的 Buffer；其 Display 就是「终端长什么样」
```

`TestBackend` 把 `Buffer`（每 cell 的字符 + 样式）攒在内存里，`Display` 出来是肉眼可读的整屏文本。**渲染结果是可断言的纯数据**——这是整个体系的地基。

不要为渲染断言去 spawn 真 pty（`expectrl` / `portable-pty`）：慢、flaky、依赖 `TERM`。pty 只在测「真终端 escape 序列协议」时才需要，那是极少数。

## 用什么测什么

| 被测对象 | 工具 | 理由 |
|---------|------|------|
| 渲染产物 | **insta 快照** + `TestBackend` | 「画面长这样」用期望值写断言太脆；快照一次锁整帧，布局/字段变动全抓 |
| 逻辑 / 算法 / 等价性（选择移动、翻页、时间格式化、布局数学） | `assert_eq!` | 期望值写断言里**表意最清晰**，别为统一上快照 |
| 纯函数的性质 / 不变量 | **proptest** | 「对任意输入恒成立」，随机数百用例 + 失败自动收缩到最小反例 |
| 交互 / 事件处理 | **喂真实 `KeyEvent` 驱动 handler** | 见下；这是测「按键→状态」的唯一真实路径 |

## 组件渲染 → 快照

把每个 widget 写成 `draw(frame, area, state, …)` 纯函数，测试就渲它：

```rust
#[test]
fn list_with_long_title_snapshot() -> Result<()> {
    let mut t = Terminal::new(TestBackend::new(50, 8))?;
    let state = state_with_long_title();   // fixture，别每个测试现搓
    t.draw(|f| super::draw(f, f.area(), &state))?;
    insta::assert_snapshot!(t.backend());
    Ok(())
}
```

- **fixture 收口成 helper**（`state_empty()` / `state_with_long_title()` / `state_cjk()`），复用。
- **每张快照带 `description`**（`insta::with_settings!{description => "…"}`），review 时逐张能认。
- 覆盖**边界形态**：空态、超长截断、CJK 宽字符对齐、窄/宽终端——布局 bug 高发区，快照一眼可见。

## 纯逻辑 → assert_eq

**能脱离渲染的逻辑抽成纯函数**（布局计算、选择移动、滚动 offset、文本截断、颜色/时间格式化），直接断言：

```rust
#[test]
fn selection_wraps_at_end() {
    assert_eq!(next_index(/*cur*/ 4, /*len*/ 5), 0);
}
```

抽得出纯函数是**好架构的副产品**：渲染与逻辑解耦后，逻辑这部分根本不需要 `TestBackend`。

## 不变量 → proptest

有清晰数学约束的纯函数上 proptest：

```rust
proptest! {
    /// 布局切出的每个子区域都落在父区域内，不越界。
    #[test]
    fn layout_subareas_within_parent(w in 1u16..200, h in 1u16..100) {
        let parent = Rect::new(0, 0, w, h);
        for area in compute_layout(parent) {
            prop_assert!(area.right() <= parent.right() && area.bottom() <= parent.bottom());
        }
    }
}
```

甜区：布局子区域不越界、滚动 offset 恒合法、颜色 lerp 不溢出、文本截断不切坏宽字符。失败种子（`proptest-regressions/*.txt`）**进 git** 当永久回归用例。

## 驱动事件循环：喂真实 KeyEvent

交互逻辑（按键改状态）直接构造 `crossterm` 事件喂进 handler，断言 state 变化。**最小的纯本地 App 就能测，不需要任何后端：**

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn j_moves_selection_down() {
    let mut app = App::new_with_items(5);                       // 纯本地 state
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.state.sel, 1);
    app.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::empty()));
    assert_eq!(app.state.sel, 4);
}
```

**注**：若 handler 入口是 `handle_event(Event)` 且过滤了 `KeyEventKind::Press`，用 `Event::Key(KeyEvent::new(...))`——`KeyEvent::new` 默认 `kind = Press`，正好通过。

再补一张快照锁「按完键画面对不对」：移动后 `draw` 一次 + `assert_snapshot!`。这就完整覆盖了「键 → 状态 → 渲染」，**全程无需后端、无需 runtime**。

## 测试放 `#[cfg(test)] mod` 内调私有

把这些测试写在被测模块**同文件**的 `#[cfg(test)] mod tests` 里——子模块能访问父模块私有项，于是 `handle_key` / 内部状态字段哪怕私有也能直接调，**不必为测试放宽可见性**。放到外部 `tests/` 目录就得把它们标 `pub`，污染公开 API。

## 快照的确定性陷阱

快照最大的敌人是**非确定性**——一闪一闪的假失败会逼人最终 `--accept` 一切，彻底失去价值：

- **动态内容**（版本号、时间戳、UUID、临时路径）必须 filter 归一：
  ```rust
  insta::with_settings!({ filters => vec![(r"v\d+\.\d+\.\d+", "v[VERSION]")] }, {
      insta::assert_snapshot!(t.backend());
  });
  ```
- **ratatui `Table` 的 flex 列宽会 flaky**：`Constraint::Min(n)` 有富余空间（slack）时，列宽求解**非确定**，同输入可能渲出不同宽度。insta 治不了——得在**生产侧改成确定性约束**（`Length` / `Percentage`，或让 `Min` 列吃满无 slack）。表格快照闪烁先查这里。
- **HashMap 顺序**：`with_settings!({ sort_maps => true }, …)`，或生产侧换有序容器。
- **`.snap` 进 git**，新增/改动后 `cargo insta review` **逐张人工确认**；**严禁** `INSTA_UPDATE=always` 盲接受，CI 用 `INSTA_UPDATE=no` / `--unreferenced=reject` 兜。

---

# 二、进阶模式（按架构条件触发）

以下三个模式**只在你的 App 有对应架构时才适用**。先判断「什么时候用」，不匹配就跳过——别给本地状态的简单 TUI 强加这些。

## 模式 A：App 跨边界依赖外部后端/IO → 抽 trait 注 fake

**什么时候用**：`App` 不只有本地 state，还要调**外部系统**——网络服务、数据库、子进程、独立 daemon、文件系统。这类依赖在测试里没法真起。

**做法**：让 `App` 不直接持有具体后端，而是 `Arc<dyn Backend>`（你的 `Client` / `Service` / `Repo` trait），测试注入一个**全 no-op 假实现**：

```rust
pub struct App {
    state: AppState,
    backend: Arc<dyn Backend>,   // ← trait object，不是具体网络/DB 类型
}

// 测试侧
struct FakeBackend;
impl Backend for FakeBackend {
    fn snapshot(&self) -> Snapshot { Snapshot::default() }   // 读：默认值
    fn submit(&self, _cmd: Command) {}                        // 命令：静默吞
    // … 其余照抄返默认值
}
```

> 这是「依赖倒置 / 端口-适配器」在测试上的回报：trait 是边界，生产接真后端、测试接假后端，`App` 代码一行不改。**前提是你的架构已经有这条 trait 边界**——没有的话，引入它要权衡，不是为测试而测试。

## 模式 B：App 持有需 runtime 的副作用依赖 → runtime-free null object

**什么时候用**：`App` 构造时要塞一个**需要 async runtime 才能建**的副作用组件——后台 worker、异步抓取器、轮询器（其 `new`/`spawn` 内部 `tokio::spawn`）。为了 `App::new` 就被迫把测试变成 `#[tokio::test]`，是纯噪音。

**做法**：给这类组件加一个**不依赖 runtime 的禁用态构造**（null object）：

```rust
impl Worker {
    /// 真构造：起后台 task（需 runtime）。
    pub fn spawn() -> Result<Self> { /* tokio::spawn … */ }

    /// 禁用态：不起 task、不建客户端。提交静默丢、拉取恒空。不需要 runtime。
    pub fn disabled() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();   // 无人收，send 失败本就被忽略
        Self { tx, ready: Default::default() }
    }
}
```

于是 `App::new(Arc::new(FakeBackend), Worker::disabled())` 在**普通同步 `#[test]`** 里就能构造。

> **这不是测试拐杖，是生产基建**。任何「外部子系统建不起来」的场景——headless、无网、无设备、TLS/证书失败——都该**降级空跑**而非让整个 App 起不来。真构造失败时 `warn! + disabled()` 兜底，比 `?` 冒泡拖垮整个 UI 健壮得多。测试只是顺带受益。（典型：图片/封面抓取器、音频引擎无设备时的 null backend。）

## 模式 C：App 每帧从后端同步 state → 跨 tick 集成测试

**什么时候用**：`App` 采用 **client/server 或「每帧从后端拉 snapshot 灌进本地镜像」**的架构（主循环里 `state = backend.snapshot()` 之类）。

**为什么非它不可**：这种架构有一类**单元测试和组件快照都抓不到的时序 bug**——**用户用按键改了某个 UI 状态，下一帧 tick 又从后端 snapshot 把它覆盖回去**。根因通常是「UI-local 状态」和「后端同步状态」语义被混为一谈（同名字段、却一个是用户光标、一个是后端的播放位置/进度锚点）。组件快照只渲某一瞬，纯逻辑测试不跑循环——唯有「喂键 + 跨 tick + 断言」能复现。

**做法**（组合模式 A/B 构造 App，再驱动一次完整时序）：

```rust
#[test]
fn ui_cursor_survives_backend_tick() -> Result<()> {
    let mut app = App::new(Arc::new(FakeBackend), Worker::disabled());
    app.apply_snapshot(Snapshot { items: make_items(6), ..Default::default() });

    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.state.sel, 2);          // 用户移动光标到 2

    // —— 关键：模拟一次后端 tick 同步，故意把那个「同名但语义不同」的字段设成别值。
    let snap = Snapshot { items: make_items(6), backend_cursor: 0, ..Default::default() };
    app.apply_snapshot(snap);
    assert_eq!(app.state.sel, 2, "tick 同步不该覆盖用户的 UI 光标");   // ← 回归点

    Ok(())
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::empty())));
}
```

再补一张渲染快照锁视觉解耦（如「UI 光标标记」与「后端当前项标记」落在**不同行**），证明二者彻底分离。

**根上的修复方向**：UI-local 光标应是纯客户端字段（永不被 snapshot 覆盖），后端同步只更新真正属于后端的状态。这类 bug 的预防比修复更重要——命名上就别让 UI 状态和后端状态共用一个字段。

---

## CI 与 headless

- 上面的体系（`TestBackend` + 模式 A 的 fake + 模式 B 的 null object）的**直接红利**：TUI 测试**不需要真终端 / 真后端 / 真网络 / runtime**，headless CI 里照常跑。
- 典型 CI 顺序：`fmt --check` → `clippy -D warnings`（**测试代码不豁免** lint）→ 跑测试（`INSTA_UPDATE=no` 防漏审快照）。若用 `nextest`，注意它**不跑 doctest**，需单独 `cargo test --doc` 兜。
- 快照偶发 flaky：**先怀疑 flex 列宽 slack**（见上），别靠 retry 掩盖。
