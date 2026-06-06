//! 命令行词法：链式切段、shlex 分词、短旗标收集。

use std::collections::BTreeSet;

/// 把链式命令按 `;` `&&` `||` `|` 与换行切段，每段 shlex 分词。
///
/// 解析失败（引号不闭合等）的段直接丢弃——fail-open；空段不产出。
pub fn each_command(raw: &str) -> Vec<Vec<String>> {
    raw.split([';', '\n', '|', '&'])
        .filter_map(shlex::split)
        .filter(|argv| !argv.is_empty())
        .collect()
}

/// 收集短旗标字母：`-rf` → `{r, f}`。
///
/// 只认 `-` + 纯 ASCII 字母的词，负数参数（`-1`）与长旗标（`--force`）不收。
pub fn short_flags(argv: &[String]) -> BTreeSet<char> {
    argv.iter()
        .filter_map(|word| word.strip_prefix('-'))
        .filter(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_alphabetic()))
        .flat_map(str::chars)
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use super::*;

    /// 便捷构造：&str 列表 → Vec<String>。
    fn owned(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn splits_chained_and_piped_commands() {
        let segments = each_command("cd /tmp && rm -fr build | cat");
        assert_eq!(
            segments,
            vec![owned(&["cd", "/tmp"]), owned(&["rm", "-fr", "build"]), owned(&["cat"])]
        );
    }

    #[test]
    fn newline_splits_commands() {
        let segments = each_command("echo hi\nrm -rf x");
        assert_eq!(segments, vec![owned(&["echo", "hi"]), owned(&["rm", "-rf", "x"])]);
    }

    #[test]
    fn quoted_payload_stays_single_token() {
        let segments = each_command(r#"echo "rm -rf /""#);
        assert_eq!(segments, vec![owned(&["echo", "rm -rf /"])]);
    }

    #[test]
    fn unparseable_segment_is_skipped() {
        // && 在引号内仍按文本切段：前半段引号不闭合被丢弃，后半段正常解析
        let segments = each_command(r#"echo "unclosed && rm -rf x"#);
        assert_eq!(segments, vec![owned(&["rm", "-rf", "x"])]);
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert_eq!(each_command(""), Vec::<Vec<String>>::new());
    }

    #[test]
    fn short_flags_collects_cluster_letters() {
        let argv = owned(&["rm", "-rf", "--force", "-1", "x"]);
        assert_eq!(short_flags(&argv), BTreeSet::from(['r', 'f']));
    }

    #[test]
    fn short_flags_ignores_non_flag_words() {
        let argv = owned(&["rm", "my-perf-report.txt"]);
        assert_eq!(short_flags(&argv), BTreeSet::new());
    }
}
