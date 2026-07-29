//! 端到端：起真实 `cc-usage` 二进制，喂假的 projects 目录，断言 stdout JSON 与 fail-open。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)] // 集成测试惯例

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use cc_usage::report::DayReport;

/// 固定时间戳，配合 `report --date` 绕开「今天是哪天」。
const STAMP: &str = "2026-07-29T01:00:00.000Z";

fn assistant(uuid: &str, id: &str, output: u64, tool: &str) -> String {
    assistant_at(STAMP, uuid, id, output, tool)
}

fn assistant_at(stamp: &str, uuid: &str, id: &str, output: u64, tool: &str) -> String {
    format!(
        r#"{{"type":"assistant","uuid":"{uuid}","timestamp":"{stamp}","message":{{"id":"{id}","model":"claude-sonnet-5","usage":{{"input_tokens":1,"output_tokens":{output},"cache_creation_input_tokens":0,"cache_read_input_tokens":0}},"content":[{{"type":"tool_use","name":"{tool}"}}]}}}}"#
    )
}

fn patch(uuid: &str) -> String {
    format!(
        r#"{{"type":"user","uuid":"{uuid}","timestamp":"{STAMP}","toolUseResult":{{"structuredPatch":[{{"lines":["+a","+b","-c"]}}]}}}}"#
    )
}

/// 主线 transcript 的内容：一条消息 + 一次改动。
fn mainline() -> String {
    format!(
        "{}\n{}\n",
        assistant("u1", "msg_a", 100, "Bash"),
        patch("u2")
    )
}

/// 造 projects 目录：主线 transcript + subagent transcript（后者必须被递归扫到）。
fn seed(root: &Path) {
    let subagents = root.join("proj/session/subagents");
    fs::create_dir_all(&subagents).unwrap();
    fs::write(root.join("proj/session.jsonl"), mainline()).unwrap();
    fs::write(
        subagents.join("agent-1.jsonl"),
        format!("{}\n", assistant("u3", "msg_b", 20, "Read")),
    )
    .unwrap();
}

fn run(projects: &Path, state: &Path, args: &[&str]) -> String {
    let output = Command::cargo_bin("cc-usage")
        .unwrap()
        .args(args)
        .env("CC_USAGE_PROJECTS_DIR", projects)
        .env("CC_USAGE_STATE_DIR", state)
        .output()
        .unwrap();
    assert!(output.status.success(), "退出码必须是 0（fail-open）");
    String::from_utf8(output.stdout).unwrap()
}

fn report(projects: &Path, state: &Path, day: &str) -> DayReport {
    serde_json::from_str(&run(projects, state, &["report", "--date", day])).unwrap()
}

fn local_day() -> String {
    cc_usage::clock::day_of(STAMP).unwrap().to_string()
}

#[test]
fn backfill_then_report_sums_mainline_and_subagent() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let state = dir.path().join("state");
    seed(&projects);

    run(&projects, &state, &["backfill", "--days", "3650"]);
    let day = report(&projects, &state, &local_day());

    assert_eq!(day.messages, 2, "主线 + subagent");
    assert_eq!(day.sources, 2);
    assert_eq!(day.tokens.output, 120);
    assert_eq!(day.edits.added, 2);
    assert_eq!(day.edits.removed, 1);
    assert_eq!(day.tools["Bash"], 1);
    assert_eq!(day.tools["Read"], 1);
    assert!(day.cost_usd > 0.0);
    assert_eq!(day.unpriced_tokens, 0);
}

/// `/compact`、fork 出 background job 时，上游会话的行会被**原样复制**进新 transcript
/// （uuid 与 message.id 不变）。实测这会让当天多算三成，必须只算一次。
#[test]
fn forked_transcript_copy_is_not_double_counted() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let state = dir.path().join("state");
    fs::create_dir_all(projects.join("proj")).unwrap();
    fs::write(projects.join("proj/session.jsonl"), mainline()).unwrap();
    // fork：复制全部历史，再追加自己的新消息
    fs::write(
        projects.join("proj/forked.jsonl"),
        format!("{}{}\n", mainline(), assistant("u9", "msg_new", 5, "Edit")),
    )
    .unwrap();

    run(&projects, &state, &["backfill", "--days", "3650"]);
    let day = report(&projects, &state, &local_day());

    assert_eq!(day.messages, 2, "复制那条只算一次，加上 fork 后的新消息");
    assert_eq!(day.tokens.output, 105);
    assert_eq!(day.edits.added, 2, "改动行同样不能翻倍");
    assert_eq!(day.tools["Bash"], 1);
    assert_eq!(day.tools["Edit"], 1);
}

/// 一个会话跨本地午夜时，午夜前后的消息各归各天——「今天」不继承昨天的量。
///
/// 边界从 [`cc_usage::clock`] 现算，不写死时区。
#[test]
fn one_session_spanning_midnight_splits_by_local_day() {
    let today = cc_usage::clock::today();
    let yesterday = cc_usage::clock::days_before(today, 1).unwrap();
    let midnight = today
        .to_zoned(jiff::tz::TimeZone::system())
        .unwrap()
        .timestamp();
    let minute = jiff::Span::new().minutes(1);
    let before = midnight.checked_sub(minute).unwrap().to_string();
    let after = midnight.checked_add(minute).unwrap().to_string();

    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("proj");
    let state = dir.path().join("state");
    fs::create_dir_all(&projects).unwrap();
    fs::write(
        projects.join("session.jsonl"),
        format!(
            "{}\n{}\n",
            assistant_at(&before, "u1", "msg_yesterday", 111, "Bash"),
            assistant_at(&after, "u2", "msg_today", 7, "Read"),
        ),
    )
    .unwrap();

    run(&projects, &state, &["backfill", "--days", "3650"]);

    let now = report(&projects, &state, &today.to_string());
    assert_eq!(now.messages, 1, "今天只认午夜之后那条");
    assert_eq!(now.tokens.output, 7);
    assert_eq!(now.tools["Read"], 1);
    assert!(!now.tools.contains_key("Bash"), "昨天的工具调用不该漏进来");

    let prev = report(&projects, &state, &yesterday.to_string());
    assert_eq!(prev.messages, 1);
    assert_eq!(prev.tokens.output, 111);
}

/// 反复跑不能翻倍——statusline 每次刷新都会调一次。
#[test]
fn repeated_runs_stay_stable() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let state = dir.path().join("state");
    seed(&projects);

    let mut seen = Vec::new();
    for _ in 0..3 {
        run(&projects, &state, &["backfill", "--days", "3650"]);
        seen.push(run(&projects, &state, &["report", "--date", &local_day()]));
    }
    assert_eq!(seen[0], seen[1]);
    assert_eq!(seen[1], seen[2]);
}

#[test]
fn empty_projects_dir_reports_zeroes() {
    let dir = tempfile::tempdir().unwrap();
    let projects = dir.path().join("projects");
    let state = dir.path().join("state");
    fs::create_dir_all(&projects).unwrap();

    let text = run(&projects, &state, &["today"]);
    let day: DayReport = serde_json::from_str(&text).unwrap();
    assert_eq!(day.tokens_total, 0);
    assert_eq!(day.sources, 0);
}

#[test]
fn missing_projects_dir_is_not_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let text = run(
        &dir.path().join("nope"),
        &dir.path().join("state"),
        &["today"],
    );
    assert!(!text.is_empty(), "仍应打印一份零值汇总");
}

#[test]
fn bad_date_prints_nothing_but_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let text = run(
        &dir.path().join("projects"),
        &dir.path().join("state"),
        &["report", "--date", "昨天"],
    );
    assert!(text.is_empty());
}
