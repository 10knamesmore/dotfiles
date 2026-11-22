use super::{Pen, Rows, width};

pub struct Node<T> {
    pub label: T,
    pub depth: usize,
    pub last: bool,
    pub parent: Option<usize>,
}

pub struct Forest<T> {
    pub nodes: Vec<Node<T>>,
    path: Vec<(usize, usize)>,
}

impl<T> Default for Forest<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            path: Vec::new(),
        }
    }
}

impl<T> Forest<T> {
    pub fn push(&mut self, indentation: usize, label: T) -> Result<(), String> {
        let mut previous = None;
        while self
            .path
            .last()
            .is_some_and(|&(level, _)| level >= indentation)
        {
            previous = self.path.pop();
        }
        if let Some((_, index)) = previous {
            self.nodes[index].last = false;
        }
        let depth = self.path.len();
        if depth > 128 {
            return Err("tree exceeds 128 levels of nesting".into());
        }
        let parent = self.path.last().map(|&(_, index)| index);
        self.path.push((indentation, self.nodes.len()));
        self.nodes.push(Node {
            depth,
            label,
            last: true,
            parent,
        });
        Ok(())
    }
}

// Each vertical guide belongs to an ancestor with another sibling below it.
pub fn branches<T>(
    nodes: &[Node<T>],
    first_depth: usize,
    show_roots: bool,
    limit: usize,
    pen: Pen,
    mut render_node: impl FnMut(&T, usize, usize, usize) -> Rows,
) -> Rows {
    let mut rows = Rows::new();
    let mut guides = Vec::new();
    for node in nodes {
        let depth = node.depth - first_depth;
        guides.truncate(depth);
        let mut stem = String::new();
        for &pending in guides.iter().skip(usize::from(!show_roots)) {
            stem.push(if pending { pen.ch('│') } else { ' ' });
            stem.push_str("  ");
        }
        let joint = if node.last { '└' } else { '├' };
        let branch = if depth > 0 || show_roots {
            format!("{}{} ", pen.ch(joint), pen.ch('─'))
        } else {
            String::new()
        };
        let continuation = if branch.is_empty() {
            String::new()
        } else {
            format!("{}  ", if node.last { ' ' } else { pen.ch('│') })
        };
        let prefix = width(&stem) + width(&branch);
        let lines = render_node(
            &node.label,
            rows.len(),
            prefix,
            limit.saturating_sub(prefix).max(8),
        );
        for (i, text) in lines.into_iter().enumerate() {
            rows.push(format!(
                "{stem}{}{text}",
                if i == 0 { &branch } else { &continuation }
            ));
        }
        guides.push(!node.last);
    }
    rows
}
