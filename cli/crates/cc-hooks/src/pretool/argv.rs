//! 命令行词法：链式切段、shlex 分词、短旗标收集。

use std::collections::BTreeSet;
use std::iter::Peekable;
use std::str::Chars;

/// 把链式命令按 `;` `&&` `||` `|` `&` 与换行切段，每段 shlex 分词。
///
/// 切段是引号感知的：单/双引号与反斜杠转义内的分隔符不切；
/// heredoc（`<<WORD`）正文整体剥除，不参与匹配。
/// 解析失败（引号不闭合等）的段直接丢弃——fail-open；空段不产出。
pub fn each_command(raw: &str) -> Vec<Vec<String>> {
    split_segments(raw)
        .iter()
        .filter_map(|segment| shlex::split(segment))
        .filter(|argv| !argv.is_empty())
        .collect()
}

/// 收集短旗标字母：`-rf` → `{r, f}`。
///
/// 只认 `-` + 纯 ASCII 字母的词，负数参数（`-1`）与长旗标（`--force`）不收；
/// 字面 `--` 之后的词是操作数（POSIX 约定），停止收集。
pub fn short_flags(argv: &[String]) -> BTreeSet<char> {
    argv.iter()
        .take_while(|word| word.as_str() != "--")
        .filter_map(|word| word.strip_prefix('-'))
        .filter(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_alphabetic()))
        .flat_map(str::chars)
        .collect()
}

/// 引号/heredoc 感知切段：返回原文片段（仍含引号，交给 shlex 分词）。
fn split_segments(raw: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    // 本行已声明、待跳过正文的 heredoc 定界符（一行可有多个：`cat <<A <<B`）
    let mut heredocs: Vec<String> = Vec::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            // 单引号：无转义，直拷到闭合
            '\'' => {
                current.push(ch);
                for inner in chars.by_ref() {
                    current.push(inner);
                    if inner == '\'' {
                        break;
                    }
                }
            }
            // 双引号：反斜杠可转义，拷到闭合
            '"' => {
                current.push(ch);
                while let Some(inner) = chars.next() {
                    current.push(inner);
                    match inner {
                        '"' => break,
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                current.push(escaped);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // 引号外的反斜杠：连同下一字符原样保留
            '\\' => {
                current.push(ch);
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            '<' if chars.peek() == Some(&'<') => {
                chars.next();
                if chars.peek() == Some(&'<') {
                    // `<<<` herestring：保留参与分词（前后补空格防粘连）
                    chars.next();
                    current.push_str(" <<< ");
                } else if let Some(delim) = heredoc_delimiter(&mut chars) {
                    // `<<WORD` 从段文本剥除，定界符入队等行尾跳正文
                    heredocs.push(delim);
                }
            }
            '\n' => {
                flush(&mut segments, &mut current);
                skip_heredoc_bodies(&mut chars, &mut heredocs);
            }
            ';' | '|' | '&' => flush(&mut segments, &mut current),
            _ => current.push(ch),
        }
    }
    flush(&mut segments, &mut current);
    segments
}

/// 收当前段进结果：纯空白段丢弃。
fn flush(segments: &mut Vec<String>, current: &mut String) {
    let segment = std::mem::take(current);
    if !segment.trim().is_empty() {
        segments.push(segment);
    }
}

/// 解析 heredoc 定界符：`<<` 之后的可选 `-`、空白、词（可引号包裹）。
///
/// 解析不出词形（如算术 `2<<3` 之外的怪输入）返回 `None`，当普通文本忽略。
fn heredoc_delimiter(chars: &mut Peekable<Chars>) -> Option<String> {
    if chars.peek() == Some(&'-') {
        chars.next();
    }
    while chars.peek().is_some_and(|ch| *ch == ' ' || *ch == '\t') {
        chars.next();
    }
    let mut delim = String::new();
    if let Some(&quote) = chars.peek().filter(|ch| matches!(ch, '\'' | '"')) {
        chars.next();
        for inner in chars.by_ref() {
            if inner == quote {
                break;
            }
            delim.push(inner);
        }
    } else {
        while let Some(&word_ch) = chars.peek() {
            if word_ch.is_alphanumeric() || word_ch == '_' {
                delim.push(word_ch);
                chars.next();
            } else {
                break;
            }
        }
    }
    (!delim.is_empty()).then_some(delim)
}

/// 跳过 heredoc 正文：逐定界符消费行，直到独立的定界符行（容忍 `<<-` 缩进）。
///
/// 输入耗尽（heredoc 未闭合）即返回——余下正文不参与匹配，fail-open。
fn skip_heredoc_bodies(chars: &mut Peekable<Chars>, heredocs: &mut Vec<String>) {
    for delim in heredocs.drain(..) {
        loop {
            match read_line(chars) {
                None => return,
                Some(line) if line.trim() == delim => break,
                Some(_) => {}
            }
        }
    }
}

/// 读一行（不含换行符）；输入耗尽返回 `None`。
fn read_line(chars: &mut Peekable<Chars>) -> Option<String> {
    chars.peek()?;
    let mut line = String::new();
    for ch in chars.by_ref() {
        if ch == '\n' {
            break;
        }
        line.push(ch);
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]
    use proptest::prelude::*;

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
            vec![
                owned(&["cd", "/tmp"]),
                owned(&["rm", "-fr", "build"]),
                owned(&["cat"])
            ]
        );
    }

    #[test]
    fn newline_splits_commands() {
        let segments = each_command("echo hi\nrm -rf x");
        assert_eq!(
            segments,
            vec![owned(&["echo", "hi"]), owned(&["rm", "-rf", "x"])]
        );
    }

    #[test]
    fn quoted_payload_stays_single_token() {
        let segments = each_command(r#"echo "rm -rf /""#);
        assert_eq!(segments, vec![owned(&["echo", "rm -rf /"])]);
    }

    #[test]
    fn quoted_separators_do_not_split() {
        // 引号内的 ; | & 不切段：commit message 含危险文本不误伤
        let segments = each_command(r#"git commit -m "fix; rm -rf temp; done""#);
        assert_eq!(
            segments,
            vec![owned(&["git", "commit", "-m", "fix; rm -rf temp; done"])]
        );
        let single = each_command("echo 'a && b | c'");
        assert_eq!(single, vec![owned(&["echo", "a && b | c"])]);
    }

    #[test]
    fn backslash_escaped_separator_does_not_split() {
        let segments = each_command(r"echo a\;b");
        assert_eq!(segments, vec![owned(&["echo", "a;b"])]);
    }

    #[test]
    fn unclosed_quote_fails_open() {
        // 引号不闭合：整串成一段且 shlex 失败 → 丢弃（fail-open）
        let segments = each_command(r#"echo "unclosed && rm -rf x"#);
        assert_eq!(segments, Vec::<Vec<String>>::new());
    }

    #[test]
    fn heredoc_body_is_stripped() {
        let segments = each_command("cat <<EOF\nrm -rf /tmp/x\nEOF\necho done");
        assert_eq!(segments, vec![owned(&["cat"]), owned(&["echo", "done"])]);
    }

    #[test]
    fn heredoc_quoted_delimiter_and_dash_variants() {
        let quoted = each_command("cat <<'EOF'\nrm -rf x\nEOF");
        assert_eq!(quoted, vec![owned(&["cat"])]);
        // <<- 容忍正文与定界符的缩进
        let dashed = each_command("cat <<-EOF\n\trm -rf x\n\tEOF\nls");
        assert_eq!(dashed, vec![owned(&["cat"]), owned(&["ls"])]);
    }

    #[test]
    fn multiple_heredocs_on_one_line() {
        let segments = each_command("cat <<A <<B\nrm -rf x\nA\nrm -rf y\nB\nls");
        assert_eq!(segments, vec![owned(&["cat"]), owned(&["ls"])]);
    }

    #[test]
    fn unterminated_heredoc_swallows_rest() {
        let segments = each_command("cat <<EOF\nrm -rf x");
        assert_eq!(segments, vec![owned(&["cat"])]);
    }

    #[test]
    fn herestring_is_not_heredoc() {
        let segments = each_command(r#"cat <<< "rm -rf x""#);
        assert_eq!(segments, vec![owned(&["cat", "<<<", "rm -rf x"])]);
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

    #[test]
    fn short_flags_stops_at_double_dash() {
        // `rm -- -rf` 删的是名为 -rf 的文件，不是递归强制
        assert_eq!(short_flags(&owned(&["rm", "--", "-rf"])), BTreeSet::new());
        assert_eq!(
            short_flags(&owned(&["rm", "-rf", "--", "x"])),
            BTreeSet::from(['r', 'f'])
        );
    }

    proptest! {
        /// fail-open 不变量：任意命令串（含未闭合引号/heredoc/控制字符）都不 panic、必终止，
        /// 且产出的每段 argv 非空。守住「词法层绝不崩、绝不锁死」。
        #[test]
        fn each_command_never_panics_and_segments_nonempty(raw in any::<String>()) {
            for segment in each_command(&raw) {
                prop_assert!(!segment.is_empty());
            }
        }

        /// 任意 argv：短旗标收集纯函数恒终止、不 panic。
        #[test]
        fn short_flags_never_panics(words in prop::collection::vec(any::<String>(), 0..8)) {
            let _ = short_flags(&words);
        }
    }
}
