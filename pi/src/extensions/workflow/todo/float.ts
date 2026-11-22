import type { Theme } from "@earendil-works/pi-coding-agent";
import { truncateToWidth, visibleWidth, wrapTextWithAnsi, type TuiMouseEvent } from "@earendil-works/pi-tui";
import { activeTodoTask, isClosedTodo, type TodoItem, type TodoPhase, type TodoStatus } from "./model.js";
import { selectTodoPreview } from "./preview.js";

/** The float may cover at most this share of the terminal height. */
export const FLOAT_MAX_HEIGHT = "60%" as const;
const FLOAT_MAX_HEIGHT_RATIO = Number.parseInt(FLOAT_MAX_HEIGHT, 10) / 100;

export interface TodoFloatOptions {
  readPhases: () => readonly TodoPhase[];
  isCollapsed: () => boolean;
  toggleCollapsed: () => void;
  terminalRows: () => number;
}

function taskMarker(status: TodoStatus): string {
  switch (status) {
    case "in_progress":
      return "→";
    case "completed":
      return "✓";
    case "abandoned":
      return "-";
    case "blocked":
      return "!";
    case "pending":
      return "○";
  }
}

function taskPaint(status: TodoStatus, theme: Theme): (text: string) => string {
  switch (status) {
    case "in_progress":
      return (text) => theme.fg("accent", text);
    case "completed":
      return (text) => theme.fg("success", text);
    case "abandoned":
      return (text) => theme.fg("dim", text);
    case "blocked":
      return (text) => theme.fg("warning", text);
    case "pending":
      return (text) => theme.fg("muted", text);
  }
}

function taskText(task: TodoItem, theme: Theme): string {
  return task.status === "completed" || task.status === "abandoned" ? theme.strikethrough(task.content) : task.content;
}

function headline(task: TodoItem, theme: Theme): string {
  return `${taskPaint(task.status, theme)(taskMarker(task.status))} ${taskPaint(task.status, theme)(taskText(task, theme))}`;
}

function taskRows(task: TodoItem, width: number, theme: Theme): string[] {
  const paint = taskPaint(task.status, theme);
  const strike = task.status === "completed" || task.status === "abandoned";
  const content = wrapTextWithAnsi(task.content, Math.max(1, width - 2));
  const rows = content.map((line, index) => {
    const text = strike ? theme.strikethrough(line) : line;
    return `${index === 0 ? paint(taskMarker(task.status)) : " "} ${paint(text)}`;
  });
  if (!task.reason) return rows;

  const reason = wrapTextWithAnsi(task.reason, Math.max(1, width - 4));
  reason.forEach((line, index) => {
    rows.push(`  ${index === 0 ? `${theme.fg("dim", "↳")} ` : "  "}${theme.fg("muted", line)}`);
  });
  return rows;
}

function phaseLine(phase: TodoPhase, theme: Theme): string {
  const closed = phase.tasks.filter(isClosedTodo).length;
  return theme.bold(theme.fg("muted", `${phase.name} ${closed}/${phase.tasks.length}`));
}

function topBorder(width: number, title: string, marker: string, theme: Theme): string {
  const markerWidth = visibleWidth(marker);
  const titleWidth = visibleWidth(title);
  const fill = Math.max(0, width - 3 - titleWidth - markerWidth);
  const line =
    theme.fg("borderMuted", "╭─") +
    title +
    theme.fg("borderMuted", "─".repeat(fill)) +
    theme.fg("accent", marker) +
    theme.fg("borderMuted", "╮");
  return truncateToWidth(line, width);
}

function bottomBorder(width: number, theme: Theme): string {
  return theme.fg("borderMuted", `╰${"─".repeat(Math.max(0, width - 2))}╯`);
}

function contentLine(content: string, width: number, theme: Theme): string {
  const inner = Math.max(0, width - 4);
  const clipped = truncateToWidth(content, inner);
  const padding = " ".repeat(Math.max(0, inner - visibleWidth(clipped)));
  return `${theme.fg("borderMuted", "│")} ${clipped}${padding} ${theme.fg("borderMuted", "│")}`;
}

/** Persistent top-right todo window: one line while collapsed, a capped task list while expanded. */
export class TodoFloat {
  public constructor(
    private readonly theme: Theme,
    private readonly options: TodoFloatOptions,
  ) {}

  public render(width: number): string[] {
    const phases = this.options.readPhases();
    const tasks = phases.flatMap((phase) => phase.tasks);
    if (tasks.length === 0) return [];

    const closed = tasks.filter(isClosedTodo).length;
    const completed = closed === tasks.length;
    const collapsed = this.options.isCollapsed();
    const bodyBudget = Math.max(1, Math.floor(this.options.terminalRows() * FLOAT_MAX_HEIGHT_RATIO) - 2);
    const bodyWidth = Math.max(1, width - 4);
    const body = completed
      ? [this.theme.fg("success", `✓ ${closed}/${tasks.length} all closed`)]
      : collapsed
        ? this.collapsedBody(phases)
        : this.expandedBody(phases, bodyBudget, bodyWidth);
    const title = this.title(closed, tasks.length, completed);
    const marker = completed ? "✓" : collapsed ? "▸" : "▾";

    return [
      topBorder(width, title, marker, this.theme),
      ...body.map((line) => contentLine(line, width, this.theme)),
      bottomBorder(width, this.theme),
    ];
  }

  public handleMouse(event: TuiMouseEvent): { handled: boolean } | undefined {
    if (event.button !== "left") return undefined;
    if (event.type === "press") return { handled: true };
    if (event.type !== "click") return undefined;
    if (!this.isCompleted()) this.options.toggleCollapsed();
    return { handled: true };
  }

  public invalidate(): void {}

  private title(closed: number, total: number, completed: boolean): string {
    const count = completed ? "" : ` ${closed}/${total}`;
    return ` ${this.theme.fg("accent", this.theme.bold("todo"))}${this.theme.fg("muted", count)} `;
  }

  private isCompleted(): boolean {
    const tasks = this.options.readPhases().flatMap((phase) => phase.tasks);
    return tasks.length > 0 && tasks.every(isClosedTodo);
  }

  private collapsedBody(phases: readonly TodoPhase[]): string[] {
    const active = activeTodoTask(phases);
    if (active) return [headline(active, this.theme)];
    return [this.theme.fg("warning", "! no active task")];
  }

  private expandedBody(phases: readonly TodoPhase[], budget: number, width: number): string[] {
    const tasks = phases.flatMap((phase) => phase.tasks);
    const rowsByTask = new Map<TodoItem, string[]>();
    const rowsFor = (task: TodoItem): string[] => {
      let rows = rowsByTask.get(task);
      if (rows === undefined) {
        rows = taskRows(task, width, this.theme);
        rowsByTask.set(task, rows);
      }
      return rows;
    };

    for (let limit = Math.min(tasks.length, budget); limit >= 1; limit -= 1) {
      const selected = selectTodoPreview(phases, limit);
      const lines: string[] = [];
      for (const phase of phases) {
        const visible = phase.tasks.filter((task) => selected.has(task));
        if (visible.length === 0) continue;
        lines.push(phaseLine(phase, this.theme));
        for (const task of visible) lines.push(...rowsFor(task));
      }
      const hidden = tasks.length - selected.size;
      if (hidden > 0) lines.push(this.theme.fg("dim", `… ${hidden} more`));
      if (lines.length <= budget) return lines;
    }

    for (let limit = Math.min(tasks.length, budget); limit >= 1; limit -= 1) {
      const selected = selectTodoPreview(phases, limit);
      const lines = tasks.filter((task) => selected.has(task)).flatMap(rowsFor);
      const hidden = tasks.length - selected.size;
      if (hidden > 0) lines.push(this.theme.fg("dim", `… ${hidden} more`));
      if (lines.length <= budget) return lines;
    }

    const selected = selectTodoPreview(phases, 1);
    const rows = tasks
      .filter((task) => selected.has(task))
      .flatMap(rowsFor)
      .slice(0, budget);
    if (rows.length > 0) rows[rows.length - 1] = `${rows[rows.length - 1]}${this.theme.fg("dim", " …")}`;
    return rows;
  }
}
