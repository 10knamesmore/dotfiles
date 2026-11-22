mod image;
mod lanes;

use super::shared::history::{self, quoted};
use crate::Options;
use rustc_hash::FxHashMap;

pub(super) struct Renderer;

impl super::Chart for Renderer {
    const IMAGE: Option<super::ImageRenderer> = Some(image::render);

    fn render_text(source: &str, options: &Options) -> Result<String, String> {
        let (suffix, lines) = history::header(source, &["gitGraph"])?;
        let (direction, rest) = orientation(suffix)?;
        let document = history::parse(rest, lines)?;
        let history = History::parse(&document.statements, options)?;
        let drawing = lanes::draw(&history.branches, &history.events, direction, options)?;
        Ok(document.finish(&drawing))
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Auto,
    Right,
    Down,
    Up,
}

#[derive(Clone, Copy)]
enum Mark {
    Normal,
    Reverse,
    Highlight,
    Merge,
    Pick,
}

impl Mark {
    fn glyph(self, ascii: bool) -> char {
        match (self, ascii) {
            (Self::Normal, false) => '●',
            (Self::Normal, true) => '*',
            (Self::Reverse, false) => '⊗',
            (Self::Reverse, true) => 'x',
            (Self::Highlight, false) => '■',
            (Self::Highlight, true) => '#',
            (Self::Merge, false) => '◉',
            (Self::Merge, true) => '@',
            (Self::Pick, false) => '◆',
            (Self::Pick, true) => 'o',
        }
    }
}

struct Branch {
    name: String,
    order: Option<usize>,
    first: usize,
    last: usize,
    head: Option<usize>,
}

struct Event {
    lane: usize,
    from: Option<usize>,
    picked: Option<usize>,
    mark: Option<Mark>,
    label: String,
    tags: Vec<String>,
    fork_from: Option<usize>,
}

impl Event {
    fn caption(&self) -> String {
        let mut label = self.label.clone();
        for tag in &self.tags {
            label.push_str(" [");
            label.push_str(tag);
            label.push(']');
        }
        label
    }
}

struct Commit {
    id: String,
    lane: usize,
    parents: [Option<usize>; 2],
    is_merge: bool,
    time: usize,
}

fn atom(s: &str) -> Result<(String, &str), String> {
    if s.starts_with(['"', '\'']) {
        return quoted(s);
    }
    let end = s.find(char::is_whitespace).unwrap_or(s.len());
    let name = &s[..end];
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '/' | '-'))
    {
        return Err(format!("expected a name in '{s}'"));
    }
    Ok((name.to_owned(), s[end..].trim_start()))
}

fn attribute(s: &str) -> Result<(&str, &str), String> {
    let (key, value) = s
        .split_once(':')
        .ok_or_else(|| format!("expected attribute followed by ':' in '{s}'"))?;
    Ok((key.trim_end(), value.trim_start()))
}

#[derive(Default)]
struct Attributes {
    id: Option<String>,
    message: Option<String>,
    parent: Option<String>,
    tags: Vec<String>,
    mark: Option<Mark>,
}

fn attributes(mut s: &str, op: &str) -> Result<Attributes, String> {
    let mut a = Attributes::default();
    while !s.is_empty() {
        let (key, value) = if op == "commit" && s.starts_with(['"', '\'']) {
            ("msg", s)
        } else {
            attribute(s)?
        };
        if key == "type" && op != "cherry-pick" {
            let (t, rest) = atom(value)?;
            a.mark = Some(match t.as_str() {
                "NORMAL" => Mark::Normal,
                "REVERSE" => Mark::Reverse,
                "HIGHLIGHT" => Mark::Highlight,
                _ => return Err(format!("unknown commit type '{t}'")),
            });
            s = rest;
        } else {
            let (text, rest) = quoted(value)?;
            match key {
                "id" => a.id = Some(text),
                "tag" => a.tags.push(text),
                "msg" if op == "commit" => a.message = Some(text),
                "parent" if op == "cherry-pick" => a.parent = Some(text),
                _ => return Err(format!("unknown {op} attribute '{key}'")),
            }
            s = rest;
        }
    }
    Ok(a)
}

struct History {
    branches: Vec<Branch>,
    branches_by_name: FxHashMap<String, usize>,
    commits: Vec<Commit>,
    commits_by_id: FxHashMap<String, usize>,
    current: usize,
    events: Vec<Event>,
}

impl History {
    fn parse(statements: &[(usize, String)], options: &Options) -> Result<Self, String> {
        let mut history = Self::new(
            options
                .config
                .git_graph
                .main_branch_name
                .as_deref()
                .unwrap_or("main"),
        );
        for (line, statement) in statements {
            history
                .execute(statement.trim_end())
                .map_err(|e| format!("gitGraph line {line}: {e}"))?;
        }
        Ok(history)
    }

    fn new(main_branch: &str) -> Self {
        Self {
            branches: vec![Branch {
                name: main_branch.into(),
                order: Some(0),
                first: 0,
                last: 0,
                head: None,
            }],
            branches_by_name: [(main_branch.into(), 0)].into_iter().collect(),
            commits: Vec::new(),
            commits_by_id: FxHashMap::default(),
            current: 0,
            events: Vec::new(),
        }
    }
    fn branch_index(&self, name: &str) -> Result<usize, String> {
        self.branches_by_name
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown branch '{name}'"))
    }
    fn push_event(&mut self, event: Event) {
        let time = self.events.len();
        self.branches[event.lane].last = time;
        if let Some(i) = event.from {
            self.branches[i].last = time;
        }
        self.events.push(event);
    }
    fn execute(&mut self, s: &str) -> Result<(), String> {
        let (op, rest) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
        let rest = rest.trim_start();
        let head = self.branches[self.current].head;
        match op {
            "branch" => {
                let (name, rest) = atom(rest)?;
                if self.branches_by_name.contains_key(&name) {
                    return Err(format!("branch '{name}' already exists"));
                }
                let order = if rest.is_empty() {
                    None
                } else {
                    let (key, n) = attribute(rest)?;
                    if key != "order" {
                        return Err(format!("unknown branch attribute '{key}'"));
                    }
                    Some(
                        n.parse()
                            .map_err(|_| format!("invalid branch order '{n}'"))?,
                    )
                };
                let from = self.current;
                self.current = self.branches.len();
                self.branches_by_name.insert(name.clone(), self.current);
                self.branches.push(Branch {
                    name,
                    order,
                    first: self.events.len(),
                    last: self.events.len(),
                    head,
                });
                self.push_event(Event {
                    lane: self.current,
                    from: Some(from),
                    picked: None,
                    mark: None,
                    label: String::new(),
                    tags: Vec::new(),
                    fork_from: head,
                });
            }
            "switch" | "checkout" => {
                let (name, rest) = atom(rest)?;
                if !rest.is_empty() {
                    return Err(format!("unexpected text after branch name: {rest}"));
                }
                self.current = self.branch_index(&name)?;
            }
            "commit" | "merge" | "cherry-pick" => {
                let mut parents = [head, None];
                let mut from = None;
                let mut picked = None;
                let mut label = String::new();
                let mut mark = Mark::Normal;
                let rest = if op == "merge" {
                    let (name, rest) = atom(rest)?;
                    let lane = self.branch_index(&name)?;
                    if lane == self.current {
                        return Err("cannot merge a branch into itself".into());
                    }
                    let other = self.branches[lane]
                        .head
                        .ok_or_else(|| format!("branch '{name}' has no commits"))?;
                    let current = head.ok_or("current branch has no commits")?;
                    if current == other {
                        return Err("both branches have the same head".into());
                    }
                    parents[1] = Some(other);
                    from = Some(lane);
                    mark = Mark::Merge;
                    rest
                } else {
                    rest
                };
                let mut a = attributes(rest, op)?;
                if op == "cherry-pick" {
                    let id = a.id.take().ok_or("cherry-pick requires id")?;
                    let source = self
                        .commits_by_id
                        .get(&id)
                        .copied()
                        .ok_or_else(|| format!("unknown commit '{id}'"))?;
                    let c = &self.commits[source];
                    if c.lane == self.current {
                        return Err("cannot cherry-pick from the current branch".into());
                    }
                    if head.is_none() {
                        return Err("current branch has no commits".into());
                    }
                    if c.is_merge && a.parent.is_none() {
                        return Err("cherry-picking a merge requires parent".into());
                    }
                    label = format!("pick {id}");
                    if let Some(p) = a.parent {
                        if !c.parents.iter().flatten().any(|&i| self.commits[i].id == p) {
                            return Err(format!("'{p}' is not an immediate parent of '{id}'"));
                        }
                        label.push_str(&format!(" (parent {p})"));
                    }
                    parents[1] = Some(source);
                    picked = Some(c.time);
                    mark = Mark::Pick;
                }
                let id = if let Some(id) = a.id.filter(|s| !s.is_empty()) {
                    if op == "merge" && self.commits_by_id.contains_key(&id) {
                        return Err(format!("duplicate merge id '{id}'"));
                    }
                    id
                } else {
                    let mut id = format!("#{}", self.commits.len() + 1);
                    while self.commits_by_id.contains_key(&id) {
                        id.push('\'');
                    }
                    id
                };
                if label.is_empty() {
                    label = id.clone();
                } else {
                    label = format!("{id}: {label}");
                }
                if let Some(msg) = a.message.filter(|s| !s.is_empty()) {
                    label.push_str(": ");
                    label.push_str(&msg);
                }
                let index = self.commits.len();
                self.commits_by_id.insert(id.clone(), index);
                self.commits.push(Commit {
                    id,
                    lane: self.current,
                    parents,
                    is_merge: op == "merge",
                    time: self.events.len(),
                });
                self.branches[self.current].head = Some(index);
                self.push_event(Event {
                    lane: self.current,
                    from,
                    picked,
                    mark: Some(a.mark.unwrap_or(mark)),
                    label,
                    tags: a.tags.into_iter().filter(|tag| !tag.is_empty()).collect(),
                    fork_from: None,
                });
            }
            _ => return Err(format!("unknown gitGraph statement '{op}'")),
        }
        Ok(())
    }
}

fn orientation(suffix: &str) -> Result<(Direction, &str), String> {
    let suffix = suffix.trim_start();
    let token = suffix.split([' ', '\t', ':']).next().unwrap_or("");
    let direction = match token {
        "LR" => Direction::Right,
        "TB" => Direction::Down,
        "BT" => Direction::Up,
        _ => return Ok((Direction::Auto, suffix.strip_prefix(':').unwrap_or(suffix))),
    };
    let rest = suffix[token.len()..]
        .trim_start()
        .strip_prefix(':')
        .ok_or_else(|| format!("expected ':' following {token}"))?;
    Ok((direction, rest))
}

fn branch_order(branches: &[Branch]) -> Vec<usize> {
    let mut order: Vec<_> = (0..branches.len()).collect();
    order.sort_by_key(|&n| {
        (
            n != 0 && branches[n].order.is_some(),
            branches[n].order.unwrap_or(0),
            n,
        )
    });
    order
}
