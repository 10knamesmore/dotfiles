//! age 加密 secrets（§10）。
//!
//! 密文 `hosts/secrets.age` 入库；明文从不入库。用 passphrase（scrypt）加密——
//! 密码来自 `~/.config/dots/age.pass` 或交互输入，避免私钥分发难题。
//! 诚实边界：age 只护 git 同步面，渲染产物本机仍是明文。

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, eyre};
use rustc_hash::FxHashMap;
use secrecy::SecretString;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// secrets 文件路径（`hosts/secrets.age`）。
fn secrets_path(repo_root: &Path) -> PathBuf {
    repo_root.join("hosts").join("secrets.age")
}

/// 读取 passphrase：优先 `~/.config/dots/age.pass`，否则交互输入（不回显）。
fn read_passphrase(home: &Path, interactive: bool) -> Result<SecretString> {
    let pass_file = home.join(".config").join("dots").join("age.pass");
    if let Ok(text) = fs::read_to_string(&pass_file) {
        return Ok(SecretString::from(text.trim().to_owned()));
    }
    if interactive {
        let entered = rpassword::prompt_password("dots secrets 口令：").wrap_err("读取口令失败")?;
        return Ok(SecretString::from(entered));
    }
    Err(eyre!(
        "无 age 口令：请创建 {} 或交互运行",
        pass_file.display()
    ))
}

/// 解密全部 secrets 为 `key → value` 映射；文件不存在则返回空。
pub fn load_all(
    repo_root: &Path,
    home: &Path,
    interactive: bool,
) -> Result<FxHashMap<String, String>> {
    let path = secrets_path(repo_root);
    let Ok(ciphertext) = fs::read(&path) else {
        return Ok(FxHashMap::default());
    };
    let pass = read_passphrase(home, interactive)?;
    let plain = decrypt(&ciphertext, &pass)?;
    let map: FxHashMap<String, String> =
        serde_json::from_slice(&plain).wrap_err("解析解密后的 secrets JSON 失败")?;
    Ok(map)
}

/// 设置一个 secret（读出→改→写回密文）。
pub fn set(repo_root: &Path, home: &Path, key: &str, value: SecretString) -> Result<()> {
    let mut map = load_all(repo_root, home, true).unwrap_or_default();
    use secrecy::ExposeSecret;
    map.insert(key.to_owned(), value.expose_secret().to_owned());
    let pass = read_passphrase(home, true)?;
    let plain = serde_json::to_vec(&map).wrap_err("序列化 secrets 失败")?;
    let cipher = encrypt(&plain, &pass)?;
    let path = secrets_path(repo_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err("建 hosts 目录失败")?;
    }
    fs::write(&path, cipher).wrap_err_with(|| format!("写 {} 失败", path.display()))?;
    Ok(())
}

/// 用 passphrase 加密。
fn encrypt(plain: &[u8], pass: &SecretString) -> Result<Vec<u8>> {
    let encryptor = age::Encryptor::with_user_passphrase(pass.clone());
    let mut out = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut out)
        .wrap_err("初始化 age 加密失败")?;
    writer.write_all(plain).wrap_err("写入明文失败")?;
    writer.finish().wrap_err("结束 age 加密失败")?;
    Ok(out)
}

/// 用 passphrase 解密。
fn decrypt(cipher: &[u8], pass: &SecretString) -> Result<Vec<u8>> {
    let decryptor = age::Decryptor::new(cipher).wrap_err("初始化 age 解密失败")?;
    let identity = age::scrypt::Identity::new(pass.clone());
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .wrap_err("age 解密失败（口令错误？）")?;
    let mut out = Vec::new();
    reader.read_to_end(&mut out).wrap_err("读取解密流失败")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_and_load_round_trip() -> Result<()> {
        let repo = tempdir()?;
        let home = tempdir()?;
        // 放一个口令文件，避免交互。
        let dots_dir = home.path().join(".config/dots");
        fs::create_dir_all(&dots_dir)?;
        fs::write(dots_dir.join("age.pass"), "test-pass\n")?;

        set(
            repo.path(),
            home.path(),
            "bsu_pass",
            SecretString::from("hunter2".to_owned()),
        )?;
        let all = load_all(repo.path(), home.path(), false)?;
        assert_eq!(all.get("bsu_pass").map(String::as_str), Some("hunter2"));
        Ok(())
    }

    #[test]
    fn load_missing_is_empty() -> Result<()> {
        let repo = tempdir()?;
        let home = tempdir()?;
        let all = load_all(repo.path(), home.path(), false)?;
        assert!(all.is_empty());
        Ok(())
    }
}
