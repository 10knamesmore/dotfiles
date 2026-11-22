use super::{Chart, ImageRenderer};
use crate::{Options, color::Rgb};

pub(crate) struct Entry {
    pub name: &'static str,
    pub example: &'static str,
    aliases: &'static [&'static str],
    pub text: fn(&str, &Options) -> Result<String, String>,
    pub image: Option<ImageRenderer>,
    title_colors: &'static [&'static str],
}

impl Entry {
    const fn new<C: Chart>(
        name: &'static str,
        aliases: &'static [&'static str],
        example: &'static str,
    ) -> Self {
        Self {
            name,
            aliases,
            example,
            text: C::render_text,
            image: C::IMAGE,
            title_colors: C::TITLE_COLORS,
        }
    }

    fn headers(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.name).chain(self.aliases.iter().copied())
    }

    pub fn title_color(&self, options: &Options) -> Option<Rgb> {
        if !options.color {
            return None;
        }
        self.title_colors
            .iter()
            .find_map(|key| options.config.theme_color(key))
    }

    pub fn image_renderer(&self) -> Result<ImageRenderer, String> {
        self.image.ok_or_else(|| {
            format!(
                "no image renderer for {}; see --png --list-types for supported diagrams",
                self.name
            )
        })
    }
}

macro_rules! chart {
    ($module:ident, $name:literal $(, $alias:literal)*) => {
        Entry::new::<super::$module::Renderer>(
            $name,
            &[$($alias),*],
            include_str!(concat!("../../examples/", $name, ".mmd")),
        )
    };
}

const FAMILIES: &[&[Entry]] = &[
    &[chart!(
        flowchart,
        "flowchart",
        "graph",
        "flowchart-v2",
        "flowchart-elk"
    )],
    &[chart!(sequence, "sequenceDiagram")],
    &[chart!(zenuml, "zenuml")],
    &[chart!(state, "stateDiagram-v2", "stateDiagram")],
    &[chart!(class, "classDiagram", "classDiagram-v2")],
    &[chart!(er, "erDiagram")],
    &[chart!(mindmap, "mindmap")],
    &[chart!(pie, "pie")],
    &[chart!(timeline, "timeline")],
    &[chart!(gantt, "gantt")],
    &[chart!(git_graph, "gitGraph")],
    &[chart!(packet, "packet-beta", "packet")],
    &[chart!(xychart, "xychart-beta", "xychart")],
    &[chart!(quadrant, "quadrantChart")],
    &[chart!(radar, "radar-beta")],
    &[chart!(architecture, "architecture-beta")],
    &[
        chart!(c4, "C4Context"),
        chart!(c4, "C4Container"),
        chart!(c4, "C4Component"),
        chart!(c4, "C4Dynamic"),
        chart!(c4, "C4Deployment"),
    ],
    &[chart!(block, "block-beta", "block")],
    &[chart!(requirement, "requirementDiagram")],
    &[chart!(sankey, "sankey-beta", "sankey")],
    &[chart!(treemap, "treemap-beta", "treemap")],
    &[chart!(kanban, "kanban")],
    &[chart!(journey, "journey")],
    &[chart!(tree_view, "treeView-beta", "treeView")],
    &[chart!(ishikawa, "ishikawa-beta", "ishikawa")],
    &[chart!(venn, "venn-beta")],
    &[chart!(
        event_modeling,
        "eventmodeling",
        "eventmodeling-beta"
    )],
    &[chart!(info, "info")],
    &[chart!(swimlane, "swimlane-beta")],
    &[chart!(agentflow, "agentflow-beta", "agentflow")],
    &[chart!(usecase, "usecase-beta")],
    &[chart!(wardley, "wardley-beta")],
    &[chart!(cynefin, "cynefin-beta")],
    &[
        chart!(railroad, "railroad-beta"),
        chart!(railroad, "railroad-abnf-beta"),
        chart!(railroad, "railroad-ebnf-beta"),
        chart!(railroad, "railroad-peg-beta"),
    ],
];

pub(crate) fn list(images: bool) -> String {
    FAMILIES
        .iter()
        .map(|family| {
            family
                .iter()
                .filter(|entry| !images || entry.image.is_some())
                .flat_map(Entry::headers)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn entries() -> impl Iterator<Item = &'static Entry> {
    FAMILIES.iter().flat_map(|family| family.iter())
}

pub(crate) fn lookup(header: &str) -> Result<&'static Entry, String> {
    entries()
        .find(|entry| entry.headers().any(|name| name == header))
        .ok_or_else(|| format!("unsupported Mermaid diagram: {header} (see --list-types)"))
}

pub(crate) fn select_examples(names: &[String]) -> Result<Vec<&'static Entry>, String> {
    if names.is_empty() {
        return Ok(entries().collect());
    }
    let mut selected = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for name in names {
        let entry = lookup(name)?;
        if seen.insert(entry.name) {
            selected.push(entry);
        }
    }
    Ok(selected)
}
