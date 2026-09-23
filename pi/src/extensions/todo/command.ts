import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { TodoPresentation } from "./presentation.js";
import { TodoSessionStore } from "./session-store.js";

/** Register `/todo` as a toggle for the persistent todo display. */
export function registerTodoCommand(
  pi: ExtensionAPI,
  store: TodoSessionStore,
  presentation: TodoPresentation,
): void {
  pi.registerCommand("todo", {
    description: "Toggle the persistent session todo display",
    handler: async (_rawArgs, ctx) => {
      const result = presentation.toggleVisibility(ctx, store.snapshot());
      if (result.warning) {
        ctx.ui.notify(result.warning, "warning");
      } else {
        ctx.ui.notify(
          result.enabled ? "Todo display enabled." : "Todo display disabled.",
          "info",
        );
      }
    },
  });
}
