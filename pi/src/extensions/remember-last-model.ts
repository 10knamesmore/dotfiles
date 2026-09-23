/** Persist interactive model choices in Pi's native settings for future sessions. */
import { getAgentDir, SettingsManager, type ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function rememberLastModel(pi: ExtensionAPI): void {
  if (process.env.PI_SUBAGENT_SHIM_SPEC !== undefined) return;

  pi.on("model_select", async (event, ctx) => {
    if (ctx.mode !== "tui" || event.source === "restore") return;
    const settings = SettingsManager.create(ctx.cwd, getAgentDir());
    settings.setDefaultModelAndProvider(event.model.provider, event.model.id);
    await settings.flush();
  });

  pi.on("thinking_level_select", async (event, ctx) => {
    if (ctx.mode !== "tui" || !ctx.model) return;
    const settings = SettingsManager.create(ctx.cwd, getAgentDir());
    settings.setModelThinkingLevel(ctx.model.provider, ctx.model.id, event.level);
    await settings.flush();
  });
}
