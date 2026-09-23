import {
  cloneTodoPhases,
  normalizeActiveTask,
  normalizeBlockerReason,
  normalizeTodoIdentifier,
  type TodoItem,
  type TodoPhase,
} from "./model.js";
import type { TodoParams } from "./schema.js";

function findPhase(phases: TodoPhase[], rawName: string): TodoPhase {
  const name = normalizeTodoIdentifier(rawName, "phase");
  const phase = phases.find((candidate) => candidate.name === name);
  if (!phase) throw new Error(`Phase ${JSON.stringify(name)} not found.`);
  return phase;
}

function findTask(
  phases: TodoPhase[],
  rawContent: string,
): { phase: TodoPhase; task: TodoItem } {
  const content = normalizeTodoIdentifier(rawContent, "task");
  for (const phase of phases) {
    const task = phase.tasks.find((candidate) => candidate.content === content);
    if (task) return { phase, task };
  }
  throw new Error(
    `Task ${JSON.stringify(content)} not found. Tasks are referenced by their exact content.`,
  );
}

function assertNoFields(
  params: TodoParams,
  fields: ReadonlyArray<keyof TodoParams>,
): void {
  for (const field of fields) {
    if (params[field] !== undefined)
      throw new Error(`${params.op} does not accept ${field}.`);
  }
}

function assertSingleTarget(params: TodoParams, required: boolean): void {
  if (params.task !== undefined && params.phase !== undefined) {
    throw new Error(`${params.op} accepts either task or phase, not both.`);
  }
  if (required && params.task === undefined && params.phase === undefined) {
    throw new Error(`${params.op} requires a task or phase target.`);
  }
}

function assertUniqueState(phases: readonly TodoPhase[]): void {
  const phaseNames = new Set<string>();
  const taskContents = new Set<string>();
  for (const phase of phases) {
    if (phaseNames.has(phase.name))
      throw new Error(`Duplicate phase ${JSON.stringify(phase.name)}.`);
    phaseNames.add(phase.name);
    for (const task of phase.tasks) {
      if (taskContents.has(task.content))
        throw new Error(`Duplicate task ${JSON.stringify(task.content)}.`);
      taskContents.add(task.content);
    }
  }
}

function initialize(params: TodoParams): TodoPhase[] {
  assertNoFields(params, ["task", "reason"]);
  if (params.list !== undefined && params.items !== undefined) {
    throw new Error("init accepts list or items, not both.");
  }
  if (params.list !== undefined && params.phase !== undefined) {
    throw new Error("init phase is only valid with the flat items form.");
  }

  let phases: TodoPhase[];
  if (params.list !== undefined) {
    if (params.list.length === 0) throw new Error("init list cannot be empty.");
    phases = params.list.map((entry) => {
      if (entry.items.length === 0)
        throw new Error(
          `Phase ${JSON.stringify(entry.phase)} must contain at least one task.`,
        );
      return {
        name: normalizeTodoIdentifier(entry.phase, "phase"),
        tasks: entry.items.map((item) => ({
          content: normalizeTodoIdentifier(item, "task"),
          status: "pending",
        })),
      };
    });
  } else if (params.items !== undefined) {
    if (params.items.length === 0)
      throw new Error("init items cannot be empty.");
    phases = [
      {
        name: normalizeTodoIdentifier(params.phase ?? "Tasks", "phase"),
        tasks: params.items.map((item) => ({
          content: normalizeTodoIdentifier(item, "task"),
          status: "pending",
        })),
      },
    ];
  } else {
    throw new Error("init requires list or items.");
  }
  assertUniqueState(phases);
  return phases;
}

function targetTasks(phases: TodoPhase[], params: TodoParams): TodoItem[] {
  assertSingleTarget(params, false);
  if (params.task !== undefined) return [findTask(phases, params.task).task];
  if (params.phase !== undefined)
    return [...findPhase(phases, params.phase).tasks];
  return phases.flatMap((phase) => phase.tasks);
}

function clearBlocker(task: TodoItem): void {
  delete task.blocker;
}

function append(phases: TodoPhase[], params: TodoParams): void {
  assertNoFields(params, ["list", "task", "reason"]);
  if (params.phase === undefined)
    throw new Error("append requires a phase name.");
  if (params.items === undefined || params.items.length === 0)
    throw new Error("append requires at least one task.");

  const phaseName = normalizeTodoIdentifier(params.phase, "phase");
  const contents = params.items.map((item) =>
    normalizeTodoIdentifier(item, "task"),
  );
  const existing = new Set(
    phases.flatMap((phase) => phase.tasks.map((task) => task.content)),
  );
  const batch = new Set<string>();
  for (const content of contents) {
    if (existing.has(content) || batch.has(content))
      throw new Error(`Task ${JSON.stringify(content)} already exists.`);
    batch.add(content);
  }

  let phase = phases.find((candidate) => candidate.name === phaseName);
  if (!phase) {
    phase = { name: phaseName, tasks: [] };
    phases.push(phase);
  }
  for (const content of contents)
    phase.tasks.push({ content, status: "pending" });
}

function remove(phases: TodoPhase[], params: TodoParams): void {
  assertNoFields(params, ["list", "items", "reason"]);
  assertSingleTarget(params, false);
  if (params.task !== undefined) {
    const hit = findTask(phases, params.task);
    hit.phase.tasks = hit.phase.tasks.filter((task) => task !== hit.task);
    return;
  }
  if (params.phase !== undefined) {
    findPhase(phases, params.phase).tasks = [];
    return;
  }
  for (const phase of phases) phase.tasks = [];
}

/**
 * Apply one operation to a detached copy and return a fully normalized state.
 *
 * Any validation error is thrown before the caller swaps or persists state.
 */
export function applyTodoMutation(
  current: readonly TodoPhase[],
  params: TodoParams,
): TodoPhase[] {
  if (params.op === "view")
    throw new Error("view is read-only and cannot be applied as a mutation.");
  if (params.op === "init") {
    const initialized = initialize(params);
    normalizeActiveTask(initialized);
    return initialized;
  }

  const next = cloneTodoPhases(current);
  switch (params.op) {
    case "start": {
      assertNoFields(params, ["list", "phase", "items", "reason"]);
      if (params.task === undefined) throw new Error("start requires a task.");
      const target = findTask(next, params.task).task;
      for (const phase of next) {
        for (const task of phase.tasks) {
          if (task.status === "in_progress" && task !== target)
            task.status = "pending";
        }
      }
      target.status = "in_progress";
      clearBlocker(target);
      break;
    }
    case "done":
    case "drop": {
      assertNoFields(params, ["list", "items", "reason"]);
      for (const task of targetTasks(next, params)) {
        task.status = params.op === "done" ? "completed" : "abandoned";
        clearBlocker(task);
      }
      break;
    }
    case "block": {
      assertNoFields(params, ["list", "items"]);
      assertSingleTarget(params, true);
      const blocker = normalizeBlockerReason(params.reason);
      for (const task of targetTasks(next, params)) {
        if (task.status === "completed" || task.status === "abandoned")
          continue;
        task.status = "blocked";
        if (blocker === undefined) clearBlocker(task);
        else task.blocker = blocker;
      }
      break;
    }
    case "unblock": {
      assertNoFields(params, ["list", "items", "reason"]);
      assertSingleTarget(params, true);
      for (const task of targetTasks(next, params)) {
        if (task.status !== "blocked") continue;
        task.status = "pending";
        clearBlocker(task);
      }
      break;
    }
    case "append":
      append(next, params);
      break;
    case "rm":
      remove(next, params);
      break;
  }

  normalizeActiveTask(next);
  return next;
}

/** Validate that a read-only view call carries no mutation fields. */
export function validateTodoView(params: TodoParams): void {
  if (params.op !== "view") throw new Error("Expected a view operation.");
  assertNoFields(params, ["list", "task", "phase", "items", "reason"]);
}
