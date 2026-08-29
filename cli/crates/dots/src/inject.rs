//! minijinja `.inject` 路径渲染。
//!
//! 用于 dots 完全拥有、整文件渲染的产物（systemd unit 等）。生成型确定性：
//! 不读目标当前值，上下文只来自仓库路径。

use minijinja::{Environment, context};

/// 通用 Result 别名。
pub type Result<T> = color_eyre::Result<T>;

/// 渲染上下文。
#[derive(Default)]
pub struct InjectCtx {
    /// 仓库根绝对路径（`{{ DOTFILES }}`）。
    pub dotfiles: String,
    /// 聚合脚本目录（`{{ SCRIPTS }}`）。
    pub scripts: String,
}

/// 渲染一段 `.inject` 模板源。
///
/// # Params:
///   - `src`: 模板内容（`{{ DOTFILES }}` / `{{ SCRIPTS }}`）
///   - `ctx`: 渲染上下文
///
/// # Return:
///   渲染产物；缺变量时报错。
pub fn render(src: &str, ctx: &InjectCtx) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.add_template("inject", src)?;
    let tpl = env.get_template("inject")?;
    let out = tpl.render(context! {
        DOTFILES => ctx.dotfiles,
        SCRIPTS => ctx.scripts,
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    fn ctx() -> InjectCtx {
        InjectCtx {
            dotfiles: "/home/u/dotfiles".into(),
            scripts: "/home/u/dotfiles/.gen/scripts".into(),
        }
    }

    #[test]
    fn renders_builtin_paths() -> Result<()> {
        let out = render("D={{ DOTFILES }} S={{ SCRIPTS }}", &ctx())?;
        assert_eq!(out, "D=/home/u/dotfiles S=/home/u/dotfiles/.gen/scripts");
        Ok(())
    }

    #[test]
    fn missing_variable_errors() {
        let r = render("{{ nonexistent }}", &ctx());
        assert!(r.is_err(), "缺变量应报错（strict 模式）");
    }

    #[test]
    fn default_filter_works() -> Result<()> {
        let out = render("{{ nonexistent | default('fallback') }}", &ctx())?;
        assert_eq!(out, "fallback");
        Ok(())
    }
}
