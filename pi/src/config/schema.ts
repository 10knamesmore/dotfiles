import { Type, type Static } from "typebox";
import { Value } from "typebox/value";

export const MODEL_TIERS = ["high", "mid", "low"] as const;
export type ModelTier = typeof MODEL_TIERS[number];
export type ModelTiers = Partial<Record<ModelTier, string>>;

export function isModelTier(value: unknown): value is ModelTier {
  return MODEL_TIERS.some((tier) => tier === value);
}

const WorkflowSettingsSchema = Type.Object({
  maxConcurrentAgents: Type.Optional(Type.Union([Type.Literal("auto"), Type.Integer({ minimum: 1, maximum: 64 })])),
  workflowApproval: Type.Optional(Type.Union([Type.Literal("always-prompt"), Type.Literal("auto")])),
  agentTimeoutMinutes: Type.Optional(Type.Integer({ minimum: 0, maximum: 240 })),
  showStatusWidget: Type.Optional(Type.Boolean()),
}, { additionalProperties: false });

export type WorkflowSettings = Required<Static<typeof WorkflowSettingsSchema>>;

const DEFAULT_WORKFLOW_SETTINGS: Readonly<WorkflowSettings> = {
  maxConcurrentAgents: "auto",
  workflowApproval: "always-prompt",
  agentTimeoutMinutes: 0,
  showStatusWidget: true,
};

const ModelReferenceSchema = Type.String({ pattern: "^[^/\\s]+/\\S+$" });
const PersonalConfigSchema = Type.Object({
  language: Type.Optional(Type.String({ minLength: 1 })),
  "subagent-workflow": Type.Optional(WorkflowSettingsSchema),
  "model-tier": Type.Optional(Type.Object({
    high: Type.Optional(ModelReferenceSchema),
    mid: Type.Optional(ModelReferenceSchema),
    low: Type.Optional(ModelReferenceSchema),
  }, { additionalProperties: false })),
}, { additionalProperties: true });

/** Personal extension settings with defaults applied; other fields survive updates. */
export interface PersonalConfig {
  /** Language requested by the todo and user-question tool descriptions. */
  language: string;
  /** User-selected provider/model-id bindings. Unconfigured tiers remain absent. */
  "model-tier": ModelTiers;
  "subagent-workflow": WorkflowSettings;
  [key: string]: unknown;
}

/** Validate file input and apply defaults only to omitted fields. */
export function parsePersonalConfig(value: unknown, path: string): PersonalConfig {
  for (const error of Value.Errors(PersonalConfigSchema, value)) {
    throw new Error(`Invalid personal configuration at ${path}${error.instancePath}: ${error.message}`);
  }
  const config = value as Static<typeof PersonalConfigSchema>;
  return {
    ...config,
    language: config.language ?? "中文",
    "model-tier": config["model-tier"] ?? {},
    "subagent-workflow": { ...DEFAULT_WORKFLOW_SETTINGS, ...config["subagent-workflow"] },
  };
}
