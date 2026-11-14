//! 用开始／结束边界展示子命令，执行时直接继承终端。

use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

use color_eyre::eyre::{Context, eyre};
use owo_colors::OwoColorize;

/// 展示步骤、显式工作目录和可复制的 shell 命令。
fn show(command: &Command, title: &str) {
    println!("\n┌─ {}", title.bold().cyan());
    if let Some(directory) = command.get_current_dir() {
        println!("│  cwd {}", directory.display());
    }
    let words = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|word| word.to_string_lossy());
    println!("│  $ {}", shell_words::join(words));
    println!("└─");
}

/// 预览命令，不执行。
pub fn preview(command: &Command, title: &str) {
    show(command, title);
    println!("── dots · {} {title} · 跳过 · dry-run ──", "–".yellow());
}

/// 继承 stdin、stdout 和 stderr；执行失败时打印结束边界并返回错误。
pub fn run(mut command: Command, title: &str) -> crate::Result<()> {
    show(&command, title);
    io::stdout().flush()?;
    let started = Instant::now();
    let result = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .wrap_err_with(|| format!("无法执行 {title}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(eyre!("{title} 失败：{status}"))
            }
        });
    let duration = format!("{:.2}s", started.elapsed().as_secs_f64());
    match &result {
        Ok(()) => println!(
            "\n── dots · {} {title} · 完成 · {} ──",
            "✓".green(),
            duration.dimmed()
        ),
        Err(error) => println!(
            "\n── dots · {} {error} · {} ──",
            "✗".red(),
            duration.dimmed()
        ),
    }
    result
}
