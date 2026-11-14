//! 终端输出辅助（彩色三态等）。

use owo_colors::OwoColorize;
use std::time::Instant;

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

/// 打印当前步骤的 dots 输出。
pub fn detail(message: &str) {
    for line in message.lines() {
        println!("      {line}");
    }
}

/// 一次命令的步骤编号与总耗时；配置读取后才能确定总步骤数。
pub struct Steps {
    /// 已开始的步骤数。
    current: usize,
    /// 配置读取前为 None。
    total: Option<usize>,
    /// 整条命令开始时间。
    started: Instant,
}

impl Steps {
    /// 打印命令标题并开始计时。
    pub fn new(title: &str) -> Self {
        header(title);
        Self {
            current: 0,
            total: None,
            started: Instant::now(),
        }
    }

    /// 配置求值完成后设置总步骤数。
    pub fn set_total(&mut self, total: usize) {
        self.total = Some(total);
    }

    /// 当前步骤的位置，配置尚未读取时不猜测总数。
    pub fn position(&self) -> String {
        match self.total {
            Some(total) => format!("{}/{total}", self.current),
            None => format!("{}/…", self.current),
        }
    }

    /// 为下一个步骤分配编号，供内部步骤或外部命令标题使用。
    pub fn next_position(&mut self) -> String {
        self.current += 1;
        self.position()
    }

    /// 开始一个 dots 内部步骤。
    pub fn begin(&mut self, source: &str, title: &str) -> Step {
        let position = self.next_position();
        println!(
            "\n{} {} · {}",
            format!("[{position}]").bold().cyan(),
            source.bold(),
            title.bold()
        );
        Step {
            started: Instant::now(),
        }
    }

    /// 打印整条命令的结果和总耗时。
    pub fn finish(&self, healthy: bool, message: &str) {
        println!();
        outcome(healthy, message, self.started);
    }
}

/// 一个正在执行的步骤；完成时打印结果和耗时。
pub struct Step {
    /// 当前步骤开始时间。
    started: Instant,
}

impl Step {
    /// 执行会返回错误的步骤；失败时先标记步骤，再向上返回错误。
    pub fn run<T>(self, action: impl FnOnce() -> crate::Result<T>) -> crate::Result<T> {
        let result = action();
        self.finish(result.is_ok(), if result.is_ok() { "完成" } else { "失败" });
        result
    }

    /// 结束步骤，状态由业务结果决定。
    pub fn finish(self, healthy: bool, message: &str) {
        outcome(healthy, message, self.started);
    }

    /// 预览时不执行步骤，也不把它标记成已经完成。
    pub fn skip(self) {
        println!("      {} 跳过 · dry-run", "–".yellow());
    }
}

/// 用符号与颜色共同表示结果。
fn outcome(healthy: bool, message: &str, started: Instant) {
    let duration = format!("{:.2}s", started.elapsed().as_secs_f64());
    if healthy {
        println!("      {} {message}   {}", "✓".green(), duration.dimmed());
    } else {
        println!("      {} {message}   {}", "✗".red(), duration.dimmed());
    }
}
