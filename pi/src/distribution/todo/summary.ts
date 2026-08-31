import {
  activeTodoTask,
  isClosedTodo,
  type TodoItem,
  type TodoPhase,
} from "./model.js";

function taskMarker(task: TodoItem): string {
  switch (task.status) {
    case "pending":
      return "[ ]";
    case "in_progress":
      return "[>]";
    case "completed":
      return "[x]";
    case "abandoned":
      return "[-]";
    case "blocked":
      return "[!]";
  }
}

/** Full model-facing summary returned by `todo` calls. */
export function formatTodoSummary(
  phases: readonly TodoPhase[],
  readOnly: boolean,
): string {
  const tasks = phases.flatMap((phase) => phase.tasks);
  if (tasks.length === 0)
    return readOnly ? "Todo list is empty." : "Todo list cleared.";

  const closed = tasks.filter(isClosedTodo).length;
  const blocked = tasks.filter((task) => task.status === "blocked").length;
  const active = activeTodoTask(phases);
  const lines = [
    `Todo progress: ${closed}/${tasks.length} closed${blocked > 0 ? `, ${blocked} blocked` : ""}.`,
    active ? `Active task: ${active.content}` : "Active task: none.",
  ];
  for (const phase of phases) {
    const phaseClosed = phase.tasks.filter(isClosedTodo).length;
    lines.push(`${phase.name} (${phaseClosed}/${phase.tasks.length}):`);
    for (const task of phase.tasks) {
      const blocker =
        task.status === "blocked" && task.blocker
          ? ` — blocked: ${task.blocker}`
          : "";
      lines.push(`  ${taskMarker(task)} ${task.content}${blocker}`);
    }
  }
  return lines.join("\n");
}

/** Compact status text consumed by Pi's footer data provider. */
export function formatTodoStatus(
  phases: readonly TodoPhase[],
): string | undefined {
  const tasks = phases.flatMap((phase) => phase.tasks);
  if (tasks.length === 0) return undefined;
  const closed = tasks.filter(isClosedTodo).length;
  const active = activeTodoTask(phases);
  if (active) {
    const chars = Array.from(active.content);
    const content =
      chars.length > 60 ? `${chars.slice(0, 59).join("")}…` : active.content;
    return `todo ${closed}/${tasks.length} · ${content}`;
  }
  const blocked = tasks.filter((task) => task.status === "blocked").length;
  return `todo ${closed}/${tasks.length}${blocked > 0 ? ` · ${blocked} blocked` : ""}`;
}
