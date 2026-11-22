use super::shared::area::{body, columns, single_line, weight};
use crate::{
    Options,
    canvas::Canvas,
    color::{Layer, Paint},
};
use rustc_hash::FxHashMap;
use std::fmt::Write;
use unicode_width::UnicodeWidthStr;

mod image;

struct Flow {
    from: usize,
    to: usize,
    value: f64,
    amount: String,
    rows: usize,
}

// Return one field and its terminator, leaving the next field in `input`.
fn field(input: &mut &str) -> Result<(String, Option<char>), String> {
    *input = input.trim_start_matches([' ', '\t']);
    let value = if let Some(mut rest) = input.strip_prefix('"') {
        let mut value = String::new();
        loop {
            let (part, tail) = rest.split_once('"').ok_or("unterminated CSV quote")?;
            value.push_str(part);
            if let Some(more) = tail.strip_prefix('"') {
                value.push('"');
                rest = more;
            } else {
                *input = tail.trim_start_matches([' ', '\t']);
                break;
            }
        }
        value
    } else {
        let end = input.find([',', '\n']).unwrap_or(input.len());
        let value = input[..end].trim().to_string();
        if value.contains('"') {
            return Err("unexpected quote in unquoted CSV field".into());
        }
        *input = &input[end..];
        value
    };
    let sep = input.chars().next();
    match sep {
        Some(',' | '\n') => *input = &input[1..],
        Some(_) => return Err("expected comma or end of record after CSV quote".into()),
        None => {}
    }
    Ok((value, sep))
}

fn records(mut input: &str) -> Result<(Vec<String>, Vec<Flow>), String> {
    let mut ids = FxHashMap::default();
    let mut labels = Vec::new();
    let mut flows = Vec::new();
    while !input.trim().is_empty() {
        let result = (|| {
            let (a, sep) = field(&mut input)?;
            if sep != Some(',') {
                return Err("missing target and weight".to_string());
            }
            let (b, sep) = field(&mut input)?;
            if sep != Some(',') {
                return Err("missing weight".to_string());
            }
            if a.trim().is_empty() || b.trim().is_empty() {
                return Err("empty Sankey endpoint".into());
            }
            let (amount, sep) = field(&mut input)?;
            if sep == Some(',') {
                return Err("expected exactly three CSV fields".into());
            }
            let amount = amount.trim().to_string();
            let value = weight(&amount)?;
            let [from, to] = [a, b].map(|name| {
                *ids.entry(name.clone()).or_insert_with(|| {
                    labels.push(name);
                    labels.len() - 1
                })
            });
            Ok(Flow {
                from,
                to,
                value,
                amount,
                rows: 0,
            })
        })();
        flows.push(result.map_err(|e| format!("Sankey record {}: {e}", flows.len() + 1))?);
        if flows.len() > 8192 {
            return Err("Sankey exceeds 8192 flows".into());
        }
    }
    if flows.is_empty() {
        return Err("expected Sankey flow records".into());
    }
    Ok((labels, flows))
}

// Cut feedback edges at a second, marked appearance of their destination so
// return flows have measurable ribbons alongside feed-forward flows.
fn ranks(labels: &mut Vec<String>, flows: &mut [Flow], ascii: bool) -> Vec<usize> {
    let n = labels.len();
    let mut successors = vec![Vec::new(); n];
    for (e, flow) in flows.iter().enumerate() {
        successors[flow.from].push(e);
    }
    let mut state = vec![0; n];
    let mut returns = vec![None; n];
    let mut finished = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack = vec![(root, 0)];
        while let Some((i, next)) = stack.last_mut() {
            if let Some(&e) = successors[*i].get(*next) {
                *next += 1;
                let j = flows[e].to;
                match state[j] {
                    0 => {
                        state[j] = 1;
                        stack.push((j, 0));
                    }
                    1 => {
                        flows[e].to = *returns[j].get_or_insert_with(|| {
                            let symbol = if ascii { "<-" } else { "↩" };
                            labels.push(format!("{symbol} {}", labels[j]));
                            labels.len() - 1
                        });
                    }
                    _ => {}
                }
            } else {
                state[*i] = 2;
                finished.push(*i);
                stack.pop();
            }
        }
    }
    let mut rank = vec![0; labels.len()];
    for i in finished.into_iter().rev() {
        for &e in &successors[i] {
            let to = flows[e].to;
            rank[to] = rank[to].max(rank[i] + 1);
        }
    }
    let last = *rank.iter().max().unwrap();
    for (i, r) in rank.iter_mut().enumerate() {
        if i >= n || successors[i].is_empty() {
            *r = last;
        }
    }
    rank
}

#[derive(Default, Clone)]
struct Port {
    // A through port carries a single ribbon across an intermediate stage.
    through: Option<usize>,
    incoming: Vec<(usize, usize)>,
    outgoing: Vec<(usize, usize)>,
    y: usize,
    rows: usize,
}

fn shade(edge: usize, ascii: bool, zero: bool) -> char {
    if zero {
        return if ascii { '.' } else { '·' };
    }
    (if ascii {
        ['=', ':', '#']
    } else {
        ['▒', '░', '▓']
    })[edge % 3]
}

struct Layout {
    stages: Vec<Vec<usize>>,
    ports: Vec<Port>,
    height: usize,
}

fn routing(rank: &[usize], flows: &[Flow]) -> Result<(Vec<Vec<usize>>, Vec<Port>), String> {
    let mut stages = vec![Vec::new(); rank.iter().max().unwrap() + 1];
    let mut ports = vec![Port::default(); rank.len()];
    for (i, &r) in rank.iter().enumerate() {
        stages[r].push(i);
    }
    for (e, flow) in flows.iter().enumerate() {
        let mut prev = flow.from;
        for stage in &mut stages[rank[flow.from] + 1..rank[flow.to]] {
            if ports.len() >= 32768 {
                return Err("Sankey layout exceeds 32768 node and routing positions".into());
            }
            let i = ports.len();
            stage.push(i);
            ports.push(Port {
                through: Some(e),
                ..Default::default()
            });
            ports[prev].outgoing.push((i, e));
            ports[i].incoming.push((prev, e));
            prev = i;
        }
        ports[prev].outgoing.push((flow.to, e));
        ports[flow.to].incoming.push((prev, e));
    }

    let mut order = vec![0; ports.len()];
    for stage in &stages {
        for (j, &i) in stage.iter().enumerate() {
            order[i] = j;
        }
    }
    for sweep in 0..4 {
        let forward = sweep % 2 == 0;
        for s in 0..stages.len() {
            let r = if forward { s } else { stages.len() - s - 1 };
            stages[r].sort_by_cached_key(|&i| {
                let near = if forward {
                    &ports[i].incoming
                } else {
                    &ports[i].outgoing
                };
                if near.is_empty() {
                    order[i] * 1024
                } else {
                    near.iter().map(|&(j, _)| order[j]).sum::<usize>() * 1024 / near.len()
                }
            });
            for (j, &i) in stages[r].iter().enumerate() {
                order[i] = j;
            }
        }
    }
    for port in &mut ports {
        port.incoming.sort_by_key(|&(j, _)| order[j]);
        port.outgoing.sort_by_key(|&(j, _)| order[j]);
    }
    Ok((stages, ports))
}

fn layout(rank: &[usize], flows: &mut [Flow]) -> Result<Layout, String> {
    let (stages, mut ports) = routing(rank, flows)?;
    // Scale every ribbon by the same quantum, independent of its stage.
    let max = flows.iter().map(|f| f.value).fold(0.0, f64::max);
    let max_rows = if flows.len() == 1 {
        3.0
    } else if max <= 10.0 && flows.iter().all(|f| f.value.fract() == 0.0) {
        max
    } else {
        8.0
    };
    for f in flows.iter_mut() {
        f.rows = if max == 0.0 {
            1
        } else {
            ((f.value / max * max_rows).round() as usize).max(1)
        };
    }
    let mut stage_heights = Vec::new();
    for stage in &stages {
        let mut top = 0;
        for &i in stage {
            let p = &mut ports[i];
            let rows =
                |links: &[(usize, usize)]| links.iter().map(|&(_, e)| flows[e].rows).sum::<usize>();
            p.rows = rows(&p.incoming).max(rows(&p.outgoing));
            p.y = top;
            top += p.rows + 1;
        }
        stage_heights.push(top - 1);
    }
    let height = *stage_heights.iter().max().unwrap();
    for (stage, stage_height) in stages.iter().zip(stage_heights) {
        for &i in stage {
            ports[i].y += (height - stage_height) / 2;
        }
    }
    Ok(Layout {
        stages,
        ports,
        height,
    })
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (mut labels, mut flows) = records(body(source, &["sankey", "sankey-beta"])?)?;
        let paint = |fg| Paint { fg, bg: None };
        let node_style = options.theme_style();
        let border = paint(node_style.stroke);
        let label = paint(node_style.text);
        let ribbon = paint(options.theme_line());
        let rank = ranks(&mut labels, &mut flows, options.ascii);
        let Layout {
            stages,
            ports,
            height,
        } = layout(&rank, &mut flows)?;
        let count = stages.len();
        let label_budget = (options.width / count).saturating_sub(7).clamp(4, 30);
        let short: Vec<_> = labels
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let name = single_line(name);
                if columns(&name).is_none_or(|w| w > label_budget) {
                    format!("#{}", i + 1)
                } else {
                    name
                }
            })
            .collect();
        let widths: Vec<_> = stages
            .iter()
            .map(|stage| {
                stage
                    .iter()
                    .filter_map(|&i| short.get(i))
                    .filter_map(|s| columns(s))
                    .max()
                    .unwrap_or(0)
                    + 2
            })
            .collect();
        let gap = (options.width.saturating_sub(widths.iter().sum()) / (count - 1)).clamp(5, 22);
        let mut x = Vec::new();
        let mut width = 0;
        for (r, &w) in widths.iter().enumerate() {
            x.push(width);
            width += w + if r + 1 < count { gap } else { 0 };
        }
        let mut canvas = Canvas::new(width, height)?;
        for r in 0..count - 1 {
            let mut in_y = FxHashMap::default();
            for &i in &stages[r + 1] {
                let p = &ports[i];
                let used: usize = p.incoming.iter().map(|&(_, e)| flows[e].rows).sum();
                let mut y = p.y + (p.rows - used) / 2;
                for &(_, e) in &p.incoming {
                    in_y.insert(e, y);
                    y += flows[e].rows;
                }
            }
            for &i in &stages[r] {
                let p = &ports[i];
                let used: usize = p.outgoing.iter().map(|&(_, e)| flows[e].rows).sum();
                let mut y = p.y + (p.rows - used) / 2;
                for &(_, e) in &p.outgoing {
                    let f = &flows[e];
                    let end = in_y[&e];
                    let curve = |t: f64| {
                        (y as f64 + (end as f64 - y as f64) * t * t * (3.0 - 2.0 * t)).round()
                            as usize
                    };
                    for k in 0..gap {
                        let a = curve(k as f64 / gap as f64);
                        let b = curve((k + 1) as f64 / gap as f64);
                        for row in a.min(b)..a.max(b) + f.rows {
                            let col = x[r] + widths[r] + k;
                            canvas.put(col, row, shade(e, options.ascii, f.value == 0.0))?;
                            canvas.paint(col, row, 1, ribbon);
                        }
                    }
                    y += f.rows;
                }
            }
        }
        let [tl, tr, bl, br] = options.corners.resolve(options.ascii, ['┏', '┓', '┗', '┛']);
        for (r, stage) in stages.iter().enumerate() {
            for &i in stage {
                let p = &ports[i];
                if let Some(e) = p.through {
                    let ch = shade(e, options.ascii, flows[e].value == 0.0);
                    for y in p.y..p.y + p.rows {
                        for col in x[r]..x[r] + widths[r] {
                            canvas.put(col, y, ch)?;
                        }
                        canvas.paint(x[r], y, widths[r], ribbon);
                    }
                } else {
                    for row in 0..p.rows {
                        let side = if options.ascii { '|' } else { '┃' };
                        let corners = if p.rows < 3 {
                            None
                        } else if row == 0 {
                            Some((tl, tr))
                        } else if row + 1 == p.rows {
                            Some((bl, br))
                        } else {
                            None
                        };
                        let (left, right) = if let Some(corners) = corners {
                            for col in x[r] + 1..x[r] + widths[r] - 1 {
                                canvas.put(
                                    col,
                                    p.y + row,
                                    if options.ascii { '-' } else { '━' },
                                )?;
                            }
                            canvas.paint(x[r] + 1, p.y + row, widths[r] - 2, border);
                            if options.ascii { ('+', '+') } else { corners }
                        } else {
                            (side, side)
                        };
                        canvas.put(x[r], p.y + row, left)?;
                        canvas.put(x[r] + widths[r] - 1, p.y + row, right)?;
                        canvas.paint(x[r], p.y + row, 1, border);
                        canvas.paint(x[r] + widths[r] - 1, p.y + row, 1, border);
                    }
                    let text = &short[i];
                    let text_width = columns(text).unwrap();
                    let col = x[r] + (widths[r] - text_width) / 2;
                    let row = p.y + (p.rows - 1) / 2;
                    canvas.text(col, row, text)?;
                    canvas.paint(col, row, text_width, label);
                }
            }
        }
        let mut out = canvas.finish();
        let arrow = if options.ascii { " -> " } else { " → " };
        let name = |i: usize| {
            let full = single_line(&labels[i]);
            if full == short[i] {
                full
            } else {
                format!("{} {full}", short[i])
            }
        };
        let mut legend = String::new();
        let mut colors = Layer::default();
        for (e, f) in flows.into_iter().enumerate() {
            let from = name(f.from);
            let to = name(f.to);
            let mark = shade(e, options.ascii, f.value == 0.0);
            writeln!(legend, "{mark} {from}{arrow}{to}: {}", f.amount).unwrap();
            colors.paint(0, e, 1, ribbon);
            colors.paint(2, e, from.width(), label);
            colors.paint(2 + from.width() + arrow.width(), e, to.width(), label);
        }
        out.push_str("\n\n");
        out.push_str(&colors.apply(legend.trim_end_matches('\n')));
        Ok(out)
    }
}
