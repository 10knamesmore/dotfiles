import type { BuildSystemPromptOptions, ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { captureContext, capturePayload, type ContextSnapshot, type PayloadSnapshot } from "./snapshot.js";
import { inspectContext } from "./view.js";

export default function contextInspector(pi: ExtensionAPI): void {
  let current: ContextSnapshot | undefined;
  let previous: ContextSnapshot | undefined;
  let latestPayload: PayloadSnapshot | undefined;
  let previousPayload: PayloadSnapshot | undefined;
  let awaitingResponse = false;
  let promptSources: Pick<BuildSystemPromptOptions, "contextFiles" | "skills"> = {};

  const clear = () => {
    current = undefined;
    previous = undefined;
    latestPayload = undefined;
    previousPayload = undefined;
    awaitingResponse = false;
    promptSources = {};
  };
  pi.on("session_start", clear);
  pi.on("session_tree", clear);
  pi.on("session_shutdown", clear);

  pi.on("before_agent_start", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    promptSources = structuredClone({
      contextFiles: event.systemPromptOptions.contextFiles,
      skills: event.systemPromptOptions.skills,
    });
  });

  pi.on("context_with_system", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    previous = current;
    current = captureContext(
      event,
      { ...promptSources, cwd: ctx.cwd },
      (previous?.number ?? 0) + 1,
      ctx.model ? `${ctx.model.provider}/${ctx.model.id}` : "none",
      ctx.getSystemPrompt(),
    );
    awaitingResponse = true;
  });

  pi.on("before_provider_request", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    previousPayload = latestPayload;
    latestPayload = capturePayload(event.payload, (previousPayload?.number ?? 0) + 1);
  });

  pi.on("message_end", (event) => {
    if (!current || !awaitingResponse || event.message.role !== "assistant") return;
    current.response = {
      model: `${event.message.provider}/${event.message.model}`,
      responseModel: event.message.responseModel,
      stopReason: event.message.stopReason,
      usage: structuredClone(event.message.usage),
    };
    awaitingResponse = false;
  });
  pi.on("agent_end", () => {
    awaitingResponse = false;
  });

  pi.registerCommand("context", {
    description: "Inspect the last request, context sources and changes (memory-only)",
    handler: async (_args, ctx) => {
      if (ctx.mode !== "tui") {
        ctx.ui.notify("/context requires the interactive terminal UI.", "info");
        return;
      }
      if (!current) {
        ctx.ui.notify("No request captured. Send a prompt after loading the extension or switching branches.", "info");
        return;
      }
      // Freeze the view while a running agent continues collecting the next request.
      await inspectContext(
        ctx,
        structuredClone(current),
        previous && structuredClone(previous),
        latestPayload && structuredClone(latestPayload),
        previousPayload && structuredClone(previousPayload),
      );
    },
  });
}
