use super::{Annotation, HEADERS, TreePaint, board, parse};
use crate::{Options, color::Rgb, graphics::Scene};
use board::image::{Caption, Label, PAD, text_on_fill, tint};

const INDENT: f32 = 29.0;
const INSET: f32 = 12.0;
const GAP: f32 = 9.0;

struct Entry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    name: Label,
    tags: Vec<(Label, Option<Rgb>)>,
    description: Option<Label>,
    highlighted: bool,
}

pub(super) fn render(source: &str, options: &Options, scene: &mut Scene) -> Result<(), String> {
    let (captions, forest) = parse(board::body(source, HEADERS)?)?;
    let caption = Caption::new(scene, &captions, options);
    let theme = TreePaint::new(options);
    let mut y = PAD + caption.height;
    let entries: Vec<_> = forest
        .nodes
        .iter()
        .map(|node| {
            let name = Label::new(scene, node.label.name);
            let mut tags = Vec::new();
            let mut description = None;
            for annotation in &node.label.annotations {
                match annotation {
                    Annotation::Class(class) => {
                        tags.push((Label::new(scene, class), theme.label.fg))
                    }
                    Annotation::Icon(icon) => tags.push((Label::new(scene, icon), theme.icon.fg)),
                    Annotation::Description(text) => description = Some(Label::new(scene, text)),
                }
            }
            let width = name.width + tags.iter().map(|(tag, _)| tag.width + 19.0).sum::<f32>();
            let width = description.as_ref().map_or(width, |d| width.max(d.width)) + 2.0 * INSET;
            let height = name.height + description.as_ref().map_or(0.0, |d| d.height) + 16.0;
            let entry = Entry {
                x: PAD + node.depth as f32 * INDENT,
                y,
                width,
                height,
                name,
                tags,
                description,
                highlighted: node.label.highlighted,
            };
            y += height + GAP;
            entry
        })
        .collect();
    let width = entries
        .iter()
        .map(|n| n.x + n.width)
        .fold(PAD + caption.width, f32::max);
    scene.resize(width + PAD, y + PAD - GAP)?;
    caption.draw(scene);

    let ink = scene.ink();
    let line = theme
        .line
        .fg
        .or_else(|| options.theme_line())
        .unwrap_or_else(|| tint(ink, 120));
    let mut trunks = vec![None; entries.len()];
    for (index, node) in forest.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            let entry = &entries[index];
            let sx = entries[parent].x + INSET;
            let y = entry.y + entry.height / 2.0;
            scene.path(&format!("M {sx} {y} H {}", entry.x), None, Some(line), 1.5);
            trunks[parent] = Some(y);
        }
    }
    for (entry, end) in entries.iter().zip(trunks) {
        if let Some(end) = end {
            let x = entry.x + INSET;
            let y = entry.y + entry.height;
            scene.path(&format!("M {x} {y} V {end}"), None, Some(line), 1.5);
        }
    }
    for entry in entries {
        let fill = if entry.highlighted {
            theme.highlight.unwrap_or_else(|| tint(ink, 38))
        } else {
            tint(ink, 9)
        };
        let border = if entry.highlighted {
            line
        } else {
            tint(ink, 68)
        };
        let fallback = text_on_fill(
            scene,
            options,
            theme.highlight.filter(|_| entry.highlighted),
        );
        let x = entry.x + INSET;
        let y = entry.y + 8.0;
        scene.rect(
            entry.x,
            entry.y,
            entry.width,
            entry.height,
            6.0,
            Some(fill),
            Some(border),
        );
        entry.name.draw(scene, x, y, theme.label.fg.or(fallback));
        let mut tag_x = x + entry.name.width + 7.0;
        for (tag, paint) in &entry.tags {
            scene.rect(
                tag_x,
                y,
                tag.width + 12.0,
                tag.height,
                4.0,
                None,
                Some(tint(ink, 68)),
            );
            tag.draw(scene, tag_x + 6.0, y, paint.or(fallback));
            tag_x += tag.width + 19.0;
        }
        if let Some(description) = &entry.description {
            description.draw(
                scene,
                x,
                y + entry.name.height,
                theme.description.fg.or(fallback),
            );
        }
    }
    Ok(())
}
