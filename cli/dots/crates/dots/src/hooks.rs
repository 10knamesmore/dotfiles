//! 执行绑定 `dots sync` 生命周期的具名命令。

use std::process::Command;

use dots_core::manifest::BeforeSyncHook;

use crate::render;
use crate::subprocess;

/// 运行或预览按声明顺序排列的 `before_sync` hook。
///
/// 任一程序启动或执行失败都会终止 sync；dry-run 展示命令但不执行。
pub fn run_before_sync(
    hooks: &[BeforeSyncHook],
    dry_run: bool,
    steps: &mut render::Steps,
) -> crate::Result<()> {
    for hook in hooks {
        let title = format!("[{}] hook · {}", steps.next_position(), hook.name);
        let mut command = Command::new(&hook.program);
        command.args(&hook.args).current_dir(&hook.cwd);
        if dry_run {
            subprocess::preview(&command, &title);
        } else {
            subprocess::run(command, &title)?;
        }
    }
    Ok(())
}
