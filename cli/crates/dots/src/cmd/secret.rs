//! `dots secret` —— age 加密 secrets 读写（§10）。

use secrecy::SecretString;

use super::{Result, find_repo_root, home_dir};
use crate::render;

/// secret 子命令。
pub enum SecretAction {
    /// 设置一个 key（交互输入 value，不回显）。
    Set {
        /// secret 键名。
        key: String,
    },
    /// 列出已有 key（不显示值）。
    List,
}

/// 运行 secret 子命令。
pub fn run(action: &SecretAction) -> Result<()> {
    let repo_root = find_repo_root()?;
    let home = home_dir()?;
    match action {
        SecretAction::Set { key } => {
            let value = rpassword::prompt_password(format!("输入 {key} 的值（不回显）："))?;
            crate::secret::set(&repo_root, &home, key, SecretString::from(value))?;
            render::ok(&format!("已写入 secret `{key}`（密文入库，明文不入库）"));
        }
        SecretAction::List => {
            let all = crate::secret::load_all(&repo_root, &home, true)?;
            render::header("secrets（仅键名）");
            let mut keys: Vec<&String> = all.keys().collect();
            keys.sort();
            for key in keys {
                render::ok(key);
            }
        }
    }
    Ok(())
}
