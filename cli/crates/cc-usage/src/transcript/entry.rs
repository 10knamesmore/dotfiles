//! JSONL 单行 → 闭合事件。
//!
//! transcript 的关键领域事实：**一条 API 响应会按 content block 拆成多行**，各行共享
//! `message.id`，`usage` 也重复（个别行是流式中途值，末行才是终值）。于是：
//!
//! - `usage` 必须按 `message.id` 去重、取末条，否则一条消息按块数翻几倍；
//! - `tool_use` 反而要**逐行**数——每行只带一个块，按 id 去重会漏掉工具调用。
//!
//! 两个键（`message.id` 与行 `uuid`）在 `/compact`、fork 的**跨文件复制**中都保持不变，
//! 去重因此必须用它们、不能用 `sessionId`（复制时会被改写成新会话 id）。

use serde::Deserialize;

use crate::clock::{self, Day};
use crate::metrics::Edits;
use crate::metrics::tokens::Tokens;

/// transcript 里对用量有贡献的一行；其余（user 文本 / attachment / 快照）丢弃。
#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    /// 一次 API 响应的用量与其中的工具块
    Message(Message),
    /// Edit/Write 写盘后的 diff
    Patch(Patch),
}

/// 一次 API 响应。
#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    /// 本行的 `uuid`（工具调用按它去重）
    pub uuid: String,
    /// `message.id`：同一条消息的多行共享它（用量按它去重）
    pub id: String,
    /// 本地日期
    pub day: Day,
    /// model id
    pub model: String,
    /// 本行报的 usage
    pub tokens: Tokens,
    /// 本行携带的 `tool_use` 块名
    pub tools: Vec<String>,
}

/// 一次文件改动。
#[derive(Debug, PartialEq, Eq)]
pub struct Patch {
    /// 本行的 `uuid`（按它去重）
    pub uuid: String,
    /// 本地日期
    pub day: Day,
    /// 增删行
    pub edits: Edits,
}

/// 解析一行；不关心的行返回 `None`。
#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    // 子串预筛只是给 patch 分支省一次解析；筛出来的是超集，筛不中照样往下走消息分支。
    if line.contains("structuredPatch")
        && let Some(patch) = parse_patch(line)
    {
        return Some(Entry::Patch(patch));
    }
    parse_message(line).map(Entry::Message)
}

/// assistant 行：`type` 为 `assistant` 且带 `message.usage` 才算。
fn parse_message(line: &str) -> Option<Message> {
    let raw: RawMessageLine = serde_json::from_str(line).ok()?;
    if raw.kind.as_deref() != Some("assistant") {
        return None;
    }
    let message = raw.message?;
    let usage = message.usage?;
    Some(Message {
        uuid: raw.uuid?,
        id: message.id?,
        day: clock::day_of(raw.timestamp.as_deref()?)?,
        model: message.model.unwrap_or_default(),
        tokens: Tokens {
            input: usage.input_tokens,
            output: usage.output_tokens,
            cache_write: usage.cache_creation_input_tokens,
            cache_read: usage.cache_read_input_tokens,
        },
        tools: message
            .content
            .into_iter()
            .filter(|block| block.kind.as_deref() == Some("tool_use"))
            .filter_map(|block| block.name)
            .collect(),
    })
}

/// 工具结果行：`toolUseResult.structuredPatch` 的 `+`/`-` 行即改动量。
///
/// 实测与 Claude Code 自己报的 `cost.total_lines_added/removed` 逐行对得上。
fn parse_patch(line: &str) -> Option<Patch> {
    let raw: RawPatchLine = serde_json::from_str(line).ok()?;
    let mut edits = Edits::default();
    for hunk in raw.tool_use_result?.structured_patch {
        for text in hunk.lines {
            match text.as_bytes().first() {
                Some(b'+') => edits.added = edits.added.saturating_add(1),
                Some(b'-') => edits.removed = edits.removed.saturating_add(1),
                _ => {}
            }
        }
    }
    Some(Patch {
        uuid: raw.uuid?,
        day: clock::day_of(raw.timestamp.as_deref()?)?,
        edits,
    })
}

/// assistant 行需要的字段。
#[derive(Deserialize)]
struct RawMessageLine {
    /// 行类型
    #[serde(rename = "type")]
    kind: Option<String>,
    /// UTC 时间戳
    timestamp: Option<String>,
    /// 行 id：跨文件复制时保持不变
    uuid: Option<String>,
    /// API 响应体
    message: Option<RawMessage>,
}

/// API 响应体需要的字段。
#[derive(Deserialize)]
struct RawMessage {
    /// 消息 id
    id: Option<String>,
    /// model id
    model: Option<String>,
    /// token 用量
    usage: Option<RawUsage>,
    /// content block（每行通常只有一个）
    #[serde(default)]
    content: Vec<RawBlock>,
}

/// usage 里要的四个计数。
#[derive(Deserialize)]
struct RawUsage {
    /// 未命中缓存的输入
    #[serde(default)]
    input_tokens: u64,
    /// 输出
    #[serde(default)]
    output_tokens: u64,
    /// 写缓存
    #[serde(default)]
    cache_creation_input_tokens: u64,
    /// 读缓存
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// content block 里要的字段。
#[derive(Deserialize)]
struct RawBlock {
    /// 块类型
    #[serde(rename = "type")]
    kind: Option<String>,
    /// 工具名（`tool_use` 块才有）
    name: Option<String>,
}

/// 工具结果行需要的字段。
#[derive(Deserialize)]
struct RawPatchLine {
    /// UTC 时间戳
    timestamp: Option<String>,
    /// 行 id
    uuid: Option<String>,
    /// 工具结果
    #[serde(rename = "toolUseResult")]
    tool_use_result: Option<RawToolResult>,
}

/// 工具结果里的 diff。
#[derive(Deserialize)]
struct RawToolResult {
    /// diff hunk 列表
    #[serde(rename = "structuredPatch", default)]
    structured_patch: Vec<RawHunk>,
}

/// 一个 diff hunk。
#[derive(Deserialize)]
struct RawHunk {
    /// 带 `+`/`-`/空格前缀的行
    #[serde(default)]
    lines: Vec<String>,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::min_ident_chars,
        clippy::missing_docs_in_private_items
    )]
    use super::*;

    const ASSISTANT: &str = r#"{"type":"assistant","uuid":"uuid-a","timestamp":"2026-07-29T01:00:00.000Z","message":{"id":"msg_1","model":"claude-sonnet-5","usage":{"input_tokens":3,"output_tokens":40,"cache_creation_input_tokens":500,"cache_read_input_tokens":9000},"content":[{"type":"tool_use","name":"Bash"}]}}"#;
    const PATCH: &str = r#"{"type":"user","uuid":"uuid-p","timestamp":"2026-07-29T01:00:01.000Z","toolUseResult":{"filePath":"/tmp/a","structuredPatch":[{"oldStart":1,"oldLines":2,"newStart":1,"newLines":3,"lines":[" keep","+added","-gone","+added2"]}]}}"#;

    #[test]
    fn parses_assistant_usage_and_tools() {
        let Some(Entry::Message(message)) = parse(ASSISTANT) else {
            panic!("应解析成消息");
        };
        assert_eq!(message.uuid, "uuid-a");
        assert_eq!(message.id, "msg_1");
        assert_eq!(message.model, "claude-sonnet-5");
        assert_eq!(message.tokens.cache_read, 9000);
        assert_eq!(message.tools, vec!["Bash".to_owned()]);
    }

    #[test]
    fn parses_structured_patch_line_counts() {
        let Some(Entry::Patch(patch)) = parse(PATCH) else {
            panic!("应解析成 patch");
        };
        assert_eq!(patch.edits.added, 2);
        assert_eq!(patch.edits.removed, 1);
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert!(parse(r#"{"type":"user","message":{"content":"纯文本"}}"#).is_none());
        assert!(parse(r#"{"type":"attachment","timestamp":"2026-07-29T01:00:00.000Z"}"#).is_none());
        assert!(parse("not json").is_none());
        assert!(parse("").is_none());
    }

    /// user 行的 content 是字符串而非块数组，别让它把 patch 分支带崩。
    #[test]
    fn patch_line_with_string_content_still_parses() {
        let line = r#"{"type":"user","uuid":"uuid-q","timestamp":"2026-07-29T01:00:01.000Z","message":{"content":"文本"},"toolUseResult":{"structuredPatch":[{"lines":["+one"]}]}}"#;
        let Some(Entry::Patch(patch)) = parse(line) else {
            panic!("应解析成 patch");
        };
        assert_eq!(patch.edits.added, 1);
    }

    #[test]
    fn assistant_without_timestamp_is_dropped() {
        let line = r#"{"type":"assistant","message":{"id":"m","usage":{"output_tokens":1}}}"#;
        assert!(parse(line).is_none());
    }
}
