import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { readPersonalConfig, updatePersonalConfig } from "@dotfiles/pi-config";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "../editor/api.js";

function isOfficialSubscription(ctx: ExtensionContext): boolean {
  const model = ctx.model;
  if (!model || model.provider !== "openai" || model.api !== "openai-responses") return false;
  const url = new URL(model.baseUrl);
  return url.origin === "https://api.openai.com" &&
    url.pathname.replace(/\/+$/u, "") === "/v1" &&
    !url.username && !url.password && !url.search && !url.hash &&
    ctx.modelRegistry.isUsingOAuth(model);
}

function isFastEnabled(ctx: ExtensionContext): boolean {
  const model = ctx.model;
  return model !== undefined && readPersonalConfig().openai.fast[`${model.provider}/${model.id}`] === true;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export default function fastMode(pi: ExtensionAPI): void {
  let fastActive = false;
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useFastRequested(() => fastActive);
  });

  const refreshStatus = (ctx: ExtensionContext) => {
    fastActive = isFastEnabled(ctx) && isOfficialSubscription(ctx);
    ctx.ui.setStatus("openai-fast", fastActive ? "fast" : undefined);
  };

  pi.on("session_start", (_event, ctx) => refreshStatus(ctx));
  pi.on("model_select", (_event, ctx) => refreshStatus(ctx));

  pi.registerCommand("fast", {
    description: "Toggle OpenAI subscription fast mode for the current model (higher usage): /fast [on|off|status]",
    getArgumentCompletions: (prefix) => {
      const matches = ["on", "off", "status"].filter((value) => value.startsWith(prefix));
      return matches.length ? matches.map((value) => ({ value, label: value })) : null;
    },
    handler: async (args, ctx) => {
      const action = args.trim().toLowerCase();
      if (!["", "on", "off", "status"].includes(action)) {
        ctx.ui.notify("Usage: /fast [on|off|status]", "warning");
        return;
      }
      if (!ctx.model) {
        ctx.ui.notify("Select a model before using /fast.", "warning");
        return;
      }
      const modelReference = `${ctx.model.provider}/${ctx.model.id}`;
      if (action !== "status") {
        updatePersonalConfig((config) => {
          config.openai.fast[modelReference] = action === "" ? !config.openai.fast[modelReference] : action === "on";
        });
        pi.appendEntry("openai-fast-setting", { model: modelReference, enabled: isFastEnabled(ctx) });
      }
      refreshStatus(ctx);
      const enabled = isFastEnabled(ctx);
      const scope = isOfficialSubscription(ctx)
        ? enabled ? " Higher subscription usage; applies from the next request." : ""
        : " Applies only to official OpenAI OAuth subscription models, not the current model.";
      ctx.ui.notify(`Fast mode: ${enabled ? "on" : "off"} (${modelReference}).${scope}`, "info");
    },
  });

  pi.on("before_provider_request", (event, ctx) => {
    refreshStatus(ctx);
    if (!fastActive) return;
    if (!isObject(event.payload)) return;
    pi.appendEntry("openai-fast-request", { model: ctx.model!.id, serviceTier: "priority" });
    return { ...event.payload, service_tier: "priority" };
  });

  pi.on("provider_stream_event", (event) => {
    if (event.provider !== "openai" || event.api !== "openai-responses") return;
    if (!isObject(event.data) || event.data.type !== "response.completed") return;
    const response = event.data.response;
    if (!isObject(response) || typeof response.service_tier !== "string") return;
    pi.appendEntry("openai-service-tier", { model: event.model, returnedTier: response.service_tier });
  });
}
