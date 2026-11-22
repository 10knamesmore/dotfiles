use crate::charts::shared::architecture_layout::{self, Diagram, Edge, End, Input, Node, Side};
use crate::color::Style;

pub(super) fn parse(source: &str) -> Result<Diagram, String> {
    architecture_layout::parse(
        source,
        |header| (header == "architecture-beta").then_some(()),
        |d, input, ()| {
            let key = input.id()?;
            if !input.heading(key, d)? {
                statement(d, key, &mut Input(input.line()))?;
            }
            Ok(())
        },
    )
}

fn side(input: &mut Input<'_>) -> Result<Side, String> {
    for (s, side) in [
        ("L", Side::Left),
        ("R", Side::Right),
        ("T", Side::Top),
        ("B", Side::Bottom),
    ] {
        if input.take(s) {
            return Ok(side);
        }
    }
    Err("expected an L, R, T or B port".into())
}

fn statement(d: &mut Diagram, key: &str, s: &mut Input<'_>) -> Result<(), String> {
    match key {
        "group" | "service" | "junction" => {
            let id = s.id()?.to_owned();
            let icon = if key == "junction" {
                String::new()
            } else if s.take("(") {
                s.delimited(")")?
            } else if s.rest().starts_with(['\'', '"']) {
                s.quoted()?
            } else {
                String::new()
            };
            let title = if key != "junction" && s.take("[") {
                s.delimited("]")?
            } else if !icon.is_empty() {
                icon.clone()
            } else {
                id.clone()
            };
            let parent = if s.rest().is_empty() {
                None
            } else {
                if s.id()? != "in" {
                    return Err("expected 'in <group>'".into());
                }
                Some(s.id()?.to_owned())
            };
            d.add(Node {
                id,
                title,
                kind: key.into(),
                icon,
                details: Vec::new(),
                parent,
                boundary: key == "group",
                style: Style::default(),
            })?;
        }
        "align" => {
            let row = match s.id()? {
                "row" => true,
                "column" => false,
                _ => return Err("expected 'align row' or 'align column'".into()),
            };
            let mut ids = Vec::new();
            while !s.rest().is_empty() {
                ids.push(s.id()?.to_owned());
            }
            if ids.len() < 2 {
                return Err("align requires at least two members".into());
            }
            d.align.push((row, ids));
        }
        from => {
            let fg = s.take("{group}");
            s.expect(":")?;
            let fs = side(s)?;
            let fa = s.take("<");
            s.expect("-")?;
            let label = if s.take("[") {
                s.delimited("]")?
            } else {
                String::new()
            };
            s.expect("-")?;
            let ta = s.take(">");
            let ts = side(s)?;
            s.expect(":")?;
            let to = s.id()?.to_owned();
            let tg = s.take("{group}");
            d.edges.push(Edge {
                from: End {
                    id: from.into(),
                    side: Some(fs),
                    group: fg,
                    arrow: fa,
                },
                to: End {
                    id: to,
                    side: Some(ts),
                    group: tg,
                    arrow: ta,
                },
                label,
                style: Style::default(),
            });
        }
    }
    if !s.rest().is_empty() {
        return Err(format!("unexpected text {:?}", s.rest()));
    }
    Ok(())
}
