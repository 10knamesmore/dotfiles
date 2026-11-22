use rustc_hash::FxHashMap;

use crate::color::Style;

use crate::charts::shared::block_layout::{
    Diagram, Edge, Grid, Kind, Line, Node, Tip, class_names, label, paint_statement, statements,
};

const KINDS: [&str; 7] = [
    "requirement",
    "functionalRequirement",
    "interfaceRequirement",
    "performanceRequirement",
    "physicalRequirement",
    "designConstraint",
    "element",
];
const RELATIONS: [&str; 7] = [
    "contains",
    "copies",
    "derives",
    "satisfies",
    "verifies",
    "refines",
    "traces",
];

fn unquote(s: &str) -> Result<&str, String> {
    let s = s.trim();
    if let Some(s) = s.strip_prefix('"') {
        s.strip_suffix('"')
            .filter(|s| !s.contains('"'))
            .ok_or_else(|| "invalid quoted name or value".into())
    } else if s.contains('"') {
        Err("unexpected quote in name or value".into())
    } else {
        Ok(s)
    }
}

fn name(s: &str) -> Result<&str, String> {
    let s = unquote(s)?;
    if s.trim().is_empty() || s.contains(['{', '}', '\n']) {
        Err("expected a requirement or element name".into())
    } else {
        Ok(s)
    }
}

fn field(node: &mut Node, line: &str) -> Result<(), String> {
    let Kind::Requirement { kind, fields } = &mut node.kind else {
        unreachable!()
    };
    let (k, v) = line.split_once(':').ok_or("expected `attribute: value`")?;
    let v = label(unquote(v)?);
    let key = match (kind.as_str(), k.trim().to_ascii_lowercase().as_str()) {
        ("element", "type") => "Type",
        ("element", "docref") => "DocRef",
        ("element", _) => return Err(format!("unknown element attribute: {k}")),
        (_, "id") => "ID",
        (_, "text") => "Text",
        (_, "risk")
            if ["low", "medium", "high"]
                .iter()
                .any(|s| v.eq_ignore_ascii_case(s)) =>
        {
            "Risk"
        }
        (_, "verifymethod")
            if ["analysis", "inspection", "test", "demonstration"]
                .iter()
                .any(|s| v.eq_ignore_ascii_case(s)) =>
        {
            "Verify method"
        }
        (_, "risk" | "verifymethod") => return Err(format!("unknown {k} value: {v}")),
        _ => return Err(format!("unknown requirement attribute: {k}")),
    };
    if let Some((_, val)) = fields.iter_mut().find(|(k, _)| k == key) {
        *val = v;
    } else {
        fields.push((key.to_owned(), v));
    }
    Ok(())
}

fn relation(line: &str) -> Result<(&str, &'static str, &str), String> {
    let mut quoted = false;
    for (at, ch) in line.char_indices() {
        if ch == '"' {
            quoted = !quoted;
        }
        if quoted {
            continue;
        }
        let rest = &line[at..];
        let (reverse, rest) = if let Some(s) = rest.strip_prefix("<-") {
            (true, s)
        } else if let Some(s) = rest.strip_prefix('-') {
            (false, s)
        } else {
            continue;
        };
        for verb in RELATIONS {
            if let Some(target) = rest.trim_start().strip_prefix(verb).and_then(|s| {
                s.trim_start()
                    .strip_prefix(if reverse { "-" } else { "->" })
            }) {
                let (a, b) = (name(&line[..at])?, name(target)?);
                return Ok(if reverse { (b, verb, a) } else { (a, verb, b) });
            }
        }
    }
    Err(format!(
        "expected requirement declaration or typed relationship: {line}"
    ))
}

pub(super) fn parse(body: &str) -> Result<(Diagram, Grid), String> {
    let mut diagram = Diagram::default();
    let mut edges = Vec::new();
    let mut active = None;
    let mut description = false;
    let mut direction = "TB";
    for (n, line) in statements(body)? {
        let mut process = || -> Result<(), String> {
            if description {
                let (s, closed) = line.strip_suffix('}').map_or((line, false), |s| (s, true));
                if !s.trim().is_empty() {
                    diagram.caption.push(s.trim().into());
                }
                description = !closed;
                return Ok(());
            }
            if active.is_none() {
                if let Some(s) = line
                    .strip_prefix("accTitle:")
                    .or_else(|| line.strip_prefix("accDescr:"))
                {
                    diagram.caption.push(s.trim().into());
                    return Ok(());
                }
                if let Some(s) = line
                    .strip_prefix("accDescr")
                    .and_then(|s| s.trim_start().strip_prefix('{'))
                {
                    let (s, closed) = s.strip_suffix('}').map_or((s, false), |s| (s, true));
                    if !s.trim().is_empty() {
                        diagram.caption.push(s.trim().into());
                    }
                    description = !closed;
                    return Ok(());
                }
                if paint_statement(line)?.is_some() {
                    return Ok(());
                }
                if let Some(dir) = line
                    .strip_prefix("direction")
                    .and_then(|s| s.strip_prefix(char::is_whitespace))
                {
                    direction = match dir.trim() {
                        "TB" => "TB",
                        "BT" => "BT",
                        "LR" => "LR",
                        "RL" => "RL",
                        _ => return Err(format!("unknown requirement direction: {dir}")),
                    };
                    return Ok(());
                }
            }
            let contents = if active.is_some() {
                line
            } else if let Some((decl, rest)) = line.split_once('{') {
                let (k, raw_name) = decl
                    .trim()
                    .split_once(char::is_whitespace)
                    .ok_or("expected requirement type and name")?;
                let kind = KINDS
                    .iter()
                    .find(|s| s.eq_ignore_ascii_case(k))
                    .ok_or_else(|| format!("unknown requirement type: {k}"))?;
                let id = if let Some((id, _)) =
                    raw_name.rsplit_once(":::").filter(|(_, c)| class_names(c))
                {
                    name(id)?
                } else {
                    name(raw_name)?
                };
                active = Some(diagram.insert(
                    id,
                    Node {
                        label: id.into(),
                        kind: Kind::Requirement {
                            kind: (*kind).into(),
                            fields: Vec::new(),
                        },
                        style: Style::default(),
                    },
                )?);
                rest.trim()
            } else {
                if let Some((id, class)) = line.rsplit_once(":::")
                    && class_names(class)
                    && diagram.names.contains_key(name(id)?)
                {
                    return Ok(());
                }
                edges.push(relation(line)?);
                return Ok(());
            };
            let (s, closed) = contents
                .strip_suffix('}')
                .map_or((contents, false), |s| (s.trim(), true));
            if !s.is_empty() {
                field(&mut diagram.nodes[active.unwrap()], s)?;
            }
            if closed {
                active = None;
            }
            Ok(())
        };
        process().map_err(|e| format!("requirement line {n}: {e}"))?;
    }
    if active.is_some() || description {
        return Err("missing `}` in requirement diagram".into());
    }
    let mut between: FxHashMap<(usize, usize), usize> = FxHashMap::default();
    for (a, verb, b) in edges {
        let find = |name: &str| {
            diagram
                .names
                .get(name)
                .copied()
                .ok_or_else(|| format!("unknown relationship endpoint: {name}"))
        };
        let (from, to) = (find(a)?, find(b)?);
        if let Some(&i) = between.get(&(from, to)) {
            diagram.edges[i].label.push('\n');
            diagram.edges[i].label.push_str(verb);
            continue;
        }
        between.insert((from, to), diagram.edges.len());
        diagram.edges.push(Edge {
            from,
            to,
            label: verb.into(),
            start: Tip::None,
            end: Tip::Arrow,
            line: Line::Solid,
        });
    }
    let grid = ranks(&diagram, direction)?;
    Ok((diagram, grid))
}

fn ranks(d: &Diagram, direction: &str) -> Result<Grid, String> {
    let n = d.nodes.len();
    if n == 0 {
        return Err("requirement diagram has no requirements or elements".into());
    }
    let mut degree = vec![0usize; n];
    let mut next = vec![Vec::new(); n];
    for e in &d.edges {
        if e.from != e.to {
            next[e.from].push(e.to);
            degree[e.to] += 1;
        }
    }
    let mut rank = vec![0; n];
    let mut done = vec![false; n];
    // Break cycles deterministically at the node with fewest outstanding predecessors.
    for _ in 0..n {
        let i = (0..n)
            .filter(|i| !done[*i])
            .min_by_key(|i| degree[*i])
            .unwrap();
        done[i] = true;
        for &j in &next[i] {
            if !done[j] {
                degree[j] -= 1;
                rank[j] = rank[j].max(rank[i] + 1);
            }
        }
    }
    let mut layers = vec![Vec::new(); rank.iter().max().unwrap() + 1];
    for (i, r) in rank.into_iter().enumerate() {
        layers[r].push(i);
    }
    if matches!(direction, "BT" | "RL") {
        layers.reverse();
    }
    let widest = layers.iter().map(Vec::len).max().unwrap();
    if widest * layers.len() > 65536 {
        return Err("requirement layout exceeds 65536 grid positions".into());
    }
    let horizontal = matches!(direction, "LR" | "RL");
    let (cols, rows) = if horizontal {
        (layers.len(), widest)
    } else {
        (widest, layers.len())
    };
    let mut grid = Grid {
        columns: Some(cols),
        slots: vec![(None, 1); cols * rows],
    };
    for (i, layer) in layers.into_iter().enumerate() {
        for (j, node) in layer.into_iter().enumerate() {
            let (col, row) = if horizontal { (i, j) } else { (j, i) };
            grid.slots[row * cols + col].0 = Some(node);
        }
    }
    Ok(grid)
}
