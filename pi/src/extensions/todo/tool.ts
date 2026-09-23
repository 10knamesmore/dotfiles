import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Text } from "@earendil-works/pi-tui";
import { sanitizeTodoDisplayLine, sanitizeTodoDisplayText } from "./display.js";
import { applyTodoMutation, validateTodoView } from "./mutation.js";
import { isClosedTodo, parseTodoPhases, type TodoPhase } from "./model.js";
import { TodoPresentation } from "./presentation.js";
import { selectTodoPreview } from "./preview.js";
import { TodoParameters, type TodoOperation } from "./schema.js";
import { TodoSessionStore, TODO_STATE_VERSION } from "./session-store.js";
import { formatTodoSummary } from "./summary.js";

/** Details persisted on every successful todo tool result. */
export interface TodoToolDetails {
  /** Snapshot format version used during branch reconstruction. */
  version: typeof TODO_STATE_VERSION;

  /** Operation that produced this result. */
  op: TodoOperation;

  /** Complete canonical state after the operation. */
  phases: TodoPhase[];

  /** Non-fatal presentation failure after a successful state change. */
  uiWarning?: string;
}

function errorMessage(error: unknown): string {
  return sanitizeTodoDisplayLine(
    error instanceof Error ? error.message : String(error),
  );
}

function taskTree(phases: readonly TodoPhase[], expanded: boolean): string {
  const selected = expanded ? undefined : selectTodoPreview(phases, 8);
  let hidden = 0;
  const lines: string[] = [];
  for (const phase of phases) {
    const phaseLines: string[] = [];
    for (const task of phase.tasks) {
      if (selected !== undefined && !selected.has(task)) {
        hidden += 1;
        continue;
      }
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
      phaseLines.push(`  ${marker} ${task.content}${reason}`);
    }
    if (phaseLines.length > 0 || expanded) {
      const closed = phase.tasks.filter(isClosedTodo).length;
      lines.push(
        `${phase.name} (${closed}/${phase.tasks.length}):`,
        ...phaseLines,
      );
    }
  }
  if (hidden > 0) lines.push(`… ${hidden} more`);
  return lines.join("\n");
}

/** Register the parent-owned model-facing todo tool. */
export function registerTodoTool(
  pi: ExtensionAPI,
  store: TodoSessionStore,
  presentation: TodoPresentation,
): void {
  pi.registerTool<typeof TodoParameters, TodoToolDetails>({
    name: "todo",
    label: "Todo",
    description:
      "Apply one atomic operation to the parent session's phased todo list. init replaces the list; start selects one task; done and drop target a task, phase, or all tasks; rm removes one task, clears one phase, or clears all tasks; block and unblock require one task or phase; append adds tasks to a phase; view is read-only. Task content and phase names are stable exact identifiers. After each mutation, the first pending task becomes active when no task is active. Use view if an identifier is unknown.",
    promptSnippet:
      "Track multi-step work as a parent-session phase and task list",
    promptGuidelines: [
      "Use todo for multi-step work and keep every introduced task and phase string stable for later operations. make sure update todos once a task was done to help user keep in pace",
      "After a successful todo mutation, continue the real work in the same turn instead of spending a turn only updating progress.",
    ],
    parameters: TodoParameters,
    executionMode: "sequential",

    async execute(_toolCallId, params, signal, _onUpdate, ctx) {
      if (signal?.aborted) throw new Error("Todo operation was cancelled.");
      const readOnly = params.op === "view";
      try {
        if (readOnly) validateTodoView(params);
        else store.install(applyTodoMutation(store.snapshot(), params));
      } catch (error) {
        throw new Error(`Todo operation rejected: ${errorMessage(error)}`, {
          cause: error,
        });
      }

      const phases = store.snapshot();
      const uiWarning = presentation.refresh(ctx, phases);
      const details: TodoToolDetails = {
        version: TODO_STATE_VERSION,
        op: params.op,
        phases,
      };
      if (uiWarning !== undefined) details.uiWarning = uiWarning;
      const summary = formatTodoSummary(phases, readOnly);
      return {
        content: [
          {
            type: "text",
            text:
              uiWarning === undefined ? summary : `${summary}\n${uiWarning}`,
          },
        ],
        details,
      };
    },

    renderCall(args, theme) {
      const op =
        typeof args.op === "string" ? sanitizeTodoDisplayLine(args.op) : "…";
      let target = "";
      if (typeof args.task === "string")
        target = ` ${JSON.stringify(sanitizeTodoDisplayLine(args.task))}`;
      else if (typeof args.phase === "string")
        target = ` ${JSON.stringify(sanitizeTodoDisplayLine(args.phase))}`;
      return new Text(
        theme.fg("toolTitle", theme.bold("todo ")) +
          theme.fg("muted", `${op}${target}`),
        0,
        0,
      );
    },

    renderResult(result, { expanded }, theme) {
      const phases = parseTodoPhases(result.details?.phases);
      if (phases === undefined) {
        const first = result.content[0];
        return new Text(
          first?.type === "text" ? sanitizeTodoDisplayText(first.text) : "",
          0,
          0,
        );
      }
      const tree = taskTree(phases, expanded);
      const rawWarning: unknown = result.details?.uiWarning;
      const warning =
        typeof rawWarning === "string" && rawWarning.length > 0
          ? `\n${theme.fg("warning", sanitizeTodoDisplayLine(rawWarning))}`
          : "";
      const title = result.details?.op === "view" ? "Todo" : "✓ Todo updated";
      return new Text(
        `${theme.fg("success", title)}${tree ? `\n${tree}` : ""}${warning}`,
        0,
        0,
      );
    },
  });
}
