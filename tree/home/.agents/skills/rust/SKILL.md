---
name: rust
description: Rust 代码编写、修改、审查、调试与测试规范，并覆盖 compile-aware 验证、Rust 测试工具选型和 ratatui TUI 架构。处理 .rs 文件、Cargo crate、Rust 测试、Rust feature/bugfix 或 Rust TUI 时使用。
---

# Rust

处理任何 Rust 代码时都必须遵守：

- 不要写 `/*param_name*/` 这类实参注释，测试与示例代码也不例外。依赖 inlay hint；不透明调用点通过 enum、具名方法或 newtype 改善 API，不用注释打补丁。
- 哈希容器如无特殊理由使用 fxhash（`FxHashMap` / `FxHashSet`），不使用 std 默认的 SipHash。仅当 key 来自不可信外部输入、需要抵抗 HashDoS 时使用默认 hasher。
- 没有明确理由时使用 typed struct，不使用 `serde_json::Value` 或 `json!()` 模糊契约。只有数据本来就是任意 JSON 且只需透传等场景才允许使用 `serde_json::Value`。

按任务加载 subskill，不要一次读完全部资料：

1. 编写或修改测试、验证 Rust feature/bugfix 时，完整读取 `subskills/testing.md`。
2. 处理 ratatui、terminal UI、事件系统、widget、layout、state 或 TUI 性能时，完整读取 `subskills/tui.md`。
3. 同时涉及 TUI 测试时，同时读取 `subskills/testing.md` 和 `subskills/tui.md`。
4. subskill 指向的 `references/` 只在对应场景需要时读取。
