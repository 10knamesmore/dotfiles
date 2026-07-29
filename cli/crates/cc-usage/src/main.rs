//! cc-usage：Claude Code 用量统计（bin）。
//!
//! 子命令 = 动作，输出恒为一行 JSON。**跑在 statusline 关键路径上**：任何失败都不打印
//! 任何东西并 exit 0——状态栏少一段远好过整行崩掉或糊上错误文本。

use std::path::PathBuf;
use std::time::SystemTime;

use cc_usage::clock::{self, Day};
use cc_usage::report::DayReport;
use cc_usage::store::Store;
use cc_usage::transcript::discover;
use clap::{Parser, Subcommand};

/// 按天指标保留多少天，更早的从记录里剔掉。
const RETENTION_DAYS: i64 = 30;

/// CLI 入口定义。
#[derive(Parser)]
#[command(name = "cc-usage", about = "Claude Code 用量统计", version)]
struct Cli {
    /// 动作子命令
    #[command(subcommand)]
    command: Command,
}

/// 各动作。
#[derive(Subcommand)]
enum Command {
    /// 扫今天动过的 transcript，打印今日跨 session 汇总
    Today,
    /// 只读已有状态打印某天汇总（不扫描）
    Report {
        /// 本地日期 `YYYY-MM-DD`（缺省今天）
        #[arg(long)]
        date: Option<String>,
    },
    /// 回扫最近 N 天全部 transcript 补历史（首次可能几十秒）
    Backfill {
        /// 回溯天数
        #[arg(long, default_value_t = RETENTION_DAYS)]
        days: i64,
    },
}

fn main() {
    let cli = Cli::parse();
    let done = match cli.command {
        Command::Today => today(),
        Command::Report { date } => report(date.as_deref()),
        Command::Backfill { days } => backfill(days),
    };
    // fail-open：定不出目录 / 状态目录不可写 / 序列化失败一律静默，不污染状态栏。
    let _ = done;
}

/// 扫当天动过的 transcript → 清过期 → 打印今日汇总。
fn today() -> Option<()> {
    let store = Store::open().ok()?;
    let day = clock::today();
    scan_since(&store, clock::day_start(day));
    store.prune(clock::days_before(day, RETENTION_DAYS)?);
    emit(&store, day)
}

/// 只读状态打印某天。
fn report(date: Option<&str>) -> Option<()> {
    let day = match date {
        Some(text) => clock::parse_day(text)?,
        None => clock::today(),
    };
    emit(&Store::open().ok()?, day)
}

/// 回扫最近 `days` 天所有 transcript，然后打印今日汇总。
fn backfill(days: i64) -> Option<()> {
    let store = Store::open().ok()?;
    let day = clock::today();
    let from = clock::days_before(day, days)?;
    scan_since(&store, clock::day_start(from));
    emit(&store, day)
}

/// 扫 mtime 不早于 `since` 的 transcript（transcript 是追加写，之前没动过的文件不
/// 可能含之后的消息）。
fn scan_since(store: &Store, since: Option<SystemTime>) {
    let Some(root) = discover::projects_root() else {
        return;
    };
    let transcripts: Vec<PathBuf> = discover::list(&root, since);
    store.refresh(&transcripts);
}

/// 打印某天汇总。
fn emit(store: &Store, day: Day) -> Option<()> {
    let (metrics, sources) = store.aggregate(day);
    let line = serde_json::to_string(&DayReport::new(day, &metrics, sources)).ok()?;
    println!("{line}");
    Some(())
}
