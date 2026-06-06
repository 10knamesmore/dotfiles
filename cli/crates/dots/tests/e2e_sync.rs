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
