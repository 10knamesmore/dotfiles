//! 真 CLI + 临时 repo/HOME 的跨次 sync Resource reconciliation。
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::min_ident_chars,
    clippy::missing_docs_in_private_items
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

/// 创建无显式 Lua Resource 的最小镜像仓库。
fn setup_repo(repo: &Path) {
    fs::write(repo.join("dots.lua"), "-- empty declarative manifest\n").unwrap();
    fs::create_dir_all(repo.join("tree/home/.config/nvim")).unwrap();
    fs::write(repo.join("tree/home/.vimrc"), "set nocompatible\n").unwrap();
    fs::write(repo.join("tree/home/.config/nvim/init.lua"), "-- nvim\n").unwrap();
    fs::write(repo.join("tree/home/.config/starship.toml"), "# starship\n").unwrap();
}

/// 运行真实 dots binary，并隔离 repo、HOME 和颜色输出。
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
fn sync_creates_all_builtin_resources_and_becomes_clean() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    run_dots(repo, home, &["sync"]).success();
    assert_eq!(
        fs::read_link(home.join(".vimrc")).unwrap(),
        repo.join("tree/home/.vimrc")
    );
    assert_eq!(
        fs::read_link(home.join(".config/nvim")).unwrap(),
        repo.join("tree/home/.config/nvim")
    );
    let zshrc = fs::read_to_string(home.join(".zshrc")).unwrap();
    assert!(zshrc.contains("# >>> dots:dots-env >>>"));
    assert!(zshrc.contains("source \"$HOME/.zshrc_dotfiles\""));
    assert!(home.join(".config/dots/env.zsh").is_file());
    assert!(home.join(".config/dots/root").is_file());

    run_dots(repo, home, &["sync"]).success();
    run_dots(repo, home, &["status"]).success();
}

#[test]
fn legacy_zshrc_stub_is_replaced_by_one_managed_block() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    fs::write(
        home.join(".zshrc"),
        "# DOTS_MANAGED: legacy\nsource \"$HOME/.zshrc_dotfiles\"\n# conda\n",
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    let zshrc = fs::read_to_string(home.join(".zshrc")).unwrap();
    assert!(!zshrc.contains("# DOTS_MANAGED:"));
    assert_eq!(zshrc.matches("source \"$HOME/.zshrc_dotfiles\"").count(), 1);
    assert!(zshrc.starts_with("# >>> dots:dots-env >>>"));
    assert!(zshrc.contains("# conda"));
}

#[test]
fn removing_declaration_deletes_unchanged_link() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    run_dots(repo, home, &["sync"]).success();
    fs::remove_file(repo.join("tree/home/.vimrc")).unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert!(fs::symlink_metadata(home.join(".vimrc")).is_err());
    run_dots(repo, home, &["status"]).success();
}

#[test]
fn retired_drift_is_preserved_and_independent_create_still_commits() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    run_dots(repo, home, &["sync"]).success();
    fs::remove_file(home.join(".vimrc")).unwrap();
    std::os::unix::fs::symlink("/tmp/foreign-vimrc", home.join(".vimrc")).unwrap();
    fs::remove_file(repo.join("tree/home/.vimrc")).unwrap();
    fs::write(repo.join("tree/home/new-file"), "new\n").unwrap();

    run_dots(repo, home, &["sync"]).failure();
    assert_eq!(
        fs::read_link(home.join(".vimrc")).unwrap(),
        Path::new("/tmp/foreign-vimrc")
    );
    assert_eq!(
        fs::read_link(home.join("new-file")).unwrap(),
        repo.join("tree/home/new-file")
    );

    run_dots(repo, home, &["forget", "~/.vimrc"]).success();
    assert_eq!(
        fs::read_link(home.join(".vimrc")).unwrap(),
        Path::new("/tmp/foreign-vimrc")
    );
    run_dots(repo, home, &["sync"]).success();
}

#[test]
fn collision_preserves_foreign_target() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    fs::write(home.join(".vimrc"), "local\n").unwrap();

    run_dots(repo, home, &["sync"]).failure();
    assert_eq!(fs::read_to_string(home.join(".vimrc")).unwrap(), "local\n");
}

#[test]
fn identical_unowned_link_is_adopted_without_rewrite() {
    use std::os::unix::fs::MetadataExt;

    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    std::os::unix::fs::symlink(repo.join("tree/home/.vimrc"), home.join(".vimrc")).unwrap();
    let inode = fs::symlink_metadata(home.join(".vimrc")).unwrap().ino();

    run_dots(repo, home, &["sync"]).success();
    assert_eq!(
        fs::symlink_metadata(home.join(".vimrc")).unwrap().ino(),
        inode
    );
    run_dots(repo, home, &["status"]).success();
}

#[test]
fn managed_block_preserves_outside_text_detects_drift_and_retires() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    fs::write(home.join(".profile"), "before\nafter\n").unwrap();
    fs::write(
        repo.join("dots.lua"),
        r#"
dots.resource.managed_block {
    target = "~/.profile",
    marker = "profile-env",
    content = "export FROM_DOTS=1",
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    let installed = fs::read_to_string(home.join(".profile")).unwrap();
    assert!(installed.contains("before\nafter"));
    assert!(installed.contains("export FROM_DOTS=1"));

    let drifted = installed.replace("export FROM_DOTS=1", "export FROM_DOTS=manual");
    fs::write(home.join(".profile"), drifted).unwrap();
    run_dots(repo, home, &["sync"]).failure();
    fs::write(home.join(".profile"), &installed).unwrap();

    fs::write(repo.join("dots.lua"), "-- declaration retired\n").unwrap();
    run_dots(repo, home, &["sync"]).success();
    let retired = fs::read_to_string(home.join(".profile")).unwrap();
    assert!(retired.contains("before\nafter"));
    assert!(!retired.contains("profile-env"));
}

#[test]
fn copied_file_tracks_content_mode_and_deletion() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    let source = repo.join("payload.bin");
    fs::write(&source, "v1").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        repo.join("dots.lua"),
        r#"dots.resource.copied_file { source = "payload.bin", target = "~/bin/payload" }"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
    let target = home.join("bin/payload");
    assert_eq!(fs::read_to_string(&target).unwrap(), "v1");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o755
    );

    fs::write(&source, "v2").unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert_eq!(fs::read_to_string(&target).unwrap(), "v2");

    fs::write(repo.join("dots.lua"), "-- declaration retired\n").unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert!(!target.exists());
}

#[test]
fn inject_source_retirement_deletes_generated_file_and_target_link() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    let source = repo.join("tree/home/.config/service.conf.inject");
    fs::write(&source, "root={{ DOTFILES }}\n").unwrap();

    run_dots(repo, home, &["sync"]).success();
    let generated = repo.join(".gen/injected/home/.config/service.conf");
    let target = home.join(".config/service.conf");
    assert!(generated.is_file());
    assert_eq!(fs::read_link(&target).unwrap(), generated);

    fs::remove_file(source).unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert!(fs::symlink_metadata(target).is_err());
    assert!(!generated.exists());
}

#[test]
fn removed_script_source_deletes_aggregated_link() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    fs::create_dir_all(repo.join("scripts/common")).unwrap();
    let source = repo.join("scripts/common/tool");
    fs::write(&source, "#!/bin/sh\n").unwrap();

    run_dots(repo, home, &["sync"]).success();
    let target = repo.join(".gen/scripts/tool");
    assert_eq!(fs::read_link(&target).unwrap(), source);

    fs::remove_file(source).unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert!(fs::symlink_metadata(target).is_err());
}

/// 创建无外部依赖的独立 Cargo binary fixture。
fn setup_cargo_binary(repo: &Path) {
    let directory = repo.join("mini");
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname='mini'\nversion='0.0.0'\nedition='2021'\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(directory.join("src/main.rs"), "fn main() {}\n").unwrap();
}

#[test]
fn cargo_binary_runs_only_during_install() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    setup_cargo_binary(repo);
    fs::write(
        repo.join("dots.lua"),
        r#"
dots.resource.cargo_binary {
    source = {
        path = "mini",
        binary = "mini",
    },
    root = "~",
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync", "--dry-run"]).success();
    assert!(!repo.join("mini/target/release/mini").exists());
    assert!(!home.join("bin/mini").exists());

    run_dots(repo, home, &["install"]).success();
    assert!(repo.join("mini/target/release/mini").is_file());
    assert!(home.join("bin/mini").is_file());
}

#[test]
fn legacy_state_schema_fails_before_external_write() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    fs::create_dir_all(repo.join(".dots")).unwrap();
    fs::write(repo.join(".dots/state.json"), r#"{"links":[]}"#).unwrap();

    run_dots(repo, home, &["sync"])
        .failure()
        .stderr(predicates::str::contains("schema 不兼容"));
    assert!(!home.join(".vimrc").exists());
}
