//! Claude Code 用量统计引擎：从 transcript 增量算出「本地某一天」的跨 session 用量。
//!
//! 数据源只认 transcript（`~/.claude/projects/**/*.jsonl`）。statusline 自己那份
//! `context_window.current_usage` 不能当累加源——它按 300ms 防抖刷新，窗口内的中间
//! 状态是**丢帧而非延迟**，实测累加出来的 token 比 transcript 真值少一成以上。
//!
//! 分层：[`transcript`] 增量读 JSONL 抽闭合事件 → [`metrics`] 聚合与计价 →
//! [`store`] 按天落盘 → [`report`] 对外汇总。bin 只做 clap 分发与 stdout JSON。

pub mod clock;
pub mod metrics;
pub mod report;
pub mod store;
pub mod transcript;
