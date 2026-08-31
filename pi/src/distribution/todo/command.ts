import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { sanitizeTodoDisplayLine } from "./display.js";
import { todoPhasesFromMarkdown, todoPhasesToMarkdown } from "./markdown.js";
import { TodoPresentation } from "./presentation.js";
import { TodoSessionStore } from "./session-store.js";

function errorMessage(error: unknown): string {
  return sanitizeTodoDisplayLine(
    error instanceof Error ? error.message : String(error),
  );
}

/** Register `/todo` viewing, widget control, and multiline editing. */
export function registerTodoCommand(
  pi: ExtensionAPI,
  store: TodoSessionStore,
  presentation: TodoPresentation,
): void {
  pi.registerCommand("todo", {
    description: "View or edit the parent session todo list",
    handler: async (rawArgs, ctx) => {
      const action = rawArgs.trim().toLowerCase() || "view";
      if (action === "view") {
        await presentation.open(ctx, store.snapshot());
        return;
      }
      if (action === "expand" || action === "collapse") {
        presentation.setExpanded(action === "expand");
        const warning = presentation.refresh(ctx, store.snapshot());
        if (warning) ctx.ui.notify(warning, "warning");
        return;
      }
      if (action !== "edit") {
        ctx.ui.notify("Usage: /todo [view|edit|expand|collapse]", "error");
        return;
      }
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/todo edit requires Pi's interactive TUI.", "error");
        return;
      }

      await ctx.waitForIdle();
      const before = store.snapshot();
      const edited = await ctx.ui.editor(
        "Edit session todo",
        todoPhasesToMarkdown(before),
      );
      if (edited === undefined) return;
      try {
        const next = todoPhasesFromMarkdown(edited);
        if (JSON.stringify(next) === JSON.stringify(before)) {
          ctx.ui.notify("Todo list unchanged.", "info");
          return;
        }
        const commit = store.commitCommand(next, ctx);
        if (commit.status === "branch-diverged") {
          try {
            ctx.ui.notify(
              `Pi advanced the todo branch but could not flush it to disk. The session will shut down before another entry can reference the unpersisted state: ${commit.reason}`,
              "error",
            );
          } finally {
            ctx.shutdown();
          }
          return;
        }
        const presentationWarning = presentation.refresh(ctx, store.snapshot());
        ctx.ui.notify(
          presentationWarning ?? "Todo list updated.",
          presentationWarning ? "warning" : "info",
        );
      } catch (error) {
        ctx.ui.notify(`Todo edit rejected: ${errorMessage(error)}`, "error");
      }
    },
  });
}
