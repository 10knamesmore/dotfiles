//! 终端输出辅助（彩色三态等）。

use owo_colors::OwoColorize;

/// 打印一个段落标题。
pub fn header(title: &str) {
    println!("\n{}", title.bold().cyan());
}

/// 打印一条 OK 行。
pub fn ok(msg: &str) {
    println!("  {} {msg}", "✔".green());
}

/// 打印一条警告行。
pub fn warn(msg: &str) {
    println!("  {} {msg}", "!".yellow());
}

/// 打印一条错误行。
pub fn err(msg: &str) {
    println!("  {} {msg}", "✗".red());
}

/// 打印一条建议（供用户粘贴到 dots.lua 的行）。
pub fn suggest(line: &str) {
    println!("  {} {}", "→".blue(), line.dimmed());
}
