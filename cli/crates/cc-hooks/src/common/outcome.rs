//! 子命令的统一返回值：业务函数不做 IO，落地交给 bin 的 wire 层。

use serde::Serialize;

/// 一次 hook 调用的完整结果。
///
/// - `output`：结构化 stdout 决策（named struct，由 wire 序列化）；`None` = 静默
/// - `notice`：stderr 留痕（fail-open 告警，`claude --debug` 可见）
/// - `audit`：审计语义行（决策/规则/命令），由 wire 层加时间戳后追加进审计日志
/// - `code`：进程退出码。PreToolUse fail-open 恒 0；其他 hook 事件可用 2（stderr 喂回模型）
#[derive(Debug)]
pub struct HookRun<T: Serialize> {
    /// 结构化 stdout 决策
    pub output: Option<T>,
    /// stderr 留痕
    pub notice: Option<String>,
    /// 审计日志语义行（不含时间戳，wire 层落盘时补）
    pub audit: Option<String>,
    /// 进程退出码
    pub code: i32,
}

impl<T: Serialize> HookRun<T> {
    /// 无意见：静默放行。
    #[must_use]
    pub fn silent() -> Self {
        Self {
            output: None,
            notice: None,
            audit: None,
            code: 0,
        }
    }

    /// 输出一条结构化决策。
    #[must_use]
    pub fn decision(output: T) -> Self {
        Self {
            output: Some(output),
            notice: None,
            audit: None,
            code: 0,
        }
    }

    /// 附加 stderr 留痕。
    #[must_use]
    pub fn with_notice(mut self, notice: String) -> Self {
        self.notice = Some(notice);
        self
    }

    /// 附加审计语义行（命中决策时记录「拦了/放了什么」，供 wire 层落盘）。
    #[must_use]
    pub fn with_audit(mut self, audit: String) -> Self {
        self.audit = Some(audit);
        self
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    #[test]
    fn constructors_set_expected_fields() {
        let silent: HookRun<serde_json::Value> = HookRun::silent();
        assert!(silent.output.is_none() && silent.notice.is_none() && silent.audit.is_none());
        assert_eq!(silent.code, 0);

        let noticed: HookRun<serde_json::Value> = HookRun::silent().with_notice("解析失败".into());
        assert_eq!(noticed.notice.as_deref(), Some("解析失败"));
        assert_eq!(noticed.code, 0, "fail-open：留痕不改退出码");

        let decided = HookRun::decision(serde_json::json!({"k": "v"}));
        assert!(decided.output.is_some() && decided.audit.is_none());
    }

    #[test]
    fn with_audit_attaches_semantic_line() {
        let run = HookRun::decision(serde_json::json!({"k": "v"}))
            .with_audit("decision=deny rule=rm-recursive-force".into());
        assert_eq!(
            run.audit.as_deref(),
            Some("decision=deny rule=rm-recursive-force")
        );
        assert_eq!(run.code, 0, "审计落盘不改 fail-open 退出码");
    }
}
