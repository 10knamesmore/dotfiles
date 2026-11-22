use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub fn body<'a>(source: &'a str, headers: &[&str]) -> Result<&'a str, String> {
    let (header, body) = source.split_once('\n').unwrap_or((source, ""));
    let header = header.trim();
    if headers.contains(&header) {
        Ok(body)
    } else {
        Err(format!("invalid area diagram header: {header}"))
    }
}

pub fn weight(text: &str) -> Result<f64, String> {
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(|| format!("expected a finite nonnegative weight, got {text:?}"))
}

pub fn single_line(text: &str) -> String {
    text.replace('\n', "\\n").replace('\t', "    ")
}

pub fn columns(text: &str) -> Option<usize> {
    // Canvas places one grapheme at a time. A cluster without its own cell
    // belongs in the legend, where the terminal can shape the full line.
    text.graphemes(true).try_fold(0, |sum, cluster| {
        let width = cluster.width();
        (width > 0).then_some(sum + width)
    })
}
