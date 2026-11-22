use crate::text::wrap;
use crate::{Options, canvas::Canvas, color::Style};
use unicode_width::UnicodeWidthStr;

pub fn body(source: &str, accepts_header: impl FnOnce(&str) -> bool) -> Result<&str, String> {
    let source = source.trim_start();
    let (header, body) = source
        .split_once(char::is_whitespace)
        .unwrap_or((source, ""));
    if accepts_header(header) {
        Ok(body)
    } else {
        Err(format!("unexpected diagram header: {header}"))
    }
}

#[derive(Clone, Copy)]
pub struct Border {
    tl: char,
    tr: char,
    bl: char,
    br: char,
    h: char,
    v: char,
    left: char,
    right: char,
}

impl Border {
    pub fn new(options: &Options) -> Self {
        let (natural, [h, v, left, right]) = if options.ascii {
            (['+'; 4], ['-', '|', '+', '+'])
        } else {
            (['╭', '╮', '╰', '╯'], ['─', '│', '├', '┤'])
        };
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, natural);
        Self {
            tl,
            tr,
            bl,
            br,
            h,
            v,
            left,
            right,
        }
    }
}

pub struct Card {
    heading: Vec<String>,
    body: Vec<String>,
    style: Style,
}

impl Card {
    pub fn new(title: &str, content: &[String], width: usize, style: Style) -> Self {
        Self {
            heading: wrap(title, width - 4),
            body: content.iter().flat_map(|s| wrap(s, width - 4)).collect(),
            style,
        }
    }

    pub fn height(&self) -> usize {
        self.heading.len() + self.body.len() + usize::from(!self.body.is_empty()) + 2
    }

    pub fn draw(
        &self,
        c: &mut Canvas,
        x: usize,
        y: usize,
        width: usize,
        b: Border,
    ) -> Result<(), String> {
        let last = self.height() - 1;
        let border = self.style.border();
        for row in 0..=last {
            let ends = if row == 0 {
                Some((b.tl, b.tr))
            } else if row == last {
                Some((b.bl, b.br))
            } else if row == self.heading.len() + 1 {
                Some((b.left, b.right))
            } else {
                None
            };
            if let Some((left, right)) = ends {
                c.put(x, y + row, left)?;
                for col in 1..width - 1 {
                    c.put(x + col, y + row, b.h)?;
                }
                c.put(x + width - 1, y + row, right)?;
                c.paint(x, y + row, width, border);
            } else {
                c.put(x, y + row, b.v)?;
                c.put(x + width - 1, y + row, b.v)?;
                c.paint(x, y + row, 1, border);
                c.paint(x + width - 1, y + row, 1, border);
            }
        }
        for (i, line) in self.heading.iter().enumerate() {
            c.text(x + 2, y + 1 + i, line)?;
            c.paint(x + 2, y + 1 + i, line.width(), self.style.ink());
        }
        for (i, line) in self.body.iter().enumerate() {
            c.text(x + 2, y + 2 + self.heading.len() + i, line)?;
        }
        Ok(())
    }
}

pub mod image {
    use crate::{charts::shared::plot::image, color::Style, graphics::Scene};

    pub struct Card {
        headings: Vec<String>,
        rows: Vec<String>,
        width: f32,
        style: Style,
    }

    impl Card {
        pub fn new(
            scene: &Scene,
            title: &str,
            content: &[String],
            width: f32,
            style: Style,
        ) -> Self {
            Self {
                headings: image::wrap(scene, title, width - 28.0),
                rows: content
                    .iter()
                    .flat_map(|s| image::wrap(scene, s, width - 28.0))
                    .collect(),
                width,
                style,
            }
        }

        fn header_height(&self) -> f32 {
            20.0 * self.headings.len() as f32 + 20.0
        }

        pub fn height(&self) -> f32 {
            self.header_height()
                + if self.rows.is_empty() {
                    0.0
                } else {
                    20.0 * self.rows.len() as f32 + 20.0
                }
        }

        pub fn draw(&self, scene: &mut Scene, x: f32, y: f32) {
            let stroke = self.style.stroke.unwrap_or(scene.ink());
            let fill = self.style.fill.unwrap_or(scene.paper());
            let ink = image::contrast(fill, scene.paper());
            let heading_y = y + self.header_height();
            scene.rect(
                x,
                y,
                self.width,
                self.height(),
                8.0,
                Some(fill),
                Some(stroke),
            );
            if !self.rows.is_empty() {
                image::line(
                    scene,
                    (x, heading_y),
                    (x + self.width, heading_y),
                    stroke,
                    1.2,
                );
            }
            for (i, text) in self.headings.iter().enumerate() {
                scene.text(
                    x + 14.0,
                    y + 10.0 + 20.0 * i as f32,
                    text,
                    self.style.text.or(Some(ink)),
                );
            }
            for (i, text) in self.rows.iter().enumerate() {
                scene.text(
                    x + 14.0,
                    heading_y + 10.0 + 20.0 * i as f32,
                    text,
                    Some(ink),
                );
            }
        }
    }
}
