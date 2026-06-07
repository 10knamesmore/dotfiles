//! 子命令的统一返回值：业务函数不做 IO，落地交给 bin 的 wire 层。

use serde::Serialize;

/// 一次 hook 调用的完整结果。
///
/// - `output`：结构化 stdout 决策（named struct，由 wire 序列化）；`None` = 静默
/// - `notice`：stderr 留痕（fail-open 告警，`claude --debug` 可见）
/// - `code`：进程退出码。PreToolUse fail-open 恒 0；其他 hook 事件可用 2（stderr 喂回模型）
#[derive(Debug)]
pub struct HookRun<T: Serialize> {
    /// 结构化 stdout 决策
    pub output: Option<T>,
    /// stderr 留痕
    pub notice: Option<String>,
    /// 进程退出码
    pub code: i32,
}

impl<T: Serialize> HookRun<T> {
    /// 无意见：静默放行。
    #[must_use]
    pub fn silent() -> Self {
        Self { output: None, notice: None, code: 0 }
    }

    /// 输出一条结构化决策。
    #[must_use]
    pub fn decision(output: T) -> Self {
        Self { output: Some(output), notice: None, code: 0 }
    }

    /// 附加 stderr 留痕。
    #[must_use]
    pub fn with_notice(mut self, notice: String) -> Self {
        self.notice = Some(notice);
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
        assert!(silent.output.is_none() && silent.notice.is_none() && silent.code == 0);

        let noticed: HookRun<serde_json::Value> =
            HookRun::silent().with_notice("解析失败".into());
        assert_eq!(noticed.notice.as_deref(), Some("解析失败"));
        assert_eq!(noticed.code, 0, "fail-open：留痕不改退出码");

        let decided = HookRun::decision(serde_json::json!({"k": "v"}));
        assert!(decided.output.is_some());
    }
}
