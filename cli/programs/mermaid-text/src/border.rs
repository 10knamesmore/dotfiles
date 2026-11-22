use std::str::FromStr;

use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy)]
pub(crate) struct BorderCorners(Option<[char; 4]>);

impl BorderCorners {
    /// Corner order follows the drawing: top left, top right, bottom left, bottom right.
    pub(crate) fn resolve(self, ascii: bool, natural: [char; 4]) -> [char; 4] {
        if ascii {
            natural
        } else {
            self.0.unwrap_or(natural)
        }
    }
}

impl FromStr for BorderCorners {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let glyphs = match value {
            "auto" => return Ok(Self(None)),
            "square" => "┌┐└┘",
            "rounded" => "╭╮╰╯",
            glyphs => glyphs,
        };
        let mut chars = glyphs.chars();
        let mut corners = [' '; 4];
        for corner in &mut corners {
            *corner = chars
                .next()
                .ok_or("expected auto, square, rounded, or four one-column characters")?;
            if corner.is_control()
                || corner.width() != Some(1)
                || (corner.is_whitespace() && *corner != ' ')
            {
                return Err("each border corner must be a printable one-column character");
            }
        }
        if chars.next().is_some() {
            return Err(
                "expected exactly four corner characters (top left, top right, bottom left, bottom right)",
            );
        }
        Ok(Self(Some(corners)))
    }
}
