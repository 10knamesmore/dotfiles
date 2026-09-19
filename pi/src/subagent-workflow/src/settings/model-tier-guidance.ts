import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { MODEL_TIERS, getPersonalConfigPath, readPersonalConfig, type ModelTiers } from "../../../config/index.js";

const SELECTION_GUIDANCE = `For subagent and workflow agent() calls, explicitly select model: "high", "mid", or "low" according to the child task. The provider/model-id mappings below are informational; do not pass them as model arguments.
- low: targeted lookup, extraction, formatting, or mechanical edits with clear instructions and little judgment.
- mid: ordinary implementation, bounded debugging, test work, or focused code review.
- high: complex root-cause analysis, cross-module design, substantial uncertainty, or tasks where mistakes are costly.
Choose the lowest tier adequate for the reasoning and judgment required. Do not judge difficulty only by file count or prompt length. Respect any tier explicitly requested by the user. Model tier is independent of thinkingLevel.
Unconfigured or unavailable tiers cannot run. Do not change the user's bindings or silently substitute another tier to bypass an error; ask the user to configure them with /model-tiers. Follow-ups preserve the original child's actual model.`;

export function modelTierGuidance(tiers: Readonly<ModelTiers>, registry: Pick<ExtensionContext["modelRegistry"], "getAvailable">): string {
  const available = new Set(registry.getAvailable().map((model) => `${model.provider}/${model.id}`));
  const mappings = MODEL_TIERS.map((tier) => {
    const model = tiers[tier];
    return `- ${tier}: ${model === undefined ? "not configured" : `${JSON.stringify(model)}${available.has(model) ? "" : " (unavailable)"}`}`;
  });
  return `${SELECTION_GUIDANCE}\nCurrent model-tier mappings:\n${mappings.join("\n")}`;
}

/** Inject current tier mappings only into sessions that can delegate tasks. */
export function registerModelTierGuidance(pi: ExtensionAPI): void {
  pi.on("before_agent_start", (event, ctx) => {
    if (!pi.getActiveTools().some((name) => name === "subagent" || name === "workflow")) return;
    let guidance: string;
    try {
      guidance = modelTierGuidance(readPersonalConfig()["model-tier"], ctx.modelRegistry);
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] personal configuration could not be loaded: ${errorMessage(error)}`);
      guidance = `${SELECTION_GUIDANCE}\nPersonal configuration is unreadable or invalid. Delegation is unavailable until the user repairs ${getPersonalConfigPath()}.`;
    }
    return { systemPrompt: `${event.systemPrompt}\n\n${guidance}` };
  });
}
