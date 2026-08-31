import type { ExtensionContext, Theme } from "@earendil-works/pi-coding-agent";
import { matchesKey, truncateToWidth } from "@earendil-works/pi-tui";
import { sanitizeTodoDisplayLine } from "./display.js";
import {
  cloneTodoPhases,
  isClosedTodo,
  type TodoItem,
  type TodoPhase,
} from "./model.js";
import { selectTodoPreview } from "./preview.js";
import { formatTodoStatus } from "./summary.js";

const TODO_STATUS_KEY = "todo";
const TODO_WIDGET_KEY = "todo";
const COLLAPSED_TASK_LIMIT = 6;

function errorMessage(error: unknown): string {
  return sanitizeTodoDisplayLine(
    error instanceof Error ? error.message : String(error),
  );
}

function allTasks(phases: readonly TodoPhase[]): TodoItem[] {
  return phases.flatMap((phase) => phase.tasks);
}

function taskLine(task: TodoItem, theme: Theme): string {
  switch (task.status) {
    case "in_progress":
      return theme.fg("accent", `  → ${task.content}`);
    case "completed":
      return theme.fg("success", `  ✓ ${theme.strikethrough(task.content)}`);
    case "abandoned":
      return theme.fg("dim", `  - ${theme.strikethrough(task.content)}`);
    case "blocked": {
      const reason = task.blocker ? ` — ${task.blocker}` : "";
      return theme.fg("warning", `  ! ${task.content}${reason}`);
    }
    case "pending":
      return theme.fg("muted", `  ○ ${task.content}`);
  }
}

function plainTaskLine(task: TodoItem): string {
  const marker =
    task.status === "in_progress"
      ? "→"
      : task.status === "completed"
        ? "✓"
        : task.status === "abandoned"
          ? "-"
          : task.status === "blocked"
            ? "!"
            : "○";
  const reason =
    task.status === "blocked" && task.blocker ? ` — ${task.blocker}` : "";
  return `  ${marker} ${task.content}${reason}`;
}

function selectedTodoLines(
  phases: readonly TodoPhase[],
  expanded: boolean,
): Array<{ phase: TodoPhase; tasks: TodoItem[] }> {
  const selection = expanded
    ? undefined
    : selectTodoPreview(phases, COLLAPSED_TASK_LIMIT);
  return phases
    .map((phase) => ({
      phase,
      tasks:
        selection === undefined
          ? [...phase.tasks]
          : phase.tasks.filter((task) => selection.has(task)),
    }))
    .filter((entry) => expanded || entry.tasks.length > 0);
}

function renderPlainLines(
  phases: readonly TodoPhase[],
  expanded: boolean,
): string[] {
  const tasks = allTasks(phases);
  const closed = tasks.filter(isClosedTodo).length;
  const lines = [`Todo ${closed}/${tasks.length}`];
  const visiblePhases = selectedTodoLines(phases, expanded);
  for (const { phase, tasks: visible } of visiblePhases) {
    const phaseClosed = phase.tasks.filter(isClosedTodo).length;
    lines.push(`${phase.name} ${phaseClosed}/${phase.tasks.length}`);
    for (const task of visible) lines.push(plainTaskLine(task));
  }
  const hidden =
    tasks.length -
    visiblePhases.reduce((count, entry) => count + entry.tasks.length, 0);
  if (hidden > 0) lines.push(`… ${hidden} more; /todo expand`);
  return lines;
}

class TodoPanelComponent {
  private readonly phases: TodoPhase[];

  public constructor(
    phases: readonly TodoPhase[],
    private readonly theme: Theme,
    private readonly expanded: boolean,
  ) {
    this.phases = cloneTodoPhases(phases);
  }

  public render(width: number): string[] {
    const tasks = allTasks(this.phases);
    const closed = tasks.filter(isClosedTodo).length;
    const lines = [
      this.theme.fg(
        "accent",
        this.theme.bold(`Todo ${closed}/${tasks.length}`),
      ),
    ];
    const visible = selectedTodoLines(this.phases, this.expanded);
    for (const { phase, tasks: phaseTasks } of visible) {
      const phaseClosed = phase.tasks.filter(isClosedTodo).length;
      lines.push(
        this.theme.fg(
          "muted",
          `${phase.name} ${phaseClosed}/${phase.tasks.length}`,
        ),
      );
      for (const task of phaseTasks) lines.push(taskLine(task, this.theme));
    }
    const visibleCount = visible.reduce(
      (count, entry) => count + entry.tasks.length,
      0,
    );
    const hidden = tasks.length - visibleCount;
    if (hidden > 0)
      lines.push(this.theme.fg("dim", `… ${hidden} more; /todo expand`));
    return lines.map((line) => truncateToWidth(line, width));
  }

  public invalidate(): void {}
}

class TodoOverlayComponent extends TodoPanelComponent {
  public constructor(
    phases: readonly TodoPhase[],
    theme: Theme,
    private readonly close: () => void,
  ) {
    super(phases, theme, true);
  }

  public handleInput(data: string): void {
    if (matchesKey(data, "escape") || matchesKey(data, "ctrl+c")) this.close();
  }

  public override render(width: number): string[] {
    return [
      "",
      ...super.render(width),
      "",
      truncateToWidth(
        "Press Escape to close; use /todo edit to change the list.",
        width,
      ),
      "",
    ];
  }
}

/** Owns the todo widget, footer status, expanded state, and interactive viewer. */
export class TodoPresentation {
  private expanded = false;

  /** Switch the sticky widget between compact and complete views. */
  public setExpanded(expanded: boolean): void {
    this.expanded = expanded;
  }

  /**
   * Refresh status and widget without changing canonical todo state.
   *
   * UI failures are returned to the caller because a persisted mutation must not
   * be reported as failed merely because its optional presentation could not update.
   */
  public refresh(
    ctx: ExtensionContext,
    phases: readonly TodoPhase[],
  ): string | undefined {
    try {
      const status = formatTodoStatus(phases);
      ctx.ui.setStatus(TODO_STATUS_KEY, status);
      if (allTasks(phases).length === 0) {
        ctx.ui.setWidget(TODO_WIDGET_KEY, undefined);
      } else if (ctx.mode === "tui") {
        const expanded = this.expanded;
        ctx.ui.setWidget(
          TODO_WIDGET_KEY,
          (_tui, theme) => new TodoPanelComponent(phases, theme, expanded),
          { placement: "aboveEditor" },
        );
      } else if (ctx.mode === "rpc") {
        ctx.ui.setWidget(
          TODO_WIDGET_KEY,
          renderPlainLines(phases, this.expanded),
          { placement: "aboveEditor" },
        );
      }
      return undefined;
    } catch (error) {
      return `Todo state is intact, but its UI could not refresh: ${errorMessage(error)}`;
    }
  }

  /** Open the complete read-only list in Pi's interactive TUI. */
  public async open(
    ctx: ExtensionContext,
    phases: readonly TodoPhase[],
  ): Promise<void> {
    if (ctx.mode !== "tui") {
      ctx.ui.notify(renderPlainLines(phases, true).join("\n"), "info");
      return;
    }
    await ctx.ui.custom<void>(
      (_tui, theme, _keybindings, done) =>
        new TodoOverlayComponent(phases, theme, done),
    );
  }
}
