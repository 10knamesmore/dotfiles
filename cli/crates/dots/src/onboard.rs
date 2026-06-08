//! 新机 host 引导（onboarding）：交互收集别名/工具链组，写 `~/.config/dots/host`
//! 并把 host 块插进 `dots.lua`。
//!
//! 设计取舍（B 方案）：别名进 `dots.lua`（共享、入 git），真实主机名只落机器本地的
//! `~/.config/dots/host`（不入库），`hosts::current()` 优先读它来匹配。

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use crate::Result;
use crate::lua::{LuaCtx, eval_manifest};

/// 工具链组选择（onboarding 的回答）。
pub enum ToolchainChoice {
    /// 不写 `toolchains()` = 全装。
    All,
    /// 只装列出的组：`toolchains({ only = {…} })`。
    Only(Vec<String>),
}

/// 新机 host 引导：当前机未在 `hosts{}` 登记、且 stdin 是交互终端时，
/// 交互收集别名/工具链组，写 `~/.config/dots/host` 并把 host 块插进 `dots.lua`。
///
/// 已登记 / 非交互 / 用户回车跳过 → 无副作用返回（装机继续，靠 sync 的「未命中非致命」兜底）。
pub fn maybe_onboard(repo_root: &Path, home: &Path) -> Result<()> {
    let hostname = crate::hosts::current();
    if host_known(repo_root, &hostname)? {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        crate::render::warn("未识别主机且非交互终端：跳过 host 引导，仅链通用配置");
        return Ok(());
    }

    crate::render::warn(&format!("未识别这台机器（探测名：{hostname}）。做一次 host 引导："));
    let alias = prompt_line("  别名（写进 dots.lua，建议不暴露身份，如 mini/cloud；回车跳过）：")?;
    let alias = alias.trim();
    if alias.is_empty() {
        crate::render::warn("未输入别名，跳过 host 引导");
        return Ok(());
    }
    let choice = prompt_toolchains()?;
    apply_onboarding(repo_root, home, alias, &choice)?;
    crate::render::ok(&format!(
        "已写入别名 `{alias}` → {} 与 dots.lua host 块",
        crate::hosts::host_file_path(home).path().display()
    ));
    Ok(())
}

/// 当前主机名是否已在 dots.lua 的 `hosts{}` 登记（eval 一次取 host_blocks）。
fn host_known(repo_root: &Path, hostname: &str) -> Result<bool> {
    let src = fs::read_to_string(repo_root.join("dots.lua")).unwrap_or_default();
    if src.trim().is_empty() {
        return Ok(false);
    }
    let ctx = LuaCtx {
        host: hostname.to_owned(),
        os: crate::cmd::os_str(crate::cmd::current_os()).to_owned(),
        home: crate::cmd::home_dir()?.display().to_string(),
        repo: repo_root.display().to_string(),
    };
    let (manifest, _handles) = eval_manifest(&src, &ctx)?;
    Ok(manifest.host_blocks.contains_key(hostname))
}

/// 落盘 onboarding 结果：写别名文件 + 把 host 块插进 dots.lua。
///
/// 纯文件副作用（不读 stdin），便于测试。
fn apply_onboarding(
    repo_root: &Path,
    home: &Path,
    alias: &str,
    choice: &ToolchainChoice,
) -> Result<()> {
    let host_file = crate::hosts::host_file_path(home);
    if let Some(parent) = host_file.path().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(host_file.path(), format!("{alias}\n"))?;

    let dots_lua = repo_root.join("dots.lua");
    let src = fs::read_to_string(&dots_lua).unwrap_or_default();
    let entry = render_host_block(alias, choice);
    fs::write(&dots_lua, insert_host_block(&src, &entry))?;
    Ok(())
}

/// 打印提示（不换行）、flush、读一行（去掉尾换行）。
fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_owned())
}

/// 问工具链范围：回车 = 全装；否则空格分隔的 only 组列表。
fn prompt_toolchains() -> Result<ToolchainChoice> {
    let line = prompt_line("  工具链组：回车=全装，或空格分隔 only 列表（core dev ai js）：")?;
    let groups: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    Ok(if groups.is_empty() {
        ToolchainChoice::All
    } else {
        ToolchainChoice::Only(groups)
    })
}

/// 渲染单个 host 块条目（缩进好、可直接插进 `hosts({` 表内）。
///
/// 末尾带逗号，作为表项；vars/link 以注释骨架给出，按需手填。
fn render_host_block(alias: &str, toolchains: &ToolchainChoice) -> String {
    let tc_line = match toolchains {
        ToolchainChoice::All => "        -- toolchains 不声明 = 全装".to_owned(),
        ToolchainChoice::Only(groups) => {
            let list = groups
                .iter()
                .map(|group| format!("\"{group}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("        toolchains({{ only = {{ {list} }} }})")
        }
    };
    [
        format!("    [\"{alias}\"] = function()"),
        tc_line,
        "        -- vars({ backlight = \"\", ddc_index = \"\" })   -- 按需手填（Linux 硬件参数）"
            .to_owned(),
        format!(
            "        -- link(\"hosts/files/{alias}/monitors.conf\", \"~/.config/hypr/monitors.conf\")"
        ),
        "    end,".to_owned(),
        String::new(), // 末尾换行
    ]
    .join("\n")
}

/// 某行是否是 `hosts` 调用的开括号行（`hosts({` / `hosts {`）。
///
/// 锚点判据：trim 后以 `hosts` 起头，紧随的首个非空白是 `(` 或 `{`，且本行含 `{`
/// （把开括号留在本行，插入点才落在表内第一项之前）。注释行、`["hosts"]` 等不误命中。
fn is_hosts_open(line: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("hosts") else {
        return false;
    };
    let rest = rest.trim_start();
    (rest.starts_with('(') || rest.starts_with('{')) && line.contains('{')
}

/// 把 host 块条目插入 `dots.lua` 源码。
///
/// 优先插在现有 `hosts({`（或 `hosts {`）开括号行的正下方——只锚定开括号行，
/// 不必匹配配对的 `})`，对嵌套 `function()…end` / `vars{}` 免疫。
/// 没有现有 `hosts` 调用则在文件尾追加一个新的 `hosts({ … })`（多次调用会被 eval 合并）。
fn insert_host_block(src: &str, entry: &str) -> String {
    let anchor = src.lines().position(is_hosts_open);
    let Some(idx) = anchor else {
        // 无现有 hosts 调用：文件尾追加一个新块。
        let mut out = src.to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\nhosts({\n");
        out.push_str(entry);
        out.push_str("})\n");
        return out;
    };
    let mut out = String::new();
    for (i, line) in src.lines().enumerate() {
        out.push_str(line);
        out.push('\n');
        if i == idx {
            out.push_str(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn render_all_omits_toolchains_call() {
        let out = render_host_block("mini", &ToolchainChoice::All);
        assert!(out.contains(r#"["mini"] = function()"#), "应含别名 key：{out}");
        assert!(out.contains("end,"), "应是带逗号的表项：{out}");
        assert!(
            !out.contains("toolchains({ only"),
            "All 不应写 toolchains() 调用：{out}"
        );
    }

    #[test]
    fn render_only_emits_toolchains_filter() {
        let out = render_host_block("cloud", &ToolchainChoice::Only(vec!["core".to_owned()]));
        assert!(out.contains(r#"["cloud"] = function()"#));
        assert!(
            out.contains(r#"toolchains({ only = { "core" } })"#),
            "应写 only 过滤：{out}"
        );
    }

    #[test]
    fn render_only_multiple_groups_comma_joined() {
        let out = render_host_block(
            "dev",
            &ToolchainChoice::Only(vec!["core".to_owned(), "dev".to_owned()]),
        );
        assert!(
            out.contains(r#"only = { "core", "dev" }"#),
            "多组应逗号连接：{out}"
        );
    }

    #[test]
    fn insert_places_entry_below_existing_hosts_open() {
        let src = "granularity(\"x\", {})\n\nhosts({\n    [\"old\"] = function()\n    end,\n})\n";
        let entry = "    [\"new\"] = function()\n    end,\n";
        let out = insert_host_block(src, entry);
        let new_pos = out.find(r#"["new"]"#);
        let old_pos = out.find(r#"["old"]"#);
        assert!(
            new_pos.is_some() && old_pos.is_some() && new_pos < old_pos,
            "新条目应插在旧条目之前：\n{out}"
        );
        assert!(out.contains("granularity"), "其余内容应保留：\n{out}");
    }

    #[test]
    fn insert_is_immune_to_nested_braces() {
        // 嵌套 vars{} 的 } 不应干扰锚点（只认 hosts({ 开括号行）。
        let src = "hosts({\n    [\"a\"] = function() vars({ x = \"1\" }) end,\n})\n";
        let entry = "    [\"b\"] = function()\n    end,\n";
        let out = insert_host_block(src, entry);
        let b_pos = out.find(r#"["b"]"#);
        let a_pos = out.find(r#"["a"]"#);
        assert!(
            b_pos.is_some() && a_pos.is_some() && b_pos < a_pos,
            "b 应紧跟 hosts({{ 行、在 a 之前：\n{out}"
        );
    }

    #[test]
    fn apply_onboarding_writes_alias_file_and_edits_dots_lua() -> crate::Result<()> {
        let repo = tempfile::tempdir()?;
        let home = tempfile::tempdir()?;
        fs::write(repo.path().join("dots.lua"), "hosts({\n})\n")?;

        apply_onboarding(
            repo.path(),
            home.path(),
            "mini",
            &ToolchainChoice::Only(vec!["core".to_owned()]),
        )?;

        // 别名落机器本地文件（不进 dots.lua/git）
        let host_file = home.path().join(".config/dots/host");
        assert_eq!(fs::read_to_string(&host_file)?.trim(), "mini");

        // dots.lua 被编辑：含别名块 + only 过滤
        let lua = fs::read_to_string(repo.path().join("dots.lua"))?;
        assert!(lua.contains(r#"["mini"] = function()"#), "应插入别名块：\n{lua}");
        assert!(
            lua.contains(r#"toolchains({ only = { "core" } })"#),
            "应含 only 过滤：\n{lua}"
        );
        Ok(())
    }

    #[test]
    fn generated_block_parses_and_registers_via_eval() -> crate::Result<()> {
        // 真实 dots.lua 形态：嵌套 vars{}、多条目、注释——插入后必须仍是合法 Lua。
        let src = "granularity(\"x\", { mode = \"file\" })\n\n\
                   hosts({\n    [\"arch\"] = function()\n        vars({ backlight = \"x\" })\n    end,\n})\n";
        let entry = render_host_block("cloud", &ToolchainChoice::Only(vec!["core".to_owned()]));
        let updated = insert_host_block(src, &entry);

        let ctx = LuaCtx {
            host: "cloud".to_owned(),
            os: "linux".to_owned(),
            home: "/home/u".to_owned(),
            repo: "/repo".to_owned(),
        };
        let (manifest, _handles) = eval_manifest(&updated, &ctx)?;
        assert!(
            manifest.host_blocks.contains_key("cloud"),
            "新别名块应注册：\n{updated}"
        );
        assert!(
            manifest.host_blocks.contains_key("arch"),
            "原有块应保留：\n{updated}"
        );
        Ok(())
    }

    #[test]
    fn insert_appends_new_hosts_call_when_absent() {
        let src = "granularity(\"x\", {})\n";
        let entry = "    [\"solo\"] = function()\n    end,\n";
        let out = insert_host_block(src, entry);
        assert!(out.contains("granularity"), "原内容保留：\n{out}");
        assert!(out.contains("hosts({"), "应追加 hosts({{ 调用：\n{out}");
        assert!(out.contains(r#"["solo"]"#), "应含新条目：\n{out}");
        // 追加的块必须语法闭合
        assert!(out.trim_end().ends_with("})"), "追加块应闭合：\n{out}");
    }
}
