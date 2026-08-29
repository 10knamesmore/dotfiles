//! 把 adapter 的 hook 结果写入 stdout、stderr 与进程退出码。

use serde::Serialize;

use super::outcome::HookRun;

/// 输出一次 hook 结果；序列化失败时保持静默和原有 exit code。
pub fn emit<T: Serialize>(run: HookRun<T>) {
    if let Some(notice) = &run.notice {
        eprintln!("{notice}");
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
