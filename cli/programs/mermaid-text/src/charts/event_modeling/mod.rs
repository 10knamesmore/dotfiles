use super::shared::sets::{Border, Card, body};
use crate::Options;
use crate::canvas::Canvas;
use crate::color::{Layer, Paint, Style};
use crate::text::wrap;
use std::collections::{BTreeMap, BTreeSet};
use unicode_width::UnicodeWidthStr;

mod grammar;
mod image;

struct Frame {
    id: String,
    reset: bool,
    kind: String,
    entity: String,
    sources: Vec<String>,
    reference: Option<String>,
    fields: Vec<String>,
}

impl Frame {
    fn rows(&self) -> Vec<String> {
        let mut rows = vec![self.entity.clone()];
        if let Some(name) = &self.reference {
            rows.push(format!("Data: {name}"));
        }
        rows.extend(self.fields.iter().cloned());
        rows
    }

    fn heading(&self, join: &str) -> String {
        let frame_name = if self.reset { "Reset" } else { "Frame" };
        format!("{frame_name} {}{join}{}", self.id, self.kind)
    }

    fn lane(&self) -> (&str, &str) {
        let group = match self.kind.as_str() {
            "UI" | "Processor" => "UI / automation",
            "Command" | "Read model" => "Commands / read models",
            "Event" => "Events",
            custom => custom,
        };
        (group, self.entity.rsplit_once('.').map_or("", |(ns, _)| ns))
    }

    fn stroke_key(&self) -> Option<&'static str> {
        Some(match self.kind.as_str() {
            "UI" => "emUiStroke",
            "Processor" => "emProcessorStroke",
            "Command" => "emCommandStroke",
            "Read model" => "emReadModelStroke",
            "Event" => "emEventStroke",
            _ => return None,
        })
    }
}

enum Caption {
    Title(String),
    Description(String),
}

#[derive(Default)]
struct Model {
    captions: Vec<Caption>,
    frames: Vec<Frame>,
    data: Vec<(String, Vec<String>)>,
    entities: Vec<String>,
    links: Vec<(String, String)>,
    notes: Vec<(String, Vec<String>)>,
}

struct Track {
    source: usize,
    target: usize,
    channel: usize,
}

struct Connections {
    adjacent: Vec<usize>,
    tracks: Vec<Track>,
    channels: usize,
}

fn connections(edges: Edges) -> Result<Connections, String> {
    let mut adjacent = Vec::new();
    let mut indirect = Vec::new();
    for (a, b) in edges {
        if a + 1 == b {
            adjacent.push(a);
        } else {
            indirect.push((a.min(b), a.max(b), a, b));
        }
    }
    indirect.sort_unstable();
    let mut occupied = Vec::new();
    let mut tracks = Vec::new();
    for (start, stop, source, target) in indirect {
        let channel = occupied
            .iter()
            .position(|&end| end < start)
            .unwrap_or(occupied.len());
        if channel == occupied.len() {
            if channel >= 256 {
                return Err("event diagram needs more than 256 simultaneous edge channels".into());
            }
            occupied.push(stop);
        } else {
            occupied[channel] = stop;
        }
        tracks.push(Track {
            source,
            target,
            channel,
        });
    }
    Ok(Connections {
        adjacent,
        tracks,
        channels: occupied.len(),
    })
}

fn edges(model: &Model, ids: &BTreeMap<String, usize>) -> Result<BTreeSet<(usize, usize)>, String> {
    let mut result = BTreeSet::new();
    let get = |s: &str| {
        ids.get(s)
            .copied()
            .ok_or_else(|| format!("unknown event frame: {s}"))
    };
    let mut explicit = BTreeSet::new();
    for (from, to) in &model.links {
        let target = get(to)?;
        explicit.insert(target);
        result.insert((get(from)?, target));
    }
    for (i, frame) in model.frames.iter().enumerate() {
        if !frame.sources.is_empty() {
            for id in &frame.sources {
                result.insert((get(id)?, i));
            }
        } else if !explicit.contains(&i) && !frame.reset {
            let lane = frame.lane();
            if let Some(j) = model.frames[..i].iter().rposition(|f| f.lane() != lane) {
                result.insert((j, i));
            }
        }
    }
    Ok(result)
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::draw);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (model, edges) = parse(source)?;
        let color = |key| {
            options
                .color
                .then(|| options.config.theme_color(key))
                .flatten()
        };
        let card_style = Style {
            stroke: color("primaryBorderColor"),
            text: color("primaryTextColor"),
            fill: None,
        };
        let relation = Paint {
            fg: color("emRelationStroke").or_else(|| options.theme_line()),
            bg: None,
        };
        let arrowhead = Paint {
            fg: color("emArrowhead").or(relation.fg),
            bg: None,
        };
        let Connections {
            adjacent,
            tracks,
            channels,
        } = connections(edges)?;
        let gutter = if channels == 0 { 0 } else { channels * 2 + 2 };
        let join = if options.ascii { " | " } else { " · " };
        let mut sections = Vec::new();
        for frame in &model.frames {
            let frame_style = Style {
                stroke: frame.stroke_key().and_then(color).or(card_style.stroke),
                ..card_style
            };
            sections.push((frame.heading(join), frame.rows(), frame_style));
        }
        if !model.entities.is_empty() {
            sections.push(("Entities".into(), model.entities, card_style));
        }
        for (name, fields) in model.data {
            sections.push((format!("Data{join}{name}"), fields, card_style));
        }
        let natural = sections
            .iter()
            .flat_map(|(h, rows, _)| std::iter::once(h).chain(rows))
            .flat_map(|s| s.lines())
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0)
            + 4;
        let width = natural
            .clamp(18, 68)
            .min(options.width.saturating_sub(gutter).max(18));
        let cards: Vec<_> = sections
            .iter()
            .map(|(h, r, style)| Card::new(h, r, width, *style))
            .collect();
        let mut ys = Vec::new();
        let mut height = 0;
        for card in &cards {
            ys.push(height);
            height += card.height() + 1;
        }
        let height = height - 1;
        let border = Border::new(options);
        let mut canvas = Canvas::new(width + gutter, height)?;
        for (card, y) in cards.iter().zip(&ys) {
            card.draw(&mut canvas, gutter, *y, width, border)?;
        }
        let mid = gutter + width / 2;
        for i in adjacent {
            let y = ys[i + 1] - 1;
            canvas.put(mid, y - 1, if options.ascii { '+' } else { '┬' })?;
            canvas.put(mid, y, if options.ascii { 'v' } else { '▼' })?;
            canvas.paint(mid, y - 1, 1, relation);
            canvas.paint(mid, y, 1, arrowhead);
        }
        if !tracks.is_empty() {
            let mut wire = vec![0u8; height * gutter];
            let mut cost = 0usize;
            for t in tracks {
                let x = gutter - 3 - t.channel * 2;
                let out = ys[t.source] + cards[t.source].height() - 2;
                let end = ys[t.target] + 1;
                cost += out.abs_diff(end) + 2 * (gutter - x);
                if cost > 4_000_000 {
                    return Err("event connector layout exceeds four million cells".into());
                }
                let (top, bottom) = (out.min(end), out.max(end));
                // N=1, E=2, S=4, W=8
                for yy in top..=bottom {
                    if yy != top {
                        wire[yy * gutter + x] |= 1;
                    }
                    if yy != bottom {
                        wire[yy * gutter + x] |= 4;
                    }
                }
                for y in [out, end] {
                    wire[y * gutter + x] |= 2;
                    for col in x + 1..gutter {
                        wire[y * gutter + col] |= 10;
                    }
                }
                canvas.put(gutter, out, if options.ascii { '+' } else { '┤' })?;
                canvas.put(gutter, end, if options.ascii { '>' } else { '▶' })?;
                canvas.paint(gutter, out, 1, relation);
                canvas.paint(gutter, end, 1, arrowhead);
            }
            for (i, &m) in wire.iter().enumerate().filter(|(_, m)| **m != 0) {
                let ch = if options.ascii {
                    match m {
                        5 => '|',
                        10 => '-',
                        _ => '+',
                    }
                } else {
                    match m {
                        3 => '╰',
                        6 => '╭',
                        5 => '│',
                        10 => '─',
                        7 => '├',
                        11 => '┴',
                        14 => '┬',
                        _ => '┼',
                    }
                };
                canvas.put(i % gutter, i / gutter, ch)?;
                canvas.paint(i % gutter, i / gutter, 1, relation);
            }
        }
        let title_paint = Paint {
            fg: color("titleColor"),
            bg: None,
        };
        let mut captions = Vec::new();
        let mut colors = Layer::default();
        for caption in model.captions {
            let (text, paint) = match caption {
                Caption::Title(text) => (text, title_paint),
                Caption::Description(text) => (text, Paint::default()),
            };
            for line in wrap(&text, options.width.max(18)) {
                colors.paint(0, captions.len(), line.width(), paint);
                captions.push(line);
            }
        }
        let mut output = colors.apply(&captions.join("\n"));
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        output.push_str(&canvas.finish());
        Ok(output)
    }
}

type Edges = BTreeSet<(usize, usize)>;

fn parse(source: &str) -> Result<(Model, Edges), String> {
    let mut model = grammar::parse(body(source, |header| {
        matches!(header, "eventmodeling-beta" | "eventmodeling")
    })?)?;
    let ids: BTreeMap<_, _> = model
        .frames
        .iter()
        .enumerate()
        .map(|(i, f)| (f.id.clone(), i))
        .collect();
    if ids.len() != model.frames.len() {
        return Err("duplicate event frame identifier".into());
    }
    let data: BTreeSet<_> = model.data.iter().map(|(name, _)| name.as_str()).collect();
    if data.len() != model.data.len() {
        return Err("duplicate event data identifier".into());
    }
    for (id, lines) in std::mem::take(&mut model.notes) {
        let idx = ids
            .get(&id)
            .ok_or_else(|| format!("unknown event frame: {id}"))?;
        model.frames[*idx].fields.extend(lines);
    }
    let edges = edges(&model, &ids)?;
    for frame in &model.frames {
        if let Some(name) = &frame.reference
            && !data.contains(name.as_str())
        {
            return Err(format!("unknown event data reference: {name}"));
        }
    }
    if model.frames.is_empty() && model.entities.is_empty() && model.data.is_empty() {
        return Err("event model contains no frames, entities or data".into());
    }
    Ok((model, edges))
}
