# Rust 文档注释规范

## 基本形式

- 使用 `///` 为条目（函数、结构体、枚举、trait、impl、模块中的公有项）编写文档注释。
- 使用 `//!` 为模块或 crate 编写顶层文档注释（通常放在 `mod.rs` 或文件顶部）。
- 文档注释使用 Markdown。

## 放置位置

- 文档注释紧贴在被注释条目上方。
- 不在 `impl` 内重复说明已由类型级文档覆盖的内容，除非该方法具有独特行为。

## 内容建议

- 第一行用简短、完整的句子概述用途（动词开头更清晰）。
- 需要时补充：参数、返回值、错误、panic、安全性、使用示例。

## 常用段落

- `# Examples`：给出最小可运行示例。
- `# Panics`：说明会 panic 的条件。
- `# Errors`：说明返回错误的条件。
- `# Safety`：`unsafe` 函数或方法必须说明安全前置条件。
- `# Arguments`：当参数语义复杂时使用。

## 自定义模板约定

- 当仓库采用自定义 Rust 文档模板时，优先遵循项目内既有段落名与顺序，保持生成结果一致。
- 若检测到项目使用 `# Params`、`# Return`、`# Error`、`# Notes`、`# Fields` 等标题，则不要自动切换回 `# Arguments` 或 `# Errors` 等 Rust 社区常见变体。
- 自定义模板下仍然使用 `///` 与 `//!`，只是 Markdown 段落结构按项目约定组织。

## 函数注释结构

- 第一行写一句简短摘要，说明函数做什么。
- 无参数函数可省略参数段，推荐顺序如下：
  - 摘要
  - `# Return`
  - `# Error`
  - `# Examples`
  - `# Notes`
- 有参数函数在摘要后增加 `# Params` 段，参数列表使用 ``- `name`: 描述`` 形式。
- 若函数不返回 `Result` 且不存在明显失败路径，可省略 `# Error`。
- 若函数返回值语义非常直接，也可以省略 `# Return`，但公共 API 与复杂逻辑建议保留。
- `# Examples` 中使用 ` ```rust ` 代码块，示例需与真实签名、类型名和调用方式一致。
- `# Notes` 用于补充约束、性能特征、副作用、调用时机等非主体信息；无额外信息时可省略。

## 类型注释结构

- 结构体、trait、枚举或其他类型的第一行先写摘要。
- 当类型存在需要解释的字段或关联数据时，使用 `# Fields` 段。
- `# Fields` 下的条目使用 ``- `field_name`: 描述`` 形式，描述字段职责、单位、约束或含义。
- 类型级文档可追加 `# Examples`，展示典型构造或使用方式。
- 没有值得说明的字段时，不强行生成 `# Fields`，保留摘要与必要示例即可。

## 文件与模块级注释

- 文件顶部的模块说明使用 `//!`。
- 文件级注释至少包含一行摘要；当模块职责不明显时，可追加 `# 模块说明` 段解释用途、边界与主要内容。
- 文件级注释应概括该模块提供的能力，而不是重复列出内部每个私有实现细节。

## 风格与边界

- 避免仅复述函数名或类型名。
- 公共 API 必须有文档注释；私有项按需要补充。
- 示例应与实际签名一致，避免过度复杂。
- 在同一仓库内统一使用同一组标题命名，不混用 `# Arguments`/`# Params`、`# Errors`/`# Error`。
- 当项目模板要求固定段落顺序时，补全文档时保持该顺序，不随意重排。

## Rust 代码块示例

注意自行补齐可能缺失的 ` 符号

### 函数注释示例

```rust
/// 将输入字符串解析为用户 ID。
///
/// # Params:
///   - `raw`: 待解析的原始字符串。
///
/// # Return:
///   解析成功后的用户 ID。
///
/// # Error:
///   当 `raw` 为空或包含非数字字符时，返回 `ParseUserIdError`。
///
/// # Examples:
/// ```rust
/// let user_id = parse_user_id("42")?;
/// assert_eq!(user_id, 42);
/// # Ok::<(), ParseUserIdError>(())
/// ``
///
/// # Notes:
///   该函数只接受十进制数字输入。
pub fn parse_user_id(raw: &str) -> Result<u64, ParseUserIdError> {
    raw.parse().map_err(|_| ParseUserIdError)
}
```

### 无参数函数示例

```rust
/// 返回当前构建目标使用的默认 API 地址。
///
/// # Return:
///   客户端在未配置覆盖值时使用的接口地址。
///
/// # Examples:
/// ```rust
/// assert!(default_endpoint().starts_with("https://"));
/// ``
pub fn default_endpoint() -> &'static str {
    "https://api.example.com"
}
```

### 类型注释示例

```rust
/// 保存工作进程运行时所需的配置。
///
/// # Fields:
///   - `name`: 用于日志和指标上报的工作进程名称。
///   - `retry_limit`: 任务失败后的最大重试次数。
///
/// # Examples:
/// ```rust
/// let config = WorkerConfig {
///     name: "sync-worker".to_string(),
///     retry_limit: 3,
/// };
/// assert_eq!(config.retry_limit, 3);
/// ``
pub struct WorkerConfig {
    pub name: String,
    pub retry_limit: u8,
}
```

### 文件级注释示例

```rust
//! 提供加载与校验应用配置的公共辅助能力。
//!
//! # 模块说明
//!   该模块集中处理配置解析、环境变量覆盖，以及启动流程中使用的
//!   配置校验逻辑。
```
