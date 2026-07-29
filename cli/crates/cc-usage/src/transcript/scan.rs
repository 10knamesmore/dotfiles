//! transcript 增量扫描：只读新增字节，折进按天账本。

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read as _, Seek as _, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::entry::{self, Entry};
use crate::clock::Day;
use crate::metrics::ledger::{ActivityEntry, Ledger, UsageEntry};

/// 扫描进度。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// 已消费字节数，只停在完整行边界（末尾半行留给下次）
    pub offset: u64,
    /// 末条消息落在哪天
    pub last: Option<LastMessage>,
}

/// 末条 assistant 消息归属的日期。
///
/// 一条响应按 content block 拆成多行、共享 `message.id`，理论上可能骑在本地午夜两侧；
/// 记住首行的日期让后续行沿用它，免得同一条消息被劈进两天各记一次。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastMessage {
    /// `message.id`
    pub id: String,
    /// 首次出现落在的本地日期
    pub day: Day,
}

/// 从 `progress.offset` 起扫 `path`，新增条目并进 `days`，进度推进到最后一个完整行末尾。
///
/// 文件写到一半（末尾没有换行）时只消费到上一个换行，半行留给下次——否则半截 JSON
/// 解析失败会被永久跳过。
pub fn advance(
    path: &Path,
    progress: &mut Progress,
    days: &mut BTreeMap<Day, Ledger>,
) -> io::Result<()> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(progress.offset))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let Some(last_newline) = buf.iter().rposition(|byte| *byte == b'\n') else {
        return Ok(());
    };
    let complete = &buf[..=last_newline];
    for raw in complete.split(|byte| *byte == b'\n') {
        let Ok(line) = std::str::from_utf8(raw) else {
            continue;
        };
        if let Some(entry) = entry::parse(line) {
            apply(entry, progress, days);
        }
    }
    progress.offset = progress.offset.saturating_add(complete.len() as u64);
    Ok(())
}

/// 把一条事件记进对应日期的账本。
fn apply(entry: Entry, progress: &mut Progress, days: &mut BTreeMap<Day, Ledger>) {
    match entry {
        Entry::Patch(patch) => {
            days.entry(patch.day).or_default().record_activity(
                patch.uuid,
                ActivityEntry {
                    tools: Vec::new(),
                    edits: patch.edits,
                },
            );
        }
        Entry::Message(message) => {
            let day = match &progress.last {
                Some(last) if last.id == message.id => last.day,
                _ => {
                    progress.last = Some(LastMessage {
                        id: message.id.clone(),
                        day: message.day,
                    });
                    message.day
                }
            };
            let ledger = days.entry(day).or_default();
            ledger.record_usage(
                message.id,
                UsageEntry {
                    model: message.model,
                    tokens: message.tokens,
                },
            );
            ledger.record_activity(
                message.uuid,
                ActivityEntry {
                    tools: message.tools,
                    edits: crate::metrics::Edits::default(),
                },
            );
        }
    }
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
    use std::io::Write as _;

    use super::*;
    use crate::clock;
    use crate::metrics::Metrics;

    /// 造一行 assistant：同 id 多行模拟「一条响应按块拆行」。
    fn assistant(uuid: &str, id: &str, output: u64, tool: Option<&str>) -> String {
        let content = tool.map_or_else(
            || r#"[{"type":"thinking"}]"#.to_owned(),
            |name| format!(r#"[{{"type":"tool_use","name":"{name}"}}]"#),
        );
        format!(
            r#"{{"type":"assistant","uuid":"{uuid}","timestamp":"2026-07-29T01:00:00.000Z","message":{{"id":"{id}","model":"claude-sonnet-5","usage":{{"input_tokens":1,"output_tokens":{output},"cache_creation_input_tokens":10,"cache_read_input_tokens":100}},"content":{content}}}}}"#
        )
    }

    fn scan_all(text: &str) -> (Progress, BTreeMap<Day, Ledger>) {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(text.as_bytes()).unwrap();
        let mut progress = Progress::default();
        let mut days = BTreeMap::new();
        advance(file.path(), &mut progress, &mut days).unwrap();
        (progress, days)
    }

    fn today_metrics(days: &BTreeMap<Day, Ledger>) -> Metrics {
        let day = clock::day_of("2026-07-29T01:00:00.000Z").unwrap();
        days.get(&day).expect("当天应有账本").fold()
    }

    #[test]
    fn repeated_id_counts_usage_once_but_tools_each_line() {
        let text = format!(
            "{}\n{}\n{}\n",
            assistant("u1", "msg_a", 5, None),
            assistant("u2", "msg_a", 40, Some("Bash")),
            assistant("u3", "msg_a", 40, Some("Read")),
        );
        let (_, days) = scan_all(&text);
        let metrics = today_metrics(&days);
        assert_eq!(metrics.messages, 1, "一条响应只算一条消息");
        assert_eq!(metrics.tokens.output, 40, "usage 取末值，不按行翻倍");
        assert_eq!(metrics.tokens.cache_read, 100);
        assert_eq!(metrics.tools["Bash"], 1);
        assert_eq!(metrics.tools["Read"], 1);
    }

    #[test]
    fn distinct_ids_accumulate() {
        let text = format!(
            "{}\n{}\n",
            assistant("u1", "msg_a", 10, None),
            assistant("u2", "msg_b", 20, None)
        );
        let (_, days) = scan_all(&text);
        let metrics = today_metrics(&days);
        assert_eq!(metrics.messages, 2);
        assert_eq!(metrics.tokens.output, 30);
    }

    /// 分两次扫（模拟 statusline 反复刷新）：结果必须与一次扫完全一致。
    #[test]
    fn incremental_scan_matches_single_pass() {
        let first = format!(
            "{}\n{}\n",
            assistant("u1", "msg_a", 5, None),
            assistant("u2", "msg_a", 40, Some("Bash"))
        );
        let second = format!("{}\n", assistant("u3", "msg_b", 7, None));

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(first.as_bytes()).unwrap();
        file.flush().unwrap();
        let mut progress = Progress::default();
        let mut days = BTreeMap::new();
        advance(file.path(), &mut progress, &mut days).unwrap();
        let after_first = progress.offset;
        file.write_all(second.as_bytes()).unwrap();
        file.flush().unwrap();
        advance(file.path(), &mut progress, &mut days).unwrap();

        assert_eq!(after_first, first.len() as u64);
        let (_, oneshot) = scan_all(&format!("{first}{second}"));
        assert_eq!(
            serde_json::to_string(&days).unwrap(),
            serde_json::to_string(&oneshot).unwrap()
        );
    }

    /// 末尾半行不能消费掉，否则那条记录永远丢。
    #[test]
    fn partial_trailing_line_is_left_for_next_pass() {
        let whole = format!("{}\n", assistant("u1", "msg_a", 10, None));
        let pending = assistant("u2", "msg_b", 20, None);
        let (head, tail) = pending.as_bytes().split_at(40);

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(whole.as_bytes()).unwrap();
        file.write_all(head).unwrap();
        file.flush().unwrap();

        let mut progress = Progress::default();
        let mut days = BTreeMap::new();
        advance(file.path(), &mut progress, &mut days).unwrap();
        assert_eq!(progress.offset, whole.len() as u64);
        assert_eq!(today_metrics(&days).messages, 1);

        // 补齐后半行 + 换行，下一次扫应把它算进来。
        file.write_all(tail).unwrap();
        file.write_all(b"\n").unwrap();
        file.flush().unwrap();
        advance(file.path(), &mut progress, &mut days).unwrap();
        assert_eq!(today_metrics(&days).messages, 2);
    }

    #[test]
    fn missing_file_is_an_error_not_a_panic() {
        let mut progress = Progress::default();
        let mut days = BTreeMap::new();
        assert!(advance(Path::new("/nonexistent/x.jsonl"), &mut progress, &mut days).is_err());
    }
}
