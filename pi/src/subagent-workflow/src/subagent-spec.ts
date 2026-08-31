import { Type, type Static, type TSchema } from "typebox";
import { Value } from "typebox/value";
import type { SubagentSpec } from "./types.js";

const ThinkingLevelSchema = Type.Union([
  Type.Literal("off"),
  Type.Literal("minimal"),
  Type.Literal("low"),
  Type.Literal("medium"),
  Type.Literal("high"),
  Type.Literal("xhigh"),
  Type.Literal("max"),
], { description: "Child reasoning level. Omit to inherit the parent conversation's current level." });

export const SubagentPromptSchema = Type.String({
  minLength: 1,
  description: "Self-contained task for the child. The child does not receive the parent conversation; include every fact and output requirement it needs.",
});

export const MODEL_DESCRIPTION = 'Fully qualified "provider/model-id", e.g. "openai-codex/gpt-5.6-luna" - the provider prefix is required because several providers can be configured. Omit to inherit the parent conversation\'s provider and model.';

/** Options shared by the subagent tool and workflow agent() calls. */
export const PublicSubagentOptionFields = {
  model: Type.Optional(Type.String({ minLength: 1, description: MODEL_DESCRIPTION })),
  thinkingLevel: Type.Optional(ThinkingLevelSchema),
  tools: Type.Optional(Type.Array(Type.String(), {
    description: "Tool-name allowlist. Normally omit to use tools discovered for the child cwd; requesting an unavailable tool fails that child.",
  })),
  excludeTools: Type.Optional(Type.Array(Type.String(), {
    description: "Tool-name denylist. Normally omit unless deliberately removing child capabilities.",
  })),
  schema: Type.Optional(Type.Record(Type.String(), Type.Unknown(), {
    description: "JSON Schema requiring validated structured output through the child's report_result tool.",
  })),
  cwd: Type.Optional(Type.String({
    description: "Child working directory. Omit to inherit the parent cwd; extension and tool discovery follows the child cwd.",
  })),
  label: Type.Optional(Type.String({ description: "Short display label for status and recovery output." })),
  isolation: Type.Optional(Type.Literal("worktree", {
    description: "Run in a temporary git worktree. Changes return as a patch and are never applied automatically.",
  })),
};

/** Runtime contract for workflow agent(prompt, opts). */
const WorkflowAgentOptionsSchema = Type.Object({
  ...PublicSubagentOptionFields,
  phase: Type.Optional(Type.String()),
}, { additionalProperties: false });

type WorkflowAgentOptions = Omit<SubagentSpec, "prompt">;

export function validateWorkflowAgentOptions(value: unknown): WorkflowAgentOptions {
  const options = value === undefined ? {} : value;
  assertSchemaValue(WorkflowAgentOptionsSchema, options, "agent() options");
  return options as Static<typeof WorkflowAgentOptionsSchema>;
}

/** Throw a stable, field-addressed error for a TypeBox runtime contract. */
export function assertSchemaValue(schema: TSchema, value: unknown, label: string): void {
  for (const error of Value.Errors(schema, value)) {
    const details = error as typeof error & {
      keyword?: string;
      instancePath?: string;
      params?: { additionalProperties?: string[]; allowedValue?: unknown };
    };
    const unknownField = details.params?.additionalProperties?.[0];
    const escapedUnknownField = unknownField?.replaceAll("~", "~0").replaceAll("/", "~1");
    const path = escapedUnknownField === undefined
      ? details.instancePath ?? ""
      : `${details.instancePath ?? ""}/${escapedUnknownField}`;
    const expected = details.keyword === "const"
      ? `; expected ${JSON.stringify(details.params?.allowedValue)}`
      : "";
    throw new TypeError(`Invalid ${label}${path}: ${error.message}${expected}`);
  }
}
