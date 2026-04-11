---
name: tui
description: 编写 Rust TUI 应用的最佳实践。使用 ratatui 库时、需要设计 TUI 架构/事件系统/widget/布局/状态管理/性能优化时使用。涵盖从项目结构到生产级架构的完整指南。
---

# TUI 开发助手（Rust + ratatui）

## 核心原则

**即时模式渲染**：每帧必须完整重绘所有 widget，ratatui 的双缓冲机制自动 diff 并只写入变化的终端单元格，无需手动优化这一层。

**不要过早优化**：ratatui 的 diff 算法已足够高效，终端尺寸小（通常 < 65535 个单元格），渲染本身极少成为瓶颈，优先保证架构清晰。

## 架构选型

| 场景 | 推荐架构 |
|------|--------|
| 简单工具 / 单视图 | 扁平 App struct + 直接事件处理 |
| 复杂交互 / 状态机清晰 | **Elm Architecture (TEA)** |
| 多个独立 UI 模块 | **Component Architecture** |
| 异步 I/O 密集 | **Action Pattern** (TEA 变体) |
| 中央状态 + 多视图 | Flux Architecture |

详见 `references/architecture-patterns.md`。

## Reference 导航

| 文件 | 加载时机 |
|------|--------|
| `references/architecture-patterns.md` | 设计整体架构 / 选型决策 |
| `references/project-structure.md` | 组织文件和模块 |
| `references/event-handling.md` | 实现事件循环 / 键鼠处理 |
| `references/widget-system.md` | 编写自定义 Widget / 使用内置 Widget |
| `references/layout-system.md` | 布局计算 / Constraint 使用 |
| `references/state-management.md` | 状态设计 / 脏检测优化 |
| `references/performance.md` | 帧率控制 / 异步 I/O 分离 / 高负载优化 |
| `references/error-handling.md` | 终端恢复 / panic hook / 错误展示 |

## 快速起步依赖

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
color-eyre = "0.6"
tokio = { version = "1", features = ["full"] }   # 异步应用
crossterm = { version = "0.28", features = ["event-stream"] }  # 异步事件
```

生成项目骨架：
```bash
cargo install cargo-generate
cargo generate --git https://github.com/ratatui/templates component --name my-app
```
