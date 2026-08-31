import {
  normalizeActiveTask,
  normalizeBlockerReason,
  normalizeTodoIdentifier,
  type TodoPhase,
  type TodoStatus,
} from "./model.js";

const STATUS_MARKER: Record<TodoStatus, string> = {
  pending: " ",
  in_progress: "/",
  completed: "x",
  abandoned: "-",
  blocked: "!",
};

const MARKER_STATUS: Readonly<Record<string, TodoStatus>> = {
  " ": "pending",
  "": "pending",
  "/": "in_progress",
  ">": "in_progress",
  x: "completed",
  X: "completed",
  "-": "abandoned",
  "~": "abandoned",
  "!": "blocked",
};

/** Serialize the complete list into the editable `/todo edit` format. */
export function todoPhasesToMarkdown(phases: readonly TodoPhase[]): string {
  if (phases.length === 0) return "";
  const lines: string[] = [];
  for (const phase of phases) {
    if (lines.length > 0) lines.push("");
    lines.push(`# ${phase.name}`);
    for (const task of phase.tasks) {
      const blocker =
        task.status === "blocked" && task.blocker
          ? ` <!-- blocker: ${task.blocker} -->`
          : "";
      lines.push(`- [${STATUS_MARKER[task.status]}] ${task.content}${blocker}`);
    }
  }
  return `${lines.join("\n")}\n`;
}

/**
 * Parse the editable checklist format and enforce the same identity invariants as tool calls.
 *
 * A blank document clears the list. Tasks before the first heading enter a `Tasks` phase.
 */
export function todoPhasesFromMarkdown(markdown: string): TodoPhase[] {
  if (!markdown.trim()) return [];

  const phases: TodoPhase[] = [];
  const phaseNames = new Set<string>();
  const taskContents = new Set<string>();
  let currentPhase: TodoPhase | undefined;
  const lines = markdown.split(/\r?\n/u);

  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index];
    if (rawLine === undefined) continue;
    const line = rawLine.trim();
    if (!line) continue;

    const heading = /^#{1,6}\s+(.+?)\s*$/u.exec(line);
    if (heading) {
      const rawName = heading[1];
      if (rawName === undefined)
        throw new Error(`Line ${index + 1}: phase name is missing.`);
      const name = normalizeTodoIdentifier(rawName, "phase");
      if (phaseNames.has(name))
        throw new Error(
          `Line ${index + 1}: duplicate phase ${JSON.stringify(name)}.`,
        );
      phaseNames.add(name);
      currentPhase = { name, tasks: [] };
      phases.push(currentPhase);
      continue;
    }

    const item = /^[-*+]\s*\[(.?)\]\s+(.+?)\s*$/u.exec(line);
    if (!item)
      throw new Error(
        `Line ${index + 1}: expected a heading or checklist item.`,
      );
    const marker = item[1];
    const rawItemContent = item[2];
    if (marker === undefined || rawItemContent === undefined) {
      throw new Error(`Line ${index + 1}: incomplete checklist item.`);
    }
    const status = MARKER_STATUS[marker];
    if (!status) throw new Error(`Line ${index + 1}: unsupported todo marker.`);
    if (!currentPhase) {
      const name = "Tasks";
      phaseNames.add(name);
      currentPhase = { name, tasks: [] };
      phases.push(currentPhase);
    }

    let rawContent = rawItemContent.trim();
    let blocker: string | undefined;
    if (status === "blocked") {
      const blockerComment = /^(.*?)\s*<!--\s*blocker:\s*(.*?)\s*-->$/u.exec(
        rawContent,
      );
      if (blockerComment) {
        const blockedContent = blockerComment[1];
        const rawBlocker = blockerComment[2];
        if (blockedContent === undefined || rawBlocker === undefined) {
          throw new Error(`Line ${index + 1}: invalid blocker annotation.`);
        }
        rawContent = blockedContent;
        blocker = normalizeBlockerReason(rawBlocker);
      }
    }
    const content = normalizeTodoIdentifier(rawContent, "task");
    if (taskContents.has(content))
      throw new Error(
        `Line ${index + 1}: duplicate task ${JSON.stringify(content)}.`,
      );
    taskContents.add(content);
    currentPhase.tasks.push(
      blocker === undefined
        ? { content, status }
        : { content, status, blocker },
    );
  }

  normalizeActiveTask(phases);
  return phases;
}
