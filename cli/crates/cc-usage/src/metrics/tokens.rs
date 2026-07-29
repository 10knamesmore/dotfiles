//! token 计数：四类分开存，因为四类单价不同。

use serde::{Deserialize, Serialize};

/// 一次或多次 API 调用的 token 用量。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tokens {
    /// 未命中缓存的输入
    pub input: u64,
    /// 输出（含 thinking）
    pub output: u64,
    /// 写入缓存
    pub cache_write: u64,
    /// 命中缓存读取
    pub cache_read: u64,
}

impl Tokens {
    /// 四类相加。
    #[must_use]
    pub const fn total(self) -> u64 {
        self.input
            .saturating_add(self.output)
            .saturating_add(self.cache_write)
            .saturating_add(self.cache_read)
    }

    /// 逐字段累加。
    pub const fn add(&mut self, other: Self) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cache_write = self.cache_write.saturating_add(other.cache_write);
        self.cache_read = self.cache_read.saturating_add(other.cache_read);
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

    fn sample(input: u64, output: u64, cache_write: u64, cache_read: u64) -> Tokens {
        Tokens {
            input,
            output,
            cache_write,
            cache_read,
        }
    }

    #[test]
    fn total_sums_all_four() {
        assert_eq!(sample(1, 2, 4, 8).total(), 15);
    }

    #[test]
    fn add_accumulates_field_wise() {
        let mut acc = sample(1, 2, 3, 4);
        acc.add(sample(10, 20, 30, 40));
        assert_eq!(acc, sample(11, 22, 33, 44));
    }
}
