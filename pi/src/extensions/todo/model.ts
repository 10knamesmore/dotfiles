/** Status of one task in the parent session todo list. */
export type TodoStatus =
  | "pending"
  | "in_progress"
  | "completed"
  | "abandoned"
  | "blocked";

/** One task whose content is its stable, human-facing identifier. */
export interface TodoItem {
  /** Exact task identifier used by later todo operations. */
  content: string;

  /** Current lifecycle state. */
  status: TodoStatus;

  /** Single-line explanation present only while the task is blocked. */
  blocker?: string;
}

/** Ordered group of todo tasks. */
export interface TodoPhase {
  /** Exact phase identifier used by later todo operations. */
  name: string;

  /** Tasks in display and automatic-activation order. */
  tasks: TodoItem[];
}

const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f\u2028\u2029]/u;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isTodoStatus(value: unknown): value is TodoStatus {
  return (
    value === "pending" ||
    value === "in_progress" ||
    value === "completed" ||
    value === "abandoned" ||
    value === "blocked"
  );
}

/**
 * Normalize and validate a phase or task identifier at an input boundary.
 *
 * Stored identifiers are single-line strings with no terminal control characters.
 */
export function normalizeTodoIdentifier(
  value: string,
  label: "phase" | "task",
): string {
  const normalized = value.trim();
  if (!normalized)
    throw new Error(`${label === "phase" ? "Phase" : "Task"} cannot be empty.`);
  if (CONTROL_CHARACTER.test(normalized)) {
    throw new Error(
      `${label === "phase" ? "Phase" : "Task"} must be one line and contain no control characters.`,
    );
  }
  if (
    label === "task" &&
    (/<!--\s*blocker:/iu.test(normalized) || normalized.includes("-->"))
  ) {
    throw new Error(
      `Task ${JSON.stringify(normalized)} cannot contain the blocker annotation delimiter.`,
    );
  }
  return normalized;
}

/** Normalize an optional blocker note to a terminal-safe single line. */
export function normalizeBlockerReason(
  value: string | undefined,
): string | undefined {
  if (value === undefined) return undefined;
  const normalized = value.replace(/\s+/gu, " ").trim();
  if (!normalized) return undefined;
  if (CONTROL_CHARACTER.test(normalized)) {
    throw new Error(
      "Blocker reason must contain no terminal control characters.",
    );
  }
  if (normalized.includes("<!--") || normalized.includes("-->")) {
    throw new Error("Blocker reason cannot contain HTML comment delimiters.");
  }
  return normalized;
}

/** Return a detached copy suitable for atomic mutation or persistence. */
export function cloneTodoPhases(phases: readonly TodoPhase[]): TodoPhase[] {
  return phases.map((phase) => ({
    name: phase.name,
    tasks: phase.tasks.map((task) =>
      task.blocker === undefined
        ? { content: task.content, status: task.status }
        : { content: task.content, status: task.status, blocker: task.blocker },
    ),
  }));
}

/**
 * Validate persisted or editor-produced state and return a detached copy.
 *
 * Invalid snapshots are rejected instead of being partially recovered. Phase names
 * are unique, task content is unique across all phases, and at most one task is active.
 */
export function parseTodoPhases(value: unknown): TodoPhase[] | undefined {
  if (!Array.isArray(value)) return undefined;

  const phases: TodoPhase[] = [];
  const phaseNames = new Set<string>();
  const taskContents = new Set<string>();
  let activeTasks = 0;

  for (const rawPhase of value) {
    if (
      !isRecord(rawPhase) ||
      typeof rawPhase.name !== "string" ||
      !Array.isArray(rawPhase.tasks)
    ) {
      return undefined;
    }
    let phaseName: string;
    try {
      phaseName = normalizeTodoIdentifier(rawPhase.name, "phase");
    } catch {
      return undefined;
    }
    if (phaseName !== rawPhase.name || phaseNames.has(phaseName))
      return undefined;
    phaseNames.add(phaseName);

    const tasks: TodoItem[] = [];
    for (const rawTask of rawPhase.tasks) {
      if (
        !isRecord(rawTask) ||
        typeof rawTask.content !== "string" ||
        !isTodoStatus(rawTask.status)
      ) {
        return undefined;
      }
      let content: string;
      try {
        content = normalizeTodoIdentifier(rawTask.content, "task");
      } catch {
        return undefined;
      }
      if (content !== rawTask.content || taskContents.has(content))
        return undefined;
      taskContents.add(content);

      if (rawTask.status === "in_progress") activeTasks += 1;
      if (activeTasks > 1) return undefined;

      if (rawTask.status === "blocked") {
        if (
          rawTask.blocker !== undefined &&
          typeof rawTask.blocker !== "string"
        )
          return undefined;
        let blocker: string | undefined;
        try {
          blocker = normalizeBlockerReason(rawTask.blocker);
        } catch {
          return undefined;
        }
        if (blocker !== rawTask.blocker && rawTask.blocker !== undefined)
          return undefined;
        tasks.push(
          blocker === undefined
            ? { content, status: "blocked" }
            : { content, status: "blocked", blocker },
        );
        continue;
      }

      if (rawTask.blocker !== undefined) return undefined;
      tasks.push({ content, status: rawTask.status });
    }
    phases.push({ name: phaseName, tasks });
  }

  if (
    activeTasks === 0 &&
    phases.some((phase) =>
      phase.tasks.some((task) => task.status === "pending"),
    )
  ) {
    return undefined;
  }
  return phases;
}

/** Enforce the single-active invariant after a successful mutation. */
export function normalizeActiveTask(phases: TodoPhase[]): void {
  const tasks = phases.flatMap((phase) => phase.tasks);
  const active = tasks.filter((task) => task.status === "in_progress");
  for (const extra of active.slice(1)) extra.status = "pending";
  if (active.length > 0) return;
  const firstPending = tasks.find((task) => task.status === "pending");
  if (firstPending) firstPending.status = "in_progress";
}

/** Return the task currently selected for work, if any. */
export function activeTodoTask(
  phases: readonly TodoPhase[],
): TodoItem | undefined {
  for (const phase of phases) {
    const active = phase.tasks.find((task) => task.status === "in_progress");
    if (active) return active;
  }
  return undefined;
}

/** Completed and deliberately abandoned tasks both count as closed work. */
export function isClosedTodo(task: TodoItem): boolean {
  return task.status === "completed" || task.status === "abandoned";
}
