//! `dots completions <shell>` —— 生成 shell 补全脚本。

use clap::CommandFactory;
use clap_complete::Shell;

use super::Result;
use crate::cli::Args;

/// 生成补全脚本到 stdout。
pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = Args::command();
    let name = cmd.get_name().to_owned();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}
