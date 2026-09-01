import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { registerTodoCommand } from "./command.js";
import { TodoPresentation } from "./presentation.js";
import { TodoSessionStore } from "./session-store.js";
import { registerTodoTool } from "./tool.js";

/** Marker written only into subprocesses launched by the vendored workflow runtime. */
const SUBAGENT_CHILD_MARKER = "PI_SUBAGENT_SHIM_SPEC";

/**
 * Register the parent-owned session todo capability.
 *
 * Vendored workflow children still load installed extensions for their own cwd.
 * The child marker therefore gates the entire capability before tool and command
 * registration, keeping `todo` out of the child active-tool report.
 */
export function registerSessionTodo(pi: ExtensionAPI): void {
  if (process.env[SUBAGENT_CHILD_MARKER] !== undefined) return;

  const store = new TodoSessionStore(pi);
  const presentation = new TodoPresentation();
  const restore = (ctx: Parameters<TodoSessionStore["restore"]>[0]): void => {
    const restoreWarning = store.restore(ctx);
    const presentationWarning = presentation.refresh(ctx, store.snapshot());
    if (restoreWarning && presentationWarning)
      ctx.ui.notify(`${restoreWarning} ${presentationWarning}`, "warning");
    else {
      const warning = restoreWarning ?? presentationWarning;
      if (warning) ctx.ui.notify(warning, "warning");
    }
  };

  pi.on("session_start", (_event, ctx) => restore(ctx));
  pi.on("session_tree", (_event, ctx) => restore(ctx));
  pi.on("turn_start", () => presentation.startTurn());
  pi.on("turn_end", (_event, ctx) => {
    const warning = presentation.finishTurn(ctx, store.snapshot());
    if (warning) ctx.ui.notify(warning, "warning");
  });
  registerTodoTool(pi, store, presentation);
  registerTodoCommand(pi, store, presentation);
}

export default registerSessionTodo;

export type { TodoItem, TodoPhase, TodoStatus } from "./model.js";
export type { TodoToolDetails } from "./tool.js";
