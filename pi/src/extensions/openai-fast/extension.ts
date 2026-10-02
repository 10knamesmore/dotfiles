import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { readPersonalConfig, updatePersonalConfig } from "../../config/index.js";

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isOpenAISubscription(ctx: ExtensionContext): boolean {
  const model = ctx.model;
  return (
    model?.provider === "openai" &&
    model.api === "openai-responses" &&
    model.baseUrl === "https://api.openai.com/v1" &&
    ctx.modelRegistry.isUsingOAuth(model)
  );
}

/** Persist /fast globally and request priority service using Pi's OpenAI subscription login. */
export default function registerOpenAIFast(pi: ExtensionAPI): void {
  let lastResponse: { model: string; reportedTier: string | undefined } | undefined;

  function updateStatus(ctx: ExtensionContext): void {
    const active = readPersonalConfig().openai.fast && isOpenAISubscription(ctx);
    ctx.ui.setStatus("openai-fast", active ? "Fast requested" : undefined);
  }

  function showStatus(ctx: ExtensionContext): void {
    const enabled = readPersonalConfig().openai.fast;
    const lines = [`OpenAI Fast: ${enabled ? "on" : "off"} (saved globally).`];
    if (!isOpenAISubscription(ctx)) {
      lines.push("Applies to official OpenAI models signed in with ChatGPT.");
    } else if (enabled) {
      lines.push("Requests Fast for subsequent turns; higher usage rates apply.");
    }
    if (lastResponse && lastResponse.model === ctx.model?.id && isOpenAISubscription(ctx)) {
      lines.push(`Last response reported: ${lastResponse.reportedTier ?? "no service tier"}.`);
    }
    ctx.ui.notify(lines.join("\n"), "info");
  }

  pi.registerCommand("fast", {
    description: "Toggle OpenAI Fast mode (higher usage): /fast [on|off|status]",
    getArgumentCompletions: (prefix) => {
      const matches = ["on", "off", "status"].filter((value) => value.startsWith(prefix.trim().toLowerCase()));
      return matches.length > 0 ? matches.map((value) => ({ value, label: value })) : null;
    },
    handler: async (args, ctx) => {
      const command = args.trim().toLowerCase();
      if (!["", "on", "off", "status"].includes(command)) {
        ctx.ui.notify("Usage: /fast [on|off|status]", "error");
        return;
      }
      if (command !== "status") {
        updatePersonalConfig((config) => {
          config.openai.fast = command === "" ? !config.openai.fast : command === "on";
        });
        pi.appendEntry("openai-fast-setting", { enabled: readPersonalConfig().openai.fast });
      }
      updateStatus(ctx);
      showStatus(ctx);
    },
  });

  pi.on("session_start", (_event, ctx) => {
    lastResponse = undefined;
    updateStatus(ctx);
  });
  pi.on("model_select", (_event, ctx) => updateStatus(ctx));

  pi.on("before_provider_request", (event, ctx) => {
    if (!ctx.model || !isOpenAISubscription(ctx)) return;
    const enabled = readPersonalConfig().openai.fast;
    updateStatus(ctx);
    if (!isObject(event.payload) || event.payload.model !== ctx.model.id) return;
    pi.appendEntry("openai-fast-request", { model: ctx.model.id, enabled });
    if (!enabled) return;
    return { ...event.payload, service_tier: "priority" };
  });

  pi.on("provider_stream_event", (event, ctx) => {
    if (event.provider !== "openai" || event.api !== "openai-responses" || !isOpenAISubscription(ctx)) return;
    if (!isObject(event.data) || event.data.type !== "response.completed") return;
    const response = event.data.response;
    if (!isObject(response)) return;
    // Keep the server's report separate from the requested mode; requesting Fast does not confirm it.
    lastResponse = {
      model: event.model,
      reportedTier: typeof response.service_tier === "string" ? response.service_tier : undefined,
    };
    pi.appendEntry("openai-fast-response", lastResponse);
  });
}
