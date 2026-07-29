//! 模型 → 计价档。
//!
//! 单价用 Anthropic 公布的标准价（USD/MTok）；cache write 记 `1.25 ×` input、
//! cache read 记 `0.1 ×` input。这套系数是对着 Claude Code 自己算的
//! `cost.total_cost_usd` 反推出来的（同一会话对到 <2%）——**别按 1h TTL 的真实
//! `2 ×` input 改**，改了「今日总额」会比状态栏上同一会话的成本还高，看着像坏了。
//!
//! 判不出档的模型不猜价：token 照记，钱单列进 unpriced（见
//! [`crate::metrics::Cost`]），别让未知模型静默按 0 元混进总额。

use super::tokens::Tokens;

/// 计价档：同档模型单价一致（`claude-opus-5` 与 `claude-opus-4-8` 同价）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Opus 系
    Opus,
    /// Sonnet 系
    Sonnet,
    /// Haiku 系
    Haiku,
    /// Fable 系
    Fable,
    /// 认不出的 model id（含 `<synthetic>` 这类占位）
    Unknown,
}

/// 单价，USD / MTok。
#[derive(Debug, Clone, Copy)]
struct Rate {
    /// 输入单价
    input: f64,
    /// 输出单价
    output: f64,
}

/// cache write 相对 input 的倍率。
const CACHE_WRITE_MULT: f64 = 1.25;
/// cache read 相对 input 的倍率。
const CACHE_READ_MULT: f64 = 0.1;
/// 单价分母：每百万 token。
const PER_MTOK: f64 = 1_000_000.0;

impl Tier {
    /// 按 model id 判档；带日期后缀的 id（`claude-haiku-4-5-20251001`）也认。
    #[must_use]
    pub fn of(model: &str) -> Self {
        if model.contains("opus") {
            Self::Opus
        } else if model.contains("sonnet") {
            Self::Sonnet
        } else if model.contains("haiku") {
            Self::Haiku
        } else if model.contains("fable") {
            Self::Fable
        } else {
            Self::Unknown
        }
    }

    /// 本档单价；[`Tier::Unknown`] 无价。
    fn rate(self) -> Option<Rate> {
        match self {
            Self::Opus => Some(Rate {
                input: 5.0,
                output: 25.0,
            }),
            Self::Sonnet => Some(Rate {
                input: 3.0,
                output: 15.0,
            }),
            Self::Haiku => Some(Rate {
                input: 1.0,
                output: 5.0,
            }),
            Self::Fable => Some(Rate {
                input: 10.0,
                output: 50.0,
            }),
            Self::Unknown => None,
        }
    }

    /// 折算成本（USD）；判不出档返回 `None`。
    #[must_use]
    pub fn cost_usd(self, tokens: Tokens) -> Option<f64> {
        let rate = self.rate()?;
        let billed = tokens.input as f64 * rate.input
            + tokens.output as f64 * rate.output
            + tokens.cache_write as f64 * rate.input * CACHE_WRITE_MULT
            + tokens.cache_read as f64 * rate.input * CACHE_READ_MULT;
        Some(billed / PER_MTOK)
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
    fn tier_of_covers_dated_ids() {
        assert_eq!(Tier::of("claude-opus-5"), Tier::Opus);
        assert_eq!(Tier::of("claude-opus-4-8"), Tier::Opus);
        assert_eq!(Tier::of("claude-sonnet-5"), Tier::Sonnet);
        assert_eq!(Tier::of("claude-haiku-4-5-20251001"), Tier::Haiku);
        assert_eq!(Tier::of("claude-fable-5"), Tier::Fable);
        assert_eq!(Tier::of("<synthetic>"), Tier::Unknown);
    }

    #[test]
    fn unknown_tier_has_no_price() {
        assert!(Tier::Unknown.cost_usd(Tokens::default()).is_none());
    }

    #[test]
    fn cache_tokens_are_discounted_against_input() {
        let write_only = Tokens {
            cache_write: 1_000_000,
            ..Tokens::default()
        };
        let read_only = Tokens {
            cache_read: 1_000_000,
            ..Tokens::default()
        };
        assert!((Tier::Sonnet.cost_usd(write_only).unwrap() - 3.75).abs() < 1e-9);
        assert!((Tier::Sonnet.cost_usd(read_only).unwrap() - 0.30).abs() < 1e-9);
    }

    /// 生产数据回归：本会话实测的按模型 token，对着 Claude Code 报的
    /// `cost.total_cost_usd`（$11.15）应落在 ±5% 内。系数被改动会在这里炸。
    #[test]
    fn matches_claude_code_session_cost() {
        let rows = [
            (
                Tier::Haiku,
                Tokens {
                    input: 118,
                    output: 5_062,
                    cache_write: 134_320,
                    cache_read: 316_745,
                },
            ),
            (
                Tier::Opus,
                Tokens {
                    input: 43,
                    output: 27_948,
                    cache_write: 90_009,
                    cache_read: 1_881_400,
                },
            ),
            (
                Tier::Sonnet,
                Tokens {
                    input: 112,
                    output: 79_666,
                    cache_write: 481_046,
                    cache_read: 19_631_757,
                },
            ),
        ];
        let total: f64 = rows
            .iter()
            .filter_map(|(tier, tokens)| tier.cost_usd(*tokens))
            .sum();
        let reported = 11.1546334;
        assert!(
            (total - reported).abs() / reported < 0.05,
            "算出 {total} 与 Claude Code 的 {reported} 差得太远"
        );
    }
}
