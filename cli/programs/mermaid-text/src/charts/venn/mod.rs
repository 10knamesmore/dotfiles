use super::shared::sets::body;
use crate::{Options, color::Style, text::wrap};
use std::collections::{BTreeMap, BTreeSet};

mod circles;
mod image;

#[derive(Default)]
struct Region {
    members: Vec<String>,
    label: Option<String>,
    size: Option<f64>,
    style: Style,
    items: Vec<Item>,
}

struct Item {
    id: String,
    label: Option<String>,
    style: Style,
}

impl Item {
    fn text(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.id)
    }
}

impl Region {
    fn name(&self, ascii: bool) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| self.members.join(if ascii { " & " } else { " ∩ " }))
    }
}

struct Diagram {
    title: String,
    regions: Vec<Region>,
}

struct Tokens<'a>(&'a str);

impl<'a> Tokens<'a> {
    fn eat(&mut self, prefix: &str) -> bool {
        if let Some(rest) = self.0.trim_start().strip_prefix(prefix) {
            self.0 = rest.trim_start();
            true
        } else {
            false
        }
    }

    fn atom(&mut self) -> Result<String, String> {
        self.0 = self.0.trim_start();
        if let Some(s) = self.0.strip_prefix('"') {
            let (value, rest) = s.split_once('"').ok_or("unclosed Venn string")?;
            self.0 = rest.trim_start();
            return Ok(value.into());
        }
        let n = self
            .0
            .find(|c: char| !(c.is_alphanumeric() || "_-+.".contains(c)))
            .unwrap_or(self.0.len());
        if n == 0 {
            return Err(format!("expected Venn identifier in `{}`", self.0));
        }
        let value = self.0[..n].to_owned();
        self.0 = self.0[n..].trim_start();
        Ok(value)
    }

    fn members(&mut self) -> Result<Vec<String>, String> {
        let mut ids = vec![self.atom()?];
        while self.eat(",") {
            ids.push(self.atom()?);
        }
        ids.sort();
        if ids.windows(2).any(|v| v[0] == v[1]) {
            return Err("repeated set in Venn intersection".into());
        }
        Ok(ids)
    }

    fn label(&mut self) -> Result<Option<String>, String> {
        if !self.eat("[") {
            return Ok(None);
        }
        if self.0.starts_with('"') {
            let value = self.atom()?;
            if !self.eat("]") {
                return Err("expected ] after quoted Venn label".into());
            }
            return Ok(Some(value));
        }
        let (label, rest) = self.0.split_once(']').ok_or("unclosed Venn label")?;
        self.0 = rest.trim_start();
        Ok(Some(label.trim().to_owned()))
    }

    fn end(&self) -> Result<(), String> {
        if self.0.trim().is_empty() {
            Ok(())
        } else {
            Err(format!("unexpected Venn syntax: `{}`", self.0))
        }
    }
}

fn style_properties(text: &str) -> Result<Style, String> {
    let mut style = Style::default();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut start = 0;
    for (end, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ',')))
    {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.checked_sub(1).ok_or("unmatched ) in Venn style")?,
            ',' if depth == 0 && !quoted => {
                let field = text[start..end].trim();
                let (key, value) = field
                    .split_once(':')
                    .ok_or_else(|| format!("expected Venn style property: {field}"))?;
                if key.is_empty()
                    || !key
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-')
                    || value.trim().is_empty()
                {
                    return Err(format!("invalid Venn style property: {field}"));
                }
                style = style.overlay(Style::css(field));
                start = end + 1;
            }
            _ => {}
        }
    }
    if quoted || depth != 0 {
        Err("unclosed Venn style value".into())
    } else {
        Ok(style)
    }
}

fn parse(body: &str) -> Result<Diagram, String> {
    let mut title = String::new();
    let mut regions: Vec<Region> = Vec::new();
    let mut index = BTreeMap::new();
    let mut text_names = BTreeSet::new();
    let mut styles = BTreeMap::<Vec<String>, Style>::new();
    let mut parent = None;
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let (keyword, args) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let mut tokens = Tokens(args.trim());
        match keyword.to_ascii_lowercase().as_str() {
            "title" => {
                title = if tokens.0.starts_with('"') {
                    let title = tokens.atom()?;
                    tokens.end()?;
                    title
                } else {
                    tokens.0.into()
                };
            }
            kind @ ("set" | "union") => {
                let members = tokens.members()?;
                if (kind == "set") != (members.len() == 1) {
                    return Err(format!("invalid {kind} membership: {line}"));
                }
                let label = tokens.label()?;
                let size = if tokens.eat(":") {
                    let number = tokens
                        .0
                        .parse::<f64>()
                        .map_err(|_| format!("invalid Venn size: {}", tokens.0))?;
                    if !number.is_finite() || number < 0.0 {
                        return Err(format!("invalid Venn size: {number}"));
                    }
                    Some(number)
                } else {
                    tokens.end()?;
                    None
                };
                let i = regions.len();
                if i >= 2048 {
                    return Err("Venn diagram exceeds 2048 regions".into());
                }
                if index.insert(members.clone(), i).is_some() {
                    return Err(format!("duplicate Venn region: {members:?}"));
                }
                regions.push(Region {
                    members,
                    label,
                    size,
                    style: Style::default(),
                    items: Vec::new(),
                });
                parent = Some(i);
            }
            "text" => {
                let names = tokens.members()?;
                let (target, name) = if tokens.0.is_empty() || tokens.0.starts_with('[') {
                    if names.len() != 1 {
                        return Err("expected a text node after Venn target".into());
                    }
                    (
                        parent.ok_or("Venn text has no containing region")?,
                        names[0].clone(),
                    )
                } else {
                    (
                        *index
                            .get(&names)
                            .ok_or_else(|| format!("unknown Venn text target: {names:?}"))?,
                        tokens.atom()?,
                    )
                };
                let label = tokens.label()?;
                tokens.end()?;
                text_names.insert(name.clone());
                regions[target].items.push(Item {
                    id: name,
                    label,
                    style: Style::default(),
                });
            }
            "style" => {
                let names = tokens.members()?;
                let style = styles.entry(names).or_default();
                *style = style.overlay(style_properties(tokens.0)?);
            }
            _ => return Err(format!("unknown Venn statement: {line}")),
        }
    }
    for region in &regions {
        for name in &region.members {
            if !index.contains_key(std::slice::from_ref(name)) {
                return Err(format!("unknown Venn set: {name}"));
            }
        }
    }
    for (names, style) in &styles {
        if let Some(&i) = index.get(names) {
            regions[i].style = *style;
        } else if !(names.len() == 1 && text_names.contains(&names[0])) {
            return Err(format!("unknown Venn style target: {names:?}"));
        }
    }
    for region in &mut regions {
        for item in &mut region.items {
            if let Some(&style) = styles.get(std::slice::from_ref(&item.id)) {
                item.style = style;
            }
        }
    }
    Ok(Diagram { title, regions })
}

struct Panel<'a> {
    sets: Vec<&'a Region>,
    intersections: Vec<(u32, &'a Region)>,
}

fn panels(diagram: &Diagram) -> Result<Vec<Panel<'_>>, String> {
    let sets: Vec<_> = diagram
        .regions
        .iter()
        .filter(|r| r.members.len() == 1)
        .collect();
    if sets.is_empty() {
        return Err("Venn diagram has no sets".into());
    }
    let indices: BTreeMap<_, _> = sets
        .iter()
        .enumerate()
        .map(|(i, r)| (&r.members[0], i))
        .collect();
    let mut groups: Vec<_> = (0..sets.len()).collect();
    let mut intersections = Vec::new();
    for region in diagram.regions.iter().filter(|r| r.members.len() > 1) {
        let members: Vec<_> = region.members.iter().map(|id| indices[id]).collect();
        if members.len() > 16 {
            return Err("Venn projection exceeds 16 intersecting sets".into());
        }
        for &member in &members[1..] {
            let old = groups[member];
            let new = groups[members[0]];
            if old != new {
                for group in &mut groups {
                    if *group == old {
                        *group = new;
                    }
                }
            }
        }
        intersections.push((members, region));
    }
    let mut seen = BTreeSet::new();
    let mut panels = Vec::new();
    for &group in &groups {
        if !seen.insert(group) {
            continue;
        }
        let component: Vec<_> = (0..sets.len()).filter(|&i| groups[i] == group).collect();
        let local: Vec<_> = intersections
            .iter()
            .filter(|(ids, _)| groups[ids[0]] == group)
            .collect();
        if component.len() <= 3 {
            let circles: Vec<_> = component.iter().map(|&i| sets[i]).collect();
            let regions: Vec<_> = local
                .iter()
                .map(|(ids, r)| {
                    let mask = ids
                        .iter()
                        .map(|id| 1 << component.iter().position(|n| n == id).unwrap())
                        .sum();
                    (mask, *r)
                })
                .collect();
            panels.push(Panel {
                sets: circles,
                intersections: regions,
            });
        } else {
            // A circle arrangement cannot realize every membership pattern of four sets.
            // Give each declared intersection its own local projection.
            for (ids, region) in local {
                let circles: Vec<_> = ids.iter().map(|&i| sets[i]).collect();
                let regions = vec![((1 << ids.len()) - 1, *region)];
                panels.push(Panel {
                    sets: circles,
                    intersections: regions,
                });
            }
        }
    }
    Ok(panels)
}

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let diagram = parse(body(source, |header| {
            header.eq_ignore_ascii_case("venn-beta")
        })?)?;
        let mut output = wrap(&diagram.title, options.width.max(12)).join("\n");
        for Panel {
            sets,
            intersections,
        } in panels(&diagram)?
        {
            let panel = circles::draw(&sets, &intersections, options)?;
            if output.len() + panel.len() > 4 * 1024 * 1024 {
                return Err("Venn output exceeds 4 MiB".into());
            }
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&panel);
        }
        Ok(output)
    }
}
