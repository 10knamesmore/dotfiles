//! 当前主机名解析。

use std::fs;
use std::path::Path;

/// 取当前主机名（= dots.lua `hosts{}` 块的匹配 key）。
///
/// 解析顺序（前者命中即返回，空白视作未命中）：
/// 1. `$DOTS_HOST` 环境变量——显式覆盖，便于测试/临时切换。
/// 2. `~/.config/dots/host`——onboarding 写的别名（B 方案：真名不进 dots.lua/git）。
/// 3. `/etc/hostname`——Linux 约定。
/// 4. `$HOSTNAME` 环境变量。
/// 5. `"unknown"`——全空兜底。
pub fn current() -> String {
    let host_file = std::env::var("HOME")
        .ok()
        .and_then(|home| host_file_path(Path::new(&home)).read());
    resolve_host(
        std::env::var("DOTS_HOST").ok().as_deref(),
        host_file.as_deref(),
        fs::read_to_string("/etc/hostname").ok().as_deref(),
        std::env::var("HOSTNAME").ok().as_deref(),
    )
}

/// onboarding 写别名的机器本地文件路径（`~/.config/dots/host`）。
pub fn host_file_path(home: &Path) -> HostFile {
    HostFile(home.join(".config").join("dots").join("host"))
}

/// `~/.config/dots/host` 的句柄。
pub struct HostFile(std::path::PathBuf);

impl HostFile {
    /// 文件路径。
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// 读取别名（trim 后非空才返回）。
    pub fn read(&self) -> Option<String> {
        let text = fs::read_to_string(&self.0).ok()?;
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }
}

/// 纯解析：按优先级取首个 trim 后非空的来源，全空则 `"unknown"`。
fn resolve_host(
    dots_host_env: Option<&str>,
    host_file: Option<&str>,
    etc_hostname: Option<&str>,
    hostname_env: Option<&str>,
) -> String {
    [dots_host_env, host_file, etc_hostname, hostname_env]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|candidate| !candidate.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn dots_host_env_wins_over_everything() {
        let got = resolve_host(Some("alias"), Some("from-file"), Some("etc"), Some("env"));
        assert_eq!(got, "alias");
    }

    #[test]
    fn host_file_used_when_env_absent() {
        let got = resolve_host(None, Some("from-file"), Some("etc"), Some("env"));
        assert_eq!(got, "from-file");
    }

    #[test]
    fn etc_hostname_used_when_overrides_absent() {
        let got = resolve_host(None, None, Some("etc-host\n"), Some("env"));
        assert_eq!(got, "etc-host");
    }

    #[test]
    fn hostname_env_is_last_real_source() {
        let got = resolve_host(None, None, None, Some("env-host"));
        assert_eq!(got, "env-host");
    }

    #[test]
    fn all_absent_falls_back_to_unknown() {
        assert_eq!(resolve_host(None, None, None, None), "unknown");
    }

    #[test]
    fn blank_sources_are_skipped() {
        // 空白（含换行/空格）视作未命中，落到下一个非空来源。
        let got = resolve_host(Some("  "), Some("\n"), Some("\t real \n"), None);
        assert_eq!(got, "real");
    }
}
