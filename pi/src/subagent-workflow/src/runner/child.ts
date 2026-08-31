/**
 * Child spec resolution shared by the subprocess backend and workflow
 * fingerprinting: model parsing/validation, thinking inheritance, the
 * subagent framing line, the missing-tool contract, and the prospective
 * extension-environment scan replay decisions compare against.
 */

import {
  DefaultResourceLoader,
  getAgentDir,
  SettingsManager,
  type ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import type { FollowUpReference, SubagentSpec, ResolvedSpec, ThinkingLevel } from "../types.js";
import type { ChildSession } from "./child-session.js";
import type { SchemaCapture } from "./schema-tool.js";

/** The slice of ExtensionContext the child spawn path actually consumes. */
export type ParentExtensionContext = Pick<ExtensionContext, "cwd" | "sessionManager" | "modelRegistry" | "model">;

export interface ParentContext {
  ctx: ParentExtensionContext;
  thinkingLevel: ThinkingLevel;
  selfPath: string;
}

export interface ConstructedChild {
  session: ChildSession;
  resolved: ResolvedSpec;
  schemaCapture?: SchemaCapture;
}

export interface ResolvedFollowUpSpec {
  spec: SubagentSpec;
  forkSessionFile: string;
  followUpOf: FollowUpReference;
}

export type ChildSpawnSpec = SubagentSpec | ResolvedFollowUpSpec;

export function submittedSpec(spec: ChildSpawnSpec): SubagentSpec {
  return "spec" in spec ? spec.spec : spec;
}

export function followUpSpawn(spec: ChildSpawnSpec): ResolvedFollowUpSpec | undefined {
  return "spec" in spec ? spec : undefined;
}

/**
 * One line of framing appended to every child's system prompt. Children
 * otherwise have no idea they are subagents: small models hallucinate when
 * prompts reference orchestrator concepts, wrap answers in markdown fences,
 * and write for a human reader who is not there. Claude Code appends the
 * same kind of note to its subagents.
 */
export const SUBAGENT_FRAMING =
  "You are a subagent: an orchestrating agent spawned you for a single task. Your final message is returned to that agent as a result - it is not shown to a person. Respond with exactly what the task asks for: raw data or findings, no preamble, no markdown code fences unless explicitly requested, no closing questions.";

export function parseModel(value: string, registry: ExtensionContext["modelRegistry"]): [string, string] {
  const slash = value.indexOf("/");
  if (slash < 1 || slash === value.length - 1) {
    // Stay strict (multiple providers can expose same-named models), but make
    // the most common authoring mistake - a bare model id - self-healing by
    // naming the qualified id when it is unambiguous.
    throw new Error(`Invalid model ${JSON.stringify(value)}. Expected "provider/model-id"${suggestQualified(value, registry)}, or omit model to inherit the parent's.`);
  }
  return [value.slice(0, slash), value.slice(slash + 1)];
}

/**
 * Full validity check for a spec's model reference, shared by subagent
 * tool-call and workflow launch validation. Returns undefined when the model
 * resolves; otherwise a message with a near-miss suggestion (the exact id
 * under another provider, or the same name tokens in a different order, e.g.
 * anthropic/claude-5-sonnet -> claude-bridge/claude-sonnet-5).
 */
export function unknownModelError(value: string, registry: ExtensionContext["modelRegistry"]): string | undefined {
  let provider: string;
  let id: string;
  try {
    [provider, id] = parseModel(value, registry);
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
  if (registry.find(provider, id)) return undefined;
  const tokens = (name: string) => name.toLowerCase().split(/[^a-z0-9]+/u).filter(Boolean).sort().join("\u0000");
  const target = tokens(id);
  const suggestions = [...new Set(registry.getAll()
    .filter((model) => model.id === id || tokens(model.id) === target)
    .map((model) => `"${model.provider}/${model.id}"`))];
  const hint = suggestions.length > 0 ? ` Did you mean ${suggestions.join(" or ")}?` : " Use a provider/model-id pair from the model list.";
  return `Model not found: ${value}.${hint}`;
}

function suggestQualified(bareId: string, registry: ExtensionContext["modelRegistry"]): string {
  const matches = registry.getAll().filter((model) => model.id === bareId).map((model) => `"${model.provider}/${model.id}"`);
  if (matches.length === 1) return `, e.g. ${matches[0]}`;
  if (matches.length > 1) return ` (matches: ${matches.join(", ")})`;
  return "";
}

export function resolveModel(spec: SubagentSpec, ctx: Pick<ParentExtensionContext, "model" | "modelRegistry">, inheritedThinking: ThinkingLevel): { model: NonNullable<ExtensionContext["model"]>; thinking: ThinkingLevel } {
  const thinking = spec.thinkingLevel ?? inheritedThinking;
  // Only an OMITTED model inherits; an empty string is an authoring error and
  // must fail loudly like any other invalid reference.
  if (spec.model === undefined) {
    if (!ctx.model) throw new Error("No model is active in the parent session, and the spec does not name one");
    return { model: ctx.model, thinking };
  }
  const [provider, id] = parseModel(spec.model, ctx.modelRegistry);
  const model = ctx.modelRegistry.find(provider, id);
  if (!model) throw new Error(unknownModelError(spec.model, ctx.modelRegistry));
  return { model, thinking };
}

/**
 * The prospective extension environment for a child cwd: what a child pi
 * process discovers there. Workflow call fingerprints record its statically
 * declared tool names, so replay decisions compare the same scan resume will
 * recompute. Deliberately unfiltered: subprocess children load every
 * discovered extension (the recursion guard excludes tool names per child,
 * not extensions), so a filtered scan would describe an environment no child
 * actually has.
 */
export interface ChildExtensionEnvironment {
  /** Sorted, deduped tool names declared by the extensions a child discovers. */
  extensionTools: string[];
}

export async function loadChildExtensionEnvironment(cwd: string): Promise<ChildExtensionEnvironment> {
  // Real settings, not inMemory(): installed packages (web access, MCP, ...)
  // are resolved from the settings manager's package list, and an empty
  // in-memory settings object would silently strip every installed extension
  // from children - on a minimal-base pi that leaves them with builtins only.
  const settingsManager = SettingsManager.create(cwd, getAgentDir());
  const extensionTools = new Set<string>();
  const resourceLoader = new DefaultResourceLoader({
    cwd, agentDir: getAgentDir(), settingsManager,
    extensionsOverride: (base) => {
      for (const extension of base.extensions) {
        for (const name of extension.tools.keys()) extensionTools.add(name);
      }
      return base;
    },
  });
  await resourceLoader.reload();
  return { extensionTools: [...extensionTools].sort() };
}

/**
 * The explicitly requested tool names that did not resolve to an active tool.
 * Deduplicated; empty when the request is satisfied (or there was none).
 */
export function missingExplicitTools(requested: readonly string[] | undefined, active: readonly string[]): string[] {
  if (!requested || requested.length === 0) return [];
  const available = new Set(active);
  return [...new Set(requested)].filter((name) => !available.has(name));
}
