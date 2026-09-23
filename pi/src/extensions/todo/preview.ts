import { isClosedTodo, type TodoItem, type TodoPhase } from "./model.js";

/**
 * Select a bounded preview while always prioritizing active and blocked work.
 *
 * The returned set is rendered in canonical phase/task order; priority only
 * determines which rows survive the cap.
 */
export function selectTodoPreview(
  phases: readonly TodoPhase[],
  limit: number,
): Set<TodoItem> {
  const tasks = phases.flatMap((phase) => phase.tasks);
  const selected = new Set<TodoItem>();
  const groups = [
    tasks.filter((task) => task.status === "in_progress"),
    tasks.filter((task) => task.status === "blocked"),
    tasks.filter((task) => task.status === "pending"),
    tasks.filter(isClosedTodo),
  ];
  for (const group of groups) {
    for (const task of group) {
      if (selected.size >= limit) return selected;
      selected.add(task);
    }
  }
  return selected;
}
