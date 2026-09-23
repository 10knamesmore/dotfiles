import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { isModelTier, getPersonalConfigPath, type ModelTiers } from "../../../../config/index.js";
import type { TierSubagentSpec } from "../subagent-spec.js";
import type { SubagentSpec } from "../types.js";

type ModelCatalog = Pick<ExtensionContext["modelRegistry"], "getAvailable">;

/** Resolve only explicitly configured tiers whose provider model is available. */
export function resolveTierModel(tier: unknown, tiers: Readonly<ModelTiers>, registry: ModelCatalog): string {
  if (!isModelTier(tier))
    throw new Error(
      'model must be "max", "high", or "mid"; provider/model-id values and omitted model are not accepted',
    );
  const reference = tiers[tier];
  if (!reference)
    throw new Error(
      `Model tier "${tier}" is not configured. Ask the user to configure it with /model-tiers or the "model-tier" field in ${getPersonalConfigPath()}.`,
    );
  const model = registry.getAvailable().find((candidate) => `${candidate.provider}/${candidate.id}` === reference);
  if (!model)
    throw new Error(
      `Model tier "${tier}" maps to unavailable model ${reference}. Ask the user to check the provider configuration or change the tier binding with /model-tiers.`,
    );
  return reference;
}

/** Pin the actual model before queueing; keep the chosen tier in the run record. */
export function resolveTierSpec(
  spec: TierSubagentSpec,
  tiers: Readonly<ModelTiers>,
  registry: ModelCatalog,
): SubagentSpec {
  return { ...spec, model: resolveTierModel(spec.model, tiers, registry), modelTier: spec.model };
}
