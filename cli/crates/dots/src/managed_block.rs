//! marker block 的结构解析、幂等更新与安全删除。

use std::ops::Range;

use dots_core::ManagedBlockPlacement;
use dots_core::normalize_managed_block_content;

/// 一个结构完整且唯一的 marker block。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatedBlock {
    /// 包含 begin/end marker 与结尾换行的替换范围。
    pub block_range: Range<usize>,

    /// 两条 marker 之间的原始内容范围。
    pub content_range: Range<usize>,
}

/// 在文本中定位唯一、成对且顺序正确的 marker block。
///
/// # Return:
///   `Ok(None)` 表示两条 marker 都不存在；结构损坏返回具体原因。
pub fn locate(text: &str, marker: &str) -> Result<Option<LocatedBlock>, String> {
    let begin = format!("# >>> dots:{marker} >>>");
    let end = format!("# <<< dots:{marker} <<<");
    let mut begin_lines = Vec::new();
    let mut end_lines = Vec::new();
    let mut offset = 0usize;

    for line in text.split_inclusive('\n') {
        let logical = line.trim_end_matches(['\r', '\n']);
        let line_end = offset + line.len();
        if logical == begin {
            begin_lines.push((offset, line_end));
        }
        if logical == end {
            end_lines.push((offset, line_end));
        }
        offset = line_end;
    }
    if offset < text.len() {
        let logical = text[offset..].trim_end_matches('\r');
        if logical == begin {
            begin_lines.push((offset, text.len()));
        }
        if logical == end {
            end_lines.push((offset, text.len()));
        }
    }

    match (begin_lines.as_slice(), end_lines.as_slice()) {
        ([], []) => Ok(None),
        ([(begin_start, begin_end)], [(end_start, end_end)]) if begin_end <= end_start => {
            Ok(Some(LocatedBlock {
                block_range: *begin_start..*end_end,
                content_range: *begin_end..*end_start,
            }))
        }
        ([], _) => Err(format!("managed block `{marker}` 缺少 begin marker")),
        (_, []) => Err(format!("managed block `{marker}` 缺少 end marker")),
        ([_], [_]) => Err(format!("managed block `{marker}` marker 次序错误")),
        _ => Err(format!("managed block `{marker}` marker 重复")),
    }
}

/// 生成完整 marker block。
pub fn render(marker: &str, content: &str) -> String {
    let body = normalize_managed_block_content(content);
    format!("# >>> dots:{marker} >>>\n{body}# <<< dots:{marker} <<<\n")
}

/// 创建或替换一个结构完整的 marker block，保留文件其他内容。
pub fn upsert(
    text: &str,
    marker: &str,
    content: &str,
    placement: ManagedBlockPlacement,
) -> Result<String, String> {
    let rendered = render(marker, content);
    match locate(text, marker)? {
        Some(located) => Ok(format!(
            "{}{}{}",
            &text[..located.block_range.start],
            rendered,
            &text[located.block_range.end..]
        )),
        None if text.is_empty() => Ok(rendered),
        None if placement == ManagedBlockPlacement::Start => Ok(format!("{rendered}{text}")),
        None if text.ends_with('\n') => Ok(format!("{text}\n{rendered}")),
        None => Ok(format!("{text}\n\n{rendered}")),
    }
}

/// 删除一个结构完整的 marker block，保留文件其他内容。
pub fn remove(text: &str, marker: &str) -> Result<String, String> {
    let located =
        locate(text, marker)?.ok_or_else(|| format!("managed block `{marker}` 已不存在"))?;
    Ok(format!(
        "{}{}",
        &text[..located.block_range.start],
        &text[located.block_range.end..]
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::min_ident_chars, clippy::missing_docs_in_private_items)]

    use super::*;

    #[test]
    fn upsert_and_remove_preserve_surrounding_text() -> color_eyre::Result<()> {
        let initial = "before\nafter\n";
        let with_block = upsert(initial, "dots", "source x", ManagedBlockPlacement::End)
            .map_err(|reason| color_eyre::eyre::eyre!(reason))?;
        assert!(with_block.contains("source x"));
        let removed =
            remove(&with_block, "dots").map_err(|reason| color_eyre::eyre::eyre!(reason))?;
        assert_eq!(removed, format!("{initial}\n"));
        Ok(())
    }

    #[test]
    fn malformed_marker_is_rejected() {
        let malformed = "# >>> dots:x >>>\nbody\n";
        assert!(locate(malformed, "x").is_err());
    }
}
