mod image;
mod layout;
mod paint;
pub mod parse;
mod routes;

use crate::Options;
use crate::graphics::Scene;

#[derive(Clone, Copy, PartialEq)]
pub enum Family {
    Swimlane,
    Agentflow,
}

#[derive(Clone, Copy)]
pub struct Syntax {
    pub family: Family,
    pub headers: &'static [&'static str],
    pub statement: fn(&mut parse::Parser, &str, &str) -> Result<bool, String>,
}

#[derive(Clone, Copy)]
struct Direction {
    horizontal: bool,
    reverse: bool,
}

struct Item {
    style: crate::color::Style,
    label: String,
    shape: String,
    owner: usize,
    group: bool,
    global: bool,
    collapsed: bool,
    lost_member: bool,
}

impl Item {
    fn new(label: &str, owner: usize) -> Self {
        Self {
            style: crate::color::Style::default(),
            label: label.into(),
            shape: String::new(),
            owner,
            group: false,
            global: false,
            collapsed: false,
            lost_member: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Line {
    Solid,
    Dotted,
    Thick,
}

#[derive(Clone, Copy, PartialEq)]
enum Tip {
    None,
    Arrow,
    Cross,
}

struct Edge {
    style: crate::color::Style,
    from: usize,
    to: usize,
    line: Line,
    tip: Tip,
    label: String,
}

struct Diagram {
    family: Family,
    direction: Direction,
    // Index zero is the containing page.
    items: Vec<Item>,
    edges: Vec<Edge>,
    title: String,
    description: String,
}

impl Diagram {
    fn visible_children(&mut self) -> Result<Vec<Vec<usize>>, String> {
        let mut visible = vec![0; self.items.len()];
        let mut children = vec![Vec::new(); self.items.len()];
        for id in 1..self.items.len() {
            let mut representative = id;
            let mut parent = self.items[id].owner;
            let mut depth = 0;
            while parent != 0 {
                if self.items[parent].collapsed {
                    representative = parent;
                }
                parent = self.items[parent].owner;
                depth += 1;
                if depth > 128 {
                    return Err("cyclic containment or more than 128 nested groups".into());
                }
            }
            visible[id] = representative;
            if representative == id {
                children[self.items[id].owner].push(id);
            }
        }
        self.edges.retain_mut(|e| {
            let (a, b) = (visible[e.from], visible[e.to]);
            let internal = a == b && (a != e.from || b != e.to);
            (e.from, e.to) = (a, b);
            !internal
        });
        let mut incident = vec![false; self.items.len()];
        for edge in &self.edges {
            incident[edge.from] = true;
            incident[edge.to] = true;
        }
        let keep: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| !item.lost_member || !children[i].is_empty() || incident[i])
            .collect();
        for members in &mut children {
            members.retain(|&id| keep[id]);
        }
        Ok(children)
    }
}

fn prepared(source: &str, syntax: Syntax) -> Result<(Diagram, Vec<Vec<usize>>), String> {
    let mut diagram = parse::diagram(source, syntax)?;
    let children = diagram.visible_children()?;
    if children[0].is_empty() {
        return Err("diagram has no lanes, flows or steps".into());
    }
    Ok((diagram, children))
}

pub fn render(source: &str, options: &Options, syntax: Syntax) -> Result<String, String> {
    let (diagram, children) = prepared(source, syntax)?;
    let plan = layout::place(
        &diagram,
        &children,
        layout::Metrics::Terminal(options.width),
    );
    paint::draw(&diagram, &plan, options)
}

pub fn render_image(
    source: &str,
    options: &Options,
    scene: &mut Scene,
    syntax: Syntax,
) -> Result<(), String> {
    let (diagram, children) = prepared(source, syntax)?;
    let plan = layout::place(&diagram, &children, layout::Metrics::Image(scene));
    image::draw(&diagram, &plan, options, scene)
}
