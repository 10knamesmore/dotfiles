//! 执行绑定 `dots sync` 生命周期的具名命令。

use std::process::Command;

use color_eyre::eyre::{Context, eyre};
use dots_core::manifest::BeforeSyncHook;

use crate::render;

/// 运行或预览按声明顺序排列的 `before_sync` hook。
///
/// 真实执行时，任一程序无法启动或返回非零状态都会终止 sync；dry-run 只输出名称。
pub fn run_before_sync(hooks: &[BeforeSyncHook], dry_run: bool) -> crate::Result<()> {
    for hook in hooks {
        if dry_run {
            render::warn(&format!("would run before_sync hook `{}`", hook.name));
            continue;
        }

        println!("  run before_sync hook `{}`", hook.name);
        let status = Command::new(&hook.program)
            .args(&hook.args)
            .current_dir(&hook.cwd)
            .status()
            .wrap_err_with(|| {
                format!(
                    "无法启动 before_sync hook `{}`：program `{}`，cwd `{}`",
                    hook.name, hook.program, hook.cwd
                )
            })?;
        if !status.success() {
            return Err(eyre!("before_sync hook `{}` 失败：{status}", hook.name));
        }
        render::ok(&format!("before_sync hook `{}`", hook.name));
    }
    Ok(())
}
