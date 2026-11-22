use std::collections::BTreeMap;

use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint, Style, with_base},
};
use unicode_width::UnicodeWidthStr;

use super::{Link, Map, Point};
use crate::text::wrap;

type Cell = (usize, usize);

struct Mark {
    cell: Cell,
    start: usize,
    label: String,
    glyph: char,
}

type Tracks = BTreeMap<Cell, (char, Paint)>;

fn connection(link: &Link, arrow: &str, width: usize, stroke: Paint, label: Paint) -> String {
    let mut text = link.from.replace('\t', "    ");
    text.push(' ');
    let arrow_start = text.len();
    text.push_str(arrow);
    let arrow_end = text.len();
    text.push(' ');
    text.push_str(&link.to.replace('\t', "    "));
    if !link.caption.is_empty() {
        text.push_str(": ");
    }
    let caption = text.len();
    text.push_str(&link.caption.replace('\t', "    "));

    let rows = wrap(&text, width);
    let mut colors = Layer::default();
    let mut pos = 0;
    for (y, row) in rows.iter().enumerate() {
        pos += text[pos..].len() - text[pos..].trim_start().len();
        let end = pos + row.len();
        for (start, stop, paint) in [
            (arrow_start, arrow_end, stroke),
            (caption, text.len(), label),
        ] {
            let lo = start.max(pos);
            let hi = stop.min(end);
            if lo < hi {
                colors.paint(text[pos..lo].width(), y, text[lo..hi].width(), paint);
            }
        }
        pos = end;
    }
    colors.apply(&rows.join("\n"))
}

fn stroke(tracks: &mut Tracks, a: Cell, b: Cell, dotted: bool, ascii: bool, paint: Paint) {
    let (dx, dy) = (b.0 as isize - a.0 as isize, b.1 as isize - a.1 as isize);
    let steps = dx.unsigned_abs().max(dy.unsigned_abs());
    let mut prev = a;
    for i in 1..=steps {
        let part = i as f64 / steps as f64;
        let next = (
            (a.0 as isize + (dx as f64 * part).round() as isize) as usize,
            (a.1 as isize + (dy as f64 * part).round() as isize) as usize,
        );
        let ch = if dotted {
            if ascii { '.' } else { '·' }
        } else if prev.1 == next.1 {
            if ascii { '-' } else { '─' }
        } else if prev.0 == next.0 {
            if ascii { '|' } else { '│' }
        } else if dx.signum() == dy.signum() {
            if ascii { '\\' } else { '╲' }
        } else if ascii {
            '/'
        } else {
            '╱'
        };
        tracks.insert(next, (ch, paint));
        prev = next;
    }
}

pub(super) fn draw(map: &Map, o: &Options) -> Result<String, String> {
    const LEFT: usize = 5;
    let width = o.width.clamp(30, 150);
    let span = width - LEFT - 2;
    let rows = (span as f64 * map.size.1 / map.size.0 * 0.48)
        .round()
        .clamp(10., 38.) as usize;
    let x = |e: f64| LEFT + 1 + (e * span as f64).round() as usize;
    let row = |v: f64| ((1. - v) * rows as f64).round() as usize;
    let dense = map.items.len() > 18;
    let color = |key| o.color.then(|| o.config.theme_color(key)).flatten();
    let fg = |fg| Paint { fg, bg: None };
    let line = o.theme_line();
    let text = o.theme_style().text;
    let background = Paint {
        fg: None,
        bg: color("wardley.backgroundColor"),
    };
    let node_background = background.bg.and_then(|_| color("background"));
    let axis = fg(color("wardley.axisColor").or(line));
    let axis_text = fg(color("wardley.axisTextColor").or(text));
    let link_paint = fg(color("wardley.linkStroke").or(line));
    let evolution = fg(color("wardley.evolutionStroke").or(color("wardleyEvolutionColor")));
    let component = Style {
        fill: color("wardley.componentFill").or(node_background),
        stroke: color("wardley.componentStroke").or(line),
        text: color("wardley.componentLabelColor").or(text),
    };
    let annotation = Style {
        fill: color("wardley.annotationFill").or(node_background),
        stroke: color("wardley.annotationStroke").or(line),
        text: color("wardley.annotationTextColor").or(text),
    };
    let item_colors = |kind: &str| match kind.split_whitespace().next() {
        Some("component") => (component.border(), fg(component.text)),
        Some("anchor") => (Paint::default(), fg(component.text)),
        Some("annotation") => (annotation.border(), annotation.ink()),
        Some("accelerator" | "deaccelerator") => (fg(component.stroke), axis_text),
        _ => (Paint::default(), axis_text),
    };
    let mut bands: Vec<Vec<Vec<(usize, usize)>>> = vec![vec![Vec::new()]; rows + 1];
    let mut marks = Vec::new();
    let mut key = Vec::new();
    for (i, item) in map.items.iter().enumerate() {
        let (v, e) = item.point;
        let px = x(e);
        let py = row(v);
        let desc = match item.kind.as_str() {
            "component" => item.name.clone(),
            "anchor" => format!("{} (anchor)", item.name),
            _ => format!(
                "{}: {}",
                item.kind.trim_start_matches("component "),
                item.name
            ),
        };
        let label = if dense
            || item.kind == "note"
            || item.kind.starts_with("annotation ")
            || item.name.width() + 2 > span / 2
            || item.kind.contains(' ')
        {
            let code = format!("#{}", i + 1);
            key.push((format!("{code}  {desc}"), item_colors(&item.kind).1));
            code
        } else {
            item.name.clone()
        };
        let glyph = match (item.kind.as_str(), o.ascii) {
            ("anchor", false) => '◆',
            ("anchor", true) => '*',
            ("note", _) => '!',
            ("accelerator", false) => '»',
            ("deaccelerator", false) => '«',
            ("accelerator", true) => '>',
            ("deaccelerator", true) => '<',
            (s, _) if s.starts_with("annotation ") => '!',
            (_, true) => 'o',
            (_, false) => '●',
        };
        let w = label.width();
        if w == 0 {
            return Err("Wardley label has no visible text".into());
        }
        let left = px.checked_sub(w + 1).filter(|p| *p > LEFT);
        let right = (px + w + 2 <= width).then_some(px + 2);
        let candidates = if item.offset.0 < 0. {
            [left, right]
        } else {
            [right, left]
        };
        let mut chosen = None;
        for (level, reserved) in bands[py].iter().enumerate() {
            for &start in candidates.iter().flatten() {
                let (lo, hi) = (start.min(px), px.max(start + w - 1));
                if reserved.iter().all(|&(a, b)| hi + 1 < a || b + 1 < lo) {
                    chosen = Some((level, start));
                    break;
                }
            }
            if chosen.is_some() {
                break;
            }
        }
        let (level, start) = match chosen {
            Some(place) => place,
            None => {
                let pos = candidates
                    .into_iter()
                    .flatten()
                    .next()
                    .ok_or("label does not fit map")?;
                bands[py].push(Vec::new());
                (bands[py].len() - 1, pos)
            }
        };
        bands[py][level].push((px.min(start), px.max(start + w - 1)));
        marks.push(Mark {
            cell: (px, level),
            start,
            label,
            glyph,
        });
    }
    let mut height = 0;
    let ys: Vec<_> = bands
        .iter()
        .map(|band| {
            let y = height;
            height += band.len();
            y
        })
        .collect();
    let mut prev = 0.;
    let mut stages = Vec::new();
    let mut stage_height = 1;
    for (label, end) in &map.stages {
        let (lo, hi) = (x(prev), x(*end));
        let lines = wrap(label, (hi - lo).max(1));
        if hi - lo < 2 || lines.len() > 3 {
            let dash = if o.ascii { "-" } else { "–" };
            key.push((format!("Evolution {prev}{dash}{end}: {label}"), axis_text));
            stages.push((lo, hi, Vec::new()));
        } else {
            stage_height = stage_height.max(lines.len());
            stages.push((lo, hi, lines));
        }
        prev = *end;
    }
    let axis_row = height + 1 + stage_height;
    let mut canvas = Canvas::new(width, axis_row + 1)?;
    for (mark, item) in marks.iter_mut().zip(&map.items) {
        mark.cell.1 += ys[row(item.point.0)];
    }
    let project = |(v, e): Point| (x(e), ys[row(v)]);
    let mut links = Vec::new();
    let mut tracks = Tracks::new();
    let mut families = BTreeMap::<&str, Vec<&str>>::new();
    for (i, item) in map.items.iter().enumerate() {
        if let Some(p) = item.parent.as_deref() {
            stroke(
                &mut tracks,
                marks[map.lookup(p)?].cell,
                marks[i].cell,
                true,
                o.ascii,
                link_paint,
            );
            families.entry(p).or_default().push(&item.name);
        }
    }
    for link in &map.links {
        let from = map.lookup(&link.from)?;
        let to = map.lookup(&link.to)?;
        stroke(
            &mut tracks,
            marks[from].cell,
            marks[to].cell,
            link.style == "-.->",
            o.ascii,
            link_paint,
        );
        let arrow = match (link.style, o.ascii) {
            ("+<>", false) => "↔",
            ("+<>", true) => "<=>",
            ("+<", false) => "⇐",
            ("+<", true) => "<=",
            ("+>" | "+'", false) => "⇒",
            ("+>" | "+'", true) => "=>",
            ("-.->", false) => "┄→",
            ("-.->", true) => "..>",
            (_, false) => "→",
            (_, true) => "->",
        };
        links.push(connection(link, arrow, width, link_paint, axis_text));
    }
    for (name, v, e) in &map.moves {
        let i = map.lookup(name)?;
        let end = project((v.unwrap_or(map.items[i].point.0), *e));
        let start = marks[i].cell;
        let end = (end.0, if v.is_some() { end.1 } else { start.1 });
        stroke(&mut tracks, start, end, true, o.ascii, evolution);
        let head = if end.0 < start.0 {
            if o.ascii { '<' } else { '◀' }
        } else if o.ascii {
            '>'
        } else {
            '▶'
        };
        tracks.insert(end, (head, evolution));
        let description = if let Some(v) = v {
            format!("{name}: trend to ({e}, {v})")
        } else {
            format!(
                "{name}: evolution {} {} {e}",
                map.items[i].point.1,
                if o.ascii { "->" } else { "→" }
            )
        };
        links.push(wrap(&description, width).join("\n"));
    }
    for (mark, item) in marks.iter().zip(&map.items) {
        let (symbol, label) = item_colors(&item.kind);
        canvas.put(mark.cell.0, mark.cell.1, mark.glyph)?;
        canvas.paint(mark.cell.0, mark.cell.1, 1, symbol);
        let gap = if mark.start < mark.cell.0 {
            mark.cell.0 - 1
        } else {
            mark.cell.0 + 1
        };
        canvas.put(gap, mark.cell.1, ' ')?;
        canvas.text(mark.start, mark.cell.1, &mark.label)?;
        let w = mark.label.width();
        canvas.paint(mark.start, mark.cell.1, w, label);
        for col in mark.cell.0.min(mark.start)..=mark.cell.0.max(mark.start + w - 1) {
            tracks.remove(&(col, mark.cell.1));
        }
    }
    for ((x, y), (glyph, paint)) in tracks {
        canvas.put(x, y, glyph)?;
        canvas.paint(x, y, 1, paint);
    }
    for y in 0..height {
        canvas.put(LEFT - 1, y, if o.ascii { '|' } else { '│' })?;
        canvas.paint(LEFT - 1, y, 1, axis);
    }
    canvas.text(0, 0, "1.0")?;
    canvas.text(0, ys[row(0.5)], "0.5")?;
    canvas.text(0, height - 1, "0.0")?;
    for y in [0, ys[row(0.5)], height - 1] {
        canvas.paint(0, y, 3, axis_text);
    }
    canvas.put(LEFT - 1, height, if o.ascii { '+' } else { '└' })?;
    for px in LEFT..width {
        canvas.put(px, height, if o.ascii { '-' } else { '─' })?;
    }
    canvas.paint(LEFT - 1, height, width - LEFT + 1, axis);
    for (lo, hi, lines) in stages {
        canvas.put(lo, height, if o.ascii { '+' } else { '┬' })?;
        for (i, s) in lines.iter().enumerate() {
            let x = lo + (hi - lo - s.width()) / 2;
            canvas.text(x, height + 1 + i, s)?;
            canvas.paint(x, height + 1 + i, s.width(), axis_text);
        }
    }
    canvas.text(LEFT, axis_row, "0.0")?;
    canvas.text(width - 3, axis_row, "1.0")?;
    canvas.paint(LEFT, axis_row, 3, axis_text);
    canvas.paint(width - 3, axis_row, 3, axis_text);
    let axis_label = if o.ascii {
        "Evolution >"
    } else {
        "Evolution →"
    };
    canvas.text((width - 9) / 2, axis_row, axis_label)?;
    canvas.paint((width - 9) / 2, axis_row, axis_label.width(), axis_text);
    let mut output = Vec::new();
    if !map.title.is_empty() {
        output.push(with_base(&wrap(&map.title, width).join("\n"), axis_text)?);
    }
    let visibility = if o.ascii {
        "Visibility ^"
    } else {
        "Visibility ↑"
    };
    output.push(with_base(visibility, axis_text)?);
    output.push(canvas.finish());
    for (p, members) in families {
        key.push((
            format!("Pipeline {p}: {}", members.join(", ")),
            Paint::default(),
        ));
    }
    if !key.is_empty() {
        output.push(String::new());
        output.push("Key".into());
        for (line, paint) in key {
            output.push(with_base(&wrap(&line, width).join("\n"), paint)?);
        }
    }
    if !links.is_empty() {
        output.push(String::new());
        output.push("Connections".into());
        output.extend(links);
    }
    with_base(&output.join("\n"), background)
}
