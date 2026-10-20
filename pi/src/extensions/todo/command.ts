import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { TodoPresentation } from "./presentation.js";

/** Register `/todo` as a collapse/expand toggle for the persistent float. */
export function registerTodoCommand(pi: ExtensionAPI, presentation: TodoPresentation): void {
  pi.registerCommand("todo", {
    description: "Toggle the todo float between collapsed and expanded",
    handler: async (_rawArgs, ctx) => {
      const warning = presentation.toggleCollapsed(ctx);
      if (warning) ctx.ui.notify(warning, "warning");
    },
  });
}
