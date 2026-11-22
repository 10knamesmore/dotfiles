use crate::{border::BorderCorners, color, source::DiagramConfig};

pub(crate) struct Options {
    pub width: usize,
    pub ascii: bool,
    pub color: bool,
    pub corners: BorderCorners,
    pub config: DiagramConfig,
}

impl Options {
    pub fn theme_style(&self) -> color::Style {
        if !self.color {
            return color::Style::default();
        }
        color::Style {
            fill: self.config.theme_color("primaryColor"),
            stroke: self.config.theme_color("primaryBorderColor"),
            text: self.config.theme_color("primaryTextColor"),
        }
    }

    pub fn theme_line(&self) -> Option<color::Rgb> {
        self.color
            .then(|| self.config.theme_color("lineColor"))
            .flatten()
    }
}
