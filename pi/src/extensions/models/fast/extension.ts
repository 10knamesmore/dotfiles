import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { updatePersonalConfig, type FastMode } from "@dotfiles/pi-config";
import { currentFastMode, openAIServiceTier, supportsFastMode } from "@dotfiles/pi-config/openai-service-tier";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "@dotfiles/pi-ui/prompt-editor";

const FAST_MODE_LABELS: Record<FastMode, string> = {
  fast: "Fast",
  ultrafast: "Ultrafast",
};

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export default function registerServiceTiers(pi: ExtensionAPI): void {
  let currentMode: FastMode | undefined;
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useFastMode(() => currentMode);
  });

  const refreshStatus = (ctx: ExtensionContext) => {
    const mode = currentFastMode(ctx);
    currentMode = mode;
    ctx.ui.setStatus("openai-service-tier", mode && FAST_MODE_LABELS[mode]);
    return mode;
  };

  pi.on("session_start", (_event, ctx) => {
    refreshStatus(ctx);
  });
  pi.on("model_select", (_event, ctx) => {
    refreshStatus(ctx);
  });

  for (const mode of ["fast", "ultrafast"] as const) {
    pi.registerCommand(mode, {
      description: `Toggle ${FAST_MODE_LABELS[mode]} for the current OpenAI subscription model`,
      handler: async (args, ctx) => {
        if (args.trim()) {
          ctx.ui.notify(`Use /${mode} without arguments.`, "warning");
          return;
        }
        const model = ctx.model;
        if (!model || !supportsFastMode(ctx)) {
          ctx.ui.notify(`Select an OpenAI model signed in with ChatGPT to use /${mode}.`, "warning");
          return;
        }
        const modelReference = `${model.provider}/${model.id}`;
        updatePersonalConfig((config) => {
          if (config.fast[modelReference] === mode) {
            delete config.fast[modelReference];
          } else {
            config.fast[modelReference] = mode;
          }
        });
        const selectedMode = refreshStatus(ctx);
        pi.appendEntry("openai-service-tier-setting", {
          model: modelReference,
          mode: selectedMode ?? "standard",
        });
        const selectedLabel = selectedMode ? FAST_MODE_LABELS[selectedMode] : "Standard";
        ctx.ui.notify(`${selectedLabel} selected for ${modelReference}.`, "info");
      },
    });
  }

  pi.on("before_provider_request", (event, ctx) => {
    const mode = refreshStatus(ctx);
    if (!mode || !ctx.model || !isObject(event.payload)) return;
    const serviceTier = openAIServiceTier(mode);
    pi.appendEntry("openai-service-tier-request", { model: ctx.model.id, serviceTier });
    return { ...event.payload, service_tier: serviceTier };
  });

  pi.on("provider_stream_event", (event) => {
    if (event.provider !== "openai" || event.api !== "openai-responses") return;
    if (!isObject(event.data) || event.data.type !== "response.completed") return;
    const response = event.data.response;
    if (!isObject(response) || typeof response.service_tier !== "string") return;
    pi.appendEntry("openai-service-tier", { model: event.model, returnedTier: response.service_tier });
  });
}
