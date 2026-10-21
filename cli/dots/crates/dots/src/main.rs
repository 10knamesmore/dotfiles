//! dots —— dotfiles 管理 CLI 入口。

use std::process::ExitCode;

fn main() -> color_eyre::Result<ExitCode> {
    color_eyre::install()?;
    dots::cli::run()
}
