import { existsSync, readFileSync, readdirSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { getAgentDir, type ExtensionAPI, type AgentToolResult, type ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type, type Static } from "typebox";
import {
  PublicSubagentOptionFields,
  SubagentPromptSchema,
  assertSchemaValue,
} from "../subagent-spec.js";
import type { SubagentHandle, SubagentResult, SubagentSpec, SubagentStatus, ThinkingLevel } from "../types.js";
import { resolveModel, submittedSpec, unknownModelError, type ChildSpawnSpec, type ParentContext, type ResolvedFollowUpSpec } from "../runner/child.js";
import { runOwnerIsLive } from "../store/lease.js";
import {
  DELIVERY_PROTOCOL_VERSION,
  queueAcknowledgedDelivery,
  writeDeliveryMarker,
  type RunDeliveryIdentity,
} from "../store/delivery-marker.js";
import { encodeCwd, sumUsage } from "../store/run-store.js";
import { jsonObject, readRunSnapshot, type RunSnapshot } from "../store/run-snapshot.js";
import { hasSessionClosedMarker } from "../store/session-closed-marker.js";
import { subagentRunner, type SubagentRunner } from "../runner/runner.js";
import { initialDetails, renderCallHeader, renderSubagentResult, type SubagentDetails } from "../ui/tool-render.js";
import { appendEntrySafely } from "../ui/entry-markers.js";
import { buildDeliveryEnvelope } from "../ui/delivery-envelope.js";
import { chunkDeliveryText, formatFailureText, safeDeliveryValue, stringifyDeliveryJson } from "../ui/delivery-safe.js";
import type { SubagentStatusWidget } from "../ui/status-widget.js";
import { reportDiagnostic } from "../diagnostics.js";
import { bindAbort, childLabel, errorMessage } from "../util.js";

export const SubagentToolParameters = Type.Object({
  prompt: Type.Optional(SubagentPromptSchema),
  ...PublicSubagentOptionFields,
  followUp: Type.Optional(Type.Object({
    id: Type.String({ minLength: 1 }),
    prompt: Type.String({ minLength: 1 }),
  }, { additionalProperties: false })),
}, { additionalProperties: false });
export type SubagentToolInput = Static<typeof SubagentToolParameters>;

const OPTION_FIELDS = Object.keys(PublicSubagentOptionFields) as Array<keyof typeof PublicSubagentOptionFields>;

export type ValidatedSubagentInput =
  | { type: "spawn"; spec: SubagentSpec }
  | { type: "followUp"; id: string; prompt: string; label?: string };

/**
 * Top-level fields a follow-up may not set.
 *
 * A fork resumes the source child's persisted session, so its execution
 * configuration has to come from that child - re-specifying any of it would
 * describe a different agent than the conversation being continued. `label` is
 * the exception: it is presentation only, and a follow-up turn usually has a
 * different purpose than the child it forks from, so it is allowed to say so.
 */
const FOLLOW_UP_CONFIG_FIELDS = OPTION_FIELDS.filter((field) => field !== "label");

export function validateSubagentInput(params: SubagentToolInput): ValidatedSubagentInput {
  assertSchemaValue(SubagentToolParameters, params, "subagent input");
  const prompt = params.prompt !== undefined;
  const followUp = params.followUp !== undefined;
  if (prompt === followUp) throw new Error("Provide exactly one of prompt or followUp");
  if (followUp) {
    const stray = FOLLOW_UP_CONFIG_FIELDS.find((field) => params[field] !== undefined);
    if (stray) {
      throw new Error(`With followUp, ${stray} is invalid at the top level: a follow-up resumes the original child's session and inherits its configuration. Only label may be set, to name the new turn.`);
    }
    if (!params.followUp!.prompt.trim()) throw new Error("Subagent prompt must not be empty");
    return { type: "followUp", id: params.followUp!.id, prompt: params.followUp!.prompt, label: params.label };
  }
  // TypeBox's minLength does not catch whitespace-only prompts.
  if (!params.prompt!.trim()) throw new Error("Subagent prompt must not be empty");
  return {
    type: "spawn",
    spec: {
      prompt: params.prompt!, model: params.model, thinkingLevel: params.thinkingLevel as ThinkingLevel | undefined,
      tools: params.tools, excludeTools: params.excludeTools, schema: params.schema, cwd: params.cwd,
      label: params.label, isolation: params.isolation,
    },
  };
}

interface FollowUpCandidate {
  runId: string;
  runDir: string;
  childId: string;
  createdAt: number;
  child: Record<string, unknown>;
  status: unknown;
  generationPending: boolean;
  requiresSessionClosedMarker: boolean;
}

interface FollowUpPersistenceReads {
  readRecord(runDir: string): unknown;
  readSnapshot(runDir: string): RunSnapshot;
}

const FOLLOW_UP_PERSISTENCE_READS: FollowUpPersistenceReads = {
  readRecord: (runDir) => {
    try {
      return JSON.parse(readFileSync(join(runDir, "run.json"), "utf8"));
    } catch {
      return undefined;
    }
  },
  readSnapshot: readRunSnapshot,
};

export function resolveFollowUpSpec(
  id: string,
  prompt: string,
  cwd: string,
  runsRoot: string = join(getAgentDir(), "subagent-workflow", "runs"),
  reads: FollowUpPersistenceReads = FOLLOW_UP_PERSISTENCE_READS,
): ResolvedFollowUpSpec {
  const qualified = parseQualifiedFollowUpId(id);
  const runRoot = join(runsRoot, encodeCwd(cwd));
  const candidates: FollowUpCandidate[] = [];
  let entries: string[];
  if (qualified) {
    entries = [qualified.runId];
  } else {
    try {
      entries = readdirSync(runRoot, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => entry.name);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") entries = [];
      else throw error;
    }
    entries = entries.filter((runId) => recordHasChild(reads.readRecord(join(runRoot, runId)), id));
  }
  for (const runId of entries) {
    const runDir = join(runRoot, runId);
    const snapshot = reads.readSnapshot(runDir);
    const record = jsonObject(snapshot.record);
    if (!record || !Array.isArray(record.children)) continue;
    const createdAt = typeof record.createdAt === "string" ? Date.parse(record.createdAt) : NaN;
    const statusChildren = jsonObject(jsonObject(snapshot.status)?.children);
    for (const value of record.children) {
      const child = jsonObject(value);
      const childId = typeof child?.id === "string" ? child.id : undefined;
      if (!childId || childId !== (qualified?.childId ?? id)) continue;
      let status = jsonObject(statusChildren?.[childId])?.status;
      if (isLiveStatus(status) && !runOwnerIsLive(runDir)) {
        status = terminalStatusFromEvents(snapshot.events, childId) ?? "aborted";
      }
      candidates.push({
        runId,
        runDir,
        childId,
        createdAt: Number.isFinite(createdAt) ? createdAt : 0,
        child: child as Record<string, unknown>,
        status,
        generationPending: snapshot.generationPending,
        requiresSessionClosedMarker: record.v === 3,
      });
    }
  }
  candidates.sort((left, right) => right.createdAt - left.createdAt || right.runId.localeCompare(left.runId));
  if (candidates.length === 0) {
    throw new Error(`No child ${JSON.stringify(id)} was found in persisted runs for ${cwd}`);
  }
  if (!qualified && candidates.length > 1) {
    throw new Error(`Child id ${JSON.stringify(id)} is ambiguous; use one of: ${candidates.map(candidateName).join(", ")}`);
  }
  const candidate = candidates[0]!;
  if (candidate.generationPending) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: source run is quarantined by generation.pending`);
  }
  if (!isTerminalStatus(candidate.status)) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: child is not terminal (status: ${String(candidate.status ?? "missing")})`);
  }
  const submitted = jsonObject(candidate.child.spec);
  const resolved = jsonObject(candidate.child.resolved);
  if (!submitted || !resolved) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted child spec or resolved configuration is missing`);
  }
  if (submitted.isolation === "worktree" || typeof resolved.worktreePath === "string") {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: worktree-origin children cannot be continued because their conversation references a checkout that no longer exists`);
  }
  const sessionFile = candidate.child.sessionFile;
  if (typeof sessionFile !== "string" || !sessionFile) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted sessionFile is missing`);
  }
  if (!isAbsolute(sessionFile)) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted sessionFile must be an absolute path`);
  }
  if (!existsSync(sessionFile)) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted sessionFile is missing`);
  }
  if (candidate.requiresSessionClosedMarker && !hasSessionClosedMarker(candidate.runDir, candidate.childId)) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: source session closure is not confirmed; wait for child shutdown to finish and retry`);
  }
  const provider = requiredString(resolved.provider, candidate, "resolved provider");
  const modelId = requiredString(resolved.modelId, candidate, "resolved modelId");
  const thinkingLevel = resolved.thinkingLevel;
  if (!isThinkingLevel(thinkingLevel)) {
    throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted thinkingLevel is invalid`);
  }
  const inheritedTools = optionalStringArray(submitted.tools, candidate, "tools");
  const tools = submitted.schema === undefined
    ? inheritedTools
    : inheritedTools?.filter((tool) => tool !== "report_result");
  const excludeTools = optionalStringArray(submitted.excludeTools, candidate, "excludeTools");
  const submittedCwd = optionalString(submitted.cwd, candidate, "cwd");
  const label = optionalString(submitted.label, candidate, "label");
  return {
    spec: {
      prompt,
      model: `${provider}/${modelId}`,
      thinkingLevel,
      ...(tools === undefined ? {} : { tools }),
      ...(excludeTools === undefined ? {} : { excludeTools }),
      ...(submittedCwd === undefined ? {} : { cwd: submittedCwd }),
      ...(label === undefined ? {} : { label }),
    },
    forkSessionFile: sessionFile,
    followUpOf: { runId: candidate.runId, childId: candidate.childId },
  };
}

function parseQualifiedFollowUpId(id: string): { runId: string; childId: string } | undefined {
  const parts = id.split("/");
  return parts.length === 2 && parts[0] && parts[1] ? { runId: parts[0], childId: parts[1] } : undefined;
}

function recordHasChild(value: unknown, childId: string): boolean {
  const children = jsonObject(value)?.children;
  return Array.isArray(children)
    && children.some((child) => jsonObject(child)?.id === childId);
}

function candidateName(candidate: Pick<FollowUpCandidate, "runId" | "childId">): string {
  return `${candidate.runId}/${candidate.childId}`;
}

function isLiveStatus(value: unknown): value is Extract<SubagentStatus, "pending" | "running"> {
  return value === "pending" || value === "running";
}

function isTerminalStatus(value: unknown): value is Extract<SubagentStatus, "completed" | "failed" | "aborted"> {
  return value === "completed" || value === "failed" || value === "aborted";
}

function terminalStatusFromEvents(events: readonly unknown[], childId: string): Extract<SubagentStatus, "completed" | "failed" | "aborted"> | undefined {
  let terminal: Extract<SubagentStatus, "completed" | "failed" | "aborted"> | undefined;
  for (const value of events) {
    const event = jsonObject(value);
    if (event?.id !== childId) continue;
    const result = event.type === "result" ? jsonObject(event.result) : undefined;
    const status = event.type === "status" ? event.status : result?.status;
    if (isTerminalStatus(status)) terminal = status;
  }
  return terminal;
}

function isThinkingLevel(value: unknown): value is ThinkingLevel {
  return value === "off" || value === "minimal" || value === "low" || value === "medium"
    || value === "high" || value === "xhigh" || value === "max";
}

function requiredString(value: unknown, candidate: FollowUpCandidate, field: string): string {
  if (typeof value === "string" && value.length > 0) return value;
  throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted ${field} is invalid`);
}

function optionalString(value: unknown, candidate: FollowUpCandidate, field: string): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value === "string") return value;
  throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted ${field} is invalid`);
}

function optionalStringArray(value: unknown, candidate: FollowUpCandidate, field: string): string[] | undefined {
  if (value === undefined) return undefined;
  if (Array.isArray(value) && value.every((item) => typeof item === "string")) return [...value];
  throw new Error(`Cannot follow up ${candidateName(candidate)}: persisted ${field} is invalid`);
}

type Detailed = AgentToolResult<SubagentDetails | undefined>;
type FollowUpResolver = (id: string, prompt: string, cwd: string) => ResolvedFollowUpSpec;

/**
 * Retitle a forked child when the caller named the new turn.
 *
 * Without this the fork keeps the source child's label, so a follow-up chain
 * reads as three copies of whatever the first child was for. The label is what
 * shows in the tool row, the status widget, and /agents, so it should describe
 * the turn being run. A blank label is ignored rather than blanking the row.
 */
export function withFollowUpLabel(resolved: ResolvedFollowUpSpec, label: string | undefined): ResolvedFollowUpSpec {
  return label === undefined || label.trim() === ""
    ? resolved
    : { ...resolved, spec: { ...resolved.spec, label } };
}

export function registerSubagentTool(pi: ExtensionAPI, selfPath: string, widget?: SubagentStatusWidget,
  runner: SubagentRunner = subagentRunner, resolveFollowUp: FollowUpResolver = resolveFollowUpSpec): void {
  const tool: ToolDefinition<typeof SubagentToolParameters, SubagentDetails | undefined> = {
    name: "subagent", label: "Subagent", parameters: SubagentToolParameters,
    description: "Spawn one ad-hoc child. Each child starts cold with only its self-contained prompt and inherits the parent's provider/model and thinking level unless overridden ('provider/model-id', never a bare model name). Add schema (JSON Schema) for validated structured output. Use isolation: 'worktree' for parallel edits; changes return as a patch, never applied automatically. For several independent children, call this tool several times in the same turn - up to about eight; beyond that, or when results must feed later spawns, or you need phases, pipelines, or resumable control flow, use workflow instead. The global semaphore paces all spawns, so never batch to control concurrency. Every run is background: the call returns as soon as the child starts and its result arrives later as a steered message, so do not wait or poll - end the turn and continue when the message arrives. (In a host with no interactive UI the call instead blocks and returns the result inline.) followUp: { id, prompt } forks a completed child's persisted session into a new child and run; it inherits that child's model, thinking level, tools, schema, cwd, and isolation, so none of those may be set at the top level - label is the one exception and should name the new turn. Compose each child for the task at hand; recurring task shapes belong in skills, not fixed agent personas.",
    async execute(_toolCallId, params, signal, _onUpdate, ctx): Promise<Detailed> {
      let input: ValidatedSubagentInput;
      try { input = validateSubagentInput(params); } catch (error) { throw new Error(errorMessage(error)); }
      const spawnSpec: ChildSpawnSpec = input.type === "followUp"
        ? withFollowUpLabel(resolveFollowUp(input.id, input.prompt, ctx.cwd), input.label)
        : input.spec;
      const spec = submittedSpec(spawnSpec);
      // A child doomed by an unknown model fails fast with a suggestion instead
      // of spawning.
      const modelProblem = spec.model === undefined ? undefined : unknownModelError(spec.model, ctx.modelRegistry);
      if (modelProblem) throw new Error(modelProblem);
      const parent: ParentContext = { ctx, thinkingLevel: pi.getThinkingLevel() as ThinkingLevel, selfPath };
      // The receipt's model cell cannot come from the handle - handle.resolved
      // is filled in asynchronously after admission - so resolve it here. This
      // duplicates the child's own resolution but is pure and cheap; a spec
      // that resolves nothing (no parent model) just leaves the cell empty
      // rather than failing an otherwise-valid spawn.
      let display: { modelId?: string; thinking?: ThinkingLevel } = {};
      try {
        const resolved = resolveModel(spec, ctx, parent.thinkingLevel);
        display = { modelId: resolved.model.id, thinking: resolved.thinking };
      } catch { /* the child will fail (or not) on its own terms */ }
      const handle = runner.spawnRun(spawnSpec, parent);
      const { runId, runDir } = handle;
      appendEntrySafely(pi, "subagent-workflow:run-started", {
        runId,
        runDir,
        childIds: [handle.id],
        labels: [spec.label ?? childLabel(spec)],
      });
      try {
        widget?.track(runId, handle, ctx);
      } catch (error) {
        // The status widget is observational. Delivery and runner cleanup must
        // remain wired even if a host UI implementation rejects the widget.
        reportDiagnostic(`[subagent-workflow] status widget failed: ${errorMessage(error)}`);
      }
      const sessionId = ctx.sessionManager.getSessionId();
      if (!ctx.hasUI) {
        // Headless hosts (print/json mode) end the session when this turn
        // ends, which disposes the child - there is no later turn to steer a
        // result into. Waiting inline is the only way the work can complete.
        // Only this waiting call binds the turn's abort signal.
        const unbindAbort = bindAbort(signal, () => { void handle.abort(); });
        try {
          const result = await handle.result;
          return fenceDirectlyDeliveredRun(pi, runner, handle, result, sessionId, (degraded) =>
            detailedResult(formatDelivery(runId, runDir, result, degraded), undefined));
        } finally {
          unbindAbort();
        }
      }
      // Background children deliberately outlive the spawning turn, so nothing
      // binds this turn's abort signal: a later Esc must not kill promised
      // work. Stop a run from /agents instead.
      void handle.result.then((result) => {
        queueCompletedRun(pi, runner, handle, result, sessionId);
      }).catch((error) => {
        reportDiagnostic(`[subagent-workflow] background delivery failed: ${errorMessage(error)}`);
      });
      return detailedResult(
        JSON.stringify({ id: handle.id, runId, runDir, status: handle.status, label: spec.label }),
        initialDetails(spec, handle, display),
      );
    },
    renderCall(args, theme) {
      // For a follow-up, prefer the caller's name for the turn; the source child
      // id is the fallback and stays visible in the run record either way.
      const label = args.followUp
        ? `follow-up · ${args.label ?? args.followUp.id}`
        : args.label ?? (args.prompt ? childLabel({ prompt: args.prompt }) : "Subagent");
      return renderCallHeader(label, theme);
    },
    renderResult(result, _options, theme, context) {
      return renderSubagentResult(result, theme, context.lastComponent);
    },
  };
  pi.registerTool(tool);
}

function detailedResult(text: string, details: SubagentDetails | undefined): Detailed {
  return { content: [{ type: "text", text }], details };
}

export function formatDelivery(runId: string, runDir: string, result: SubagentResult, degraded?: string): string {
  const safeRunId = safeDeliveryValue(runId);
  const safeRunDir = safeDeliveryValue(runDir);
  const delivered = {
    ...result,
    id: safeDeliveryValue(result.id),
    ...(result.sessionFile === undefined ? {} : { sessionFile: safeDeliveryValue(result.sessionFile) }),
    resolved: { ...result.resolved, label: safeDeliveryValue(result.resolved.label) },
    ...(result.error === undefined ? {} : { error: formatFailureText(result.error) }),
  };
  const runRecord = `${safeRunDir}/run.json`;
  const eventsRecord = `${safeRunDir}/events.jsonl`;
  return buildDeliveryEnvelope({
    header: [
      `Subagent run ${safeRunId}`,
      `Run directory: ${safeRunDir}`,
      `Status: ${delivered.status}`,
      `Child ${delivered.id} (${delivered.resolved.label}): ${delivered.status}`,
    ],
    failures: delivered.status === "failed"
      ? [`Failed child ${delivered.id} (${delivered.resolved.label}): ${delivered.error ?? "Unknown error"}`]
      : delivered.status === "aborted" ? [`Aborted child ${delivered.id} (${delivered.resolved.label})`] : [],
    recovery: delivered.status === "failed" ? ["Recovery: respawn the child with the same prompt and options."] : [],
    warnings: degraded ? [`Warning: run persistence degraded (${safeDeliveryValue(degraded)}); the run directory may be incomplete`] : [],
    artifacts: [`Run record: ${runRecord}`],
    auxiliaryArtifacts: delivered.sessionFile === undefined ? [] : [`Child ${delivered.id} session: ${delivered.sessionFile}`],
    resultPreview: chunkDeliveryText(stringifyDeliveryJson({ type: "subagent_results", results: [delivered] })),
    truncationMarker: degraded
      ? `[truncated - result may be incomplete at ${eventsRecord}; run persistence degraded]`
      : `[truncated - full result remains available via ${eventsRecord}]`,
  });
}

/**
 * Complete a run whose result was delivered directly (inline tool return or
 * navigator follow-up view): record completion, then fence catch-up so the
 * result is never redelivered to the model. `deliver` runs between the two
 * so an inline delivery can still surface a degraded-persistence warning.
 */
export function fenceDirectlyDeliveredRun<T>(pi: ExtensionAPI, runner: SubagentRunner, handle: SubagentHandle,
  result: SubagentResult, sessionId: string, deliver: (degraded: string | undefined) => T): T {
  const { runId, runDir } = handle;
  recordCompletedRun(pi, handle, result);
  const identity = resultDeliveryIdentity(runId, result);
  const delivered = deliver(runner.markDelivered(runId));
  if (!writeDeliveryMarker(runDir, sessionId, identity)) {
    throw new Error(`Run ${runId} changed generation before direct delivery could be recorded`);
  }
  return delivered;
}

function queueCompletedRun(pi: ExtensionAPI, runner: SubagentRunner, handle: SubagentHandle,
  result: SubagentResult, sessionId: string): void {
  const { runId, runDir } = handle;
  recordCompletedRun(pi, handle, result);
  queueAcknowledgedDelivery(pi, {
    sessionId,
    message: formatDelivery(runId, runDir, result, runner.finalizedRunWarning(runId)),
    targets: [{ runDir, identity: resultDeliveryIdentity(runId, result) }],
  });
  runner.markDelivered(runId);
}

function recordCompletedRun(pi: ExtensionAPI, handle: SubagentHandle, result: SubagentResult): void {
  appendEntrySafely(pi, "subagent-workflow:run-completed", {
    runId: handle.runId,
    runDir: handle.runDir,
    generation: resultDeliveryIdentity(handle.runId, result).generation,
    perChild: [{ id: result.id, status: result.status, label: result.resolved.label }],
    usageTotals: sumUsage([result.usage]),
    durationMs: Date.now() - handle.startedAt,
  });
}

function resultDeliveryIdentity(runId: string, result: SubagentResult): RunDeliveryIdentity {
  const generation = result.generation;
  if (generation === undefined || !Number.isSafeInteger(generation) || generation < 1) {
    throw new Error(`Run ${runId} completed without a valid delivery generation`);
  }
  return { protocol: DELIVERY_PROTOCOL_VERSION, generation };
}
