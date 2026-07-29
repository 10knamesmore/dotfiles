//! 去重账本：按跨文件稳定的键攒条目，最后折成 [`Metrics`]。
//!
//! **不能按文件各自累加指标再相加**：Claude Code 在 `/compact`、fork 出 background job
//! 等场景会把上游会话的行**原样复制**进新 transcript——`uuid` 与 `message.id` 保持不变，
//! 只有 `sessionId` 被改写。实测一天里 100 条消息同时存在于两个文件，按文件累加多算 37%。
//!
//! 两把键在复制中都不变，所以拿它们做全局去重：
//!
//! - API 用量按 `message.id`——一条响应按 content block 拆成多行、共享同一个 id
//! - 工具调用 / 文件改动按行 `uuid`——每行一个块，各有各的 uuid

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::tokens::Tokens;
use super::{Edits, Metrics};

/// 一条 API 响应的用量。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UsageEntry {
    /// model id
    pub model: String,
    /// 该响应的 token 用量
    pub tokens: Tokens,
}

/// 一行带来的活动。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityEntry {
    /// 该行里的 `tool_use` 块名
    pub tools: Vec<String>,
    /// 该行落盘的改动行
    pub edits: Edits,
}

impl ActivityEntry {
    /// 没有工具也没有改动的行不必进账本（thinking/text 行占绝大多数）。
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.tools.is_empty() && self.edits.added == 0 && self.edits.removed == 0
    }
}

/// 某一天的去重账本。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Ledger {
    /// message.id → 用量
    pub usage: BTreeMap<String, UsageEntry>,
    /// 行 uuid → 活动
    pub activity: BTreeMap<String, ActivityEntry>,
}

impl Ledger {
    /// 记一条响应的用量。
    ///
    /// 同 id 重复时取 token 更多的那份：一条响应拆成的多行里，靠前的行可能是流式中途值
    /// （实测出现过 `output_tokens` 先 1 后 309），末行才是终值。
    pub fn record_usage(&mut self, id: String, entry: UsageEntry) {
        match self.usage.get(&id) {
            Some(existing) if existing.tokens.total() >= entry.tokens.total() => {}
            _ => {
                self.usage.insert(id, entry);
            }
        }
    }

    /// 记一行的活动；空行不入账。
    pub fn record_activity(&mut self, uuid: String, entry: ActivityEntry) {
        if !entry.is_noop() {
            self.activity.insert(uuid, entry);
        }
    }

    /// 并入另一份账本（跨 transcript 汇总时按键去重）。
    pub fn merge(&mut self, other: &Self) {
        for (id, entry) in &other.usage {
            self.record_usage(id.clone(), entry.clone());
        }
        for (uuid, entry) in &other.activity {
            self.activity.insert(uuid.clone(), entry.clone());
        }
    }

    /// 折成指标。
    #[must_use]
    pub fn fold(&self) -> Metrics {
        let mut metrics = Metrics::default();
        for entry in self.usage.values() {
            metrics.add_usage(&entry.model, entry.tokens);
            metrics.messages = metrics.messages.saturating_add(1);
        }
        for entry in self.activity.values() {
            for tool in &entry.tools {
                metrics.add_tool(tool);
            }
            metrics.add_edits(entry.edits);
        }
        metrics
    }

    /// 空账本（该 transcript 那天没贡献）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.usage.is_empty() && self.activity.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::min_ident_chars,
        clippy::missing_docs_in_private_items
    )]
    use super::*;

    fn usage(output: u64) -> UsageEntry {
        UsageEntry {
            model: "claude-sonnet-5".to_owned(),
            tokens: Tokens {
                output,
                ..Tokens::default()
            },
        }
    }

    fn tool(name: &str) -> ActivityEntry {
        ActivityEntry {
            tools: vec![name.to_owned()],
            edits: Edits::default(),
        }
    }

    #[test]
    fn same_message_id_counts_once_keeping_the_larger_usage() {
        let mut ledger = Ledger::default();
        ledger.record_usage("msg_a".to_owned(), usage(1));
        ledger.record_usage("msg_a".to_owned(), usage(309));
        ledger.record_usage("msg_a".to_owned(), usage(309));
        let metrics = ledger.fold();
        assert_eq!(metrics.messages, 1);
        assert_eq!(metrics.tokens.output, 309);
    }

    #[test]
    fn earlier_partial_usage_never_overwrites_the_final_one() {
        let mut ledger = Ledger::default();
        ledger.record_usage("msg_a".to_owned(), usage(309));
        ledger.record_usage("msg_a".to_owned(), usage(1));
        assert_eq!(ledger.fold().tokens.output, 309);
    }

    /// 两个 transcript 复制了同一批行时，合并后不能翻倍。
    #[test]
    fn merging_copied_ledgers_does_not_double_count() {
        let mut original = Ledger::default();
        original.record_usage("msg_a".to_owned(), usage(100));
        original.record_activity("uuid-1".to_owned(), tool("Bash"));

        let copy = original.clone();
        original.merge(&copy);

        let metrics = original.fold();
        assert_eq!(metrics.messages, 1);
        assert_eq!(metrics.tokens.output, 100);
        assert_eq!(metrics.tools["Bash"], 1);
    }

    #[test]
    fn distinct_keys_accumulate() {
        let mut ledger = Ledger::default();
        ledger.record_usage("msg_a".to_owned(), usage(10));
        ledger.record_usage("msg_b".to_owned(), usage(20));
        ledger.record_activity("uuid-1".to_owned(), tool("Bash"));
        ledger.record_activity("uuid-2".to_owned(), tool("Bash"));
        let metrics = ledger.fold();
        assert_eq!(metrics.messages, 2);
        assert_eq!(metrics.tokens.output, 30);
        assert_eq!(metrics.tools["Bash"], 2);
    }

    #[test]
    fn noop_lines_stay_out_of_the_ledger() {
        let mut ledger = Ledger::default();
        ledger.record_activity("uuid-1".to_owned(), ActivityEntry::default());
        assert!(ledger.activity.is_empty());
        assert!(ledger.is_empty());
    }
}
