/**
 * Core contracts for pi-subagent-workflow.
 *
 * The subagent SPEC is the primitive of the whole system: an ad-hoc,
 * per-call description of a worker. There are no agent types, personas,
 * or registries anywhere - a worker is exactly what its spec says.
 */

import type { ModelTier } from "../../../config/index.js";

/** Thinking levels mirror pi's own vocabulary. */
export type ThinkingLevel = "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";

/** The ad-hoc subagent spec - the primitive. */
export interface SubagentSpec {
  /** Task prompt written by the orchestrator. Required. */
  prompt: string;
  /** Internal provider/model-id, resolved from a tier or a persisted follow-up. */
  model?: string;
  /** Publicly selected tier, recorded alongside the pinned model for new children. */
  modelTier?: ModelTier;
  /** Thinking level for the child. Default: inherit from parent. */
  thinkingLevel?: ThinkingLevel;
  /**
   * Tool allowlist narrowing (names). The child process builds its toolset
   * from builtins plus extensions it discovers for its own cwd, then narrows
   * it with tools/excludeTools. Explicitly requested names that do not resolve
   * fail the spawn.
   */
  tools?: string[];
  /** Tool denylist narrowing (names). */
  excludeTools?: string[];
  /**
   * JSON Schema for structured output. When present, the child receives a
   * synthetic terminating tool and must finish by calling it; the validated
   * arguments become the result value. Bounded repair turns on mismatch.
   */
  schema?: Record<string, unknown>;
  /** Working directory for the child. Default: parent cwd. */
  cwd?: string;
  /** Short display label for UI and logs. Default: derived from the prompt. */
  label?: string;
  /** Workflow phase attribution. Set by the workflow runtime. */
  phase?: string;
}

export interface WorkflowPhase {
  title: string;
  detail?: string;
}

export interface WorkflowMeta {
  name: string;
  description: string;
  phases?: WorkflowPhase[];
}

export type SubagentStatus =
  | "pending" // queued behind the concurrency semaphore
  | "running"
  | "completed"
  | "failed"
  | "aborted";

/** Terminal result of a child run. */
export interface SubagentResult {
  id: string;
  /** Delivery generation that produced this terminal result. */
  generation?: number;
  status: Extract<SubagentStatus, "completed" | "failed" | "aborted">;
  /** Persisted child session containing the full assistant transcript, when available. */
  sessionFile?: string;
  /** Final assistant text (always present on completion, even with schema). */
  text: string;
  /** Schema-validated structured value, when the spec had a schema. */
  structured?: unknown;
  /** Error description when status is "failed". */
  error?: string;
  usage: UsageSummary;
  /** The fully-resolved spec the child actually ran with (for journaling/UI). */
  resolved: ResolvedSpec;
}

/** Aggregated token/cost usage for a child run. */
export interface UsageSummary {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  cost: number;
  turns: number;
}

/** What the runner resolved the spec into (recorded per run). */
export interface FollowUpReference {
  runId: string;
  childId: string;
}

export interface ResolvedSpec {
  provider: string;
  modelId: string;
  thinkingLevel: ThinkingLevel;
  /** Active tool names the child started with. */
  tools: string[];
  cwd: string;
  label: string;
}

/** Live events emitted by a running child, for renderers and the navigator. */
export type SubagentEvent =
  | { type: "status"; id: string; status: SubagentStatus }
  | { type: "activity"; id: string; description: string } // e.g. current tool call
  | { type: "usage"; id: string; usage: UsageSummary }
  | { type: "result"; id: string; result: SubagentResult };

/** Handle to a spawned child, owned by the runner. */
export interface SubagentHandle {
  id: string;
  /** Durable run containing this child. */
  runId: string;
  runDir: string;
  generation?: number;
  spec: SubagentSpec;
  resolved: ResolvedSpec | undefined; // undefined until construction completes
  status: SubagentStatus;
  startedAt: number;
  /** Resolves with the terminal result (never rejects; failures are results). */
  result: Promise<SubagentResult>;
  /** Send a steering message to the child mid-run (buffered until ready). */
  steer(text: string): Promise<void>;
  abort(): Promise<void>;
  /** Subscribe to live events; returns unsubscribe. */
  subscribe(listener: (event: SubagentEvent) => void): () => void;
}
