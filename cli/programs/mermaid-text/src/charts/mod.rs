use crate::{Options, graphics::Scene};

mod agentflow;
mod architecture;
mod block;
mod c4;
mod catalog;
mod class;
mod cynefin;
mod er;
mod event_modeling;
mod flowchart;
mod gantt;
mod git_graph;
mod info;
mod ishikawa;
mod journey;
mod kanban;
mod mindmap;
mod packet;
mod pie;
mod quadrant;
mod radar;
mod railroad;
mod requirement;
mod sankey;
mod sequence;
mod shared;
mod state;
mod swimlane;
mod timeline;
mod tree_view;
mod treemap;
mod usecase;
mod venn;
mod wardley;
mod xychart;
mod zenuml;

pub(crate) use catalog::{Entry, list, lookup, select_examples};

type ImageRenderer = fn(&str, &Options, &mut Scene) -> Result<(), String>;

trait Chart {
    const IMAGE: Option<ImageRenderer> = None;
    const TITLE_COLORS: &'static [&'static str] = &["titleColor"];

    fn render_text(source: &str, options: &Options) -> Result<String, String>;
}
