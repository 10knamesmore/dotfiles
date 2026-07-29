//! 对外汇总结构。
//!
//! 只给数值与闭合字段，成本/token 怎么排版是前端（statusline 脚本）的事。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::clock::Day;
use crate::metrics::tokens::Tokens;
use crate::metrics::{Edits, Metrics};

/// 某一天的跨 session 汇总。
///
/// `Deserialize` 是给消费方（测试、别的脚本）读这份 JSON 用的，写侧只用 `Serialize`。
#[derive(Debug, Serialize, Deserialize)]
pub struct DayReport {
    /// 本地日期
    pub date: Day,
    /// 折算成本（USD）
    pub cost_usd: f64,
    /// 判不出计价档、没算进 [`DayReport::cost_usd`] 的 token
    pub unpriced_tokens: u64,
    /// 四类 token 明细
    pub tokens: Tokens,
    /// 四类相加
    pub tokens_total: u64,
    /// 改动行
    pub edits: Edits,
    /// 去重后的 assistant 消息数
    pub messages: u64,
    /// 有贡献的 transcript 数（含 subagent，所以会多于人眼看到的会话数）
    pub sources: u64,
    /// 按 model id 拆分的 token
    pub by_model: BTreeMap<String, Tokens>,
    /// 各工具调用次数
    pub tools: BTreeMap<String, u64>,
}

impl DayReport {
    /// 由聚合指标构造。
    #[must_use]
    pub fn new(date: Day, metrics: &Metrics, sources: u64) -> Self {
        let cost = metrics.cost();
        Self {
            date,
            cost_usd: cost.usd,
            unpriced_tokens: cost.unpriced_tokens,
            tokens: metrics.tokens,
            tokens_total: metrics.tokens.total(),
            edits: metrics.edits,
            messages: metrics.messages,
            sources,
            by_model: metrics.by_model.clone(),
            tools: metrics.tools.clone(),
        }
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
    use crate::clock;

    #[test]
    fn serialises_flat_numbers_for_the_shell() {
        let mut metrics = Metrics::default();
        metrics.add_usage(
            "claude-sonnet-5",
            Tokens {
                input: 1,
                output: 2,
                cache_write: 3,
                cache_read: 4,
            },
        );
        let day = clock::parse_day("2026-07-29").unwrap();
        let json = serde_json::to_value(DayReport::new(day, &metrics, 2)).unwrap();
        assert_eq!(json["date"], "2026-07-29");
        assert_eq!(json["tokens_total"], 10);
        assert_eq!(json["sources"], 2);
        assert!(json["cost_usd"].as_f64().unwrap() > 0.0);
    }
}
