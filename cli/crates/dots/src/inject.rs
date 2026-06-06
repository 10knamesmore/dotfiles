//! minijinja `.inject` 渲染（路径注入 B，§6）。
//!
//! 用于 dots 完全拥有、整文件渲染的产物（systemd unit 等）。生成型确定性：
//! 不读目标当前值，上下文只来自仓库/host/secret。

use minijinja::{Environment, context};
use rustc_hash::FxHashMap;

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 渲染上下文。
#[derive(Default)]
pub struct InjectCtx {
    /// 仓库根绝对路径（`{{ DOTFILES }}`）。
    pub dotfiles: String,
    /// 聚合脚本目录（`{{ SCRIPTS }}`）。
    pub scripts: String,
    /// 当前机 vars（`{{ host.x }}`）。
    pub host: FxHashMap<String, String>,
    /// 解密的 secrets（`{{ secret.k }}`）。
    pub secret: FxHashMap<String, String>,
}

/// 渲染一段 `.inject` 模板源。
///
/// # Params:
///   - `src`: 模板内容（`{{ DOTFILES }}` / `{{ host.x }}` / `{{ secret.k }}`）
///   - `ctx`: 渲染上下文
///
/// # Return:
///   渲染产物；缺变量时报错（= doctor「未解析变量」检查）。
pub fn render(src: &str, ctx: &InjectCtx) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_template("inject", src)?;
    let tpl = env.get_template("inject")?;
    let out = tpl.render(context! {
        DOTFILES => ctx.dotfiles,
        SCRIPTS => ctx.scripts,
        host => ctx.host,
        secret => ctx.secret,
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    fn ctx() -> InjectCtx {
        let mut host = FxHashMap::default();
        host.insert("backlight".into(), "amdgpu_bl1".into());
        let mut secret = FxHashMap::default();
        secret.insert("bsu_pass".into(), "hunter2".into());
        InjectCtx {
            dotfiles: "/home/u/dotfiles".into(),
            scripts: "/home/u/dotfiles/.gen/scripts".into(),
            host,
            secret,
        }
    }

    #[test]
    fn renders_builtin_and_host_and_secret() -> Result<()> {
        let out = render(
            "D={{ DOTFILES }} B={{ host.backlight }} P={{ secret.bsu_pass }}",
            &ctx(),
        )?;
        assert_eq!(out, "D=/home/u/dotfiles B=amdgpu_bl1 P=hunter2");
        Ok(())
    }

    #[test]
    fn missing_variable_errors() {
        let r = render("{{ host.nonexistent }}", &ctx());
        assert!(r.is_err(), "缺变量应报错（strict 模式）");
    }

    #[test]
    fn default_filter_works() -> Result<()> {
        let out = render("{{ host.nope | default('fallback') }}", &ctx())?;
        assert_eq!(out, "fallback");
        Ok(())
    }
}
