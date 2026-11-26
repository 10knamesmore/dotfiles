import { sanitizeFooterText } from "@dotfiles/pi-ui/status-format";
import type { DesignSnapshot } from "./types.js";

const NEW_DESIGN_END = "</pi_design_new>";
const DESIGN_ATTACHMENT = /(?:^|\n\n)<pi_design_context>\n[^\n]*\n[^\n]*\n([^\n]+)\n<\/pi_design_context>$/;

export function newDesignRequest(prompt: string, skill: string): string {
  return [
    `${prompt}\n`,
    "<pi_design_new>",
    `Create a new design from the user's request. Read ${skill} first.`,
    "Choose a project name and directory from the request; use ~/Documents if no path is given.",
    "Create the React/Vite project, install dependencies, and call pi_design.open(path, name=...) to connect it.",
    NEW_DESIGN_END,
  ].join("\n");
}

export function isNewDesignRequest(text: string): boolean {
  return text.endsWith(NEW_DESIGN_END);
}

function attachmentData(state: DesignSnapshot, first: boolean) {
  const selected = state.selection.element;
  return {
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
    changes: state.changes,
  };
}

export function designContext(state: DesignSnapshot, first: boolean): string {
  return [
    "<pi_design_context>",
    "Design state captured for this message. Edits are saved in the project files.",
    "Read the design skill before working. Read the latest full state with pi_design.current().read_state().",
    JSON.stringify(attachmentData(state, first)),
    "</pi_design_context>",
  ].join("\n");
}

export function designContextSummary(text: string): string | undefined {
  const json = DESIGN_ATTACHMENT.exec(text)?.[1];
  if (!json) return;
  try {
    const data = JSON.parse(json) as ReturnType<typeof attachmentData>;
    const project = data.project?.name;
    const board = data.artboards.find(({ id }) => id === data.selection.artboard)?.name ?? data.selection.artboard;
    const selected = [board, data.selection.element].filter(Boolean).join(" / ");
    let reason = selected ? `Selected ${selected}` : "No selection";
    if (data.elements) reason = "Connected";
    else if (data.changes.length) reason = "Design data updated";
    return sanitizeFooterText([project, reason].filter(Boolean).join(" · "));
  } catch {
    return "";
  }
}

export function displayDesignContext(markdown: string): string {
  return markdown
    .replace(DESIGN_ATTACHMENT, "")
    .replace(/\n\n<pi_design_new>\n[\s\S]*?\n<\/pi_design_new>$/, "\n\n*New design*");
}
