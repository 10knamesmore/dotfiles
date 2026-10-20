import { activeTodoTask, isClosedTodo, type TodoItem, type TodoPhase } from "./model.js";

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
export function formatTodoSummary(phases: readonly TodoPhase[], readOnly: boolean): string {
  const tasks = phases.flatMap((phase) => phase.tasks);
  if (tasks.length === 0) return readOnly ? "Todo list is empty." : "Todo list cleared.";

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
      const reason = task.reason ? ` — ${task.reason}` : "";
      lines.push(`  ${taskMarker(task)} ${task.content}${reason}`);
    }
  }
  return lines.join("\n");
}
