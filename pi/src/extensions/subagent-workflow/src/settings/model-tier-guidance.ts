import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { getPersonalConfigPath, readPersonalConfig, type ModelTier, type ModelTiers } from "@dotfiles/pi-config";

const selectionGuidance = (tierLabel: (tier: ModelTier) => string = (tier) => tier) =>
  `For subagent and workflow agent() calls, explicitly select model: "max", "high", or "mid" according to the child task.
- ${tierLabel("mid")}: first choice for code exploration, repository navigation, search, evidence gathering, initial triage, quick feedback, and well-scoped implementation or edits. Use it for exploration even in large or unfamiliar codebases.
- ${tierLabel("high")}: implementation, debugging, or review requiring sustained reasoning across interacting behaviors and tradeoffs.
- ${tierLabel("max")}: the hardest root-cause analysis, cross-module design with major uncertainty, or consequential decisions requiring the deepest judgment.
Start with mid unless the child's concrete reasoning requirements justify high or max. Gathering evidence for a difficult task still belongs on mid; reserve stronger tiers for the difficult analysis or decision itself. Respect any tier explicitly requested by the user.`;

export function modelTierGuidance(
  tiers: Readonly<ModelTiers>,
  registry: Pick<ExtensionContext["modelRegistry"], "getAvailable">,
): string {
  const available = new Set(registry.getAvailable().map((model) => `${model.provider}/${model.id}`));
  return selectionGuidance((tier) => {
    const model = tiers[tier];
    if (model === undefined) return `${tier}(not configured)`;
    return `${tier}(${model}${available.has(model) ? "" : ", unavailable"})`;
  });
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
      guidance = `${selectionGuidance()}\nPersonal configuration is unreadable or invalid. Delegation is unavailable until the user repairs ${getPersonalConfigPath()}.`;
    }
    return { systemPrompt: `${event.systemPrompt}\n\n${guidance}` };
  });
}
