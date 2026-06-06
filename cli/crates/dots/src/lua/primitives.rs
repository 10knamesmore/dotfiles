//! effect 阶段写原语（§6.4）。
//!
//! 注册到 Lua：`vars`/`link`（per-host）、`dots.run_once`、`dots.json.merge`/`set`、
//! `dots.file.ensure_block`。统一保证：atomic（temp+rename）、无差异不写盘、
//! 读-改-写原语记 ownership（doctor 漂移检测）。

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mlua::{Lua, LuaSerdeExt, Table, Value};

use crate::hooks::EffectState;

/// 共享 effect 上下文。
type Eff = Rc<RefCell<EffectState>>;

/// 把 `~` 展开为 `$HOME`。
fn expand_home(path: &str, home: &Path) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else if path == "~" {
        home.to_owned()
    } else {
        PathBuf::from(path)
    }
}

/// 原子写：内容无差异则跳过。返回是否真写。
fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<bool> {
    if let Ok(existing) = fs::read(path) {
        if existing == bytes {
            return Ok(false);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("dots-tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(true)
}

/// 深合并 `overlay` 进 `base`（object 递归合并，其余覆盖）。
fn merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    use serde_json::Value::Object;
    match (base, overlay) {
        (Object(base_obj), Object(overlay_obj)) => {
            for (key, value) in overlay_obj {
                merge_json(
                    base_obj.entry(key).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (base_slot, overlay_val) => *base_slot = overlay_val,
    }
}

/// 安装全部 effect 原语到 Lua。
pub fn install(lua: &Lua, effect: &Eff) -> mlua::Result<()> {
    install_vars(lua, effect)?;
    install_link(lua, effect)?;
    install_dots_table(lua, effect)?;
    Ok(())
}

/// 安装 `vars` 原语：写入 per-host 变量表。
fn install_vars(lua: &Lua, effect: &Eff) -> mlua::Result<()> {
    let effect_ref = effect.clone();
    let func = lua.create_function(move |_, table: Table| {
        let mut st = effect_ref.borrow_mut();
        for pair in table.pairs::<String, String>() {
            let (key, value) = pair?;
            st.host_vars.insert(key, value);
        }
        Ok(())
    })?;
    lua.globals().set("vars", func)?;
    Ok(())
}

/// 安装 `link` 原语：追加额外的软链接条目。
fn install_link(lua: &Lua, effect: &Eff) -> mlua::Result<()> {
    let effect_ref = effect.clone();
    let func = lua.create_function(move |_, (src_rel, target): (String, String)| {
        let mut st = effect_ref.borrow_mut();
        let source = st.repo_root.join(&src_rel);
        let target = expand_home(&target, &st.home.clone());
        st.extra_links.push((source, target));
        Ok(())
    })?;
    lua.globals().set("link", func)?;
    Ok(())
}

/// 安装 `dots.{run_once,json,file}` 原语表。
fn install_dots_table(lua: &Lua, effect: &Eff) -> mlua::Result<()> {
    let dots: Table = lua.globals().get("dots")?;

    // dots.run_once(key, cmd)
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(move |_, (key, cmd): (String, String)| {
            let mut st = effect_ref.borrow_mut();
            if st.state.has_run(&key) {
                return Ok(false);
            }
            if !st.dry_run {
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .status()
                    .map_err(mlua::Error::external)?;
                if !status.success() {
                    return Err(mlua::Error::external(format!("run_once `{key}` 命令失败")));
                }
            }
            st.state.mark_run(key);
            Ok(true)
        })?;
        dots.set("run_once", func)?;
    }

    // dots.run(cmd)：每次 sync 都执行（dry-run 跳过），捕获输出富返回
    // { code, stdout, stderr, ok }。非零退出留一行告警但不致命——分支逻辑交给调用方。
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(move |lua, cmd: String| {
            let result = lua.create_table()?;
            if effect_ref.borrow().dry_run {
                result.set("code", 0)?;
                result.set("stdout", "")?;
                result.set("stderr", "")?;
                result.set("ok", true)?;
                return Ok(result);
            }
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .map_err(mlua::Error::external)?;
            // 被信号杀死等无退出码的情况归一为 -1
            let code = output.status.code().unwrap_or(-1);
            if code != 0 {
                crate::render::warn(&format!("dots.run 退出码 {code}：{cmd}"));
            }
            result.set("code", code)?;
            result.set("stdout", lua.create_string(&output.stdout)?)?;
            result.set("stderr", lua.create_string(&output.stderr)?)?;
            result.set("ok", code == 0)?;
            Ok(result)
        })?;
        dots.set("run", func)?;
    }

    // dots.json = { merge = fn, set = fn }
    let json = lua.create_table()?;
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(move |lua, (path, table): (String, Table)| {
            let st = effect_ref.borrow();
            let target = expand_home(&path, &st.home);
            let dry = st.dry_run;
            drop(st);
            let overlay: serde_json::Value = lua.from_value(Value::Table(table))?;
            json_merge_write(&target, overlay, &effect_ref, dry)?;
            Ok(())
        })?;
        json.set("merge", func)?;
    }
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(
            move |lua, (path, keypath, value): (String, String, Value)| {
                let st = effect_ref.borrow();
                let target = expand_home(&path, &st.home);
                let dry = st.dry_run;
                drop(st);
                let parsed: serde_json::Value = lua.from_value(value)?;
                let overlay = nest_keypath(&keypath, parsed);
                json_merge_write(&target, overlay, &effect_ref, dry)?;
                Ok(())
            },
        )?;
        json.set("set", func)?;
    }

    // dots.json.decode(text) → (table|nil, err?)：纯解析不碰盘。
    // JSON null 映射为 Lua nil（serialize_unit_to_null(false)），
    // 否则 truthy 的 null 哨兵会骗过 `if obj.field then` 判断。
    {
        let func = lua.create_function(|lua, text: String| {
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(parsed) => {
                    let options = mlua::SerializeOptions::new()
                        .serialize_none_to_null(false)
                        .serialize_unit_to_null(false);
                    let value = lua.to_value_with(&parsed, options)?;
                    Ok((value, Value::Nil))
                }
                Err(error) => Ok((
                    Value::Nil,
                    Value::String(lua.create_string(error.to_string())?),
                )),
            }
        })?;
        json.set("decode", func)?;
    }
    dots.set("json", json)?;

    // dots.cargo = { build = fn }：release 编译某 bin，返回产物绝对路径。
    // 走 --message-format=json 而非猜 target/ 路径——兼容 CARGO_TARGET_DIR 重定向。
    let cargo = lua.create_table()?;
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(move |lua, (dir, bin): (String, String)| {
            if effect_ref.borrow().dry_run {
                return Ok((Value::Nil, Value::String(lua.create_string("dry-run")?)));
            }
            match cargo_build_bin(&dir, &bin) {
                Ok(path) => Ok((Value::String(lua.create_string(&path)?), Value::Nil)),
                Err(reason) => {
                    crate::render::warn(&format!("dots.cargo.build 失败：{bin}"));
                    Ok((Value::Nil, Value::String(lua.create_string(&reason)?)))
                }
            }
        })?;
        cargo.set("build", func)?;
    }
    dots.set("cargo", cargo)?;

    // dots.file = { ensure_block = fn, install = fn }
    let file = lua.create_table()?;
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(
            move |_, (path, marker, content): (String, String, String)| {
                let st = effect_ref.borrow();
                let target = expand_home(&path, &st.home);
                let dry = st.dry_run;
                drop(st);
                ensure_block(&target, &marker, &content, dry).map_err(mlua::Error::external)?;
                Ok(())
            },
        )?;
        file.set("ensure_block", func)?;
    }
    {
        let effect_ref = effect.clone();
        let func = lua.create_function(move |_, (src, dest): (String, String)| {
            let st = effect_ref.borrow();
            let src = expand_home(&src, &st.home);
            let dest = expand_home(&dest, &st.home);
            let dry = st.dry_run;
            drop(st);
            if dry {
                println!("  ⇄ will install {} → {}", src.display(), dest.display());
                return Ok(());
            }
            install_file(&src, &dest).map_err(mlua::Error::external)?;
            Ok(())
        })?;
        file.set("install", func)?;
    }
    dots.set("file", file)?;

    Ok(())
}

/// `cargo build --release --bin <bin>` 并从 JSON 消息流取产物绝对路径。
/// 错误（spawn 失败/编译失败/找不到工件）统一归为 `Err(原因字符串)`，由 Lua 侧分支。
fn cargo_build_bin(dir: &str, bin: &str) -> Result<String, String> {
    let manifest = format!("{dir}/Cargo.toml");
    let output = std::process::Command::new("cargo")
        .args(["build", "--release", "--bin", bin, "--message-format=json"])
        .args(["--quiet", "--manifest-path", &manifest])
        .output()
        .map_err(|err| format!("无法启动 cargo：{err}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    // 逐行 JSON：取 compiler-artifact 中 target.name == bin 且 executable 非空的那条
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if msg["reason"] == "compiler-artifact" && msg["target"]["name"] == bin {
            if let Some(path) = msg["executable"].as_str() {
                return Ok(path.to_owned());
            }
        }
    }
    Err(format!("编译成功但未找到 bin `{bin}` 的产物路径"))
}

/// 原子安装文件：内容无差异跳写；否则 temp + rename（免 ETXTBSY），保留源权限位。
fn install_file(src: &Path, dest: &Path) -> std::io::Result<bool> {
    let bytes = fs::read(src)?;
    if let Ok(existing) = fs::read(dest) {
        if existing == bytes {
            return Ok(false);
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("dots-tmp");
    fs::write(&tmp, &bytes)?;
    fs::set_permissions(&tmp, fs::metadata(src)?.permissions())?;
    fs::rename(&tmp, dest)?;
    Ok(true)
}

/// 把 `a.b.c` + value 包成嵌套 object。
fn nest_keypath(keypath: &str, value: serde_json::Value) -> serde_json::Value {
    let mut nested = value;
    for seg in keypath.split('.').rev() {
        let mut obj = serde_json::Map::new();
        obj.insert(seg.to_owned(), nested);
        nested = serde_json::Value::Object(obj);
    }
    nested
}

/// 读-改-写 JSON：合并 overlay、记 ownership、atomic 写。dry-run 只声明意图。
fn json_merge_write(
    target: &Path,
    overlay: serde_json::Value,
    effect: &Eff,
    dry: bool,
) -> mlua::Result<()> {
    let top_keys: Vec<String> = overlay
        .as_object()
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();
    if dry {
        println!(
            "  ⇄ will merge {:?} into {} (preserve other keys)",
            top_keys,
            target.display()
        );
        return Ok(());
    }
    let mut base: serde_json::Value = match fs::read_to_string(target) {
        Ok(text) if !text.trim().is_empty() => {
            serde_json::from_str(&text).map_err(mlua::Error::external)?
        }
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };
    merge_json(&mut base, overlay);
    let pretty = serde_json::to_string_pretty(&base).map_err(mlua::Error::external)?;
    atomic_write(target, pretty.as_bytes()).map_err(mlua::Error::external)?;
    // 记 ownership
    let mut st = effect.borrow_mut();
    for key in top_keys {
        st.state.record_own(target.to_owned(), key);
    }
    Ok(())
}

/// 文本 managed-block：用 marker 包裹的区间幂等替换。
fn ensure_block(target: &Path, marker: &str, content: &str, dry: bool) -> std::io::Result<()> {
    let begin = format!("# >>> dots:{marker} >>>");
    let end = format!("# <<< dots:{marker} <<<");
    let block = format!("{begin}\n{content}\n{end}\n");
    let existing = fs::read_to_string(target).unwrap_or_default();
    let rebuilt = match (existing.find(&begin), existing.find(&end)) {
        (Some(begin_pos), Some(end_pos)) if end_pos > begin_pos => {
            let after = existing[end_pos..]
                .find('\n')
                .map(|offset| end_pos + offset + 1)
                .unwrap_or(existing.len());
            format!("{}{}{}", &existing[..begin_pos], block, &existing[after..])
        }
        _ => {
            if existing.is_empty() {
                block
            } else {
                format!("{existing}\n{block}")
            }
        }
    };
    if dry {
        println!("  ⇄ will ensure block `{marker}` in {}", target.display());
        return Ok(());
    }
    atomic_write(target, rebuilt.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn merge_preserves_other_keys() {
        let mut base: serde_json::Value = serde_json::json!({"model":"opus","theme":"dark"});
        let overlay = serde_json::json!({"hooks":{"Stop":[]}});
        merge_json(&mut base, overlay);
        assert_eq!(base["model"], "opus"); // 保留
        assert_eq!(base["theme"], "dark");
        assert!(base["hooks"].is_object()); // 新增
    }

    #[test]
    fn nest_keypath_builds_nested() {
        let v = nest_keypath("hooks.Stop", serde_json::json!([1, 2]));
        assert_eq!(v, serde_json::json!({"hooks":{"Stop":[1,2]}}));
    }

    #[test]
    fn merge_is_idempotent() {
        let mut a = serde_json::json!({"x":1});
        let o = serde_json::json!({"hooks":{"k":"v"}});
        merge_json(&mut a, o.clone());
        let snapshot = a.clone();
        merge_json(&mut a, o);
        assert_eq!(a, snapshot); // fn(fn(x)) == fn(x)
    }
}
