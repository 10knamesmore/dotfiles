use std::collections::BTreeMap;

use crate::charts::shared::architecture_layout::{self, Diagram, Edge, End, Input, Node, Side};
use crate::color::{Rgb, Style};

pub(super) fn parse(source: &str) -> Result<Diagram, String> {
    let mut scope = Vec::new();
    let diagram = architecture_layout::parse(
        source,
        |header| match header {
            "C4Dynamic" => Some(true),
            "C4Context" | "C4Container" | "C4Component" | "C4Deployment" => Some(false),
            _ => None,
        },
        |d, input, auto_number| {
            if input.take("}") {
                scope.pop().ok_or("unmatched closing boundary")?;
            } else if !input.take(";") {
                let key = input.id()?;
                if !input.heading(key, d)? {
                    statement(d, &mut scope, key, input, auto_number)?;
                }
            }
            Ok(())
        },
    )?;
    if let Some(group) = scope.last() {
        return Err(format!("unclosed boundary {group:?}"));
    }
    Ok(diagram)
}

struct Arg {
    name: Option<String>,
    value: String,
}

fn arguments(input: &mut Input<'_>) -> Result<Vec<Arg>, String> {
    input.expect("(")?;
    let mut args = Vec::new();
    if input.take(")") {
        return Ok(args);
    }
    loop {
        let name = if input.take("$") {
            let name = input.id()?.to_owned();
            input.expect("=")?;
            Some(name)
        } else {
            None
        };
        let value = if input.rest().starts_with(['"', '\'']) {
            input.quoted()?
        } else {
            let s = input.rest();
            let end = s.find([',', ')']).ok_or("unclosed C4 call")?;
            if s[..end].contains('(') {
                return Err("quote text containing parentheses".into());
            }
            input.0 = &s[end..];
            s[..end].trim().to_owned()
        };
        args.push(Arg { name, value });
        if input.take(")") {
            break;
        }
        input.expect(",")?;
    }
    Ok(args)
}

type Fields<'a> = BTreeMap<&'a str, String>;

fn fields<'a>(args: Vec<Arg>, names: &'a [&'a str]) -> Result<Fields<'a>, String> {
    let mut values = Fields::new();
    let mut pos = names.iter().copied();
    for arg in args {
        let name = match arg.name {
            Some(key) => *names
                .iter()
                .find(|&&n| n == key)
                .ok_or_else(|| format!("unknown parameter ${key}"))?,
            None => pos
                .find(|n| !values.contains_key(n))
                .ok_or("too many parameters")?,
        };
        if values
            .insert(name, arg.value)
            .is_some_and(|v| !v.is_empty())
        {
            return Err(format!("duplicate parameter {name}"));
        }
    }
    Ok(values)
}

fn required(values: &mut Fields<'_>, key: &str) -> Result<String, String> {
    values
        .remove(key)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("expected {key}"))
}

fn color(values: &Fields<'_>, name: &str) -> Option<Rgb> {
    Rgb::parse(values.get(name)?)
}

fn detail(values: &mut Fields<'_>, name: &str) -> Option<String> {
    let value = values.remove(name).filter(|s| !s.is_empty())?;
    Some(match name {
        "techn" | "type" => format!("[{value}]"),
        "tags" => format!("tags: {value}"),
        "sprite" => format!("sprite: {value}"),
        _ => value,
    })
}

const REL_FIELDS: &[&str] = &[
    "from", "to", "label", "techn", "descr", "sprite", "tags", "link",
];

fn statement(
    d: &mut Diagram,
    scope: &mut Vec<String>,
    key: &str,
    input: &mut Input<'_>,
    auto_number: bool,
) -> Result<(), String> {
    if key == "direction" {
        d.direction = Some(match input.line() {
            "TB" | "TD" => Side::Bottom,
            "BT" => Side::Top,
            "LR" => Side::Right,
            "RL" => Side::Left,
            other => return Err(format!("unknown direction {other:?}")),
        });
        return Ok(());
    }
    let args = arguments(input)?;
    match key {
        "UpdateLayoutConfig" => {
            let mut v = fields(args, &["c4ShapeInRow", "c4BoundaryInRow"])?;
            for (key, slot) in [
                ("c4ShapeInRow", &mut d.shapes_per_row),
                ("c4BoundaryInRow", &mut d.groups_per_row),
            ] {
                if let Some(raw) = v.remove(key) {
                    *slot = Some(
                        raw.parse::<usize>()
                            .ok()
                            .filter(|&n| n > 0)
                            .ok_or_else(|| format!("{key} must be a positive integer"))?,
                    );
                }
            }
        }
        "UpdateElementStyle" => {
            let mut v = fields(
                args,
                &[
                    "elementName",
                    "bgColor",
                    "fontColor",
                    "borderColor",
                    "shadowing",
                    "shape",
                    "sprite",
                    "techn",
                    "legendText",
                    "legendSprite",
                ],
            )?;
            let i = d.index(&required(&mut v, "elementName")?)?;
            let node = &mut d.nodes[i];
            node.style = node.style.overlay(Style {
                fill: color(&v, "bgColor"),
                stroke: color(&v, "borderColor"),
                text: color(&v, "fontColor"),
            });
            for name in ["techn", "sprite", "legendText", "legendSprite"] {
                if let Some(s) = detail(&mut v, name) {
                    node.details.push(s);
                }
            }
        }
        "UpdateRelStyle" => {
            let mut v = fields(
                args,
                &["from", "to", "textColor", "lineColor", "offsetX", "offsetY"],
            )?;
            let from = required(&mut v, "from")?;
            let to = required(&mut v, "to")?;
            d.index(&from)?;
            d.index(&to)?;
            if let Some(edge) = d
                .edges
                .iter_mut()
                .find(|e| e.from.id == from && e.to.id == to)
            {
                edge.style = edge.style.overlay(Style {
                    fill: None,
                    stroke: color(&v, "lineColor"),
                    text: color(&v, "textColor"),
                });
            }
        }
        "Rel" | "BiRel" | "RelIndex" | "Rel_Back" | "Rel_R" | "Rel_Right" | "Rel_L"
        | "Rel_Left" | "Rel_U" | "Rel_Up" | "Rel_D" | "Rel_Down" => {
            let mut v = if key == "RelIndex" {
                fields(
                    args,
                    &[
                        "index", "from", "to", "label", "techn", "descr", "sprite", "tags", "link",
                    ],
                )?
            } else {
                fields(args, REL_FIELDS)?
            };
            let index = if key == "RelIndex" {
                Some(required(&mut v, "index")?)
            } else if auto_number {
                Some((d.edges.len() + 1).to_string())
            } else {
                None
            };
            let from = required(&mut v, "from")?;
            let to = required(&mut v, "to")?;
            let mut label = v.remove("label").ok_or("expected relationship label")?;
            if let Some(index) = index {
                label = format!("{index}. {label}");
            }
            for name in ["techn", "descr", "sprite", "tags", "link"] {
                if let Some(value) = detail(&mut v, name) {
                    label.push(' ');
                    label.push_str(&value);
                }
            }
            let side = match key {
                "Rel_R" | "Rel_Right" => Some(Side::Right),
                "Rel_L" | "Rel_Left" => Some(Side::Left),
                "Rel_D" | "Rel_Down" => Some(Side::Bottom),
                "Rel_U" | "Rel_Up" => Some(Side::Top),
                _ => None,
            };
            d.edges.push(Edge {
                from: End {
                    id: from,
                    side,
                    group: false,
                    arrow: matches!(key, "Rel_Back" | "BiRel"),
                },
                to: End {
                    id: to,
                    side: side.map(Side::opposite),
                    group: false,
                    arrow: key != "Rel_Back",
                },
                label,
                style: Style::default(),
            });
        }
        _ => {
            let core = key.strip_suffix("_Ext").unwrap_or(key);
            let boundary = matches!(
                key,
                "Boundary" | "Enterprise_Boundary" | "System_Boundary" | "Container_Boundary"
            );
            let deployment = matches!(key, "Deployment_Node" | "Node" | "Node_L" | "Node_R");
            let typed_system = matches!(core, "SystemDb" | "SystemQueue")
                && args.iter().take_while(|a| a.name.is_none()).count() >= 4
                && args.iter().any(|a| a.name.as_deref() == Some("sprite"));
            let names: &[&str] = if boundary {
                &["alias", "label", "type", "sprite", "tags", "link"]
            } else if deployment {
                &["alias", "label", "type", "descr", "sprite", "tags", "link"]
            } else if matches!(
                core,
                "Component"
                    | "ComponentQueue"
                    | "ComponentDb"
                    | "Container"
                    | "ContainerQueue"
                    | "ContainerDb"
            ) || typed_system
            {
                &["alias", "label", "techn", "descr", "sprite", "tags", "link"]
            } else if matches!(core, "System" | "SystemDb" | "SystemQueue" | "Person") {
                &["alias", "label", "descr", "sprite", "tags", "link"]
            } else {
                return Err(format!("unknown C4 statement {key:?}"));
            };
            let mut v = fields(args, names)?;
            let id = required(&mut v, "alias")?;
            let title = v
                .remove("label")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| id.clone());
            let details = ["type", "techn", "descr", "sprite", "tags", "link"]
                .iter()
                .filter_map(|n| detail(&mut v, n))
                .collect();
            let parent = scope.last().cloned();
            if boundary || deployment {
                input.expect("{")?;
                scope.push(id.clone());
            }
            d.add(Node {
                id,
                title,
                kind: key.to_owned(),
                icon: String::new(),
                details,
                parent,
                boundary: boundary || deployment,
                style: Style::default(),
            })?;
        }
    }
    Ok(())
}
