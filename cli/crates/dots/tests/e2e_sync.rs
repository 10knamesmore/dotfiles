//! 端到端：在临时 $HOME + 仓库上跑真实 `dots sync`，验证链接收敛与幂等。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)] // 集成测试惯例

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

/// 搭一个最小仓库：dots.lua + tree/home 下一个文件、一个 .config 子目录。
fn setup_repo(repo: &Path) {
    fs::write(repo.join("dots.lua"), "-- 空清单，纯镜像\n").unwrap();
    fs::create_dir_all(repo.join("tree/home/.config/nvim")).unwrap();
    fs::write(repo.join("tree/home/.vimrc"), "set nocompatible\n").unwrap();
    fs::write(repo.join("tree/home/.config/nvim/init.lua"), "-- nvim\n").unwrap();
    fs::write(repo.join("tree/home/.config/starship.toml"), "# starship\n").unwrap();
}

/// 跑 `dots <args>`，注入 DOTFILES_DIR/HOME。
fn run_dots(repo: &Path, home: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("dots")
        .unwrap()
        .args(args)
        .env("DOTFILES_DIR", repo)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .assert()
}

#[test]
fn sync_creates_links_and_is_idempotent() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 首次 sync
    run_dots(repo, home, &["sync"]).success();

    // 顶层文件成链
    let vimrc = home.join(".vimrc");
    assert!(
        fs::symlink_metadata(&vimrc)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(&vimrc).unwrap(),
        repo.join("tree/home/.vimrc")
    );

    // .config 容器下钻：nvim 整目录链、starship.toml 文件链
    assert_eq!(
        fs::read_link(home.join(".config/nvim")).unwrap(),
        repo.join("tree/home/.config/nvim")
    );
    assert_eq!(
        fs::read_link(home.join(".config/starship.toml")).unwrap(),
        repo.join("tree/home/.config/starship.toml")
    );

    // ~/.zshrc stub 与 env.zsh 写出
    assert!(
        fs::read_to_string(home.join(".zshrc"))
            .unwrap()
            .contains("DOTS_MANAGED")
    );
    assert!(
        fs::read_to_string(home.join(".config/dots/env.zsh"))
            .unwrap()
            .contains("DOTFILES_DIR")
    );

    // 再次 sync 幂等：status 全绿
    run_dots(repo, home, &["sync"]).success();
    run_dots(repo, home, &["status"]).success();
}

#[test]
fn sync_relinks_stale_repo_link() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 预置一个指向仓库内「旧路径」的断链（模拟 install.py 时代 general/ 链）。
    let stale = repo.join("general/.vimrc"); // 不存在 → 断链
    std::os::unix::fs::symlink(&stale, home.join(".vimrc")).unwrap();

    // sync 应识别为仓库内旧链 → 重建到 tree/ 新目标
    run_dots(repo, home, &["sync"]).success();
    assert_eq!(
        fs::read_link(home.join(".vimrc")).unwrap(),
        repo.join("tree/home/.vimrc")
    );
}

#[test]
fn status_reports_missing_with_nonzero_exit() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 未 sync，直接 status：应有 missing → 非零退出
    run_dots(repo, home, &["status"]).failure();
}

/// 取与 dots::hosts::current() 同源的主机名（/etc/hostname → HOSTNAME → unknown）。
fn current_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_owned()))
}

#[test]
fn on_host_activate_fires_after_host_block() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    let host = current_hostname();
    fs::write(
        repo.join("dots.lua"),
        format!(
            r#"
hosts {{ ["{host}"] = function() vars {{ marker = "1" }} end }}
on {{
  on_host_activate = function()
    dots.file.ensure_block(dots.home .. "/.hook-mark", "e2e", "fired")
  end,
}}
"#
        ),
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    let mark = fs::read_to_string(home.join(".hook-mark")).unwrap_or_default();
    assert!(
        mark.contains("fired"),
        "on_host_activate 应在命中 host 块后触发，.hook-mark 内容：{mark:?}"
    );
}

#[test]
fn entry_pre_false_skips_entry_links() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // nvim 条目被 pre 阻止；其余照常链接
    fs::write(
        repo.join("dots.lua"),
        r#"
granularity("home/.config/nvim", {
  mode = "dir",
  pre = function() return false end,
})
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    assert!(
        !home.join(".config/nvim").exists(),
        "pre 返回 false 的条目不应链接"
    );
    assert!(
        home.join(".vimrc").exists(),
        "其他条目应不受影响照常链接"
    );
}

#[test]
fn entry_pre_nil_links_normally() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // pre 无显式 return（nil）→ 继续链接
    fs::write(
        repo.join("dots.lua"),
        r#"
granularity("home/.config/nvim", {
  mode = "dir",
  pre = function() end,
})
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    assert!(
        home.join(".config/nvim").exists(),
        "pre 返回 nil 应继续链接"
    );
}

#[test]
fn entry_post_runs_after_link() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    fs::write(
        repo.join("dots.lua"),
        r#"
granularity("home/.config/nvim", {
  mode = "dir",
  post = function()
    dots.file.ensure_block(dots.home .. "/.post-mark", "e2e", "post ran")
  end,
})
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    let mark = fs::read_to_string(home.join(".post-mark")).unwrap_or_default();
    assert!(mark.contains("post ran"), "post 应在链接后执行：{mark:?}");
}

#[test]
fn entry_post_skipped_when_pre_blocks() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    fs::write(
        repo.join("dots.lua"),
        r#"
granularity("home/.config/nvim", {
  mode = "dir",
  pre = function() return false end,
  post = function()
    dots.file.ensure_block(dots.home .. "/.post-mark", "e2e", "post ran")
  end,
})
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    assert!(
        !home.join(".post-mark").exists(),
        "被 pre 阻止的条目其 post 不应执行"
    );
}

#[test]
fn entry_pre_evaluated_on_dry_run_without_writes() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    fs::write(
        repo.join("dots.lua"),
        r#"
granularity("home/.config/nvim", {
  mode = "dir",
  pre = function() return false end,
})
"#,
    )
    .unwrap();

    // dry-run：pre 照常评估（输出含跳过提示），但不写盘
    let output = run_dots(repo, home, &["sync", "--dry-run"]).success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).to_string();
    assert!(
        stdout.contains("home/.config/nvim"),
        "dry-run 输出应包含被跳过条目的提示：{stdout}"
    );
    assert!(!home.join(".config/nvim").exists());
    assert!(!home.join(".vimrc").exists(), "dry-run 不应写盘");
}

#[test]
fn post_sync_runs_after_shell_env_and_systemd() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 顺序探针：post_sync 时 env.zsh 必须已写出。
    // run_once 的命令退出非零会让 sync 整体报错，故 sync success 即证明顺序正确。
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    dots.run_once("probe-env", "test -f " .. dots.home .. "/.config/dots/env.zsh")
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
}

#[test]
fn on_host_activate_silent_when_hosts_table_empty() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 无 hosts 块 → on_host_activate 不触发，sync 正常
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  on_host_activate = function()
    dots.file.ensure_block(dots.home .. "/.hook-mark", "e2e", "fired")
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    assert!(
        !home.join(".hook-mark").exists(),
        "未命中 host 块时 on_host_activate 不应触发"
    );
}
