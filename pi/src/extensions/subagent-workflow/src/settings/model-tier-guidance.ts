import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { MODEL_TIERS, getPersonalConfigPath, readPersonalConfig, type ModelTiers } from "../../../../config/index.js";

const SELECTION_GUIDANCE = `For subagent and workflow agent() calls, explicitly select model: "max", "high", or "mid" according to the child task. The provider/model-id mappings below are informational; do not pass them as model arguments.
- mid: first choice for code exploration, repository navigation, search, evidence gathering, initial triage, quick feedback, and well-scoped implementation or edits. Use it for exploration even in large or unfamiliar codebases.
- high: implementation, debugging, or review requiring sustained reasoning across interacting behaviors and tradeoffs.
- max: the hardest root-cause analysis, cross-module design with major uncertainty, or consequential decisions requiring the deepest judgment.
Start with mid unless the child's concrete reasoning requirements justify high or max. Gathering evidence for a difficult task still belongs on mid; reserve stronger tiers for the difficult analysis or decision itself. File count, prompt length, an unfamiliar codebase, or the words "implement", "debug", and "review" alone do not justify a higher tier. Respect any tier explicitly requested by the user.
Model tier and thinkingLevel are independent: mid with thinkingLevel: "max" is valid. Omit thinkingLevel to inherit the parent's current level; do not lower it merely because you chose mid.
Unconfigured or unavailable tiers cannot run. Do not change the user's bindings or silently substitute another tier to bypass an error; ask the user to configure them with /model-tiers. Follow-ups preserve the original child's actual model.`;

export function modelTierGuidance(
  tiers: Readonly<ModelTiers>,
  registry: Pick<ExtensionContext["modelRegistry"], "getAvailable">,
): string {
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
