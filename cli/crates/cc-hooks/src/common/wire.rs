//! 把 hook 业务结果统一写入 stdout、stderr、审计日志与进程退出码。

use std::io::Write as _;
use std::path::Path;

use serde::Serialize;

use super::outcome::HookRun;

/// 输出一次 hook 结果，并按需追加 best-effort 审计日志。
///
/// 序列化或审计 IO 失败都不会把半截协议写到 stdout，也不会改变业务层指定的
/// fail-open 退出码。
pub fn emit<T: Serialize>(run: HookRun<T>, audit_path: Option<&Path>) {
    if let Some(notice) = &run.notice {
        eprintln!("{notice}");
        append_audit_log(audit_path, notice);
    }
    if let Some(audit) = &run.audit {
        append_audit_log(audit_path, audit);
    }
    if let Some(output) = run.output
        && let Ok(line) = serde_json::to_string(&output)
    {
        println!("{line}");
    }
    if run.code != 0 {
        std::process::exit(run.code);
    }
}

/// 给审计行补 epoch 秒时间戳后追加到指定文件。
fn append_audit_log(path: Option<&Path>, line: &str) {
    let Some(path) = path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{stamp} {line}");
    }
}
