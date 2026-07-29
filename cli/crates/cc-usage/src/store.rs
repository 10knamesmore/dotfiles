//! 扫描状态落盘。
//!
//! 两块：`progress/<hash>.json` 记每个 transcript 扫到哪儿，`ledger/<日期>/<hash>.json`
//! 记它那天贡献的条目。汇总某天只读那天的目录，过期清理就是删整个日期目录。
//!
//! 账本里存的是「从文件开头累计」的**绝对值**而非增量——多个 session 的状态栏并发刷新
//! 同一个 transcript，最坏是某次写回偏旧、下次自愈，**不会重复计数**，所以不需要加锁。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clock::Day;
use crate::metrics::Metrics;
use crate::metrics::ledger::Ledger;
use crate::transcript::scan::{self, Progress};

/// 覆盖状态目录（测试与非常规安装用）。
pub const STATE_ENV: &str = "CC_USAGE_STATE_DIR";

/// 扫描进度记录。
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProgressRecord {
    /// transcript 绝对路径（文件名是路径哈希，靠这个字段回指）
    transcript: PathBuf,
    /// 扫到哪儿
    progress: Progress,
}

/// 状态目录。
pub struct Store {
    /// 状态根目录
    root: PathBuf,
}

impl Store {
    /// 按环境打开：`CC_USAGE_STATE_DIR` 优先，否则 `$XDG_CACHE_HOME`/`~/.cache` 下的
    /// `cc-usage`。目录会建好。
    pub fn open() -> io::Result<Self> {
        let root = default_root().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "定不出状态目录（HOME 未设）")
        })?;
        Self::at(root)
    }

    /// 用指定目录打开。
    pub fn at(root: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(root.join("progress"))?;
        fs::create_dir_all(root.join("ledger"))?;
        Ok(Self { root })
    }

    /// 扫这些 transcript，逐个更新状态。单个文件失败只跳过它（fail-open）。
    pub fn refresh(&self, transcripts: &[PathBuf]) {
        for path in transcripts {
            let _ = self.refresh_one(path);
        }
    }

    /// 汇总某天：把那天所有 transcript 的账本按键去重合并后折成指标。
    ///
    /// 必须先合账本再折指标——指标只会加不会减，先折就没法去掉跨文件重复的那份了。
    #[must_use]
    pub fn aggregate(&self, day: Day) -> (Metrics, u64) {
        let mut merged = Ledger::default();
        let mut sources = 0_u64;
        for path in list_json(&self.day_dir(day)) {
            let Some(ledger) = read_json::<Ledger>(&path) else {
                continue;
            };
            if !ledger.is_empty() {
                merged.merge(&ledger);
                sources = sources.saturating_add(1);
            }
        }
        (merged.fold(), sources)
    }

    /// 清理：`cutoff` 之前的日期目录整个删；transcript 已消失的进度连带其账本删。
    pub fn prune(&self, cutoff: Day) {
        let Ok(entries) = fs::read_dir(self.root.join("ledger")) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let stale = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<Day>().ok())
                .is_none_or(|day| day < cutoff);
            if stale {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
        for path in list_json(&self.root.join("progress")) {
            let gone =
                read_json::<ProgressRecord>(&path).is_none_or(|record| !record.transcript.exists());
            if gone {
                if let Some(hash) = path.file_stem().and_then(|stem| stem.to_str()) {
                    self.forget_ledgers(hash);
                }
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// 扫单个 transcript 并写回状态。
    ///
    /// 文件比进度还短说明被重写/截断过，此时必须连历史账本一起清空重扫——只把进度
    /// 归零会把旧字节算两遍。
    fn refresh_one(&self, path: &Path) -> io::Result<()> {
        let hash = hash_path(path);
        let progress_file = self.root.join("progress").join(format!("{hash}.json"));
        let mut record = read_json::<ProgressRecord>(&progress_file).unwrap_or_default();
        record.transcript = path.to_path_buf();

        let size = fs::metadata(path)?.len();
        if size < record.progress.offset {
            record.progress = Progress::default();
            self.forget_ledgers(&hash);
        }
        if size == record.progress.offset {
            return Ok(());
        }

        let mut fresh: BTreeMap<Day, Ledger> = BTreeMap::new();
        scan::advance(path, &mut record.progress, &mut fresh)?;
        for (day, ledger) in fresh {
            let file = self.day_dir(day).join(format!("{hash}.json"));
            let mut merged = read_json::<Ledger>(&file).unwrap_or_default();
            merged.merge(&ledger);
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent)?;
            }
            write_json(&file, &merged)?;
        }
        write_json(&progress_file, &record)
    }

    /// 某天的账本目录。
    fn day_dir(&self, day: Day) -> PathBuf {
        self.root.join("ledger").join(day.to_string())
    }

    /// 删掉某个 transcript 在所有日期下的账本。
    fn forget_ledgers(&self, hash: &str) {
        let Ok(entries) = fs::read_dir(self.root.join("ledger")) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let _ = fs::remove_file(entry.path().join(format!("{hash}.json")));
        }
    }
}

/// 缺省状态目录。
fn default_root() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os(STATE_ENV) {
        return Some(PathBuf::from(custom));
    }
    if let Some(cache) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(cache).join("cc-usage"));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache/cc-usage"))
}

/// transcript 路径的 FNV-1a 哈希，避免把长路径拍平成超长文件名。
fn hash_path(transcript: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in transcript.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 目录下的 `*.json`。
fn list_json(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect()
}

/// 读 JSON；不存在或坏掉都当没有。
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

/// 原子写：临时文件带 pid，免得并发刷新互相踩。
fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = serde_json::to_string(value).map_err(io::Error::other)?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
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
    use crate::clock;

    fn line(uuid: &str, id: &str, output: u64) -> String {
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","timestamp":"2026-07-29T01:00:00.000Z","message":{{"id":"{id}","model":"claude-sonnet-5","usage":{{"input_tokens":0,"output_tokens":{output},"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"content":[]}}}}"#
        )
    }

    fn day() -> Day {
        clock::day_of("2026-07-29T01:00:00.000Z").unwrap()
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        store: Store,
        transcript: PathBuf,
    }

    impl Fixture {
        fn scan(&self) {
            self.store.refresh(std::slice::from_ref(&self.transcript));
        }

        fn output(&self) -> u64 {
            self.store.aggregate(day()).0.tokens.output
        }
    }

    fn fixture(text: &str) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("session.jsonl");
        fs::write(&transcript, text).unwrap();
        let store = Store::at(dir.path().join("state")).unwrap();
        Fixture {
            _dir: dir,
            store,
            transcript,
        }
    }

    #[test]
    fn refresh_then_aggregate_sums_the_day() {
        let text = format!("{}\n{}\n", line("u1", "msg_a", 10), line("u2", "msg_b", 20));
        let fix = fixture(&text);
        fix.scan();
        let (metrics, sources) = fix.store.aggregate(day());
        assert_eq!(metrics.tokens.output, 30);
        assert_eq!(metrics.messages, 2);
        assert_eq!(sources, 1);
    }

    #[test]
    fn repeated_refresh_is_idempotent() {
        let fix = fixture(&format!("{}\n", line("u1", "msg_a", 10)));
        fix.scan();
        fix.scan();
        fix.scan();
        assert_eq!(fix.output(), 10);
    }

    /// `/compact`、fork 会把上游会话的行原样复制进新 transcript：两个文件、同一批
    /// uuid 与 message.id，汇总时必须只算一次。
    #[test]
    fn transcript_copied_into_a_second_file_is_not_double_counted() {
        let text = format!("{}\n{}\n", line("u1", "msg_a", 10), line("u2", "msg_b", 20));
        let fix = fixture(&text);
        let copy = fix.transcript.with_file_name("forked.jsonl");
        fs::write(&copy, &text).unwrap();

        fix.store.refresh(&[fix.transcript.clone(), copy.clone()]);

        let (metrics, sources) = fix.store.aggregate(day());
        assert_eq!(metrics.tokens.output, 30, "复制的那份不能再算一遍");
        assert_eq!(metrics.messages, 2);
        assert_eq!(sources, 2, "两个文件都有贡献，只是内容重合");
    }

    /// transcript 被重写变短（换了内容）时，历史账本必须一起清，否则旧字节算两遍。
    #[test]
    fn shrunk_transcript_restarts_from_scratch() {
        let long = format!(
            "{}\n{}\n{}\n",
            line("u1", "a", 10),
            line("u2", "b", 10),
            line("u3", "c", 10)
        );
        let fix = fixture(&long);
        fix.scan();
        assert_eq!(fix.output(), 30);

        fs::write(&fix.transcript, format!("{}\n", line("u9", "z", 7))).unwrap();
        fix.scan();
        assert_eq!(fix.output(), 7);
    }

    #[test]
    fn prune_drops_vanished_transcripts_and_old_days() {
        let fix = fixture(&format!("{}\n", line("u1", "msg_a", 10)));
        fix.scan();

        fix.store.prune(clock::days_before(day(), 1).unwrap());
        assert_eq!(fix.output(), 10, "还在保留期内");

        fix.store.prune(clock::days_before(day(), -1).unwrap());
        assert_eq!(fix.output(), 0, "整天被剔掉");
    }

    #[test]
    fn prune_forgets_ledgers_of_deleted_transcripts() {
        let fix = fixture(&format!("{}\n", line("u1", "msg_a", 10)));
        fix.scan();
        fs::remove_file(&fix.transcript).unwrap();
        fix.store.prune(clock::days_before(day(), 7).unwrap());
        assert_eq!(fix.output(), 0);
    }

    #[test]
    fn corrupt_state_is_replaced_not_fatal() {
        let fix = fixture(&format!("{}\n", line("u1", "msg_a", 10)));
        let progress = fix
            .store
            .root
            .join("progress")
            .join(format!("{}.json", hash_path(&fix.transcript)));
        fs::write(&progress, "{ 坏 json").unwrap();
        fix.scan();
        assert_eq!(fix.output(), 10);
    }

    #[test]
    fn unreadable_transcript_is_skipped() {
        let fix = fixture("");
        let missing = fix.transcript.with_file_name("missing.jsonl");
        fix.store.refresh(std::slice::from_ref(&missing));
        assert_eq!(fix.store.aggregate(day()).0.messages, 0);
    }
}
