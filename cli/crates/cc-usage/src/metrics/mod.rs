//! 用量指标：按天累计的 token / 成本 / 改动行 / 工具调用。
//!
//! 指标本身只会加不会减，所以去重必须发生在它之前——见 [`ledger`]。

pub mod ledger;
pub mod price;
pub mod tokens;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use self::price::Tier;
use self::tokens::Tokens;

/// 代码改动行数（Edit/Write 落盘 diff 里的 `+`/`-` 行）。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Edits {
    /// 新增行
    pub added: u64,
    /// 删除行
    pub removed: u64,
}

/// 计价结果。
#[derive(Debug, Default, Clone, Copy)]
pub struct Cost {
    /// 能判档的模型折算出的美元
    pub usd: f64,
    /// 判不出档、没算进 [`Cost::usd`] 的 token 总量
    pub unpriced_tokens: u64,
}

/// 一段时间内的用量。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Metrics {
    /// 全模型合计 token
    pub tokens: Tokens,
    /// 按 model id 拆分的 token（计价与排查用）
    pub by_model: BTreeMap<String, Tokens>,
    /// 改动行
    pub edits: Edits,
    /// 各工具的调用次数
    pub tools: BTreeMap<String, u64>,
    /// 去重后的 assistant 消息数
    pub messages: u64,
}

impl Metrics {
    /// 记一次 API 用量（同时进合计与按模型两处）。
    pub fn add_usage(&mut self, model: &str, tokens: Tokens) {
        self.tokens.add(tokens);
        self.by_model
            .entry(model.to_owned())
            .or_default()
            .add(tokens);
    }

    /// 记一次工具调用。
    pub fn add_tool(&mut self, name: &str) {
        let slot = self.tools.entry(name.to_owned()).or_default();
        *slot = slot.saturating_add(1);
    }

    /// 记一次文件改动。
    pub const fn add_edits(&mut self, edits: Edits) {
        self.edits.added = self.edits.added.saturating_add(edits.added);
        self.edits.removed = self.edits.removed.saturating_add(edits.removed);
    }

    /// 折算成本：能判档的按档算钱，判不出档的 token 单列。
    #[must_use]
    pub fn cost(&self) -> Cost {
        let mut cost = Cost::default();
        for (model, tokens) in &self.by_model {
            match Tier::of(model).cost_usd(*tokens) {
                Some(usd) => cost.usd += usd,
                None => cost.unpriced_tokens = cost.unpriced_tokens.saturating_add(tokens.total()),
            }
        }
        cost
    }

    /// 有没有任何用量（汇总时判某天是否算「有贡献」）。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.total() == 0
            && self.edits.added == 0
            && self.edits.removed == 0
            && self.tools.is_empty()
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

    #[test]
    fn add_usage_feeds_both_total_and_per_model() {
        let mut metrics = Metrics::default();
        metrics.add_usage(
            "claude-sonnet-5",
            Tokens {
                output: 100,
                ..Tokens::default()
            },
        );
        metrics.add_usage(
            "claude-opus-5",
            Tokens {
                output: 50,
                ..Tokens::default()
            },
        );
        assert_eq!(metrics.tokens.output, 150);
        assert_eq!(metrics.by_model["claude-sonnet-5"].output, 100);
        assert_eq!(metrics.by_model["claude-opus-5"].output, 50);
    }

    #[test]
    fn unknown_model_tokens_are_reported_not_silently_free() {
        let mut metrics = Metrics::default();
        metrics.add_usage(
            "<synthetic>",
            Tokens {
                output: 7,
                ..Tokens::default()
            },
        );
        let cost = metrics.cost();
        assert!(cost.usd.abs() < f64::EPSILON);
        assert_eq!(cost.unpriced_tokens, 7);
    }

    #[test]
    fn empty_metrics_report_empty() {
        assert!(Metrics::default().is_empty());
        let mut metrics = Metrics::default();
        metrics.add_edits(Edits {
            added: 1,
            removed: 0,
        });
        assert!(!metrics.is_empty());
    }
}
