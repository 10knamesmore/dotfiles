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
    assert!(home.join(".vimrc").exists(), "其他条目应不受影响照常链接");
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

#[test]
fn sync_prunes_orphan_records_after_target_deleted() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 首次 sync：建链 + 记台账
    run_dots(repo, home, &["sync"]).success();

    // 用户手动删掉受管链接，仓库侧源也撤掉（声明随 tree 消失）
    fs::remove_file(home.join(".vimrc")).unwrap();
    fs::remove_file(repo.join("tree/home/.vimrc")).unwrap();

    // 再 sync：该链接已不被期望且磁盘已无 → 台账孤儿记录应被修剪
    run_dots(repo, home, &["sync"]).success();

    let out = run_dots(repo, home, &["status"]).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("0 孤儿"), "status 仍报孤儿：{stdout}");
}

#[test]
fn dots_run_executes_every_sync_and_skips_dry_run() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    dots.run("echo ran >> " .. dots.home .. "/probe.log")
  end,
}
"#,
    )
    .unwrap();

    // dry-run 不执行
    run_dots(repo, home, &["sync", "--dry-run"]).success();
    assert!(
        !home.join("probe.log").exists(),
        "dry-run 不应执行 dots.run"
    );

    // 与 run_once 的区别：每次 sync 都执行
    run_dots(repo, home, &["sync"]).success();
    run_dots(repo, home, &["sync"]).success();
    let log = fs::read_to_string(home.join("probe.log")).unwrap();
    assert_eq!(log.lines().count(), 2, "应每次 sync 都执行：{log}");
}

#[test]
fn dots_run_failure_warns_but_sync_succeeds() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 返回值必须是 false；失败不致命（sync 仍 success），但要在输出留痕
    // 富返回：code/stdout/stderr/ok 四字段；断言写在 Lua 里，不符即 error → sync 失败
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local r = dots.run("echo outprobe; echo errprobe >&2; exit 3")
    if r.code ~= 3 then error("code 应为 3: " .. tostring(r.code)) end
    if not r.stdout:find("outprobe", 1, true) then error("stdout 丢失: " .. r.stdout) end
    if not r.stderr:find("errprobe", 1, true) then error("stderr 丢失: " .. r.stderr) end
    if r.ok then error("ok 应为 false") end
    local good = dots.run("true")
    if not (good.ok and good.code == 0) then error("true 应 ok") end
  end,
}
"#,
    )
    .unwrap();

    let out = run_dots(repo, home, &["sync"]).success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("dots.run 退出码 3"),
        "非零退出应留痕：{stdout}"
    );
}

#[test]
fn dots_repo_exposes_repo_root() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // dots.repo 应指向仓库根：读 tree 下已知文件验证
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local probe = dots.run("cat " .. dots.repo .. "/tree/home/.vimrc")
    if not probe.ok then error("dots.repo 不可用: " .. tostring(dots.repo)) end
    if not probe.stdout:find("nocompatible") then error("内容不符: " .. probe.stdout) end
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
}

/// 在仓库内搭一个最小 cargo bin 项目（无依赖，离线可编译）。
/// `[workspace]` 空表：避免被外层 workspace 误收编。
fn setup_minibin(repo: &Path, main_rs: &str) {
    let dir = repo.join("minibin");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"minibin\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    fs::write(dir.join("src/main.rs"), main_rs).unwrap();
}

#[test]
fn dots_cargo_build_compiles_and_returns_bin_path() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    setup_minibin(repo, "fn main() {}\n");

    // 断言写在 Lua 里：返回产物绝对路径，且可执行
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local bin, err = dots.cargo.build(dots.repo .. "/minibin", "minibin")
    if not bin then error("build 应返回路径，err: " .. tostring(err)) end
    if not bin:find("minibin", 1, true) then error("路径不含 bin 名: " .. bin) end
    if not dots.run("test -x '" .. bin .. "'").ok then error("产物不可执行: " .. bin) end
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
}

#[test]
fn dots_cargo_build_failure_returns_nil_and_err() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    setup_minibin(repo, "fn main() { 编译不过 }\n");

    // 编译失败：nil + 错误信息，sync 整体不炸（优雅降级交给调用方分支）
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local bin, err = dots.cargo.build(dots.repo .. "/minibin", "minibin")
    if bin ~= nil then error("编译失败应返回 nil，得到: " .. tostring(bin)) end
    if type(err) ~= "string" or #err == 0 then error("第二返回值应带错误信息") end
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
}

#[test]
fn dots_cargo_build_skipped_on_dry_run() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);
    setup_minibin(repo, "fn main() {}\n");

    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local bin, why = dots.cargo.build(dots.repo .. "/minibin", "minibin")
    if bin ~= nil then error("dry-run 不应编译") end
    if why ~= "dry-run" then error("原因应为 dry-run: " .. tostring(why)) end
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync", "--dry-run"]).success();
    assert!(
        !repo.join("minibin/target").exists(),
        "dry-run 不应真的跑 cargo build"
    );
}

#[test]
fn dots_file_install_atomic_idempotent_preserves_mode() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 源：带可执行位的「二进制」
    let src = repo.join("payload.bin");
    fs::write(&src, "v1").unwrap();
    fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).unwrap();

    // dest 用 ~ 路径，验证展开
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    dots.file.install(dots.repo .. "/payload.bin", "~/bin/payload")
  end,
}
"#,
    )
    .unwrap();

    // dry-run 不落盘
    run_dots(repo, home, &["sync", "--dry-run"]).success();
    let dest = home.join("bin/payload");
    assert!(!dest.exists(), "dry-run 不应安装");

    // 真装：普通文件（非链）、内容、可执行位
    run_dots(repo, home, &["sync"]).success();
    let meta = fs::symlink_metadata(&dest).unwrap();
    assert!(meta.file_type().is_file(), "应为普通文件而非软链");
    assert_eq!(fs::read_to_string(&dest).unwrap(), "v1");
    assert_ne!(meta.permissions().mode() & 0o111, 0, "应保留可执行位");

    // 幂等：内容无差异不重写（rename 必换 inode，inode 不变即证明跳写）
    let ino_before = meta.ino();
    run_dots(repo, home, &["sync"]).success();
    assert_eq!(
        fs::metadata(&dest).unwrap().ino(),
        ino_before,
        "无差异应跳写"
    );

    // 源变更 → 内容跟进且 inode 更换（原子替换）
    fs::write(&src, "v2").unwrap();
    run_dots(repo, home, &["sync"]).success();
    assert_eq!(fs::read_to_string(&dest).unwrap(), "v2");
    assert_ne!(fs::metadata(&dest).unwrap().ino(), ino_before);
}

#[test]
fn dots_json_decode_parses_into_lua_table() {
    let repo_dir = tempdir().unwrap();
    let home_dir = tempdir().unwrap();
    let repo = repo_dir.path();
    let home = home_dir.path();
    setup_repo(repo);

    // 断言全写在 Lua 里：标量/数组/嵌套各型 + null→nil + 坏 JSON 双返回
    fs::write(
        repo.join("dots.lua"),
        r#"
on {
  post_sync = function()
    local obj = dots.json.decode('{"name":"cc-hook","num":3,"arr":[1,2],"nested":{"k":"v"},"nul":null}')
    if obj.name ~= "cc-hook" then error("string 字段: " .. tostring(obj.name)) end
    if obj.num ~= 3 then error("number 字段") end
    if obj.arr[2] ~= 2 then error("数组下标") end
    if obj.nested.k ~= "v" then error("嵌套表") end
    if obj.nul ~= nil then error("JSON null 必须映射为 Lua nil，得到: " .. tostring(obj.nul)) end

    local bad, err = dots.json.decode("not json")
    if bad ~= nil then error("坏 JSON 应返回 nil") end
    if type(err) ~= "string" or #err == 0 then error("第二返回值应带错误信息") end
  end,
}
"#,
    )
    .unwrap();

    run_dots(repo, home, &["sync"]).success();
}
