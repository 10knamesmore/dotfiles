import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { captureContext, capturePayload, type ContextSnapshot, type PayloadSnapshot } from "./snapshot.js";
import { inspectContext } from "./view.js";

export default function contextInspector(pi: ExtensionAPI): void {
  let current: ContextSnapshot | undefined;
  let latestPayload: PayloadSnapshot | undefined;
  let payloadNumber = 0;

  const clear = () => {
    current = undefined;
    latestPayload = undefined;
    payloadNumber = 0;
  };
  pi.on("session_start", clear);
  pi.on("session_tree", clear);
  pi.on("session_shutdown", clear);

  pi.on("context_with_system", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    current = captureContext(event, ctx.getSystemPrompt());
  });

  pi.on("before_provider_request", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    latestPayload = capturePayload(event.payload, ++payloadNumber);
  });

  pi.registerCommand("context", {
    description: "Inspect the latest context and provider payload",
    handler: async (_args, ctx) => {
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/context requires the interactive terminal UI.", "info");
        return;
      }
      if (!current) {
        ctx.ui.notify("No request captured. Send a prompt after loading the extension or switching branches.", "info");
        return;
      }
      // Observations are replaced, so this inspector keeps its opening snapshot.
      await inspectContext(ctx, current, latestPayload, pi.getSettings().externalEditor);
    },
  });
}
