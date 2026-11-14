import type { DesignChanges, DesignState } from "./types.js";

/** Capture the design at message-send time, including messages queued during a model turn. */
export function designContext(state: DesignState, changes: DesignChanges, first: boolean): string {
  const selected = state.selection.element;
  const data = {
    project: state.project,
    revision: state.revision,
    selection: state.selection,
    selected_element: selected ? state.design.elements[selected] : undefined,
    artboards: state.workspace.artboards,
    parameters: Object.fromEntries(
      Object.entries(state.design.parameters).map(([name, parameter]) => [name, parameter.value]),
    ),
    comments: state.workspace.comments.filter((comment) => !comment.resolved),
    elements: first ? state.design.elements : undefined,
    changes: changes.changes.filter((change) => change.revision <= state.revision),
  };
  return [
    "<pi_design_context>",
    "Browser design state captured when this message was sent. Edits already persist in project files.",
    "Read the design skill before working. Read the latest full state with pi_design.current().read_state().",
    JSON.stringify(data),
    "</pi_design_context>",
  ].join("\n");
}

export function displayDesignContext(markdown: string): string {
  return markdown
    .replace(/\n\n<pi_design_context>\n[\s\S]*?\n<\/pi_design_context>$/, "\n\n*Design context attached*")
    .replace(/\n\n<pi_design_new>\n[\s\S]*?\n<\/pi_design_new>$/, "\n\n*New design*");
}
