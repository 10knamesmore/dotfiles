import type { ExtensionContext, Theme } from "@earendil-works/pi-coding-agent";
import { truncateToWidth } from "@earendil-works/pi-tui";
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
const TODO_AUTO_HIDE_TURNS = 2;

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
): Array<{ phase: TodoPhase; tasks: TodoItem[] }> {
  const selection = selectTodoPreview(phases, COLLAPSED_TASK_LIMIT);
  return phases
    .map((phase) => ({
      phase,
      tasks: phase.tasks.filter((task) => selection.has(task)),
    }))
    .filter((entry) => entry.tasks.length > 0);
}

function renderPlainLines(phases: readonly TodoPhase[]): string[] {
  const tasks = allTasks(phases);
  const closed = tasks.filter(isClosedTodo).length;
  const lines = [`Todo ${closed}/${tasks.length}`];
  const visiblePhases = selectedTodoLines(phases);
  for (const { phase, tasks: visible } of visiblePhases) {
    const phaseClosed = phase.tasks.filter(isClosedTodo).length;
    lines.push(`${phase.name} ${phaseClosed}/${phase.tasks.length}`);
    for (const task of visible) lines.push(plainTaskLine(task));
  }
  const hidden =
    tasks.length -
    visiblePhases.reduce((count, entry) => count + entry.tasks.length, 0);
  if (hidden > 0) lines.push(`… ${hidden} more`);
  return lines;
}

class TodoPanelComponent {
  private readonly phases: TodoPhase[];

  public constructor(phases: readonly TodoPhase[], private readonly theme: Theme) {
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
    const visible = selectedTodoLines(this.phases);
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
    if (hidden > 0) lines.push(this.theme.fg("dim", `… ${hidden} more`));
    return lines.map((line) => truncateToWidth(line, width));
  }

  public invalidate(): void {}
}

/** Owns todo visibility, progress status, and the compact widget. */
export class TodoPresentation {
  private displayEnabled = true;
  private autoHidden = false;
  private hasRefreshed = false;
  private allTasksWereClosed = false;
  private turnSequence = 0;
  private currentTurnSequence = 0;
  private completionTurnSequence: number | undefined;
  private completedTurns = 0;

  /** Toggle the user-controlled persistent display preference. */
  public toggleVisibility(
    ctx: ExtensionContext,
    phases: readonly TodoPhase[],
  ): { enabled: boolean; warning: string | undefined } {
    const currentlyDisplayed =
      this.displayEnabled &&
      !this.autoHidden &&
      allTasks(phases).length > 0;
    this.displayEnabled = !currentlyDisplayed;
    if (this.displayEnabled) this.autoHidden = false;
    return {
      enabled: this.displayEnabled,
      warning: this.refresh(ctx, phases),
    };
  }

  /** Record the turn in which a todo mutation may complete the list. */
  public startTurn(): void {
    this.turnSequence += 1;
    this.currentTurnSequence = this.turnSequence;
  }

  /** Hide a completed list after two subsequent complete turns. */
  public finishTurn(
    ctx: ExtensionContext,
    phases: readonly TodoPhase[],
  ): string | undefined {
    if (
      this.completionTurnSequence === undefined ||
      this.currentTurnSequence === this.completionTurnSequence
    )
      return undefined;

    this.completedTurns += 1;
    if (this.completedTurns < TODO_AUTO_HIDE_TURNS) return undefined;

    this.autoHidden = true;
    this.completionTurnSequence = undefined;
    return this.refresh(ctx, phases);
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
    const tasks = allTasks(phases);
    const allTasksClosed =
      tasks.length > 0 && tasks.every((task) => isClosedTodo(task));
    if (this.hasRefreshed && !this.allTasksWereClosed && allTasksClosed) {
      this.completionTurnSequence = this.currentTurnSequence;
      this.completedTurns = 0;
      this.autoHidden = false;
    }
    if (!allTasksClosed) {
      this.completionTurnSequence = undefined;
      this.completedTurns = 0;
      this.autoHidden = false;
    }
    this.allTasksWereClosed = allTasksClosed;
    this.hasRefreshed = true;

    try {
      const shouldDisplay =
        this.displayEnabled && !this.autoHidden && tasks.length > 0;
      ctx.ui.setStatus(
        TODO_STATUS_KEY,
        shouldDisplay ? formatTodoStatus(phases) : undefined,
      );
      if (!shouldDisplay) {
        ctx.ui.setWidget(TODO_WIDGET_KEY, undefined);
      } else if (ctx.mode === "tui") {
        ctx.ui.setWidget(
          TODO_WIDGET_KEY,
          (_tui, theme) => new TodoPanelComponent(phases, theme),
          { placement: "aboveEditor" },
        );
      } else if (ctx.mode === "rpc") {
        ctx.ui.setWidget(TODO_WIDGET_KEY, renderPlainLines(phases), {
          placement: "aboveEditor",
        });
      }
      return undefined;
    } catch (error) {
      return `Todo state is intact, but its UI could not refresh: ${errorMessage(error)}`;
    }
  }
}
