use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Keep whole words when they fit and never split a grapheme cluster. A single
/// wide cluster can occupy more than the requested number of display columns.
pub(crate) fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut rows = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut used = 0;
        let mut space = String::new();
        for word in paragraph.split_word_bounds() {
            if word.chars().all(char::is_whitespace) {
                if !line.is_empty() {
                    for ch in word.chars() {
                        if ch == '\t' {
                            space.push_str("    ");
                        } else {
                            space.push(ch);
                        }
                    }
                }
                continue;
            }
            let gap = space.width();
            if !line.is_empty() && used + gap + word.width() > columns {
                rows.push(std::mem::take(&mut line));
                used = 0;
            } else {
                line.push_str(&space);
                used += gap;
            }
            space.clear();
            for grapheme in word.graphemes(true) {
                let width = grapheme.width();
                if !line.is_empty() && used + width > columns {
                    rows.push(std::mem::take(&mut line));
                    used = 0;
                }
                line.push_str(grapheme);
                used += width;
            }
        }
        rows.push(line);
    }
    rows
}
